import {
  keywordGroupSchema,
  validateRegexPattern,
  validateKeywordGroup,
  formatValidationErrors,
} from '../keywordSchema';

describe('validateRegexPattern', () => {
  it('应视空字符串为有效', () => {
    const result = validateRegexPattern('');
    expect(result.valid).toBe(true);
    expect(result.error).toBeUndefined();
  });

  it('应视仅含空白字符的字符串为有效', () => {
    const result = validateRegexPattern('   ');
    expect(result.valid).toBe(true);
  });

  it('应验证合法正则表达式', () => {
    const result = validateRegexPattern('^[a-z]+$');
    expect(result.valid).toBe(true);
    expect(result.error).toBeUndefined();
  });

  it('应拒绝无效正则表达式', () => {
    const result = validateRegexPattern('[');
    expect(result.valid).toBe(false);
    expect(result.error).toBeDefined();
  });

  it('应拒绝含非法标志的正则（由 new RegExp 抛出）', () => {
    const result = validateRegexPattern('(?<invalid');
    expect(result.valid).toBe(false);
    expect(result.error).toBeDefined();
  });
});

describe('validateKeywordGroup', () => {
  it('应验证通过合法的关键词组数据', () => {
    const data = {
      name: '  Error Patterns  ',
      color: 'red' as const,
      patterns: [
        { regex: 'ERROR', comment: '错误日志' },
        { regex: 'FATAL', comment: '致命错误' },
      ],
    };
    const result = validateKeywordGroup(data);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.name).toBe('Error Patterns');
    }
  });

  it('应拒绝名称过短的数据', () => {
    const data = {
      name: 'A',
      color: 'blue' as const,
      patterns: [{ regex: 'test', comment: '' }],
    };
    const result = validateKeywordGroup(data);
    expect(result.success).toBe(false);
    if (!result.success) {
      const errors = formatValidationErrors(result);
      expect(errors.name).toBeDefined();
    }
  });

  it('应拒绝名称过长的数据', () => {
    const data = {
      name: 'A'.repeat(51),
      color: 'blue' as const,
      patterns: [{ regex: 'test', comment: '' }],
    };
    const result = validateKeywordGroup(data);
    expect(result.success).toBe(false);
    if (!result.success) {
      const errors = formatValidationErrors(result);
      expect(errors.name).toBeDefined();
    }
  });

  it('应拒绝非法颜色值', () => {
    const data = {
      name: 'Valid Name',
      color: 'yellow' as any,
      patterns: [{ regex: 'test', comment: '' }],
    };
    const result = validateKeywordGroup(data);
    expect(result.success).toBe(false);
    if (!result.success) {
      const errors = formatValidationErrors(result);
      expect(errors.color).toBeDefined();
    }
  });

  it('应拒绝空 patterns 数组', () => {
    const data = {
      name: 'Valid Name',
      color: 'blue' as const,
      patterns: [] as { regex: string; comment: string }[],
    };
    const result = validateKeywordGroup(data);
    expect(result.success).toBe(false);
    if (!result.success) {
      const errors = formatValidationErrors(result);
      expect(errors.patterns).toBeDefined();
    }
  });

  it('应拒绝全部为空正则的 patterns', () => {
    const data = {
      name: 'Valid Name',
      color: 'blue' as const,
      patterns: [
        { regex: '', comment: '空模式' },
        { regex: '   ', comment: '空白模式' },
      ],
    };
    const result = validateKeywordGroup(data);
    expect(result.success).toBe(false);
    if (!result.success) {
      const errors = formatValidationErrors(result);
      expect(errors.patterns).toBeDefined();
    }
  });

  it('应拒绝含重复正则模式的数据', () => {
    const data = {
      name: 'Valid Name',
      color: 'blue' as const,
      patterns: [
        { regex: 'ERROR', comment: '错误' },
        { regex: 'ERROR', comment: '重复错误' },
      ],
    };
    const result = validateKeywordGroup(data);
    expect(result.success).toBe(false);
    if (!result.success) {
      const errors = formatValidationErrors(result);
      expect(errors.patterns).toBeDefined();
    }
  });

  it('应拒绝含无效正则语法的数据', () => {
    const data = {
      name: 'Valid Name',
      color: 'blue' as const,
      patterns: [{ regex: '[invalid', comment: '坏正则' }],
    };
    const result = validateKeywordGroup(data);
    expect(result.success).toBe(false);
    if (!result.success) {
      const errors = formatValidationErrors(result);
      expect(errors.patterns).toBeDefined();
    }
  });

  it('应视 trim 后相同的模式为重复', () => {
    const data = {
      name: 'Valid Name',
      color: 'blue' as const,
      patterns: [
        { regex: 'ERROR', comment: '错误' },
        { regex: '  ERROR  ', comment: '带空格的重复' },
      ],
    };
    const result = validateKeywordGroup(data);
    expect(result.success).toBe(false);
    if (!result.success) {
      const errors = formatValidationErrors(result);
      expect(errors.patterns).toBeDefined();
    }
  });
});

describe('formatValidationErrors', () => {
  it('成功验证时返回空对象', () => {
    const result = validateKeywordGroup({
      name: 'Test',
      color: 'blue',
      patterns: [{ regex: 'test', comment: '' }],
    });
    const errors = formatValidationErrors(result);
    expect(errors).toEqual({});
  });

  it('应提取 name 字段错误', () => {
    const result = validateKeywordGroup({
      name: 'X',
      color: 'blue',
      patterns: [{ regex: 'test', comment: '' }],
    });
    const errors = formatValidationErrors(result);
    expect(errors.name).toBeDefined();
    expect(errors.name).toContain('2');
  });

  it('应提取 color 字段错误', () => {
    const result = validateKeywordGroup({
      name: 'Test',
      color: 'invalid' as any,
      patterns: [{ regex: 'test', comment: '' }],
    });
    const errors = formatValidationErrors(result);
    expect(errors.color).toBeDefined();
  });

  it('应提取 patterns 数组级别错误（空数组）', () => {
    const result = validateKeywordGroup({
      name: 'Test',
      color: 'blue',
      patterns: [],
    });
    const errors = formatValidationErrors(result);
    expect(errors.patterns).toBeDefined();
    expect(errors.patterns!.length).toBeGreaterThan(0);
  });

  it('应提取 patterns 数组级别错误（全部为空）', () => {
    const result = validateKeywordGroup({
      name: 'Test',
      color: 'blue',
      patterns: [{ regex: '', comment: '' }],
    });
    const errors = formatValidationErrors(result);
    expect(errors.patterns).toBeDefined();
  });

  it('应提取 patterns 数组级别错误（重复模式）', () => {
    const result = validateKeywordGroup({
      name: 'Test',
      color: 'blue',
      patterns: [
        { regex: 'dup', comment: '' },
        { regex: 'dup', comment: '' },
      ],
    });
    const errors = formatValidationErrors(result);
    expect(errors.patterns).toBeDefined();
  });

  it('应提取单个 pattern 的 regex 字段错误', () => {
    const result = validateKeywordGroup({
      name: 'Test',
      color: 'blue',
      patterns: [{ regex: '[bad', comment: '' }],
    });
    const errors = formatValidationErrors(result);
    expect(errors.patterns).toBeDefined();
    expect(errors.patterns!.some((e) => e.includes('正则'))).toBe(true);
  });
});
