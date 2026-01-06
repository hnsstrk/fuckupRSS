/**
 * Text Diff Utility for Revision Comparison
 * Uses word-level diffing with LCS (Longest Common Subsequence) algorithm
 */

export interface DiffSegment {
  type: 'unchanged' | 'added' | 'removed';
  text: string;
}

/**
 * Strip HTML tags and decode entities for clean text comparison
 */
function stripHtml(html: string): string {
  // Create a temporary element to decode HTML entities
  const tmp = document.createElement('div');
  tmp.innerHTML = html;
  // Get text content which strips tags and decodes entities
  return tmp.textContent || tmp.innerText || '';
}

/**
 * Tokenize text into words while preserving whitespace
 */
function tokenize(text: string): string[] {
  // Split on whitespace boundaries while keeping whitespace as separate tokens
  return text.split(/(\s+)/).filter(t => t.length > 0);
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
  dp: number[][]
): DiffSegment[] {
  const segments: DiffSegment[] = [];
  let i = oldTokens.length;
  let j = newTokens.length;

  // Temporary arrays for building segments in reverse
  const temp: DiffSegment[] = [];

  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && oldTokens[i - 1] === newTokens[j - 1]) {
      // Same token - unchanged
      temp.push({ type: 'unchanged', text: oldTokens[i - 1] });
      i--;
      j--;
    } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      // Token in new but not in old - added
      temp.push({ type: 'added', text: newTokens[j - 1] });
      j--;
    } else if (i > 0) {
      // Token in old but not in new - removed
      temp.push({ type: 'removed', text: oldTokens[i - 1] });
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
 * Compute word-level diff between two text strings
 * @param oldText - The older/previous text (can be HTML)
 * @param newText - The newer/current text (can be HTML)
 * @param stripTags - Whether to strip HTML tags before comparison (default: true)
 * @returns Array of diff segments
 */
export function computeWordDiff(
  oldText: string,
  newText: string,
  stripTags = true
): DiffSegment[] {
  // Handle edge cases
  if (!oldText && !newText) return [];
  if (!oldText) return [{ type: 'added', text: stripTags ? stripHtml(newText) : newText }];
  if (!newText) return [{ type: 'removed', text: stripTags ? stripHtml(oldText) : oldText }];

  // Optionally strip HTML
  const cleanOld = stripTags ? stripHtml(oldText) : oldText;
  const cleanNew = stripTags ? stripHtml(newText) : newText;

  // Tokenize
  const oldTokens = tokenize(cleanOld);
  const newTokens = tokenize(cleanNew);

  // Early return if texts are identical
  if (cleanOld === cleanNew) {
    return [{ type: 'unchanged', text: cleanNew }];
  }

  // Compute LCS and build diff
  const dp = computeLCS(oldTokens, newTokens);
  return buildDiffSegments(oldTokens, newTokens, dp);
}

/**
 * Convert diff segments to HTML with highlighting classes
 * @param segments - Array of diff segments
 * @returns HTML string with span tags for styling
 */
export function diffToHtml(segments: DiffSegment[]): string {
  return segments
    .map(seg => {
      const escaped = escapeHtml(seg.text);
      switch (seg.type) {
        case 'added':
          return `<span class="diff-added">${escaped}</span>`;
        case 'removed':
          return `<span class="diff-removed">${escaped}</span>`;
        default:
          return escaped;
      }
    })
    .join('');
}

/**
 * Escape HTML special characters
 */
function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

/**
 * Check if there are any changes in the diff
 */
export function hasChanges(segments: DiffSegment[]): boolean {
  return segments.some(seg => seg.type !== 'unchanged');
}

/**
 * Get statistics about the diff
 */
export function getDiffStats(segments: DiffSegment[]): {
  addedWords: number;
  removedWords: number;
  unchangedWords: number;
} {
  let addedWords = 0;
  let removedWords = 0;
  let unchangedWords = 0;

  for (const seg of segments) {
    const wordCount = seg.text.trim().split(/\s+/).filter(w => w.length > 0).length;
    switch (seg.type) {
      case 'added':
        addedWords += wordCount;
        break;
      case 'removed':
        removedWords += wordCount;
        break;
      default:
        unchangedWords += wordCount;
    }
  }

  return { addedWords, removedWords, unchangedWords };
}
