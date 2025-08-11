import { KnowledgeGraphManager, DagNode } from '@codeloops/core';
import { z } from 'zod';
export declare const CriticSchema: {
  actorNodeId: z.ZodString;
};
export declare class Critic {
  private readonly kg;
  private readonly criticAgent;
  constructor(kg: KnowledgeGraphManager);
  review({
    actorNodeId,
    project,
    projectContext,
  }: {
    actorNodeId: string;
    project: string;
    projectContext: string;
  }): Promise<DagNode>;
}
//# sourceMappingURL=Critic.d.ts.map
