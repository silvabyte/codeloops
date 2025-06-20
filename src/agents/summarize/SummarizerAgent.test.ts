import { describe, it, expect, vi, beforeEach } from 'vitest';
import { SummarizerAgent, SummaryOutputSchema, type SummaryResponse } from './SummarizerAgent.ts';
import { createLogger } from '../../logger.ts';
import { DagNode } from '../../engine/KnowledgeGraph.ts';

// Mock the dependencies
vi.mock('../../config/models.ts', () => ({
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

vi.mock('../../config/index.ts', () => ({
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

describe('SummarizerAgent', () => {
  const mockLogger = createLogger({ withFile: false, withDevStdout: false });
  let summarizerAgent: SummarizerAgent;

  beforeEach(() => {
    vi.clearAllMocks();
    summarizerAgent = new SummarizerAgent({ logger: mockLogger });
  });

  describe('Schema Validation', () => {
    it('should have valid SummaryOutputSchema', () => {
      expect(SummaryOutputSchema).toBeDefined();

      // Test valid responses
      const validResponse = {
        summary: 'This is a comprehensive summary of the work done',
        keyPoints: ['Point 1', 'Point 2', 'Point 3'],
        actionItems: ['Action 1', 'Action 2'],
      };

      const result = SummaryOutputSchema.safeParse(validResponse);
      expect(result.success).toBe(true);
    });

    it('should require summary and keyPoints fields', () => {
      const invalidResponse = {
        actionItems: ['Some action'],
      };

      const result = SummaryOutputSchema.safeParse(invalidResponse);
      expect(result.success).toBe(false);
    });

    it('should allow optional actionItems to be undefined', () => {
      const minimalResponse = {
        summary: 'A basic summary',
        keyPoints: ['Key point 1'],
      };

      const result = SummaryOutputSchema.safeParse(minimalResponse);
      expect(result.success).toBe(true);
    });

    it('should validate array types for keyPoints and actionItems', () => {
      const invalidResponse = {
        summary: 'Valid summary',
        keyPoints: 'Not an array',
        actionItems: 'Also not an array',
      };

      const result = SummaryOutputSchema.safeParse(invalidResponse);
      expect(result.success).toBe(false);
    });
  });

  describe('Constructor', () => {
    it('should initialize with proper configuration', () => {
      expect(summarizerAgent).toBeInstanceOf(SummarizerAgent);
    });

    it('should handle missing model reference gracefully', async () => {
      const { getModelReference } = vi.mocked(await import('../../config/models.ts'));
      getModelReference.mockReturnValueOnce(null);

      expect(() => new SummarizerAgent({ logger: mockLogger })).not.toThrow();
    });
  });

  describe('summarizeNodes', () => {
    const createMockNode = (overrides: Partial<DagNode> = {}): DagNode => ({
      id: 'test-node-id',
      project: 'test-project',
      projectContext: '/test/project',
      thought: 'This is a test thought',
      role: 'actor',
      createdAt: '2024-01-01T00:00:00Z',
      parents: [],
      children: [],
      tags: ['test'],
      artifacts: [{ name: 'test.ts', path: 'src/test.ts' }],
      ...overrides,
    });

    it('should successfully summarize a sequence of nodes', async () => {
      const mockNodes = [
        createMockNode({
          thought: 'Implemented user authentication system',
          tags: ['security', 'authentication'],
          artifacts: [{ name: 'auth.ts', path: 'src/auth.ts' }],
        }),
        createMockNode({
          id: 'critic-node',
          role: 'critic',
          thought: '✔ Approved',
          verdict: 'approved',
          target: 'test-node-id',
        }),
        createMockNode({
          id: 'revision-node',
          thought: 'Added input validation and error handling',
          tags: ['validation', 'error-handling'],
        }),
      ];

      const expectedResponse: SummaryResponse = {
        summary: 'Implemented a complete user authentication system with proper validation',
        keyPoints: [
          'Created authentication module with secure login flow',
          'Added comprehensive input validation',
          'Implemented error handling throughout the system',
        ],
        actionItems: ['Add unit tests for authentication flows'],
      };

      vi.spyOn(summarizerAgent, 'send').mockResolvedValueOnce(expectedResponse);

      const result = await summarizerAgent.summarizeNodes(mockNodes);

      expect(result).toEqual(expectedResponse);
      expect(summarizerAgent.send).toHaveBeenCalledOnce();
    });

    it('should throw error when no nodes provided', async () => {
      await expect(summarizerAgent.summarizeNodes([])).rejects.toThrow(
        'Cannot summarize: No nodes provided',
      );
    });

    it('should throw error when nodes is null/undefined', async () => {
      await expect(summarizerAgent.summarizeNodes(null as unknown as DagNode[])).rejects.toThrow(
        'Cannot summarize: No nodes provided',
      );
    });

    it('should include node context in prompt', async () => {
      const mockNodes = [
        createMockNode({
          thought: 'Refactored database connection logic',
          tags: ['database', 'refactoring'],
          artifacts: [
            { name: 'db.ts', path: 'src/database/db.ts' },
            { name: 'connection.ts', path: 'src/database/connection.ts' },
          ],
        }),
      ];

      const mockResponse: SummaryResponse = {
        summary: 'Database refactoring completed',
        keyPoints: ['Improved connection handling'],
      };

      vi.spyOn(summarizerAgent, 'send').mockResolvedValueOnce(mockResponse);

      await summarizerAgent.summarizeNodes(mockNodes);

      const sendCall = vi.mocked(summarizerAgent.send).mock.calls[0];
      const prompt = sendCall[0];

      // Verify that the prompt includes key node information
      expect(prompt).toContain('test-node-id');
      expect(prompt).toContain('Refactored database connection logic');
      expect(prompt).toContain('database');
      expect(prompt).toContain('refactoring');
      expect(prompt).toContain('db.ts');
      expect(prompt).toContain('connection.ts');
    });

    it('should handle nodes with different roles', async () => {
      const mockNodes = [
        createMockNode({ role: 'actor', thought: 'Actor thought' }),
        createMockNode({ role: 'critic', thought: '✔ Approved', verdict: 'approved' }),
        createMockNode({ role: 'summary', thought: 'Previous summary' }),
      ];

      const mockResponse: SummaryResponse = {
        summary: 'Mixed role sequence processed',
        keyPoints: ['Actor and critic interaction documented'],
      };

      vi.spyOn(summarizerAgent, 'send').mockResolvedValueOnce(mockResponse);

      const result = await summarizerAgent.summarizeNodes(mockNodes);

      expect(result).toEqual(mockResponse);
    });

    it('should handle nodes with no artifacts', async () => {
      const mockNodes = [
        createMockNode({ artifacts: [] }),
        createMockNode({ artifacts: undefined }),
      ];

      const mockResponse: SummaryResponse = {
        summary: 'Conceptual work without artifacts',
        keyPoints: ['Planning and design completed'],
      };

      vi.spyOn(summarizerAgent, 'send').mockResolvedValueOnce(mockResponse);

      await expect(summarizerAgent.summarizeNodes(mockNodes)).resolves.toEqual(mockResponse);
    });

    it('should handle agent send failures gracefully', async () => {
      const mockNodes = [createMockNode()];
      const sendError = new Error('AI model temporarily unavailable');

      vi.spyOn(summarizerAgent, 'send').mockRejectedValueOnce(sendError);

      await expect(summarizerAgent.summarizeNodes(mockNodes)).rejects.toThrow(
        'AI model temporarily unavailable',
      );
    });
  });

  describe('summarize (legacy compatibility)', () => {
    const createMockNode = (overrides: Partial<DagNode> = {}): DagNode => ({
      id: 'test-node-id',
      project: 'test-project',
      projectContext: '/test/project',
      thought: 'This is a test thought',
      role: 'actor',
      createdAt: '2024-01-01T00:00:00Z',
      parents: [],
      children: [],
      tags: ['test'],
      artifacts: [{ name: 'test.ts', path: 'src/test.ts' }],
      ...overrides,
    });

    it('should return formatted summary with key points', async () => {
      const mockNodes = [createMockNode()];
      const mockResponse: SummaryResponse = {
        summary: 'Main summary content',
        keyPoints: ['Key point 1', 'Key point 2'],
        actionItems: ['Action 1', 'Action 2'],
      };

      vi.spyOn(summarizerAgent, 'send').mockResolvedValueOnce(mockResponse);

      const result = await summarizerAgent.summarize(mockNodes);

      expect(result.summary).toContain('Main summary content');
      expect(result.summary).toContain('Key Points:');
      expect(result.summary).toContain('• Key point 1');
      expect(result.summary).toContain('• Key point 2');
      expect(result.summary).toContain('Action Items:');
      expect(result.summary).toContain('• Action 1');
      expect(result.summary).toContain('• Action 2');
      expect(result.error).toBeUndefined();
    });

    it('should format summary without action items when none provided', async () => {
      const mockNodes = [createMockNode()];
      const mockResponse: SummaryResponse = {
        summary: 'Main summary content',
        keyPoints: ['Key point 1'],
      };

      vi.spyOn(summarizerAgent, 'send').mockResolvedValueOnce(mockResponse);

      const result = await summarizerAgent.summarize(mockNodes);

      expect(result.summary).toContain('Main summary content');
      expect(result.summary).toContain('Key Points:');
      expect(result.summary).not.toContain('Action Items:');
      expect(result.error).toBeUndefined();
    });

    it('should return error when summarizeNodes fails', async () => {
      const mockNodes = [createMockNode()];
      const error = new Error('Summarization failed');

      vi.spyOn(summarizerAgent, 'send').mockRejectedValueOnce(error);

      const result = await summarizerAgent.summarize(mockNodes);

      expect(result.summary).toBe('');
      expect(result.error).toBe('Summarization failed');
    });

    it('should handle unknown error types', async () => {
      const mockNodes = [createMockNode()];

      vi.spyOn(summarizerAgent, 'send').mockRejectedValueOnce('Unknown error');

      const result = await summarizerAgent.summarize(mockNodes);

      expect(result.summary).toBe('');
      expect(result.error).toBe('Unknown summarization error');
    });
  });

  describe('isEnabled', () => {
    it('should return true when agent is enabled', () => {
      expect(summarizerAgent.isEnabled()).toBe(true);
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

      const disabledAgent = new SummarizerAgent({ logger: mockLogger });
      expect(disabledAgent.isEnabled()).toBe(false);
    });
  });

  describe('Error Handling', () => {
    it('should propagate configuration errors', async () => {
      const { createModel } = vi.mocked(await import('../../config/models.ts'));
      createModel.mockImplementationOnce(() => {
        throw new Error('Invalid model configuration');
      });

      expect(() => new SummarizerAgent({ logger: mockLogger })).toThrow(
        'Invalid model configuration',
      );
    });

    it('should handle malformed node data gracefully', async () => {
      const malformedNodes = [
        {
          id: 'test',
          role: 'actor',
          // Missing required fields
        } as DagNode,
      ];

      const mockResponse: SummaryResponse = {
        summary: 'Handled malformed data',
        keyPoints: ['Graceful degradation applied'],
      };

      vi.spyOn(summarizerAgent, 'send').mockResolvedValueOnce(mockResponse);

      // Should not throw, but handle gracefully
      await expect(summarizerAgent.summarizeNodes(malformedNodes)).resolves.toEqual(mockResponse);
    });
  });

  describe('Prompt Building', () => {
    it('should build comprehensive summarization prompts', async () => {
      const mockNodes = [
        {
          id: 'node-1',
          project: 'test-project',
          projectContext: '/test/project',
          thought: 'Implemented comprehensive logging system',
          role: 'actor',
          createdAt: '2024-01-01T00:00:00Z',
          parents: [],
          children: [],
          tags: ['logging', 'monitoring', 'infrastructure'],
          artifacts: [
            { name: 'logger.ts', path: 'src/utils/logger.ts' },
            { name: 'monitoring.ts', path: 'src/monitoring/monitoring.ts' },
          ],
        } as DagNode,
        {
          id: 'node-2',
          project: 'test-project',
          projectContext: '/test/project',
          thought: '✔ Approved',
          role: 'critic',
          verdict: 'approved',
          target: 'node-1',
          createdAt: '2024-01-01T01:00:00Z',
          parents: ['node-1'],
          children: [],
          tags: [],
          artifacts: [],
        } as DagNode,
      ];

      const mockResponse: SummaryResponse = {
        summary: 'Logging system implementation completed',
        keyPoints: ['Infrastructure improvements made'],
      };

      vi.spyOn(summarizerAgent, 'send').mockResolvedValueOnce(mockResponse);

      await summarizerAgent.summarizeNodes(mockNodes);

      const prompt = vi.mocked(summarizerAgent.send).mock.calls[0][0];

      // Verify prompt structure and content
      expect(prompt).toContain('analyze and summarize the following sequence of 2 nodes');
      expect(prompt).toContain('Node Sequence');
      expect(prompt).toContain('Implemented comprehensive logging system');
      expect(prompt).toContain('logging');
      expect(prompt).toContain('monitoring');
      expect(prompt).toContain('logger.ts');
      expect(prompt).toContain('monitoring.ts');
      expect(prompt).toContain('approved');
      expect(prompt).toContain('main progression of work');
      expect(prompt).toContain('structured summary');
    });

    it('should handle large sequences of nodes', async () => {
      const mockNodes = Array.from({ length: 50 }, (_, i) => ({
        id: `node-${i}`,
        project: 'test-project',
        projectContext: '/test/project',
        thought: `Thought ${i}`,
        role: i % 3 === 0 ? 'actor' : i % 3 === 1 ? 'critic' : 'summary',
        createdAt: `2024-01-01T${String(i).padStart(2, '0')}:00:00Z`,
        parents: i > 0 ? [`node-${i - 1}`] : [],
        children: [],
        tags: [`tag-${i}`],
        artifacts: [],
      })) as DagNode[];

      const mockResponse: SummaryResponse = {
        summary: 'Large sequence processed',
        keyPoints: ['Multiple iterations completed'],
      };

      vi.spyOn(summarizerAgent, 'send').mockResolvedValueOnce(mockResponse);

      await summarizerAgent.summarizeNodes(mockNodes);

      const prompt = vi.mocked(summarizerAgent.send).mock.calls[0][0];
      expect(prompt).toContain('sequence of 50 nodes');
    });
  });

  describe('Factory Function', () => {
    it('should create SummarizerAgent instance via factory', async () => {
      const { createSummarizerAgent } = await import('./SummarizerAgent.ts');
      const agent = createSummarizerAgent({ logger: mockLogger });
      expect(agent).toBeInstanceOf(SummarizerAgent);
    });
  });
});
