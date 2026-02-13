//! String-based similarity functions
//!
//! Provides various string similarity metrics including:
//! - Token Set Ratio (for name variants like "Trump" <-> "Donald Trump")
//! - Levenshtein distance
//! - Jaro-Winkler distance
//! - Abbreviation detection

use std::collections::HashSet;
use strsim::{jaro_winkler, normalized_levenshtein};

/// Token Set Ratio: Measures similarity based on token overlap.
///
/// Returns 1.0 if one string's tokens are a subset of the other's tokens.
/// This is ideal for detecting name variants like "Trump" <-> "Donald Trump".
///
/// # Examples
/// ```
/// assert_eq!(token_set_ratio("Trump", "Donald Trump"), 1.0);
/// assert_eq!(token_set_ratio("Donald Trump", "Trump Donald"), 1.0);
/// assert!(token_set_ratio("Trump", "Merkel") < 0.1);
/// ```
pub fn token_set_ratio(a: &str, b: &str) -> f64 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    let a_tokens: HashSet<&str> = a_lower.split_whitespace().collect();
    let b_tokens: HashSet<&str> = b_lower.split_whitespace().collect();

    // Empty check
    if a_tokens.is_empty() || b_tokens.is_empty() {
        return 0.0;
    }

    // If one is a subset of the other -> perfect match for name variants
    if a_tokens.is_subset(&b_tokens) || b_tokens.is_subset(&a_tokens) {
        return 1.0;
    }

    // Otherwise: Jaccard similarity of tokens
    let intersection = a_tokens.intersection(&b_tokens).count();
    let union = a_tokens.union(&b_tokens).count();

    if union > 0 {
        intersection as f64 / union as f64
    } else {
        0.0
    }
}

/// Check if a single-token keyword is an exact token within a multi-token keyword.
/// This is a strong indicator for name variants like "Trump" <-> "Donald Trump".
///
/// Returns a score (0.0-1.0) based on how significant the match is:
/// - 0.85: Single token matches a token in a 2-word keyword
/// - 0.80: Single token matches a token in a 3-word keyword
/// - 0.75: Single token matches a token in a 4+ word keyword
/// - 0.0: No exact token match
pub fn calculate_exact_token_match_score(a: &str, b: &str) -> f64 {
    let a_tokens: Vec<&str> = a.split_whitespace().collect();
    let b_tokens: Vec<&str> = b.split_whitespace().collect();

    // We need exactly one single-token keyword and one multi-token keyword
    let (single, multi) = if a_tokens.len() == 1 && b_tokens.len() > 1 {
        (a_tokens[0], &b_tokens)
    } else if b_tokens.len() == 1 && a_tokens.len() > 1 {
        (b_tokens[0], &a_tokens)
    } else {
        return 0.0;
    };

    // Check if the single token appears exactly in the multi-token keyword
    if multi.iter().any(|t| t.eq_ignore_ascii_case(single)) {
        match multi.len() {
            2 => 0.85, // "Trump" in "Donald Trump" - very strong
            3 => 0.80, // "Trump" in "Donald J Trump" - strong
            _ => 0.75, // Longer phrases - still significant
        }
    } else {
        0.0
    }
}

/// Detect if one string is an abbreviation/acronym of the other.
///
/// # Examples
/// - "EU" <-> "European Union" -> high score
/// - "USA" <-> "United States of America" -> high score
/// - "BRD" <-> "Bundesrepublik Deutschland" -> high score
pub fn calculate_abbreviation_score(short: &str, long: &str) -> f64 {
    // Ensure short is the shorter one
    let (short, long) = if short.len() <= long.len() {
        (short, long)
    } else {
        (long, short)
    };

    // Only check if the short string looks like an acronym (2-6 chars)
    if short.len() < 2 || short.len() > 6 {
        return 0.0;
    }

    // Split long string into words
    let words: Vec<&str> = long.split_whitespace().collect();
    if words.is_empty() {
        return 0.0;
    }

    // Check if short string is formed from first letters of words in long string
    let first_letters: String = words
        .iter()
        .filter(|w| {
            ![
                "and", "of", "the", "for", "in", "der", "die", "das", "und", "für", "von",
            ]
            .contains(&w.to_lowercase().as_str())
        })
        .filter_map(|w| w.chars().next())
        .collect::<String>()
        .to_lowercase();

    if first_letters == short.to_lowercase() {
        return 0.95;
    }

    // Check if acronym matches (ignoring minor words)
    let all_first_letters: String = words
        .iter()
        .filter_map(|w| w.chars().next())
        .collect::<String>()
        .to_lowercase();

    if all_first_letters == short.to_lowercase() {
        return 0.9;
    }

    // Partial match
    let short_chars: Vec<char> = short.to_lowercase().chars().collect();
    let matching = short_chars
        .iter()
        .zip(first_letters.chars())
        .filter(|(a, b)| a == &b)
        .count();

    if matching > 0 && matching >= short_chars.len() / 2 {
        return 0.5 + (matching as f64 / short_chars.len() as f64) * 0.3;
    }

    0.0
}

/// Calculate comprehensive string similarity using multiple methods.
///
/// Combines:
/// 1. Levenshtein distance (for typos)
/// 2. Jaro-Winkler (for short strings and prefix matches)
/// 3. Substring containment
/// 4. Token overlap (Jaccard similarity)
/// 5. Abbreviation detection
/// 6. Exact token match (for name variants)
/// 7. Token Set Ratio (for flexible name matching)
pub fn calculate_string_similarity(a: &str, b: &str) -> f64 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    // 1. Exact match (case-insensitive)
    if a_lower == b_lower {
        return 1.0;
    }

    // 2. Token Set Ratio (highest priority for name variants)
    let token_set = token_set_ratio(&a_lower, &b_lower);
    if token_set >= 1.0 {
        return 1.0; // One is subset of other (e.g., "Trump" in "Donald Trump")
    }

    // 3. Exact token match detection
    let exact_token_score = calculate_exact_token_match_score(&a_lower, &b_lower);

    // 4. Abbreviation detection
    let abbrev_score = calculate_abbreviation_score(&a_lower, &b_lower);

    // 5. Normalized Levenshtein distance
    let levenshtein = normalized_levenshtein(&a_lower, &b_lower);

    // 6. Jaro-Winkler
    let jaro = jaro_winkler(&a_lower, &b_lower);

    // 7. Substring containment
    let substring_score = if a_lower.contains(&b_lower) || b_lower.contains(&a_lower) {
        let ratio = a.len().min(b.len()) as f64 / a.len().max(b.len()) as f64;
        0.3 + (0.4 * ratio)
    } else {
        0.0
    };

    // 8. Token overlap (Jaccard)
    let a_tokens: HashSet<&str> = a_lower.split_whitespace().collect();
    let b_tokens: HashSet<&str> = b_lower.split_whitespace().collect();
    let token_overlap = if !a_tokens.is_empty() && !b_tokens.is_empty() {
        let intersection = a_tokens.intersection(&b_tokens).count();
        let union = a_tokens.union(&b_tokens).count();
        if union > 0 {
            intersection as f64 / union as f64
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Combine scores with weights
    // Priority: token_set > exact_token > abbreviation > substring > token_overlap > string_distance
    let combined = if token_set > 0.5 {
        // High token overlap (but not subset)
        token_set * 0.5 + jaro * 0.25 + levenshtein * 0.25
    } else if exact_token_score > 0.7 {
        // Single token matches multi-token keyword
        exact_token_score * 0.6 + jaro * 0.2 + levenshtein * 0.2
    } else if abbrev_score > 0.7 {
        // Strong abbreviation match
        abbrev_score * 0.6 + levenshtein * 0.2 + jaro * 0.2
    } else if substring_score > 0.5 {
        // Substring match
        substring_score * 0.4 + levenshtein * 0.3 + jaro * 0.3
    } else if token_overlap > 0.5 {
        // Token overlap
        token_overlap * 0.4 + levenshtein * 0.3 + jaro * 0.3
    } else {
        // Fall back to string distance measures
        levenshtein * 0.5 + jaro * 0.5
    };

    combined.clamp(0.0, 1.0)
}

#[cfg(test)]
mod string_tests {
    use super::*;

    #[test]
    fn test_token_set_ratio_subset() {
        assert!((token_set_ratio("Trump", "Donald Trump") - 1.0).abs() < 0.01);
        assert!((token_set_ratio("Donald Trump", "Trump") - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_token_set_ratio_reorder() {
        assert!((token_set_ratio("Donald Trump", "Trump Donald") - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_token_set_ratio_different() {
        assert!(token_set_ratio("Trump", "Merkel") < 0.1);
    }

    #[test]
    fn test_exact_token_match() {
        assert!(calculate_exact_token_match_score("trump", "donald trump") > 0.8);
        assert!(calculate_exact_token_match_score("biden", "joe biden") > 0.8);
        assert!(calculate_exact_token_match_score("merkel", "trump") < 0.1);
    }

    #[test]
    fn test_abbreviation_score() {
        assert!(calculate_abbreviation_score("eu", "european union") > 0.9);
        assert!(calculate_abbreviation_score("usa", "united states of america") > 0.8);
    }

    #[test]
    fn test_string_similarity_name_variant() {
        let score = calculate_string_similarity("Trump", "Donald Trump");
        assert!(score >= 0.9, "Expected >= 0.9, got {}", score);
    }
}
