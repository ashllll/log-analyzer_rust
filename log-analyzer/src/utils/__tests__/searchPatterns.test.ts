import {
  looksLikeRegexPattern,
  escapeRegexLiteral,
  splitQueryByPipe,
  splitQueryByPipeWithFlags,
} from "../searchPatterns";
import { parseQueryFromString } from "../../services/SearchQueryBuilder";

describe("looksLikeRegexPattern", () => {
  it("detects patterns with quantifiers as regex", () => {
    expect(looksLikeRegexPattern("error+")).toBe(true);
    expect(looksLikeRegexPattern("warn*")).toBe(true);
    expect(looksLikeRegexPattern("debug?")).toBe(true);
  });

  it("detects patterns with anchors as regex", () => {
    expect(looksLikeRegexPattern("^error")).toBe(true);
    expect(looksLikeRegexPattern("error$")).toBe(false);
  });

  it("detects patterns with character classes as regex", () => {
    expect(looksLikeRegexPattern("[error]")).toBe(true);
    expect(looksLikeRegexPattern("\\d+")).toBe(true);
  });

  it("detects patterns with grouping as regex", () => {
    expect(looksLikeRegexPattern("(error|warn)")).toBe(true);
  });

  it("detects patterns with alternation as regex", () => {
    expect(looksLikeRegexPattern("error|warn")).toBe(true);
  });

  it("detects patterns with curly braces as regex", () => {
    expect(looksLikeRegexPattern("a{2,4}")).toBe(true);
  });

  it("does NOT treat plain keywords as regex", () => {
    expect(looksLikeRegexPattern("error")).toBe(false);
    expect(looksLikeRegexPattern("NullPointerException")).toBe(false);
  });

  it("does NOT treat Java class names (containing dots) as regex", () => {
    expect(looksLikeRegexPattern("java.lang.NullPointerException")).toBe(false);
    expect(
      looksLikeRegexPattern("org.springframework.web.servlet.DispatcherServlet")
    ).toBe(false);
  });

  it("does NOT treat file paths as regex", () => {
    expect(looksLikeRegexPattern("app.log")).toBe(false);
    expect(looksLikeRegexPattern("/var/log/app.log")).toBe(false);
  });

  it("does NOT treat URLs as regex", () => {
    expect(looksLikeRegexPattern("http://localhost:8080")).toBe(false);
    expect(looksLikeRegexPattern("example.com")).toBe(false);
  });

  it("does NOT treat IP addresses as regex", () => {
    expect(looksLikeRegexPattern("192.168.1.1")).toBe(false);
    expect(looksLikeRegexPattern("10.0.0.1")).toBe(false);
  });

  it("does NOT treat decimal numbers as regex", () => {
    expect(looksLikeRegexPattern("3.14")).toBe(false);
    expect(looksLikeRegexPattern("0.001")).toBe(false);
  });

  it("does NOT treat dollar amounts as regex", () => {
    expect(looksLikeRegexPattern("$100")).toBe(false);
    expect(looksLikeRegexPattern("price: $49.99")).toBe(false);
  });

  it("returns false for empty strings", () => {
    expect(looksLikeRegexPattern("")).toBe(false);
    expect(looksLikeRegexPattern("   ")).toBe(false);
  });
});

describe("escapeRegexLiteral", () => {
  it("escapes regex metacharacters", () => {
    expect(escapeRegexLiteral("error+")).toBe("error\\+");
    expect(escapeRegexLiteral("a.b")).toBe("a\\.b");
    expect(escapeRegexLiteral("a|b")).toBe("a\\|b");
  });
});

describe("splitQueryByPipe", () => {
  it("splits basic queries by pipe", () => {
    expect(splitQueryByPipe("error | timeout")).toEqual(["error", "timeout"]);
    expect(splitQueryByPipe("a|b|c")).toEqual(["a", "b", "c"]);
  });

  it("does NOT split pipes inside brackets", () => {
    expect(splitQueryByPipe("(error|timeout)")).toEqual(["(error|timeout)"]);
    expect(splitQueryByPipe("a | (b|c) | d")).toEqual(["a", "(b|c)", "d"]);
    expect(splitQueryByPipe("[foo|bar]")).toEqual(["[foo|bar]"]);
  });

  it("treats escaped pipe as literal", () => {
    expect(splitQueryByPipe("foo\\|bar")).toEqual(["foo|bar"]);
    expect(splitQueryByPipe("a | b\\|c | d")).toEqual(["a", "b|c", "d"]);
  });

  it("handles empty and whitespace-only input", () => {
    expect(splitQueryByPipe("")).toEqual([]);
    expect(splitQueryByPipe("   ")).toEqual([]);
    expect(splitQueryByPipe("  error  |  timeout  ")).toEqual([
      "error",
      "timeout",
    ]);
  });

  it("handles nested brackets correctly", () => {
    expect(splitQueryByPipe("(a|(b|c))")).toEqual(["(a|(b|c))"]);
    expect(splitQueryByPipe("x | (a|(b|c)) | y")).toEqual([
      "x",
      "(a|(b|c))",
      "y",
    ]);
  });

  it("handles mismatched brackets gracefully", () => {
    expect(splitQueryByPipe("a) | b")).toEqual(["a)", "b"]);
    expect(splitQueryByPipe("(a | b")).toEqual(["(a | b"]);
  });
});

describe("splitQueryByPipeWithFlags", () => {
  it("flags terms containing an escaped pipe", () => {
    expect(splitQueryByPipeWithFlags("foo\\|bar")).toEqual({
      terms: ["foo|bar"],
      escapedFlags: [true],
    });
    expect(splitQueryByPipeWithFlags("a | b\\|c | d")).toEqual({
      terms: ["a", "b|c", "d"],
      escapedFlags: [false, true, false],
    });
  });

  it("does not flag terms with only unescaped pipes", () => {
    expect(splitQueryByPipeWithFlags("error | timeout")).toEqual({
      terms: ["error", "timeout"],
      escapedFlags: [false, false],
    });
  });
});

describe("parseQueryFromString escaped pipe", () => {
  it("keeps escaped-pipe terms literal", () => {
    const query = parseQueryFromString("timeout\\|retry", [], {
      caseSensitive: false,
      regexEnabled: true,
    });

    expect(query.terms).toHaveLength(1);
    expect(query.terms[0].value).toBe("timeout|retry");
    expect(query.terms[0].isRegex).toBe(false);
  });

  it("does not interpret a trailing escaped pipe as regex", () => {
    const query = parseQueryFromString("timeout\\|", [], {
      caseSensitive: false,
      regexEnabled: true,
    });

    expect(query.terms[0].value).toBe("timeout|");
    expect(query.terms[0].isRegex).toBe(false);
  });
});
