/**
 * 搜索查询类型定义
 * 
 * 本文件定义了结构化查询模型的所有核心类型
 */

/**
 * 查询操作符
 */
export type QueryOperator = 'AND' | 'OR' | 'NOT';

/**
 * 关键词来源
 */
export type TermSource = 'user' | 'preset';

/**
 * 验证严重性
 */
export type ValidationSeverity = 'error' | 'warning' | 'info';

/**
 * 单个搜索条件
 */
export interface SearchTerm {
  /** 唯一标识符 */
  id: string;
  
  /** 搜索值 */
  value: string;
  
  /** 操作符（AND/OR/NOT） */
  operator: QueryOperator;
  
  /** 来源（用户输入/预置） */
  source: TermSource;
  
  /** 关联的预置组 ID（如果是预置关键词） */
  presetGroupId?: string;
  
  /** 是否为正则表达式 */
  isRegex: boolean;
  
  /** 优先级（0-100） */
  priority: number;
  
  /** 是否启用 */
  enabled: boolean;
  
  /** 是否区分大小写 */
  caseSensitive: boolean;
}

/**
 * 查询元数据
 */
export interface QueryMetadata {
  createdAt: number;
  lastModified: number;
  executionCount: number;
  label?: string;
}

/**
 * 时间范围
 */
export interface TimeRange {
  start?: string;
  end?: string;
}

/**
 * 搜索过滤器
 */
export interface SearchFilters {
  levels?: string[];
  timeRange?: TimeRange;
  filePattern?: string;
}

/**
 * 完整搜索查询
 */
export interface SearchQuery {
  /** 查询 ID */
  id: string;
  
  /** 搜索条件列表 */
  terms: SearchTerm[];
  
  /** 全局操作符 */
  globalOperator: QueryOperator;
  
  /** 高级过滤器 */
  filters?: SearchFilters;
  
  /** 元数据 */
  metadata: QueryMetadata;
}

/**
 * 验证问题
 */
export interface ValidationIssue {
  termId?: string;
  severity: ValidationSeverity;
  code: string;
  message: string;
}

/**
 * 查询验证结果
 */
export interface QueryValidation {
  isValid: boolean;
  issues: ValidationIssue[];
}

/**
 * 优化后的查询
 */
export interface OptimizedQuery {
  original: SearchQuery;
  enabledTerms: SearchTerm[];
  termsByOperator: Map<QueryOperator, SearchTerm[]>;
  normalization: {
    deduplicatedCount: number;
    removedInvalidCount: number;
  };
}

/**
 * 关键词统计信息
 * 用于显示每个关键词的匹配数量和占比
 */
export interface KeywordStatistics {
  /** 关键词文本 */
  keyword: string;
  
  /** 该关键词匹配的总行数 */
  matchCount: number;
  
  /** 占总结果的百分比 */
  matchPercentage: number;
}

/**
 * 搜索结果摘要
 * 包含所有关键词的统计信息和搜索元数据
 */
export interface SearchResultSummary {
  /** 总匹配行数 */
  totalMatches: number;
  
  /** 关键词统计数组 */
  keywordStats: KeywordStatistics[];
  
  /** 搜索耗时（毫秒） */
  searchDurationMs: number;
  
  /** 是否因超限截断 */
  truncated: boolean;
}

/**
 * 关键词统计（前端扩展）
 * 在KeywordStatistics基础上添加高亮颜色
 */
export interface KeywordStat extends KeywordStatistics {
  /** 高亮颜色 */
  color: string;
}
