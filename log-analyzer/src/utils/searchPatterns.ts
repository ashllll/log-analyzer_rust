/**
 * Regex metacharacters that are unlikely to appear in natural text.
 * Excludes `.` (dots appear in Java class names, IPs, file paths, URLs)
 * and `$` (appears in dollar amounts, shell variables).
 * Only characters that unambiguously indicate regex intent are included.
 */
const REGEX_META_CHARS = /[()[\]{}+*?|^\\]/;

export const looksLikeRegexPattern = (value: string): boolean => {
  const trimmed = value.trim();
  return trimmed.length > 0 && REGEX_META_CHARS.test(trimmed);
};

export const escapeRegexLiteral = (value: string): string =>
  value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
