import fs from 'node:fs/promises';
import { lock, unlock } from 'proper-lockfile';
import * as fsSync from 'node:fs';
import path from 'node:path';
import { z } from 'zod';
import readline from 'node:readline';
import { APP_PATHS } from '../config/index.ts';
import { CodeLoopsLogger } from '../logger.ts';

// -----------------------------------------------------------------------------
// Interfaces & Schemas --------------------------------------------------------
// -----------------------------------------------------------------------------

export interface WithProjectContext {
  project: string;
  projectContext: string;
}

export const FILE_REF = z.object({
  name: z.string(), // human label ("UML‑AuthSeq")
  uri: z.string().optional(), // optional external link or S3 key
  /** Absolute or repo‑relative path, e.g. "QuickRecorder/CameraOverlay.swift" */
  path: z.string(),
  /** Optional hash to lock content for provenance */
  hash: z.string().optional(),
  /** Optional MIME, e.g. "text/x-swift" */
  contentType: z.string().optional(),
});
export type ArtifactRef = z.infer<typeof FILE_REF>;

// Schema for validating DagNode entries
export const DagNodeSchema = z.object({
  id: z.string(),
  project: z.string(),
  projectContext: z.string(),
  thought: z.string(),
  role: z.enum(['actor', 'critic', 'summary']),
  createdAt: z.string().datetime(),
  parents: z.array(z.string()),
  children: z.array(z.string()),
  verdict: z.enum(['approved', 'needs_revision', 'reject']).optional(),
  verdictReason: z.string().optional(),
  verdictReferences: z.array(z.string()).optional(),
  target: z.string().optional(),
  summarizedSegment: z.array(z.string()).optional(),
  artifacts: z.array(FILE_REF).optional(),
  tags: z.array(z.string()).optional(),
  diff: z.string().optional(),
});

export interface DagNode extends WithProjectContext {
  id: string;
  thought: string;
  role: 'actor' | 'critic' | 'summary';
  verdict?: 'approved' | 'needs_revision' | 'reject';
  verdictReason?: string;
  verdictReferences?: string[];
  target?: string; // nodeId this criticises
  parents: string[];
  children: string[];
  createdAt: string; // ISO timestamp
  summarizedSegment?: string[]; // IDs of nodes summarized (for summary nodes)
  artifacts?: ArtifactRef[];
  tags?: string[];
  diff?: string; // The changes introduced for this step
}

export interface SummaryNode extends DagNode {
  role: 'summary';
  summarizedSegment: string[]; // IDs of nodes summarized
}

export interface DeletedNode extends DagNode {
  deletedAt: string; // ISO timestamp
  deletedReason?: string;
  deletedBy?: string;
}

export interface DependentNode {
  id: string;
  thought: string;
  role: string;
  tags?: string[];
}

// -----------------------------------------------------------------------------
// KnowledgeGraphManager -------------------------------------------------------
// -----------------------------------------------------------------------------

export class KnowledgeGraphManager {
  private logFilePath: string = path.resolve(APP_PATHS.data, 'knowledge_graph.ndjson');
  private deletedLogFilePath: string = path.resolve(
    APP_PATHS.data,
    'knowledge_graph.deleted.ndjson',
  );
  private backupDir: string = path.resolve(APP_PATHS.data, 'backup');
  private logger: CodeLoopsLogger;

  constructor(logger: CodeLoopsLogger) {
    this.logger = logger;
  }

  async init() {
    this.logger.info(`[KnowledgeGraphManager] Initializing from ${this.logFilePath}`);
    await this.loadLog();
  }

  private async loadLog() {
    if (!(await fs.stat(this.logFilePath).catch(() => null))) {
      this.logger.info(`[KnowledgeGraphManager] Creating new log file at ${this.logFilePath}`);
      await fs.mkdir(path.dirname(this.logFilePath), { recursive: true });
      await fs.writeFile(this.logFilePath, '');
      return;
    }
  }

  private parseDagNode(line: string): DagNode | null {
    try {
      const parsed = JSON.parse(line);
      const validated = DagNodeSchema.parse(parsed);
      return validated as DagNode;
    } catch (err) {
      this.logger.error({ err, line }, 'Invalid DagNode entry');
      return null;
    }
  }

  async appendEntity(entity: DagNode, retries = 3) {
    if (await this.wouldCreateCycle(entity)) {
      throw new Error(`Appending node ${entity.id} would create a cycle`);
    }

    entity.createdAt = new Date().toISOString();
    const line = JSON.stringify(entity) + '\n';
    let err: Error | null = null;

    for (let attempt = 1; attempt <= retries; attempt++) {
      try {
        await lock(this.logFilePath, { retries: 0 });
        await fs.appendFile(this.logFilePath, line, 'utf8');
        return;
      } catch (e: unknown) {
        err = e as Error;
        this.logger.warn({ err, attempt }, `Retry ${attempt} failed appending entity`);
        if (attempt === retries) break;
        await new Promise((resolve) => setTimeout(resolve, 100 * attempt));
      } finally {
        try {
          await unlock(this.logFilePath);
        } catch (unlockErr) {
          this.logger.error({ err: unlockErr }, 'Failed to unlock file');
        }
      }
    }

    this.logger.error({ err }, 'Error appending entity after retries');
    throw err;
  }

  private async wouldCreateCycle(entity: DagNode): Promise<boolean> {
    const visited = new Set<string>();
    async function dfs(id: string, manager: KnowledgeGraphManager): Promise<boolean> {
      if (visited.has(id)) return true;
      visited.add(id);
      const node = await manager.getNode(id);
      if (!node) return false;
      for (const childId of node.children) {
        if (childId === entity.id || (await dfs(childId, manager))) return true;
      }
      return false;
    }
    for (const parentId of entity.parents) {
      if (await dfs(parentId, this)) return true;
    }
    return false;
  }

  async getNode(id: string): Promise<DagNode | undefined> {
    const fileStream = fsSync.createReadStream(this.logFilePath);
    const rl = readline.createInterface({ input: fileStream, crlfDelay: Infinity });
    try {
      for await (const line of rl) {
        const entry = this.parseDagNode(line);
        if (entry?.id === id) {
          return entry;
        }
      }
      return undefined;
    } catch (err) {
      this.logger.error({ err }, 'Error getting node');
      throw err;
    } finally {
      rl.close();
      fileStream.close();
    }
  }

  async *streamDagNodes(project: string): AsyncGenerator<DagNode, void, unknown> {
    const fileStream = fsSync.createReadStream(this.logFilePath);
    const rl = readline.createInterface({ input: fileStream, crlfDelay: Infinity });
    try {
      for await (const line of rl) {
        const node = this.parseDagNode(line);
        if (node?.project === project) {
          yield node;
        }
      }
    } catch (err) {
      this.logger.error({ err }, 'Error streaming nodes');
      throw err;
    } finally {
      rl.close();
      fileStream.close();
    }
  }

  async allDagNodes(project: string): Promise<DagNode[]> {
    const nodes: DagNode[] = [];
    for await (const node of this.streamDagNodes(project)) {
      nodes.push(node);
    }
    return nodes;
  }

  async resume({
    project,
    limit = 5,
    includeDiffs = 'latest',
  }: {
    project: string;
    limit?: number;
    includeDiffs?: 'all' | 'latest' | 'none';
  }): Promise<DagNode[]> {
    const nodes = await this.export({ project, limit });

    // Handle diff inclusion based on includeDiffs parameter
    if (includeDiffs === 'none') {
      // Remove diff from all nodes
      return nodes.map((node) => ({ ...node, diff: undefined }));
    } else if (includeDiffs === 'latest') {
      // Only include diff for the most recent node (last in array)
      return nodes.map((node, index) => ({
        ...node,
        diff: index === nodes.length - 1 ? node.diff : undefined,
      }));
    }

    // includeDiffs === 'all' - return nodes as is with all diffs
    return nodes;
  }

  async export({
    project,
    filterFn,
    limit,
  }: {
    project: string;
    filterFn?: (node: DagNode) => boolean;
    limit?: number;
  }): Promise<DagNode[]> {
    const nodes: DagNode[] = [];
    const fileStream = fsSync.createReadStream(this.logFilePath);
    const rl = readline.createInterface({ input: fileStream, crlfDelay: Infinity });
    try {
      for await (const line of rl) {
        const node = this.parseDagNode(line);
        if (!node || node.project !== project) continue;
        if (filterFn && !filterFn(node)) continue;
        nodes.push(node);
        if (limit && nodes.length > limit) nodes.shift();
      }
      return nodes;
    } catch (err) {
      this.logger.error({ err }, 'Error exporting nodes');
      throw err;
    } finally {
      rl.close();
      fileStream.close();
    }
  }

  async listProjects(): Promise<string[]> {
    const projects = new Set<string>();
    const fileStream = fsSync.createReadStream(this.logFilePath);
    const rl = readline.createInterface({ input: fileStream, crlfDelay: Infinity });
    try {
      for await (const line of rl) {
        const entry = this.parseDagNode(line);
        if (entry?.project && !projects.has(entry.project)) {
          projects.add(entry.project);
        }
      }
      return Array.from(projects);
    } catch (err) {
      this.logger.error({ err }, 'Error listing projects');
      throw err;
    } finally {
      rl.close();
      fileStream.close();
    }
  }

  // -----------------------------------------------------------------------------
  // Soft Delete Methods ---------------------------------------------------------
  // -----------------------------------------------------------------------------

  async findDependentNodes(
    nodeIds: string[],
    project: string,
  ): Promise<Map<string, DependentNode[]>> {
    const dependentsMap = new Map<string, DependentNode[]>();
    nodeIds.forEach((id) => dependentsMap.set(id, []));

    for await (const node of this.streamDagNodes(project)) {
      for (const nodeId of nodeIds) {
        if (node.parents.includes(nodeId)) {
          const dependents = dependentsMap.get(nodeId) || [];
          dependents.push({
            id: node.id,
            thought: node.thought,
            role: node.role,
            tags: node.tags,
          });
          dependentsMap.set(nodeId, dependents);
        }
      }
    }

    return dependentsMap;
  }

  async findAffectedSummaryNodes(nodeIds: string[], project: string): Promise<SummaryNode[]> {
    const affectedSummaries: SummaryNode[] = [];

    for await (const node of this.streamDagNodes(project)) {
      if (node.role === 'summary' && node.summarizedSegment) {
        const hasDeletedNode = node.summarizedSegment.some((id) => nodeIds.includes(id));
        if (hasDeletedNode) {
          affectedSummaries.push(node as SummaryNode);
        }
      }
    }

    return affectedSummaries;
  }

  private async createBackup(): Promise<string> {
    await fs.mkdir(this.backupDir, { recursive: true });
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const backupPath = path.join(this.backupDir, `knowledge_graph_${timestamp}.ndjson`);

    await fs.copyFile(this.logFilePath, backupPath);
    this.logger.info(`[KnowledgeGraphManager] Created backup at ${backupPath}`);

    return backupPath;
  }

  private async appendToDeletedLog(nodes: DeletedNode[]): Promise<void> {
    const lines = nodes.map((node) => JSON.stringify(node) + '\n').join('');
    await fs.appendFile(this.deletedLogFilePath, lines, 'utf8');
  }

  async softDeleteNodes(
    nodeIds: string[],
    project: string,
    reason?: string,
    deletedBy?: string,
  ): Promise<{
    deletedNodes: DeletedNode[];
    backupPath: string;
    affectedSummaries: SummaryNode[];
  }> {
    // Create backup first
    const backupPath = await this.createBackup();

    // Find nodes to delete
    const nodesToDelete: DagNode[] = [];
    const remainingNodes: DagNode[] = [];

    for await (const node of this.streamDagNodes(project)) {
      if (nodeIds.includes(node.id)) {
        nodesToDelete.push(node);
      } else {
        remainingNodes.push(node);
      }
    }

    // Convert to deleted nodes
    const deletedNodes: DeletedNode[] = nodesToDelete.map((node) => ({
      ...node,
      deletedAt: new Date().toISOString(),
      deletedReason: reason,
      deletedBy: deletedBy,
    }));

    // Append to deleted log
    await this.appendToDeletedLog(deletedNodes);

    // Find affected summary nodes before rebuilding
    const affectedSummaries = await this.findAffectedSummaryNodes(nodeIds, project);

    // Rebuild the main graph without deleted nodes
    await this.rebuildGraphWithoutDeleted(project, nodeIds);

    this.logger.info(
      `[KnowledgeGraphManager] Soft deleted ${deletedNodes.length} nodes from project ${project}`,
    );

    return {
      deletedNodes,
      backupPath,
      affectedSummaries,
    };
  }

  private async rebuildGraphWithoutDeleted(
    project: string,
    deletedNodeIds: string[],
  ): Promise<void> {
    // Read all nodes from all projects
    const allNodes: DagNode[] = [];
    const fileStream = fsSync.createReadStream(this.logFilePath);
    const rl = readline.createInterface({ input: fileStream, crlfDelay: Infinity });

    try {
      for await (const line of rl) {
        const node = this.parseDagNode(line);
        if (node && !deletedNodeIds.includes(node.id)) {
          // Update parent/child references to exclude deleted nodes
          node.parents = node.parents.filter((id) => !deletedNodeIds.includes(id));
          node.children = node.children.filter((id) => !deletedNodeIds.includes(id));
          allNodes.push(node);
        }
      }
    } catch (err) {
      this.logger.error({ err }, 'Error rebuilding graph without deleted nodes');
      throw err;
    } finally {
      rl.close();
      fileStream.close();
    }

    // Write back all non-deleted nodes
    const tempPath = `${this.logFilePath}.tmp`;
    const lines = allNodes.map((node) => JSON.stringify(node) + '\n').join('');
    await fs.writeFile(tempPath, lines, 'utf8');

    // Atomic replace
    await fs.rename(tempPath, this.logFilePath);
  }
}
