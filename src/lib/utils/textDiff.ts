/**
 * Text Diff Utility for Revision Comparison
 * Uses word-level diffing with LCS (Longest Common Subsequence) algorithm
 */

export interface DiffSegment {
  type: "unchanged" | "added" | "removed" | "modified";
  text: string;
  oldText?: string; // For 'modified' type - what was replaced
  changeIndex?: number; // For navigation (1-indexed)
}

/**
 * Strip HTML tags and decode entities for clean text comparison
 * Uses DOMParser instead of innerHTML to prevent XSS attacks
 */
function stripHtml(html: string): string {
  const parser = new DOMParser();
  const doc = parser.parseFromString(html, "text/html");
  return doc.body.textContent || "";
}

/**
 * Tokenize text into words while preserving whitespace
 */
function tokenize(text: string): string[] {
  // Split on whitespace boundaries while keeping whitespace as separate tokens
  return text.split(/(\s+)/).filter((t) => t.length > 0);
}

/**
 * Compute LCS (Longest Common Subsequence) table
 */
function computeLCS(a: string[], b: string[]): number[][] {
  const m = a.length;
  const n = b.length;
  const dp: number[][] = Array(m + 1)
    .fill(null)
    .map(() => Array(n + 1).fill(0));

  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (a[i - 1] === b[j - 1]) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }
  return dp;
}

/**
 * Build diff segments by backtracking through LCS table
 */
function buildDiffSegments(
  oldTokens: string[],
  newTokens: string[],
  dp: number[][],
): DiffSegment[] {
  const segments: DiffSegment[] = [];
  let i = oldTokens.length;
  let j = newTokens.length;

  // Temporary arrays for building segments in reverse
  const temp: DiffSegment[] = [];

  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && oldTokens[i - 1] === newTokens[j - 1]) {
      // Same token - unchanged
      temp.push({ type: "unchanged", text: oldTokens[i - 1] });
      i--;
      j--;
    } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      // Token in new but not in old - added
      temp.push({ type: "added", text: newTokens[j - 1] });
      j--;
    } else if (i > 0) {
      // Token in old but not in new - removed
      temp.push({ type: "removed", text: oldTokens[i - 1] });
      i--;
    }
  }

  // Reverse to get correct order
  temp.reverse();

  // Merge consecutive segments of the same type
  for (const seg of temp) {
    if (segments.length > 0 && segments[segments.length - 1].type === seg.type) {
      segments[segments.length - 1].text += seg.text;
    } else {
      segments.push({ ...seg });
    }
  }

  return segments;
}

/**
 * Detect modifications by finding adjacent remove+add pairs and merging them
 * into a single 'modified' segment. Also assigns changeIndex for navigation.
 * @param segments - Array of diff segments from buildDiffSegments
 * @returns New array with merged 'modified' segments and change indices
 */
export function detectModifications(segments: DiffSegment[]): DiffSegment[] {
  const result: DiffSegment[] = [];
  let changeIndex = 0;

  for (let i = 0; i < segments.length; i++) {
    const current = segments[i];
    const next = segments[i + 1];

    // Check for removed immediately followed by added (modification pattern)
    if (current.type === "removed" && next && next.type === "added") {
      changeIndex++;
      result.push({
        type: "modified",
        text: next.text, // The new text
        oldText: current.text, // What was replaced
        changeIndex,
      });
      i++; // Skip the next segment since we merged it
    } else if (current.type === "unchanged") {
      result.push({ ...current });
    } else {
      // Standalone added or removed
      changeIndex++;
      result.push({
        ...current,
        changeIndex,
      });
    }
  }

  return result;
}

/**
 * Compute word-level diff between two text strings
 * @param oldText - The older/previous text (can be HTML)
 * @param newText - The newer/current text (can be HTML)
 * @param stripTags - Whether to strip HTML tags before comparison (default: true)
 * @returns Array of diff segments
 */
export function computeWordDiff(oldText: string, newText: string, stripTags = true): DiffSegment[] {
  // Handle edge cases
  if (!oldText && !newText) return [];
  if (!oldText) return [{ type: "added", text: stripTags ? stripHtml(newText) : newText }];
  if (!newText) return [{ type: "removed", text: stripTags ? stripHtml(oldText) : oldText }];

  // Optionally strip HTML
  const cleanOld = stripTags ? stripHtml(oldText) : oldText;
  const cleanNew = stripTags ? stripHtml(newText) : newText;

  // Tokenize
  const oldTokens = tokenize(cleanOld);
  const newTokens = tokenize(cleanNew);

  // Early return if texts are identical
  if (cleanOld === cleanNew) {
    return [{ type: "unchanged", text: cleanNew }];
  }

  // Compute LCS and build diff
  const dp = computeLCS(oldTokens, newTokens);
  return buildDiffSegments(oldTokens, newTokens, dp);
}

/**
 * Options for diffToHtml rendering
 */
export interface DiffToHtmlOptions {
  showWhitespace?: boolean;
}

/**
 * Convert diff segments to HTML with highlighting classes
 * @param segments - Array of diff segments
 * @param options - Optional rendering options
 * @returns HTML string with span tags for styling
 */
export function diffToHtml(segments: DiffSegment[], options: DiffToHtmlOptions = {}): string {
  const { showWhitespace = false } = options;

  return segments
    .map((seg) => {
      // Step 1: Get the raw text
      let text = seg.text;
      let oldText = seg.oldText || "";

      // Step 2: If showWhitespace, replace whitespace chars with Unicode symbols BEFORE escaping
      // This prevents corruption of HTML entities (e.g., &nbsp; -> &·nbsp;)
      if (showWhitespace) {
        text = visualizeWhitespace(text);
        oldText = visualizeWhitespace(oldText);
      }

      // Step 3: Escape HTML (this won't affect the Unicode symbols)
      const escaped = escapeHtml(text);
      const escapedOld = escapeHtml(oldText);

      // Step 4: Wrap Unicode whitespace symbols with styling spans AFTER escaping
      const finalText = showWhitespace ? wrapWhitespaceChars(escaped) : escaped;
      const finalOldText = showWhitespace ? wrapWhitespaceChars(escapedOld) : escapedOld;

      const dataChange = seg.changeIndex ? ` data-change="${seg.changeIndex}"` : "";

      switch (seg.type) {
        case "added":
          return `<span class="diff-added"${dataChange}>${finalText}</span>`;
        case "removed":
          return `<span class="diff-removed"${dataChange}>${finalText}</span>`;
        case "modified":
          // Show both old (strikethrough) and new (highlighted) text
          return `<span class="diff-modified"${dataChange}><span class="diff-modified-old">${finalOldText}</span><span class="diff-modified-new">${finalText}</span></span>`;
        default:
          return finalText;
      }
    })
    .join("");
}

/**
 * Escape HTML special characters
 */
function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

/**
 * Check if there are any changes in the diff
 */
export function hasChanges(segments: DiffSegment[]): boolean {
  return segments.some((seg) => seg.type !== "unchanged");
}

/**
 * Get statistics about the diff
 */
export function getDiffStats(segments: DiffSegment[]): {
  addedWords: number;
  removedWords: number;
  modifiedSegments: number;
  unchangedWords: number;
} {
  let addedWords = 0;
  let removedWords = 0;
  let modifiedSegments = 0;
  let unchangedWords = 0;

  for (const seg of segments) {
    const wordCount = seg.text
      .trim()
      .split(/\s+/)
      .filter((w) => w.length > 0).length;
    switch (seg.type) {
      case "added":
        addedWords += wordCount;
        break;
      case "removed":
        removedWords += wordCount;
        break;
      case "modified": {
        modifiedSegments++;
        // Count the old text as removed words
        const oldWordCount = (seg.oldText || "")
          .trim()
          .split(/\s+/)
          .filter((w) => w.length > 0).length;
        removedWords += oldWordCount;
        // Count the new text as added words
        addedWords += wordCount;
        break;
      }
      default:
        unchangedWords += wordCount;
    }
  }

  return { addedWords, removedWords, modifiedSegments, unchangedWords };
}

/**
 * Visualize whitespace characters by replacing them with visible symbols
 * @param text - The text to process
 * @returns Text with whitespace replaced by visible symbols
 */
export function visualizeWhitespace(text: string): string {
  return text
    .replace(/\n/g, "\u00B6\n") // Add pilcrow before newlines
    .replace(/\t/g, "\u2192") // Replace tabs with arrows
    .replace(/ /g, "\u00B7") // Replace spaces with middle dots
    .replace(/\u00A0/g, "\u00B0"); // Non-breaking space with degree sign
}

/**
 * Wrap whitespace symbols with span tags for styling
 * @param html - HTML string containing whitespace symbols
 * @returns HTML with wrapped whitespace symbols
 */
export function wrapWhitespaceChars(html: string): string {
  return html
    .replace(/\u00B6/g, '<span class="ws-char ws-newline">\u00B6</span>')
    .replace(/\u2192/g, '<span class="ws-char ws-tab">\u2192</span>')
    .replace(/\u00B7/g, '<span class="ws-char ws-space">\u00B7</span>')
    .replace(/\u00B0/g, '<span class="ws-char ws-nbsp">\u00B0</span>');
}
