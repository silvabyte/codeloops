import { KnowledgeGraphManager, ArtifactRef, DagNode, ActorThinkInput } from '@codeloops/core';
export declare class Actor {
  private readonly kg;
  constructor(kg: KnowledgeGraphManager);
  think(
    input: ActorThinkInput & {
      artifacts?: Partial<ArtifactRef>[];
      project: string;
      diff?: string;
    },
  ): Promise<{
    node: DagNode;
  }>;
}
//# sourceMappingURL=Actor.d.ts.map
