const REGEX_META_CHARS = /[()[\]{}+*?|^$.\\]/;

export const looksLikeRegexPattern = (value: string): boolean => {
  const trimmed = value.trim();
  return trimmed.length > 0 && REGEX_META_CHARS.test(trimmed);
};

export const escapeRegexLiteral = (value: string): string =>
  value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
