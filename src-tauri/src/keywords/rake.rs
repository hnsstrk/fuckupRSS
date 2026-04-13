//! Local RAKE (Rapid Automatic Keyword Extraction) implementation.
//!
//! Replaces the `keyword_extraction` crate (LGPL-3.0) with a permissively
//! licensed implementation. The algorithm follows the original paper:
//! Rose, S., Engel, D., Cramer, N., & Cowley, W. (2010).

use std::collections::{HashMap, HashSet};

/// Extract keywords from text using the RAKE algorithm.
///
/// Returns a Vec of (keyword_phrase, score) sorted by score descending.
pub fn extract(text: &str, stopwords: &HashSet<String>, max_keywords: usize) -> Vec<(String, f32)> {
    if text.is_empty() {
        return Vec::new();
    }

    // 1. Split text into candidate phrases by stopwords and punctuation
    let candidates = split_into_candidates(text, stopwords);

    if candidates.is_empty() {
        return Vec::new();
    }

    // 2. Build word frequency and co-occurrence degree maps
    let mut word_freq: HashMap<&str, u32> = HashMap::new();
    let mut word_degree: HashMap<&str, u32> = HashMap::new();

    for phrase_words in &candidates {
        let degree = phrase_words.len() as u32 - 1;
        for word in phrase_words {
            *word_freq.entry(word).or_insert(0) += 1;
            *word_degree.entry(word).or_insert(0) += degree;
        }
    }

    // 3. Calculate word scores: degree(w) / frequency(w)
    let mut word_score: HashMap<&str, f64> = HashMap::new();
    for (word, freq) in &word_freq {
        let degree = word_degree.get(word).copied().unwrap_or(0);
        // RAKE word score = (degree + frequency) / frequency
        word_score.insert(word, (degree + freq) as f64 / *freq as f64);
    }

    // 4. Score candidate phrases by summing word scores
    let mut phrase_scores: HashMap<String, f64> = HashMap::new();
    for phrase_words in &candidates {
        let phrase = phrase_words.join(" ");
        let score: f64 = phrase_words
            .iter()
            .map(|w| word_score.get(*w).copied().unwrap_or(0.0))
            .sum();
        // Keep the highest score if a phrase appears multiple times
        let entry = phrase_scores.entry(phrase).or_insert(0.0);
        if score > *entry {
            *entry = score;
        }
    }

    // 5. Sort by score descending and return top N
    let mut results: Vec<(String, f32)> = phrase_scores
        .into_iter()
        .map(|(phrase, score)| (phrase, score as f32))
        .collect();

    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(max_keywords);
    results
}

/// Split text into candidate keyword phrases.
///
/// A candidate is a sequence of content words (non-stopwords) separated
/// by stopwords or punctuation.
fn split_into_candidates<'a>(text: &'a str, stopwords: &HashSet<String>) -> Vec<Vec<&'a str>> {
    let mut candidates = Vec::new();
    let mut current_phrase: Vec<&str> = Vec::new();

    for word in text.split_whitespace() {
        // Strip punctuation from word boundaries
        let cleaned = word.trim_matches(|c: char| !c.is_alphanumeric());

        if cleaned.is_empty() {
            // Pure punctuation acts as phrase delimiter
            if !current_phrase.is_empty() {
                candidates.push(std::mem::take(&mut current_phrase));
            }
            continue;
        }

        let lower = cleaned.to_lowercase();

        // Check if this is a stopword or too short
        if stopwords.contains(&lower) || lower.len() < 2 {
            if !current_phrase.is_empty() {
                candidates.push(std::mem::take(&mut current_phrase));
            }
        } else {
            current_phrase.push(cleaned);
        }

        // Check if original word ended with sentence-ending punctuation
        let ends_with_punct = word.ends_with('.')
            || word.ends_with('!')
            || word.ends_with('?')
            || word.ends_with(',')
            || word.ends_with(';')
            || word.ends_with(':');
        if ends_with_punct && !current_phrase.is_empty() {
            candidates.push(std::mem::take(&mut current_phrase));
        }
    }

    if !current_phrase.is_empty() {
        candidates.push(current_phrase);
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_stopwords() -> HashSet<String> {
        [
            "the", "is", "a", "an", "and", "of", "in", "to", "for", "with", "on", "at", "by",
            "from",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    #[test]
    fn test_empty_text() {
        let stopwords = test_stopwords();
        let result = extract("", &stopwords, 10);
        assert!(result.is_empty());
    }

    #[test]
    fn test_basic_extraction() {
        let stopwords = test_stopwords();
        let text = "Compatibility of systems of linear constraints over the set of natural numbers";
        let result = extract(text, &stopwords, 10);

        assert!(!result.is_empty());
        // Multi-word phrases should score higher than single words
        let has_multi_word = result.iter().any(|(phrase, _)| phrase.contains(' '));
        assert!(has_multi_word, "RAKE should produce multi-word phrases");
    }

    #[test]
    fn test_scores_are_descending() {
        let stopwords = test_stopwords();
        let text = "Machine learning algorithms for natural language processing and text analysis";
        let result = extract(text, &stopwords, 10);

        for window in result.windows(2) {
            assert!(
                window[0].1 >= window[1].1,
                "Scores should be in descending order"
            );
        }
    }

    #[test]
    fn test_max_keywords_limit() {
        let stopwords = test_stopwords();
        let text =
            "The quick brown fox jumps over the lazy dog near the river bank and the tall mountain";
        let result = extract(text, &stopwords, 3);
        assert!(result.len() <= 3);
    }

    #[test]
    fn test_stopwords_not_in_results() {
        let stopwords = test_stopwords();
        let text = "The analysis of the system is important for the development";
        let result = extract(text, &stopwords, 10);

        for (phrase, _) in &result {
            for word in phrase.split_whitespace() {
                assert!(
                    !stopwords.contains(&word.to_lowercase()),
                    "Stopword '{}' found in result phrase '{}'",
                    word,
                    phrase
                );
            }
        }
    }
}
