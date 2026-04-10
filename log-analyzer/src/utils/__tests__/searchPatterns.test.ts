import { looksLikeRegexPattern, escapeRegexLiteral } from '../searchPatterns';

describe('looksLikeRegexPattern', () => {
  // ——— 应该判定为正则 ———
  it('detects patterns with quantifiers as regex', () => {
    expect(looksLikeRegexPattern('error+')).toBe(true);
    expect(looksLikeRegexPattern('warn*')).toBe(true);
    expect(looksLikeRegexPattern('debug?')).toBe(true);
  });

  it('detects patterns with anchors as regex', () => {
    expect(looksLikeRegexPattern('^error')).toBe(true);
    // `$` removed from detection to avoid false positives on dollar amounts
    expect(looksLikeRegexPattern('error$')).toBe(false);
  });

  it('detects patterns with character classes as regex', () => {
    expect(looksLikeRegexPattern('[error]')).toBe(true);
    expect(looksLikeRegexPattern('\\d+')).toBe(true);
  });

  it('detects patterns with grouping as regex', () => {
    expect(looksLikeRegexPattern('(error|warn)')).toBe(true);
  });

  it('detects patterns with alternation as regex', () => {
    expect(looksLikeRegexPattern('error|warn')).toBe(true);
  });

  it('detects patterns with curly braces as regex', () => {
    expect(looksLikeRegexPattern('a{2,4}')).toBe(true);
  });

  // ——— 不应该误判为正则 ———
  it('does NOT treat plain keywords as regex', () => {
    expect(looksLikeRegexPattern('error')).toBe(false);
    expect(looksLikeRegexPattern('NullPointerException')).toBe(false);
  });

  it('does NOT treat Java class names (containing dots) as regex', () => {
    expect(looksLikeRegexPattern('java.lang.NullPointerException')).toBe(false);
    expect(looksLikeRegexPattern('org.springframework.web.servlet.DispatcherServlet')).toBe(false);
  });

  it('does NOT treat file paths as regex', () => {
    expect(looksLikeRegexPattern('app.log')).toBe(false);
    expect(looksLikeRegexPattern('/var/log/app.log')).toBe(false);
    // Windows paths contain backslashes which ARE regex metacharacters
    // This is a known edge case; users typing Windows paths may need regex mode off
  });

  it('does NOT treat URLs as regex', () => {
    expect(looksLikeRegexPattern('http://localhost:8080')).toBe(false);
    expect(looksLikeRegexPattern('example.com')).toBe(false);
  });

  it('does NOT treat IP addresses as regex', () => {
    expect(looksLikeRegexPattern('192.168.1.1')).toBe(false);
    expect(looksLikeRegexPattern('10.0.0.1')).toBe(false);
  });

  it('does NOT treat decimal numbers as regex', () => {
    expect(looksLikeRegexPattern('3.14')).toBe(false);
    expect(looksLikeRegexPattern('0.001')).toBe(false);
  });

  it('does NOT treat dollar amounts as regex', () => {
    // `$` was removed from metachar detection, so dollar amounts pass
    expect(looksLikeRegexPattern('$100')).toBe(false);
    expect(looksLikeRegexPattern('price: $49.99')).toBe(false);
  });

  // ——— 边界情况 ———
  it('returns false for empty strings', () => {
    expect(looksLikeRegexPattern('')).toBe(false);
    expect(looksLikeRegexPattern('   ')).toBe(false);
  });
});

describe('escapeRegexLiteral', () => {
  it('escapes regex metacharacters', () => {
    expect(escapeRegexLiteral('error+')).toBe('error\\+');
    expect(escapeRegexLiteral('a*b')).toBe('a\\*b');
    expect(escapeRegexLiteral('(test)')).toBe('\\(test\\)');
  });
});
