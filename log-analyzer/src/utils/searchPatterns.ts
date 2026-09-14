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
  value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/**
 * Smart split of a query string by `|` (pipe).
 *
 * Rules:
 * - `|` at the top level (not inside any bracket pair) is a term separator.
 * - `\|` is treated as a literal `|` and does NOT split.
 * - `|` inside `()`, `[]`, or `{}` is protected and does NOT split.
 *
 * Examples:
 *   "error | timeout"          → ["error", "timeout"]
 *   "(error|timeout)"          → ["(error|timeout)"]
 *   "foo\\|bar"                → ["foo|bar"]
 *   "a | (b|c) | d"            → ["a", "(b|c)", "d"]
 */
export interface SplitQueryResult {
  terms: string[];
  /** escapedFlags[i] === true 表示 terms[i] 含转义管道（字面量 `|` 意图） */
  escapedFlags: boolean[];
}

export function splitQueryByPipeWithFlags(query: string): SplitQueryResult {
  const terms: string[] = [];
  const escapedFlags: boolean[] = [];
  let current = "";
  let currentHasEscapedPipe = false;
  let depth = 0;
  let escaped = false;

  for (let i = 0; i < query.length; i++) {
    const char = query[i];

    if (escaped) {
      if (char === "|") {
        current += "|";
        currentHasEscapedPipe = true;
      } else {
        current += "\\" + char;
      }
      escaped = false;
      continue;
    }

    if (char === "\\") {
      escaped = true;
      continue;
    }

    if (char === "(" || char === "[" || char === "{") {
      depth++;
      current += char;
      continue;
    }

    if (char === ")" || char === "]" || char === "}") {
      if (depth > 0) {
        depth--;
      }
      current += char;
      continue;
    }

    if (char === "|" && depth === 0) {
      const trimmed = current.trim();
      if (trimmed.length > 0) {
        terms.push(trimmed);
        escapedFlags.push(currentHasEscapedPipe);
      }
      current = "";
      currentHasEscapedPipe = false;
      continue;
    }

    current += char;
  }

  // dangling backslash at end
  if (escaped) {
    current += "\\";
  }

  const trimmed = current.trim();
  if (trimmed.length > 0) {
    terms.push(trimmed);
    escapedFlags.push(currentHasEscapedPipe);
  }

  return { terms, escapedFlags };
}

export function splitQueryByPipe(query: string): string[] {
  return splitQueryByPipeWithFlags(query).terms;
}
