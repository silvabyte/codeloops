import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { KnowledgeGraphManager, DagNode } from './KnowledgeGraph.js';
import fs from 'node:fs/promises';
import path from 'node:path';
import { v4 as uuid } from 'uuid';
import os from 'node:os';
import { createLogger, setGlobalLogger, getInstance as getLogger } from '../logger.js';

const logger = createLogger({ withFile: false, withDevStdout: true });
setGlobalLogger(logger);

describe('KnowledgeGraphManager - Soft Delete Functions', () => {
  let kg: KnowledgeGraphManager;
  let testDataDir: string;
  let logFilePath: string;
  let deletedLogFilePath: string;
  let backupDir: string;

  beforeEach(async () => {
    // Create a unique test directory
    testDataDir = path.join(os.tmpdir(), `kg-delete-test-${uuid()}`);
    await fs.mkdir(testDataDir, { recursive: true });
    logFilePath = path.join(testDataDir, 'knowledge_graph.ndjson');
    deletedLogFilePath = path.join(testDataDir, 'knowledge_graph.deleted.ndjson');
    backupDir = path.join(testDataDir, 'backup');

    // Create a KnowledgeGraphManager instance with custom paths
    kg = new KnowledgeGraphManager(getLogger());
    // @ts-expect-error - Accessing private properties for testing
    kg.logFilePath = logFilePath;
    // @ts-expect-error - Accessing private properties for testing
    kg.deletedLogFilePath = deletedLogFilePath;
    // @ts-expect-error - Accessing private properties for testing
    kg.backupDir = backupDir;
    await kg.init();
  });

  afterEach(async () => {
    try {
      await fs.rm(testDataDir, { recursive: true, force: true });
    } catch (error) {
      console.error('Error cleaning up test directory:', error);
    }
  });

  // Helper function to create a test node
  const createTestNode = (
    project: string,
    role: 'actor' | 'critic' | 'summary' = 'actor',
    parents: string[] = [],
    summarizedSegment?: string[],
  ): DagNode => ({
    id: uuid(),
    project,
    projectContext: `/path/to/${project}`,
    thought: `Test thought for ${project}`,
    role,
    parents,
    children: [],
    createdAt: '',
    tags: ['test-tag'],
    artifacts: [],
    ...(role === 'summary' && summarizedSegment ? { summarizedSegment } : {}),
  });

  describe('findDependentNodes', () => {
    it('should find nodes that depend on target nodes', async () => {
      const project = 'test-project';

      // Create a chain: nodeA -> nodeB -> nodeC
      const nodeA = createTestNode(project);
      const nodeB = createTestNode(project, 'actor', [nodeA.id]);
      const nodeC = createTestNode(project, 'actor', [nodeB.id]);

      await kg.appendEntity(nodeA);
      await kg.appendEntity(nodeB);
      await kg.appendEntity(nodeC);

      // Find dependents of nodeA
      const dependentsMap = await kg.findDependentNodes([nodeA.id], project);

      expect(dependentsMap.has(nodeA.id)).toBe(true);
      const dependents = dependentsMap.get(nodeA.id)!;
      expect(dependents).toHaveLength(1);
      expect(dependents[0].id).toBe(nodeB.id);
    });

    it('should return empty dependents for nodes with no children', async () => {
      const project = 'test-project';
      const nodeA = createTestNode(project);
      await kg.appendEntity(nodeA);

      const dependentsMap = await kg.findDependentNodes([nodeA.id], project);

      expect(dependentsMap.has(nodeA.id)).toBe(true);
      expect(dependentsMap.get(nodeA.id)).toHaveLength(0);
    });
  });

  describe('findAffectedSummaryNodes', () => {
    it('should find summary nodes that reference deleted nodes', async () => {
      const project = 'test-project';

      // Create regular nodes
      const nodeA = createTestNode(project);
      const nodeB = createTestNode(project);
      await kg.appendEntity(nodeA);
      await kg.appendEntity(nodeB);

      // Create summary node that references both
      const summaryNode = createTestNode(project, 'summary', [], [nodeA.id, nodeB.id]);
      await kg.appendEntity(summaryNode);

      // Find affected summaries when deleting nodeA
      const affectedSummaries = await kg.findAffectedSummaryNodes([nodeA.id], project);

      expect(affectedSummaries).toHaveLength(1);
      expect(affectedSummaries[0].id).toBe(summaryNode.id);
    });

    it('should return empty array when no summaries are affected', async () => {
      const project = 'test-project';
      const nodeA = createTestNode(project);
      await kg.appendEntity(nodeA);

      const affectedSummaries = await kg.findAffectedSummaryNodes([nodeA.id], project);

      expect(affectedSummaries).toHaveLength(0);
    });
  });

  describe('softDeleteNodes', () => {
    it('should successfully soft delete nodes', async () => {
      const project = 'test-project';

      // Create test nodes
      const nodeA = createTestNode(project);
      const nodeB = createTestNode(project);
      await kg.appendEntity(nodeA);
      await kg.appendEntity(nodeB);

      // Soft delete nodeA
      const result = await kg.softDeleteNodes([nodeA.id], project, 'test deletion', 'test-user');

      expect(result.deletedNodes).toHaveLength(1);
      expect(result.deletedNodes[0].id).toBe(nodeA.id);
      expect(result.deletedNodes[0].deletedReason).toBe('test deletion');
      expect(result.deletedNodes[0].deletedBy).toBe('test-user');
      expect(result.deletedNodes[0].deletedAt).toBeDefined();
      expect(result.backupPath).toContain('backup');
    });

    it('should create backup before deletion', async () => {
      const project = 'test-project';
      const nodeA = createTestNode(project);
      await kg.appendEntity(nodeA);

      const result = await kg.softDeleteNodes([nodeA.id], project);

      // Check backup was created
      expect(result.backupPath).toBeDefined();
      const backupExists = await fs.access(result.backupPath).then(
        () => true,
        () => false,
      );
      expect(backupExists).toBe(true);
    });

    it('should write deleted nodes to deleted log', async () => {
      const project = 'test-project';
      const nodeA = createTestNode(project);
      await kg.appendEntity(nodeA);

      await kg.softDeleteNodes([nodeA.id], project, 'test deletion');

      // Check deleted log was created and contains our node
      const deletedLogExists = await fs.access(deletedLogFilePath).then(
        () => true,
        () => false,
      );
      expect(deletedLogExists).toBe(true);

      const deletedContent = await fs.readFile(deletedLogFilePath, 'utf-8');
      expect(deletedContent).toContain(nodeA.id);
      expect(deletedContent).toContain('test deletion');
    });

    it('should rebuild main graph without deleted nodes', async () => {
      const project = 'test-project';

      // Create nodes
      const nodeA = createTestNode(project);
      const nodeB = createTestNode(project);
      const nodeC = createTestNode(project, 'actor', [nodeA.id]); // nodeC depends on nodeA

      await kg.appendEntity(nodeA);
      await kg.appendEntity(nodeB);
      await kg.appendEntity(nodeC);

      // Delete nodeA
      await kg.softDeleteNodes([nodeA.id], project);

      // Check remaining nodes
      const remainingNodes = await kg.allDagNodes(project);
      expect(remainingNodes).toHaveLength(2);
      expect(remainingNodes.find((n) => n.id === nodeA.id)).toBeUndefined();
      expect(remainingNodes.find((n) => n.id === nodeB.id)).toBeDefined();
      expect(remainingNodes.find((n) => n.id === nodeC.id)).toBeDefined();

      // Check that nodeC no longer has nodeA as parent
      const updatedNodeC = remainingNodes.find((n) => n.id === nodeC.id);
      expect(updatedNodeC?.parents).not.toContain(nodeA.id);
    });

    it('should handle multiple nodes deletion', async () => {
      const project = 'test-project';

      // Create nodes
      const nodeA = createTestNode(project);
      const nodeB = createTestNode(project);
      const nodeC = createTestNode(project);

      await kg.appendEntity(nodeA);
      await kg.appendEntity(nodeB);
      await kg.appendEntity(nodeC);

      // Delete multiple nodes
      const result = await kg.softDeleteNodes([nodeA.id, nodeB.id], project, 'bulk delete');

      expect(result.deletedNodes).toHaveLength(2);
      const remainingNodes = await kg.allDagNodes(project);
      expect(remainingNodes).toHaveLength(1);
      expect(remainingNodes[0].id).toBe(nodeC.id);
    });

    it('should preserve nodes from other projects', async () => {
      const project1 = 'project-1';
      const project2 = 'project-2';

      // Create nodes in different projects
      const nodeA = createTestNode(project1);
      const nodeB = createTestNode(project2);

      await kg.appendEntity(nodeA);
      await kg.appendEntity(nodeB);

      // Delete node from project1
      await kg.softDeleteNodes([nodeA.id], project1);

      // Check project2 node is preserved
      const project2Nodes = await kg.allDagNodes(project2);
      expect(project2Nodes).toHaveLength(1);
      expect(project2Nodes[0].id).toBe(nodeB.id);

      // Check project1 node is gone
      const project1Nodes = await kg.allDagNodes(project1);
      expect(project1Nodes).toHaveLength(0);
    });

    it('should find affected summary nodes correctly', async () => {
      const project = 'test-project';

      // Create regular nodes
      const nodeA = createTestNode(project);
      const nodeB = createTestNode(project);
      await kg.appendEntity(nodeA);
      await kg.appendEntity(nodeB);

      // Create summary node
      const summaryNode = createTestNode(project, 'summary', [], [nodeA.id, nodeB.id]);
      await kg.appendEntity(summaryNode);

      // Delete nodeA
      const result = await kg.softDeleteNodes([nodeA.id], project);

      expect(result.affectedSummaries).toHaveLength(1);
      expect(result.affectedSummaries[0].id).toBe(summaryNode.id);
    });
  });

  describe('edge cases', () => {
    it('should handle deletion of non-existent nodes gracefully', async () => {
      const project = 'test-project';
      const nonExistentId = uuid();

      // This should not throw, just return empty results
      const result = await kg.softDeleteNodes([nonExistentId], project);

      expect(result.deletedNodes).toHaveLength(0);
      expect(result.backupPath).toBeDefined(); // Backup should still be created
    });

    it('should handle empty node IDs array', async () => {
      const project = 'test-project';

      const result = await kg.softDeleteNodes([], project);

      expect(result.deletedNodes).toHaveLength(0);
      expect(result.backupPath).toBeDefined();
    });

    it('should handle deletion when graph is empty', async () => {
      const project = 'test-project';
      const nodeId = uuid();

      const result = await kg.softDeleteNodes([nodeId], project);

      expect(result.deletedNodes).toHaveLength(0);
      expect(result.backupPath).toBeDefined();
    });
  });
});
