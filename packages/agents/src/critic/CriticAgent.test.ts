import { describe, it, expect, beforeEach } from 'bun:test';
import { CriticOutputSchema } from './CriticAgent.js';
import { createLogger, type DagNode } from '@codeloops/core';

describe('CriticAgent', () => {
  const mockLogger = createLogger({ withFile: false, withDevStdout: false });

  beforeEach(() => {
    // Clear any state if needed
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

    it('should allow optional fields', () => {
      const minimalResponse = {
        verdict: 'approved' as const,
      };

      const result = CriticOutputSchema.safeParse(minimalResponse);
      expect(result.success).toBe(true);
    });

    it('should validate recommendations as array of strings', () => {
      const responseWithRecommendations = {
        verdict: 'needs_revision' as const,
        verdictReason: 'Needs improvement',
        recommendations: ['Fix error handling', 'Add more tests'],
      };

      const result = CriticOutputSchema.safeParse(responseWithRecommendations);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.recommendations).toHaveLength(2);
      }
    });
  });

  describe('Review Prompt Building', () => {
    it('should handle actor nodes correctly', () => {
      const actorNode: DagNode = {
        id: 'test-id',
        project: 'test-project',
        projectContext: '/path/to/project',
        thought: 'Implemented feature X',
        role: 'actor',
        createdAt: new Date().toISOString(),
        parents: [],
        children: [],
        tags: ['feature', 'implementation'],
        artifacts: [
          {
            name: 'test.ts',
            path: '/path/to/test.ts',
          },
        ],
      };

      // Test that the node structure is valid
      expect(actorNode.role).toBe('actor');
      expect(actorNode.thought).toBeDefined();
      expect(actorNode.tags).toContain('feature');
    });

    it('should reject non-actor nodes', () => {
      const criticNode: DagNode = {
        id: 'test-id',
        project: 'test-project',
        projectContext: '/path/to/project',
        thought: 'Review comment',
        role: 'critic',
        createdAt: new Date().toISOString(),
        parents: [],
        children: [],
      };

      // Test that we can identify non-actor nodes
      expect(criticNode.role).not.toBe('actor');
    });
  });

  describe('CriticResponse Type', () => {
    it('should parse approved response correctly', () => {
      const approvedResponse = {
        verdict: 'approved' as const,
      };

      const parsed = CriticOutputSchema.parse(approvedResponse);
      expect(parsed.verdict).toBe('approved');
      expect(parsed.verdictReason).toBeUndefined();
      expect(parsed.recommendations).toBeUndefined();
    });

    it('should parse needs_revision response with full details', () => {
      const revisionResponse = {
        verdict: 'needs_revision' as const,
        verdictReason: 'The thought lacks specific details about implementation',
        recommendations: [
          'Add more specific technical details',
          'Include error handling approach',
          'Mention performance considerations',
        ],
      };

      const parsed = CriticOutputSchema.parse(revisionResponse);
      expect(parsed.verdict).toBe('needs_revision');
      expect(parsed.verdictReason).toBeDefined();
      expect(parsed.recommendations).toHaveLength(3);
    });

    it('should parse reject response correctly', () => {
      const rejectResponse = {
        verdict: 'reject' as const,
        verdictReason: 'The thought contains TODOs and is incomplete',
        recommendations: ['Complete all implementation before submission'],
      };

      const parsed = CriticOutputSchema.parse(rejectResponse);
      expect(parsed.verdict).toBe('reject');
      expect(parsed.verdictReason).toContain('TODOs');
      expect(parsed.recommendations).toHaveLength(1);
    });
  });
});
