//! TF-IDF based keyword extraction
//!
//! Extracts keywords from article text using Term Frequency-Inverse Document Frequency.
//! Includes stemming support for German and English text.

use rust_stemmers::{Algorithm, Stemmer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use unicode_segmentation::UnicodeSegmentation;

use super::stopwords::STOPWORDS;
use crate::keywords::find_canonical_keyword_with_db;

/// A keyword candidate with its TF-IDF score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordCandidate {
    /// The keyword term (normalized to lowercase)
    pub term: String,
    /// TF-IDF score (higher = more relevant)
    pub score: f64,
    /// Raw frequency in the document
    pub frequency: u32,
    /// Term frequency (normalized by document length)
    pub tf: f64,
    /// Inverse document frequency
    pub idf: f64,
}

/// Corpus statistics for IDF calculation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorpusStats {
    /// Total number of documents in the corpus
    pub total_documents: u64,
    /// Number of documents containing each term
    pub document_frequencies: HashMap<String, u64>,
}

impl CorpusStats {
    /// Load corpus stats from database
    pub fn load_from_db(conn: &rusqlite::Connection) -> Result<Self, rusqlite::Error> {
        // Get total document count (articles with content_full)
        let total_documents: u64 = conn.query_row(
            "SELECT COUNT(*) FROM fnords WHERE content_full IS NOT NULL AND content_full != ''",
            [],
            |row| row.get(0),
        )?;

        // Load document frequencies
        let mut stmt = conn.prepare("SELECT term, document_count FROM corpus_stats")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
        })?;

        let mut document_frequencies = HashMap::new();
        for row in rows {
            let (term, count) = row?;
            document_frequencies.insert(term, count);
        }

        Ok(Self {
            total_documents,
            document_frequencies,
        })
    }

    /// Update corpus stats in database with terms from a document
    pub fn update_db_with_document(
        conn: &rusqlite::Connection,
        terms: &[String],
    ) -> Result<(), rusqlite::Error> {
        let unique_terms: std::collections::HashSet<_> = terms.iter().collect();

        for term in unique_terms {
            let term_lower = term.to_lowercase();
            conn.execute(
                "INSERT INTO corpus_stats (term, document_count, last_updated)
                 VALUES (?1, 1, CURRENT_TIMESTAMP)
                 ON CONFLICT(term) DO UPDATE SET
                 document_count = document_count + 1,
                 last_updated = CURRENT_TIMESTAMP",
                rusqlite::params![&term_lower],
            )?;
        }

        Ok(())
    }

    /// Calculate IDF for a term
    /// Uses smoothed IDF: log((N + 1) / (df + 1)) + 1
    pub fn idf(&self, term: &str) -> f64 {
        let df = self
            .document_frequencies
            .get(&term.to_lowercase())
            .copied()
            .unwrap_or(0) as f64;
        let n = self.total_documents as f64;
        // Smoothed IDF to avoid division by zero and extreme values
        ((n + 1.0) / (df + 1.0)).ln() + 1.0
    }

    /// Update corpus stats with terms from a document (in-memory)
    #[allow(dead_code)] // Public API for in-memory corpus building
    pub fn add_document(&mut self, terms: &[String]) {
        self.total_documents += 1;
        let unique_terms: std::collections::HashSet<_> = terms.iter().collect();
        for term in unique_terms {
            *self
                .document_frequencies
                .entry(term.to_lowercase())
                .or_insert(0) += 1;
        }
    }

    /// Check if corpus has meaningful statistics (at least 10 documents)
    pub fn is_meaningful(&self) -> bool {
        self.total_documents >= 10
    }
}

/// TF-IDF based keyword extractor
pub struct TfIdfExtractor {
    /// Minimum word length to consider
    pub min_word_length: usize,
    /// Maximum number of keywords to return
    pub max_keywords: usize,
    /// Minimum TF-IDF score threshold
    pub min_score: f64,
    /// Whether to apply stemming to words
    pub use_stemming: bool,
}

impl Default for TfIdfExtractor {
    fn default() -> Self {
        Self {
            min_word_length: 3,
            max_keywords: 20,
            min_score: 0.1,
            use_stemming: false, // Disabled: too aggressive with proper nouns (Iran->ira, Scotland->scotla)
        }
    }
}

impl TfIdfExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure minimum word length
    #[allow(dead_code)] // Builder API for TF-IDF configuration
    pub fn with_min_word_length(mut self, len: usize) -> Self {
        self.min_word_length = len;
        self
    }

    /// Configure maximum keywords to return
    pub fn with_max_keywords(mut self, max: usize) -> Self {
        self.max_keywords = max;
        self
    }

    /// Enable or disable stemming
    #[allow(dead_code)] // Builder API for TF-IDF configuration
    pub fn with_stemming(mut self, enabled: bool) -> Self {
        self.use_stemming = enabled;
        self
    }

    /// Apply stemming to a word using both German and English stemmers
    /// Returns the shorter stem (more aggressive normalization)
    /// Skips stemming for proper nouns
    fn stem_word(&self, word: &str, original_word: &str) -> String {
        if !self.use_stemming {
            return word.to_string();
        }

        // Check if this is likely a proper noun (was originally capitalized)
        // Skip stemming for proper nouns to avoid "Iran" -> "ira"
        if self.is_likely_proper_noun(original_word) {
            return word.to_string();
        }

        let de_stemmer = Stemmer::create(Algorithm::German);
        let en_stemmer = Stemmer::create(Algorithm::English);

        let de_stem = de_stemmer.stem(word);
        let en_stem = en_stemmer.stem(word);

        // Use the shorter stem (more normalized)
        // But ensure minimum length of 3

        if de_stem.len() <= en_stem.len() && de_stem.len() >= 3 {
            de_stem.to_string()
        } else if en_stem.len() >= 3 {
            en_stem.to_string()
        } else if de_stem.len() >= 3 {
            de_stem.to_string()
        } else {
            word.to_string() // Keep original if stems are too short
        }
    }

    /// Check if a word is likely a proper noun based on capitalization and known lists
    fn is_likely_proper_noun(&self, original_word: &str) -> bool {
        // All-uppercase words (acronyms like NATO, EU) are proper nouns
        if original_word
            .chars()
            .all(|c| c.is_uppercase() || !c.is_alphabetic())
        {
            return true;
        }

        // Words that start with uppercase and have lowercase rest (like "Trump", "Berlin")
        let mut chars = original_word.chars();
        if let Some(first) = chars.next() {
            if first.is_uppercase() && chars.all(|c| c.is_lowercase() || !c.is_alphabetic()) {
                return true;
            }
        }

        false
    }

    /// Tokenize text into words
    fn tokenize(&self, text: &str) -> Vec<String> {
        self.tokenize_with_stopwords(text, None)
    }

    /// Tokenize text into words, filtering additional user-defined stopwords
    /// Also applies canonicalization to map synonyms to their canonical forms
    /// Preserves original case for proper noun detection in stemming
    fn tokenize_with_stopwords(
        &self,
        text: &str,
        user_stopwords: Option<&HashSet<String>>,
    ) -> Vec<String> {
        text.unicode_words()
            .filter(|w| {
                let lower = w.to_lowercase();
                w.len() >= self.min_word_length
                    && !STOPWORDS.contains(lower.as_str())
                    && !user_stopwords.is_some_and(|sw| sw.contains(&lower))
                    && w.chars().all(|c| c.is_alphabetic())
            })
            .map(|original| {
                let lower = original.to_lowercase();
                // Apply canonicalization first (e.g., "european" -> "Europäische Union")
                // If no canonical form exists, apply stemming (with proper noun awareness)
                find_canonical_keyword_with_db(&lower)
                    .unwrap_or_else(|| self.stem_word(&lower, original))
            })
            .collect()
    }

    /// Calculate term frequencies for a document
    fn calculate_tf(&self, tokens: &[String]) -> HashMap<String, (u32, f64)> {
        let mut freq: HashMap<String, u32> = HashMap::new();
        for token in tokens {
            *freq.entry(token.clone()).or_insert(0) += 1;
        }

        let total = tokens.len() as f64;
        freq.into_iter()
            .map(|(term, count)| {
                // Augmented TF to prevent bias towards longer documents
                let tf = 0.5 + 0.5 * (count as f64 / total);
                (term, (count, tf))
            })
            .collect()
    }

    /// Extract keywords using TF-IDF with corpus statistics
    pub fn extract(&self, text: &str, corpus_stats: &CorpusStats) -> Vec<KeywordCandidate> {
        let tokens = self.tokenize(text);
        if tokens.is_empty() {
            return Vec::new();
        }

        let term_freqs = self.calculate_tf(&tokens);

        let mut candidates: Vec<KeywordCandidate> = term_freqs
            .into_iter()
            .map(|(term, (frequency, tf))| {
                let idf = corpus_stats.idf(&term);
                let score = tf * idf;
                KeywordCandidate {
                    term,
                    score,
                    frequency,
                    tf,
                    idf,
                }
            })
            .filter(|c| c.score >= self.min_score)
            .collect();

        // Sort by score descending
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return top N keywords
        candidates.truncate(self.max_keywords);
        candidates
    }

    /// Extract keywords without corpus statistics (uses simple TF)
    /// Useful for initial extraction before corpus is built
    pub fn extract_simple(&self, text: &str) -> Vec<KeywordCandidate> {
        let tokens = self.tokenize(text);
        if tokens.is_empty() {
            return Vec::new();
        }

        let term_freqs = self.calculate_tf(&tokens);

        let mut candidates: Vec<KeywordCandidate> = term_freqs
            .into_iter()
            .map(|(term, (frequency, tf))| {
                // Use frequency as a proxy for importance without IDF
                let score = tf * (frequency as f64).ln().max(1.0);
                KeywordCandidate {
                    term,
                    score,
                    frequency,
                    tf,
                    idf: 1.0, // No IDF available
                }
            })
            .filter(|c| c.frequency >= 2) // Require at least 2 occurrences
            .collect();

        // Sort by score descending
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return top N keywords
        candidates.truncate(self.max_keywords);
        candidates
    }

    /// Get tokens from text (useful for corpus building)
    pub fn get_tokens(&self, text: &str) -> Vec<String> {
        self.tokenize(text)
    }

    /// Smart extraction: uses corpus stats if meaningful, otherwise falls back to simple extraction
    pub fn extract_smart(
        &self,
        text: &str,
        corpus_stats: Option<&CorpusStats>,
    ) -> Vec<KeywordCandidate> {
        match corpus_stats {
            Some(stats) if stats.is_meaningful() => self.extract(text, stats),
            _ => self.extract_simple(text),
        }
    }

    /// Extract keywords with user-defined stopwords
    #[allow(dead_code)] // Public API for custom stopword filtering
    pub fn extract_with_stopwords(
        &self,
        text: &str,
        corpus_stats: &CorpusStats,
        user_stopwords: &HashSet<String>,
    ) -> Vec<KeywordCandidate> {
        let tokens = self.tokenize_with_stopwords(text, Some(user_stopwords));
        if tokens.is_empty() {
            return Vec::new();
        }

        let term_freqs = self.calculate_tf(&tokens);

        let mut candidates: Vec<KeywordCandidate> = term_freqs
            .into_iter()
            .map(|(term, (frequency, tf))| {
                let idf = corpus_stats.idf(&term);
                let score = tf * idf;
                KeywordCandidate {
                    term,
                    score,
                    frequency,
                    tf,
                    idf,
                }
            })
            .filter(|c| c.score >= self.min_score)
            .collect();

        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(self.max_keywords);
        candidates
    }

    /// Smart extraction with user stopwords
    #[allow(dead_code)] // Public API for custom stopword filtering with fallback
    pub fn extract_smart_with_stopwords(
        &self,
        text: &str,
        corpus_stats: Option<&CorpusStats>,
        user_stopwords: &HashSet<String>,
    ) -> Vec<KeywordCandidate> {
        match corpus_stats {
            Some(stats) if stats.is_meaningful() => {
                self.extract_with_stopwords(text, stats, user_stopwords)
            }
            _ => self.extract_simple_with_stopwords(text, user_stopwords),
        }
    }

    /// Simple extraction with user stopwords (no corpus stats)
    pub fn extract_simple_with_stopwords(
        &self,
        text: &str,
        user_stopwords: &HashSet<String>,
    ) -> Vec<KeywordCandidate> {
        let tokens = self.tokenize_with_stopwords(text, Some(user_stopwords));
        if tokens.is_empty() {
            return Vec::new();
        }

        let term_freqs = self.calculate_tf(&tokens);

        let mut candidates: Vec<KeywordCandidate> = term_freqs
            .into_iter()
            .map(|(term, (frequency, tf))| {
                let score = tf * (frequency as f64).ln().max(1.0);
                KeywordCandidate {
                    term,
                    score,
                    frequency,
                    tf,
                    idf: 1.0,
                }
            })
            .filter(|c| c.frequency >= 2)
            .collect();

        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(self.max_keywords);
        candidates
    }

    /// Get tokens with user stopwords filtered (useful for corpus building)
    #[allow(dead_code)] // Public API for corpus building with custom stopwords
    pub fn get_tokens_with_stopwords(
        &self,
        text: &str,
        user_stopwords: &HashSet<String>,
    ) -> Vec<String> {
        self.tokenize_with_stopwords(text, Some(user_stopwords))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_filters_stopwords() {
        let extractor = TfIdfExtractor::new();
        let tokens = extractor.tokenize("Der schnelle braune Fuchs springt über den faulen Hund");

        // Should filter out "der", "den", "über"
        assert!(!tokens.contains(&"der".to_string()));
        assert!(!tokens.contains(&"den".to_string()));
        assert!(!tokens.contains(&"über".to_string()));

        // Should keep content words
        assert!(tokens.contains(&"schnelle".to_string()));
        assert!(tokens.contains(&"braune".to_string()));
        assert!(tokens.contains(&"fuchs".to_string()));
    }

    #[test]
    fn test_tokenize_filters_short_words() {
        let extractor = TfIdfExtractor::new().with_min_word_length(4);
        let tokens = extractor.tokenize("Das ist ein Test für schnelle Maschinen");

        // Should filter out words shorter than 4 chars
        assert!(!tokens.contains(&"ist".to_string()));
        assert!(!tokens.contains(&"ein".to_string()));
        assert!(!tokens.contains(&"für".to_string()));

        // Should keep longer words (not in stopwords)
        assert!(tokens.contains(&"test".to_string()));
        assert!(tokens.contains(&"schnelle".to_string()));
        assert!(tokens.contains(&"maschinen".to_string()));
    }

    #[test]
    fn test_extract_simple() {
        let extractor = TfIdfExtractor::new().with_max_keywords(5);
        let text = "Die Regierung plant neue Gesetze. Die Regierung will die Wirtschaft stärken. \
                    Die Regierung hat sich mit der Opposition getroffen. \
                    Regierung und Opposition diskutieren über Gesetze.";

        let keywords = extractor.extract_simple(text);

        // "Regierung" (canonicalized from "regierung") should be a top keyword (appears 4 times)
        assert!(!keywords.is_empty());
        let top_term = &keywords[0].term;
        // Note: "regierung" is canonicalized to "Regierung" via SYNONYM_GROUPS
        assert!(
            top_term == "Regierung" || top_term == "regierung",
            "Expected 'Regierung' or 'regierung', got '{}'",
            top_term
        );
    }

    #[test]
    fn test_corpus_stats_idf() {
        let mut stats = CorpusStats::default();

        // Add some documents
        stats.add_document(&["politik".to_string(), "wirtschaft".to_string()]);
        stats.add_document(&["politik".to_string(), "technik".to_string()]);
        stats.add_document(&["sport".to_string(), "kultur".to_string()]);

        // "politik" appears in 2/3 documents -> lower IDF
        // "sport" appears in 1/3 documents -> higher IDF
        // "unbekannt" appears in 0/3 documents -> highest IDF

        let idf_politik = stats.idf("politik");
        let idf_sport = stats.idf("sport");
        let idf_unbekannt = stats.idf("unbekannt");

        assert!(idf_sport > idf_politik);
        assert!(idf_unbekannt > idf_sport);
    }

    #[test]
    fn test_extract_with_corpus() {
        let extractor = TfIdfExtractor::new();

        // Build corpus stats
        let mut stats = CorpusStats::default();
        stats.add_document(&[
            "politik".to_string(),
            "regierung".to_string(),
            "gesetz".to_string(),
        ]);
        stats.add_document(&[
            "politik".to_string(),
            "wahl".to_string(),
            "partei".to_string(),
        ]);
        stats.add_document(&[
            "wirtschaft".to_string(),
            "handel".to_string(),
            "export".to_string(),
        ]);

        // Extract from a new document
        let text = "Die Regierung plant neue Gesetze zur Wirtschaft. \
                    Der neue Handelsvertrag soll den Export fördern.";

        let keywords = extractor.extract(text, &stats);

        // Should return keywords with TF-IDF scores
        assert!(!keywords.is_empty());
        for kw in &keywords {
            assert!(kw.score > 0.0);
            assert!(kw.idf > 0.0);
        }
    }

    #[test]
    fn test_stemming_german() {
        let extractor = TfIdfExtractor::new().with_stemming(true);

        // Test German word stemming (lowercase words, not proper nouns)
        let stem_iran = extractor.stem_word("iranischen", "iranischen");
        let stem_iran2 = extractor.stem_word("iranische", "iranische");
        let stem_iran3 = extractor.stem_word("iran", "iran");

        // All should stem to the same root
        assert_eq!(
            stem_iran, stem_iran2,
            "iranischen and iranische should have same stem"
        );
        println!(
            "iranischen -> {}, iranische -> {}, iran -> {}",
            stem_iran, stem_iran2, stem_iran3
        );

        // Test more German words
        let stem_deutsch = extractor.stem_word("deutschen", "deutschen");
        let stem_deutsch2 = extractor.stem_word("deutsche", "deutsche");
        assert_eq!(
            stem_deutsch, stem_deutsch2,
            "deutschen and deutsche should have same stem"
        );
        println!(
            "deutschen -> {}, deutsche -> {}",
            stem_deutsch, stem_deutsch2
        );

        let stem_reg = extractor.stem_word("regierung", "regierung");
        let stem_reg2 = extractor.stem_word("regierungen", "regierungen");
        println!("regierung -> {}, regierungen -> {}", stem_reg, stem_reg2);
    }

    #[test]
    fn test_stemming_consolidates_keywords() {
        let extractor = TfIdfExtractor::new()
            .with_stemming(true)
            .with_max_keywords(10);

        // Text with various forms of the same words
        let text = "Die iranische Regierung und die iranischen Minister. \
                    Der iranischer Botschafter traf die deutsche Delegation. \
                    Die deutschen Diplomaten und der deutsche Minister.";

        let keywords = extractor.extract_simple(text);

        // Print keywords for debugging
        println!("Keywords with stemming:");
        for kw in &keywords {
            println!(
                "  {} (freq: {}, score: {:.2})",
                kw.term, kw.frequency, kw.score
            );
        }

        // With stemming/canonicalization, "iranischen" and "iranischer" are consolidated
        // Note: Only some forms are in SYNONYM_GROUPS (iranisch, iranischen, iranischer)
        // "iranische" is not currently in the list, so only 2 of 3 get consolidated
        let iran_kw = keywords.iter().find(|k| {
            let lower = k.term.to_lowercase();
            lower.starts_with("iran") || lower == "iran"
        });
        assert!(iran_kw.is_some(), "Should find iran-related keyword");
        if let Some(kw) = iran_kw {
            // With canonicalization, forms in SYNONYM_GROUPS become "Iran"
            assert!(
                kw.frequency >= 2,
                "Iranian forms should be consolidated, got freq={}",
                kw.frequency
            );
        }
    }
}
