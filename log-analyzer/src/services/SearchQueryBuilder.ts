import { 
  SearchQuery, 
  SearchTerm, 
  QueryOperator,
  QueryValidation,
  OptimizedQuery,
  ValidationIssue,
  TermSource
} from '../types/search';

/**
 * 搜索查询构建器
 * 
 * 提供流畅的API来构建、验证和转换搜索查询
 */
export class SearchQueryBuilder {
  private query: SearchQuery;

  private constructor(query: SearchQuery) {
    this.query = query;
  }

  /**
   * 创建新的空查询
   */
  static create(): SearchQueryBuilder {
    return new SearchQueryBuilder({
      id: SearchQueryBuilder.generateId(),
      terms: [],
      globalOperator: 'AND',
      metadata: {
        createdAt: Date.now(),
        lastModified: Date.now(),
        executionCount: 0
      }
    });
  }

  /**
   * 从查询字符串创建
   * @param queryString 查询字符串（如 "error | timeout"）
   * @param keywordGroups 预置关键词组
   */
  static fromString(
    queryString: string,
    keywordGroups: any[] = []
  ): SearchQueryBuilder {
    const builder = SearchQueryBuilder.create();
    
    if (!queryString || queryString.trim().length === 0) {
      return builder;
    }

    const terms = queryString
      .split('|')
      .map(t => t.trim())
      .filter(t => t.length > 0);

    terms.forEach((value, idx) => {
      // 检查是否是预置关键词
      let source: TermSource = 'user';
      let presetGroupId: string | undefined;

      for (const group of keywordGroups) {
        if (group.patterns && group.patterns.some((p: any) => p.regex === value)) {
          source = 'preset';
          presetGroupId = group.id;
          break;
        }
      }

      builder.addTerm(value, {
        source,
        presetGroupId,
        priority: terms.length - idx
      });
    });

    return builder;
  }

  /**
   * 从 JSON 导入
   */
  static import(json: string): SearchQueryBuilder {
    const query = JSON.parse(json) as SearchQuery;
    return new SearchQueryBuilder(query);
  }

  private static generateId(): string {
    return `query_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  private static generateTermId(): string {
    return `term_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * 添加搜索项
   */
  addTerm(
    value: string,
    options?: {
      operator?: QueryOperator;
      source?: TermSource;
      presetGroupId?: string;
      isRegex?: boolean;
      priority?: number;
      caseSensitive?: boolean;
    }
  ): this {
    const term: SearchTerm = {
      id: SearchQueryBuilder.generateTermId(),
      value,
      operator: options?.operator ?? 'AND',
      source: options?.source ?? 'user',
      presetGroupId: options?.presetGroupId,
      isRegex: options?.isRegex ?? false,
      priority: options?.priority ?? 0,
      enabled: true,
      caseSensitive: options?.caseSensitive ?? false
    };

    this.query.terms.push(term);
    this.updateMetadata();
    return this;
  }

  /**
   * 移除搜索项
   */
  removeTerm(termId: string): this {
    this.query.terms = this.query.terms.filter(t => t.id !== termId);
    this.updateMetadata();
    return this;
  }

  /**
   * 切换搜索项的启用/禁用状态
   */
  toggleTerm(termId: string): this {
    const term = this.query.terms.find(t => t.id === termId);
    if (term) {
      term.enabled = !term.enabled;
      this.updateMetadata();
    }
    return this;
  }

  /**
   * 更新搜索项的值
   */
  updateTermValue(termId: string, newValue: string): this {
    const term = this.query.terms.find(t => t.id === termId);
    if (term) {
      term.value = newValue;
      this.updateMetadata();
    }
    return this;
  }

  /**
   * 清空所有搜索项
   */
  clear(): this {
    this.query.terms = [];
    this.updateMetadata();
    return this;
  }

  /**
   * 验证整个查询
   */
  validate(): QueryValidation {
    const issues: ValidationIssue[] = [];

    // 检查是否为空
    if (this.query.terms.length === 0) {
      issues.push({
        severity: 'error',
        code: 'EMPTY_QUERY',
        message: '查询不能为空（至少需要一个搜索项）'
      });
      return { isValid: false, issues };
    }

    // 检查是否有启用的项
    const enabledTerms = this.query.terms.filter(t => t.enabled);
    if (enabledTerms.length === 0) {
      issues.push({
        severity: 'error',
        code: 'NO_ENABLED_TERMS',
        message: '至少需要启用一个搜索项'
      });
    }

    // 验证每个项
    this.query.terms.forEach(term => {
      const termIssues = this.validateTerm(term);
      issues.push(...termIssues);
    });

    // 检查重复
    const duplicates = this.findDuplicates();
    duplicates.forEach(termId => {
      issues.push({
        termId,
        severity: 'warning',
        code: 'DUPLICATE_TERM',
        message: '存在重复的搜索条件'
      });
    });

    return {
      isValid: issues.filter(i => i.severity === 'error').length === 0,
      issues
    };
  }

  private validateTerm(term: SearchTerm): ValidationIssue[] {
    const issues: ValidationIssue[] = [];

    // 检查空值
    if (!term.value.trim()) {
      issues.push({
        termId: term.id,
        severity: 'error',
        code: 'EMPTY_VALUE',
        message: '搜索值不能为空'
      });
    }

    // 检查长度
    if (term.value.length > 100) {
      issues.push({
        termId: term.id,
        severity: 'warning',
        code: 'VALUE_TOO_LONG',
        message: '搜索值过长（> 100 字符），可能影响性能'
      });
    }

    // 验证正则表达式
    if (term.isRegex) {
      try {
        new RegExp(term.value);
      } catch (e: any) {
        issues.push({
          termId: term.id,
          severity: 'error',
          code: 'INVALID_REGEX',
          message: `无效的正则表达式：${e.message}`
        });
      }
    }

    return issues;
  }

  private findDuplicates(): string[] {
    const seen = new Map<string, string>();
    const duplicates: string[] = [];

    this.query.terms.forEach(term => {
      const key = `${term.value.toLowerCase()}|${term.source}`;
      if (seen.has(key)) {
        duplicates.push(term.id);
      } else {
        seen.set(key, term.id);
      }
    });

    return duplicates;
  }

  /**
   * 转换为查询字符串（用于显示）
   */
  toQueryString(): string {
    return this.query.terms
      .filter(t => t.enabled)
      .map(t => t.value)
      .join('|');
  }

  /**
   * 转换为优化的查询（用于执行）
   */
  toOptimizedQuery(): OptimizedQuery {
    const enabledTerms = this.query.terms.filter(t => t.enabled);
    const termsByOperator = new Map<QueryOperator, SearchTerm[]>();

    enabledTerms.forEach(term => {
      const list = termsByOperator.get(term.operator) || [];
      list.push(term);
      termsByOperator.set(term.operator, list);
    });

    return {
      original: this.query,
      enabledTerms,
      termsByOperator,
      normalization: {
        deduplicatedCount: this.query.terms.length - new Set(
          this.query.terms.map(t => t.value.toLowerCase())
        ).size,
        removedInvalidCount: this.query.terms.filter(t => !t.enabled).length
      }
    };
  }

  /**
   * 转换为 JSON
   */
  toJSON(): SearchQuery {
    return JSON.parse(JSON.stringify(this.query));
  }

  /**
   * 导出为字符串
   */
  export(): string {
    return JSON.stringify(this.query, null, 2);
  }

  /**
   * 设置全局操作符
   */
  setGlobalOperator(operator: QueryOperator): this {
    this.query.globalOperator = operator;
    this.updateMetadata();
    return this;
  }

  /**
   * 获取查询摘要
   */
  getSummary() {
    return {
      termCount: this.query.terms.length,
      enabledTermCount: this.query.terms.filter(t => t.enabled).length,
      hasPreset: this.query.terms.some(t => t.source === 'preset'),
      hasRegex: this.query.terms.some(t => t.isRegex),
      globalOperator: this.query.globalOperator
    };
  }

  /**
   * 获取所有项（包括禁用的）
   */
  getTerms(): SearchTerm[] {
    return [...this.query.terms];
  }

  /**
   * 获取启用的项
   */
  getEnabledTerms(): SearchTerm[] {
    return this.query.terms.filter(t => t.enabled);
  }

  /**
   * 检查是否包含某个关键词
   */
  has(value: string): boolean {
    return this.query.terms.some(t => 
      t.value.toLowerCase() === value.toLowerCase()
    );
  }

  /**
   * 根据值查找项
   */
  findTermByValue(value: string): SearchTerm | undefined {
    return this.query.terms.find(t => 
      t.value.toLowerCase() === value.toLowerCase()
    );
  }

  /**
   * 获取查询对象
   */
  getQuery(): SearchQuery {
    return { ...this.query };
  }

  private updateMetadata(): void {
    this.query.metadata.lastModified = Date.now();
  }
}
