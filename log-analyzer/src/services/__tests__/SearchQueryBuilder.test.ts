import { SearchQueryBuilder } from '../SearchQueryBuilder';

describe('SearchQueryBuilder', () => {
  describe('create', () => {
    it('should create empty query', () => {
      const builder = SearchQueryBuilder.create();
      const query = builder.getQuery();
      
      expect(query.terms).toHaveLength(0);
      expect(query.globalOperator).toBe('AND');
      expect(query.id).toBeTruthy();
      expect(query.metadata.createdAt).toBeGreaterThan(0);
    });
  });

  describe('fromString', () => {
    it('should parse single keyword', () => {
      const builder = SearchQueryBuilder.fromString('error');
      const query = builder.getQuery();
      
      expect(query.terms).toHaveLength(1);
      expect(query.terms[0].value).toBe('error');
      expect(query.terms[0].source).toBe('user');
    });

    it('should parse multiple keywords', () => {
      const builder = SearchQueryBuilder.fromString('error | timeout');
      const query = builder.getQuery();
      
      expect(query.terms).toHaveLength(2);
      expect(query.terms[0].value).toBe('error');
      expect(query.terms[1].value).toBe('timeout');
    });

    it('should handle empty string', () => {
      const builder = SearchQueryBuilder.fromString('');
      const query = builder.getQuery();
      
      expect(query.terms).toHaveLength(0);
    });

    it('should trim whitespace', () => {
      const builder = SearchQueryBuilder.fromString('  error  |  timeout  ');
      const query = builder.getQuery();
      
      expect(query.terms).toHaveLength(2);
      expect(query.terms[0].value).toBe('error');
      expect(query.terms[1].value).toBe('timeout');
    });

    it('should detect preset keywords', () => {
      const keywordGroups = [
        {
          id: 'group1',
          patterns: [{ regex: 'error' }]
        }
      ];
      
      const builder = SearchQueryBuilder.fromString('error', keywordGroups);
      const query = builder.getQuery();
      
      expect(query.terms[0].source).toBe('preset');
      expect(query.terms[0].presetGroupId).toBe('group1');
    });
  });

  describe('addTerm', () => {
    it('should add single term', () => {
      const builder = SearchQueryBuilder.create().addTerm('error');
      const query = builder.getQuery();
      
      expect(query.terms).toHaveLength(1);
      expect(query.terms[0].value).toBe('error');
      expect(query.terms[0].enabled).toBe(true);
      expect(query.terms[0].operator).toBe('AND');
    });

    it('should add multiple terms', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error')
        .addTerm('timeout');
      
      const query = builder.getQuery();
      expect(query.terms).toHaveLength(2);
    });

    it('should respect custom options', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error', {
          source: 'preset',
          priority: 10,
          isRegex: true,
          operator: 'OR',
          caseSensitive: true
        });
      
      const query = builder.getQuery();
      expect(query.terms[0].source).toBe('preset');
      expect(query.terms[0].priority).toBe(10);
      expect(query.terms[0].isRegex).toBe(true);
      expect(query.terms[0].operator).toBe('OR');
      expect(query.terms[0].caseSensitive).toBe(true);
    });

    it('should update metadata on add', () => {
      const builder = SearchQueryBuilder.create();
      const beforeTime = Date.now();
      
      builder.addTerm('error');
      
      const query = builder.getQuery();
      expect(query.metadata.lastModified).toBeGreaterThanOrEqual(beforeTime);
    });
  });

  describe('removeTerm', () => {
    it('should remove term by id', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error')
        .addTerm('timeout');
      
      const termId = builder.getQuery().terms[0].id;
      builder.removeTerm(termId);
      
      expect(builder.getQuery().terms).toHaveLength(1);
      expect(builder.getQuery().terms[0].value).toBe('timeout');
    });

    it('should handle non-existent id', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error');
      
      builder.removeTerm('non-existent');
      
      expect(builder.getQuery().terms).toHaveLength(1);
    });
  });

  describe('toggleTerm', () => {
    it('should toggle term enabled state', () => {
      const builder = SearchQueryBuilder.create().addTerm('error');
      const termId = builder.getQuery().terms[0].id;
      
      expect(builder.getQuery().terms[0].enabled).toBe(true);
      
      builder.toggleTerm(termId);
      expect(builder.getQuery().terms[0].enabled).toBe(false);
      
      builder.toggleTerm(termId);
      expect(builder.getQuery().terms[0].enabled).toBe(true);
    });

    it('should handle non-existent id', () => {
      const builder = SearchQueryBuilder.create().addTerm('error');
      
      expect(() => builder.toggleTerm('non-existent')).not.toThrow();
    });
  });

  describe('updateTermValue', () => {
    it('should update term value', () => {
      const builder = SearchQueryBuilder.create().addTerm('error');
      const termId = builder.getQuery().terms[0].id;
      
      builder.updateTermValue(termId, 'warning');
      
      expect(builder.getQuery().terms[0].value).toBe('warning');
    });
  });

  describe('clear', () => {
    it('should remove all terms', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error')
        .addTerm('timeout')
        .clear();
      
      expect(builder.getQuery().terms).toHaveLength(0);
    });
  });

  describe('validate', () => {
    it('should fail on empty query', () => {
      const builder = SearchQueryBuilder.create();
      const result = builder.validate();
      
      expect(result.isValid).toBe(false);
      expect(result.issues[0].code).toBe('EMPTY_QUERY');
    });

    it('should pass on valid query', () => {
      const builder = SearchQueryBuilder.create().addTerm('error');
      const result = builder.validate();
      
      expect(result.isValid).toBe(true);
      expect(result.issues).toHaveLength(0);
    });

    it('should detect empty term value', () => {
      const builder = SearchQueryBuilder.create().addTerm('  ');
      const result = builder.validate();
      
      expect(result.isValid).toBe(false);
      expect(result.issues.some(i => i.code === 'EMPTY_VALUE')).toBe(true);
    });

    it('should detect invalid regex', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('[invalid', { isRegex: true });
      const result = builder.validate();
      
      expect(result.isValid).toBe(false);
      expect(result.issues.some(i => i.code === 'INVALID_REGEX')).toBe(true);
    });

    it('should warn on duplicate terms', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error')
        .addTerm('error');
      const result = builder.validate();
      
      const warnings = result.issues.filter(i => i.severity === 'warning');
      expect(warnings.length).toBeGreaterThan(0);
      expect(warnings.some(w => w.code === 'DUPLICATE_TERM')).toBe(true);
    });

    it('should detect disabled terms', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error');
      const termId = builder.getQuery().terms[0].id;
      builder.toggleTerm(termId);
      
      const result = builder.validate();
      expect(result.isValid).toBe(false);
      expect(result.issues[0].code).toBe('NO_ENABLED_TERMS');
    });

    it('should warn on value too long', () => {
      const longValue = 'a'.repeat(101);
      const builder = SearchQueryBuilder.create().addTerm(longValue);
      const result = builder.validate();
      
      const warnings = result.issues.filter(i => i.severity === 'warning');
      expect(warnings.some(w => w.code === 'VALUE_TOO_LONG')).toBe(true);
    });
  });

  describe('toQueryString', () => {
    it('should generate query string', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error')
        .addTerm('timeout');
      
      const queryString = builder.toQueryString();
      expect(queryString).toBe('error | timeout');
    });

    it('should exclude disabled terms', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error')
        .addTerm('timeout');
      
      const termId = builder.getQuery().terms[0].id;
      builder.toggleTerm(termId);
      
      const queryString = builder.toQueryString();
      expect(queryString).toBe('timeout');
    });

    it('should return empty string for empty query', () => {
      const builder = SearchQueryBuilder.create();
      expect(builder.toQueryString()).toBe('');
    });
  });

  describe('toOptimizedQuery', () => {
    it('should create optimized query', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error')
        .addTerm('timeout');
      
      const optimized = builder.toOptimizedQuery();
      expect(optimized.enabledTerms).toHaveLength(2);
      expect(optimized.termsByOperator.get('AND')).toHaveLength(2);
    });

    it('should group by operator', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error', { operator: 'AND' })
        .addTerm('warning', { operator: 'OR' })
        .addTerm('debug', { operator: 'NOT' });
      
      const optimized = builder.toOptimizedQuery();
      expect(optimized.termsByOperator.get('AND')).toHaveLength(1);
      expect(optimized.termsByOperator.get('OR')).toHaveLength(1);
      expect(optimized.termsByOperator.get('NOT')).toHaveLength(1);
    });

    it('should calculate normalization stats', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error')
        .addTerm('error') // duplicate
        .addTerm('timeout');
      
      const termId = builder.getQuery().terms[2].id;
      builder.toggleTerm(termId); // disable one
      
      const optimized = builder.toOptimizedQuery();
      expect(optimized.normalization.removedInvalidCount).toBe(1);
    });
  });

  describe('export and import', () => {
    it('should export and import query', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error')
        .addTerm('timeout');
      
      const exported = builder.export();
      const imported = SearchQueryBuilder.import(exported);
      
      expect(imported.getQuery().terms).toHaveLength(2);
      expect(imported.toQueryString()).toBe(builder.toQueryString());
    });

    it('should preserve all properties', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error', {
          source: 'preset',
          presetGroupId: 'group1',
          isRegex: true,
          priority: 10
        });
      
      const exported = builder.export();
      const imported = SearchQueryBuilder.import(exported);
      const term = imported.getQuery().terms[0];
      
      expect(term.source).toBe('preset');
      expect(term.presetGroupId).toBe('group1');
      expect(term.isRegex).toBe(true);
      expect(term.priority).toBe(10);
    });
  });

  describe('utility methods', () => {
    it('should set global operator', () => {
      const builder = SearchQueryBuilder.create()
        .setGlobalOperator('OR');
      
      expect(builder.getQuery().globalOperator).toBe('OR');
    });

    it('should get summary', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error', { source: 'preset', isRegex: true })
        .addTerm('timeout');
      
      const summary = builder.getSummary();
      expect(summary.termCount).toBe(2);
      expect(summary.enabledTermCount).toBe(2);
      expect(summary.hasPreset).toBe(true);
      expect(summary.hasRegex).toBe(true);
    });

    it('should check if has value', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error');
      
      expect(builder.has('error')).toBe(true);
      expect(builder.has('ERROR')).toBe(true); // case insensitive
      expect(builder.has('timeout')).toBe(false);
    });

    it('should find term by value', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error');
      
      const term = builder.findTermByValue('error');
      expect(term).toBeDefined();
      expect(term?.value).toBe('error');
      
      const notFound = builder.findTermByValue('timeout');
      expect(notFound).toBeUndefined();
    });

    it('should get all terms', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error')
        .addTerm('timeout');
      
      const terms = builder.getTerms();
      expect(terms).toHaveLength(2);
    });

    it('should get enabled terms only', () => {
      const builder = SearchQueryBuilder.create()
        .addTerm('error')
        .addTerm('timeout');
      
      const termId = builder.getQuery().terms[0].id;
      builder.toggleTerm(termId);
      
      const enabledTerms = builder.getEnabledTerms();
      expect(enabledTerms).toHaveLength(1);
      expect(enabledTerms[0].value).toBe('timeout');
    });
  });

  describe('toJSON', () => {
    it('should return deep copy', () => {
      const builder = SearchQueryBuilder.create().addTerm('error');
      const json = builder.toJSON();
      
      // Modify the copy
      json.terms[0].value = 'modified';
      
      // Original should be unchanged
      expect(builder.getQuery().terms[0].value).toBe('error');
    });
  });
});
