import { describe, it, expect, vi, beforeEach } from 'vitest';
import { CriticAgent, CriticOutputSchema, type CriticResponse } from './CriticAgent.ts';
import { createLogger } from '../../logger.ts';
import { DagNode } from '../../engine/KnowledgeGraph.ts';

// Mock the dependencies
vi.mock('../config/models.ts', () => ({
  createModel: vi.fn().mockReturnValue({
    generateObject: vi.fn(),
    generateText: vi.fn(),
  }),
  getModelReference: vi.fn().mockReturnValue('openai.gpt-4o-mini'),
  getModelConfigFromPath: vi.fn().mockReturnValue({
    temperature: 0.3,
    maxTokens: 2000,
    enabled: true,
  }),
}));

vi.mock('../config/index.ts', () => ({
  getConfig: vi.fn().mockReturnValue({
    get: vi.fn().mockReturnValue('openai.gpt-4o-mini'),
  }),
}));

vi.mock('@voltagent/core', () => ({
  Agent: vi.fn().mockImplementation(() => ({
    generateObject: vi.fn(),
    generateText: vi.fn(),
  })),
  createHooks: vi.fn().mockReturnValue({}),
}));

vi.mock('@voltagent/vercel-ai', () => ({
  VercelAIProvider: vi.fn(),
}));

describe('CriticAgent', () => {
  const mockLogger = createLogger({ withFile: false, withDevStdout: false });
  let criticAgent: CriticAgent;

  beforeEach(() => {
    vi.clearAllMocks();
    criticAgent = new CriticAgent({ logger: mockLogger });
  });

  describe('Schema Validation', () => {
    it('should have valid CriticOutputSchema', () => {
      expect(CriticOutputSchema).toBeDefined();

      // Test valid responses
      const validResponse = {
        verdict: 'approved' as const,
        verdictReason: 'Good implementation',
        recommendations: ['Keep up the good work'],
      };

      const result = CriticOutputSchema.safeParse(validResponse);
      expect(result.success).toBe(true);
    });

    it('should validate verdict enum values', () => {
      const validVerdicts = ['approved', 'needs_revision', 'reject'] as const;

      validVerdicts.forEach((verdict) => {
        const response = { verdict };
        const result = CriticOutputSchema.safeParse(response);
        expect(result.success).toBe(true);
      });
    });

    it('should reject invalid verdict values', () => {
      const invalidResponse = {
        verdict: 'invalid_verdict',
      };

      const result = CriticOutputSchema.safeParse(invalidResponse);
      expect(result.success).toBe(false);
    });

    it('should allow optional fields to be undefined', () => {
      const minimalResponse = {
        verdict: 'approved' as const,
      };

      const result = CriticOutputSchema.safeParse(minimalResponse);
      expect(result.success).toBe(true);
    });
  });

  describe('Constructor', () => {
    it('should initialize with proper configuration', () => {
      expect(criticAgent).toBeInstanceOf(CriticAgent);
    });

    it('should handle missing model reference gracefully', async () => {
      const { getModelReference } = vi.mocked(await import('../../config/models.ts'));
      getModelReference.mockReturnValueOnce(null);

      expect(() => new CriticAgent({ logger: mockLogger })).not.toThrow();
    });
  });

  describe('reviewActorNode', () => {
    const createMockActorNode = (overrides: Partial<DagNode> = {}): DagNode => ({
      id: 'test-node-id',
      project: 'test-project',
      projectContext: '/test/project',
      thought: 'This is a test thought about implementing a feature',
      role: 'actor',
      createdAt: '2024-01-01T00:00:00Z',
      parents: [],
      children: [],
      tags: ['implementation', 'feature'],
      artifacts: [{ name: 'test.ts', path: 'src/test.ts' }],
      ...overrides,
    });

    it('should successfully review an actor node', async () => {
      const mockActorNode = createMockActorNode();
      const expectedResponse: CriticResponse = {
        verdict: 'approved',
        verdictReason: 'Clear and well-structured implementation',
        recommendations: ['Consider adding more comments'],
      };

      // Mock the send method to return expected response
      vi.spyOn(criticAgent, 'send').mockResolvedValueOnce(expectedResponse);

      const result = await criticAgent.reviewActorNode(mockActorNode);

      expect(result).toEqual(expectedResponse);
      expect(criticAgent.send).toHaveBeenCalledOnce();
    });

    it('should reject non-actor nodes', async () => {
      const mockCriticNode = createMockActorNode({ role: 'critic' });

      await expect(criticAgent.reviewActorNode(mockCriticNode)).rejects.toThrow(
        'Cannot review non-actor node. Node role: critic',
      );
    });

    it('should handle summary nodes rejection', async () => {
      const mockSummaryNode = createMockActorNode({ role: 'summary' });

      await expect(criticAgent.reviewActorNode(mockSummaryNode)).rejects.toThrow(
        'Cannot review non-actor node. Node role: summary',
      );
    });

    it('should include node context in prompt', async () => {
      const mockActorNode = createMockActorNode({
        thought: 'Implemented new user authentication system',
        tags: ['security', 'authentication'],
        artifacts: [
          { name: 'auth.ts', path: 'src/auth/auth.ts' },
          { name: 'user.ts', path: 'src/models/user.ts' },
        ],
      });

      const mockResponse: CriticResponse = { verdict: 'approved' };
      vi.spyOn(criticAgent, 'send').mockResolvedValueOnce(mockResponse);

      await criticAgent.reviewActorNode(mockActorNode);

      const sendCall = vi.mocked(criticAgent.send).mock.calls[0];
      const prompt = sendCall[0];

      // Verify that the prompt includes key node information
      expect(prompt).toContain('test-node-id');
      expect(prompt).toContain('Implemented new user authentication system');
      expect(prompt).toContain('security');
      expect(prompt).toContain('authentication');
      expect(prompt).toContain('auth.ts');
      expect(prompt).toContain('user.ts');
    });

    it('should handle different verdict types', async () => {
      const mockActorNode = createMockActorNode();

      const testCases = [
        { verdict: 'approved' as const, reason: undefined },
        { verdict: 'needs_revision' as const, reason: 'Needs more detail' },
        { verdict: 'reject' as const, reason: 'Fundamentally flawed' },
      ];

      for (const testCase of testCases) {
        const mockResponse: CriticResponse = {
          verdict: testCase.verdict,
          verdictReason: testCase.reason,
        };

        vi.spyOn(criticAgent, 'send').mockResolvedValueOnce(mockResponse);

        const result = await criticAgent.reviewActorNode(mockActorNode);

        expect(result.verdict).toBe(testCase.verdict);
        expect(result.verdictReason).toBe(testCase.reason);
      }
    });

    it('should handle agent send failures gracefully', async () => {
      const mockActorNode = createMockActorNode();
      const sendError = new Error('AI model temporarily unavailable');

      vi.spyOn(criticAgent, 'send').mockRejectedValueOnce(sendError);

      await expect(criticAgent.reviewActorNode(mockActorNode)).rejects.toThrow(
        'AI model temporarily unavailable',
      );
    });

    it('should handle empty thought content', async () => {
      const mockActorNode = createMockActorNode({ thought: '' });
      const mockResponse: CriticResponse = { verdict: 'needs_revision' };

      vi.spyOn(criticAgent, 'send').mockResolvedValueOnce(mockResponse);

      const result = await criticAgent.reviewActorNode(mockActorNode);

      expect(result.verdict).toBe('needs_revision');
    });

    it('should handle nodes with no artifacts', async () => {
      const mockActorNode = createMockActorNode({ artifacts: [] });
      const mockResponse: CriticResponse = { verdict: 'approved' };

      vi.spyOn(criticAgent, 'send').mockResolvedValueOnce(mockResponse);

      await expect(criticAgent.reviewActorNode(mockActorNode)).resolves.toEqual(mockResponse);
    });

    it('should handle nodes with many artifacts', async () => {
      const manyArtifacts = Array.from({ length: 10 }, (_, i) => ({
        name: `file${i}.ts`,
        path: `src/file${i}.ts`,
      }));

      const mockActorNode = createMockActorNode({ artifacts: manyArtifacts });
      const mockResponse: CriticResponse = { verdict: 'approved' };

      vi.spyOn(criticAgent, 'send').mockResolvedValueOnce(mockResponse);

      await expect(criticAgent.reviewActorNode(mockActorNode)).resolves.toEqual(mockResponse);
    });
  });

  describe('isEnabled', () => {
    it('should return true when agent is enabled', () => {
      expect(criticAgent.isEnabled()).toBe(true);
    });

    it('should return false when agent is disabled', async () => {
      const { getModelConfigFromPath } = vi.mocked(await import('../../config/models.ts'));

      // Clear existing mock and set up new behavior
      getModelConfigFromPath.mockClear();
      getModelConfigFromPath.mockReturnValue({
        temperature: 0.3,
        maxTokens: 2000,
        enabled: false,
      });

      const disabledAgent = new CriticAgent({ logger: mockLogger });
      expect(disabledAgent.isEnabled()).toBe(false);
    });
  });

  describe('Error Handling', () => {
    it('should propagate configuration errors', async () => {
      const { createModel } = vi.mocked(await import('../../config/models.ts'));
      createModel.mockImplementationOnce(() => {
        throw new Error('Invalid model configuration');
      });

      expect(() => new CriticAgent({ logger: mockLogger })).toThrow('Invalid model configuration');
    });

    it('should handle malformed node data', async () => {
      const malformedNode = {
        id: 'test',
        role: 'actor',
        // Missing required fields
      } as DagNode;

      const mockResponse: CriticResponse = { verdict: 'needs_revision' };
      vi.spyOn(criticAgent, 'send').mockResolvedValueOnce(mockResponse);

      // Should not throw, but handle gracefully
      await expect(criticAgent.reviewActorNode(malformedNode)).resolves.toEqual(mockResponse);
    });
  });

  describe('Prompt Building', () => {
    const createMockActorNode = (overrides: Partial<DagNode> = {}): DagNode => ({
      id: 'test-node-id',
      project: 'test-project',
      projectContext: '/test/project',
      thought: 'This is a test thought about implementing a feature',
      role: 'actor',
      createdAt: '2024-01-01T00:00:00Z',
      parents: [],
      children: [],
      tags: ['implementation', 'feature'],
      artifacts: [{ name: 'test.ts', path: 'src/test.ts' }],
      ...overrides,
    });

    it('should build comprehensive review prompts', async () => {
      const mockActorNode = createMockActorNode({
        thought: 'Added comprehensive error handling to the API endpoints',
        tags: ['error-handling', 'api', 'robustness'],
        artifacts: [
          { name: 'api.ts', path: 'src/api/api.ts' },
          { name: 'errors.ts', path: 'src/utils/errors.ts' },
        ],
      });

      const mockResponse: CriticResponse = { verdict: 'approved' };
      vi.spyOn(criticAgent, 'send').mockResolvedValueOnce(mockResponse);

      await criticAgent.reviewActorNode(mockActorNode);

      const prompt = vi.mocked(criticAgent.send).mock.calls[0][0];

      // Verify prompt structure and content
      expect(prompt).toContain('Please review the following actor thought');
      expect(prompt).toContain('Actor Node Information');
      expect(prompt).toContain('Added comprehensive error handling');
      expect(prompt).toContain('error-handling');
      expect(prompt).toContain('api.ts');
      expect(prompt).toContain('errors.ts');
      expect(prompt).toContain('verdict');
      expect(prompt).toContain('approved/needs_revision/reject');
    });
  });
});
