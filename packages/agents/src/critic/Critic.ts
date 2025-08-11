import { v4 as uuid } from 'uuid';
import {
  type KnowledgeGraphManager,
  type DagNode,
  getInstance as getLogger,
} from '@codeloops/core';
import { CriticAgent } from './CriticAgent.js';
import { z } from 'zod';

export const CriticSchema = {
  actorNodeId: z.string().describe('ID of the actor node to critique.'),
};

export class Critic {
  private readonly criticAgent: CriticAgent;

  constructor(private readonly kg: KnowledgeGraphManager) {
    this.criticAgent = new CriticAgent({ logger: getLogger() });
  }

  async review({
    actorNodeId,
    project,
    projectContext,
  }: {
    actorNodeId: string;
    project: string;
    projectContext: string;
  }): Promise<DagNode> {
    const target = await this.kg.getNode(actorNodeId);
    if (!target || (target as DagNode).role !== 'actor') {
      throw new Error('invalid target for critic');
    }

    let verdict: DagNode['verdict'] = 'approved';
    let reason: DagNode['verdictReason'] | undefined;

    if ((target as DagNode).thought.trim() === '') {
      verdict = 'needs_revision';
    }

    if (verdict === 'approved') {
      try {
        const criticResponse = await this.criticAgent.reviewActorNode(target as DagNode);
        verdict = criticResponse.verdict;
        reason = criticResponse.verdictReason;
      } catch (err) {
        getLogger().error({ err }, 'CriticAgent failed to review actor node');
        // Fallback to needs_revision if critic agent fails
        verdict = 'needs_revision';
        reason = 'Critic agent evaluation failed';
      }
    }

    const criticNode: DagNode = {
      id: uuid(),
      project,
      thought:
        verdict === 'approved'
          ? '✔ Approved'
          : verdict === 'needs_revision'
            ? '✏ Needs revision'
            : '✗ Rejected',
      role: 'critic',
      verdict,
      ...(reason && { verdictReason: reason }),
      target: actorNodeId,
      parents: [actorNodeId],
      children: [],
      tags: [],
      artifacts: [],
      createdAt: '', // Will be set by appendEntity
      projectContext,
    };

    // Update the target node's children to include this critic node
    if (target && !target.children.includes(criticNode.id)) {
      target.children.push(criticNode.id);
      // Update the target node in the knowledge graph
      await this.kg.appendEntity(target);
    }

    // Persist the critic node
    await this.kg.appendEntity(criticNode);

    return criticNode;
  }
}
