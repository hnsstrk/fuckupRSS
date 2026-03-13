import { describe, it, expect } from "vitest";

function parseStructuredContent(content: string) {
  try {
    const parsed = JSON.parse(content);
    if (parsed.tldr && parsed.topics) {
      return parsed;
    }
  } catch {
    // Legacy markdown briefing
  }
  return null;
}

function parseArticleRefs(refsJson: string | null) {
  if (!refsJson) return [];
  try {
    return JSON.parse(refsJson);
  } catch {
    return [];
  }
}

describe("BriefingView Helpers", () => {
  describe("parseStructuredContent", () => {
    it("parses valid structured briefing JSON", () => {
      const content = JSON.stringify({
        tldr: { overview: "Test", trends: "Trends", conclusion: "Fazit" },
        topics: [
          {
            title: "Topic 1",
            body: "Text",
            article_indices: [0],
            keywords: ["DAX"],
          },
        ],
      });
      const result = parseStructuredContent(content);
      expect(result).not.toBeNull();
      expect(result!.tldr.overview).toBe("Test");
      expect(result!.topics).toHaveLength(1);
    });

    it("returns null for legacy markdown content", () => {
      const content = "## Überblick\n\nDies ist ein altes Briefing.";
      expect(parseStructuredContent(content)).toBeNull();
    });

    it("returns null for invalid JSON", () => {
      expect(parseStructuredContent("{broken")).toBeNull();
    });

    it("returns null for JSON without tldr", () => {
      const content = JSON.stringify({ topics: [] });
      expect(parseStructuredContent(content)).toBeNull();
    });

    it("returns null for JSON without topics", () => {
      const content = JSON.stringify({
        tldr: { overview: "x", trends: "y", conclusion: "z" },
      });
      expect(parseStructuredContent(content)).toBeNull();
    });
  });

  describe("parseArticleRefs", () => {
    it("parses valid article refs", () => {
      const refs = JSON.stringify([
        { index: 0, fnord_id: 42, title: "Test", source: "RSS" },
      ]);
      const result = parseArticleRefs(refs);
      expect(result).toHaveLength(1);
      expect(result[0].fnord_id).toBe(42);
    });

    it("returns empty array for null", () => {
      expect(parseArticleRefs(null)).toEqual([]);
    });

    it("returns empty array for invalid JSON", () => {
      expect(parseArticleRefs("{broken")).toEqual([]);
    });

    it("returns empty array for empty string", () => {
      expect(parseArticleRefs("")).toEqual([]);
    });
  });
});
