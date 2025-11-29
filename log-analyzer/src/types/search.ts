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
