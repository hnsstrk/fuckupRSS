//! TF-IDF based keyword extraction
//!
//! Extracts keywords from article text using Term Frequency-Inverse Document Frequency.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

use super::stopwords::STOPWORDS;

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

/// Corpus statistics for IDF calculation (for future corpus-wide TF-IDF)
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorpusStats {
    /// Total number of documents in the corpus
    pub total_documents: u64,
    /// Number of documents containing each term
    pub document_frequencies: HashMap<String, u64>,
}

#[allow(dead_code)]
impl CorpusStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate IDF for a term
    /// Uses smoothed IDF: log((N + 1) / (df + 1)) + 1
    pub fn idf(&self, term: &str) -> f64 {
        let df = self.document_frequencies.get(term).copied().unwrap_or(0) as f64;
        let n = self.total_documents as f64;
        // Smoothed IDF to avoid division by zero and extreme values
        ((n + 1.0) / (df + 1.0)).ln() + 1.0
    }

    /// Update corpus stats with terms from a document
    pub fn add_document(&mut self, terms: &[String]) {
        self.total_documents += 1;
        let unique_terms: std::collections::HashSet<_> = terms.iter().collect();
        for term in unique_terms {
            *self.document_frequencies.entry(term.clone()).or_insert(0) += 1;
        }
    }
}

/// TF-IDF based keyword extractor
pub struct TfIdfExtractor {
    /// Minimum word length to consider
    pub min_word_length: usize,
    /// Maximum number of keywords to return
    pub max_keywords: usize,
    /// Minimum TF-IDF score threshold (for future use with corpus-wide TF-IDF)
    #[allow(dead_code)]
    pub min_score: f64,
}

impl Default for TfIdfExtractor {
    fn default() -> Self {
        Self {
            min_word_length: 3,
            max_keywords: 20,
            min_score: 0.1,
        }
    }
}

impl TfIdfExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure minimum word length
    #[allow(dead_code)]
    pub fn with_min_word_length(mut self, len: usize) -> Self {
        self.min_word_length = len;
        self
    }

    /// Configure maximum keywords to return
    pub fn with_max_keywords(mut self, max: usize) -> Self {
        self.max_keywords = max;
        self
    }

    /// Tokenize text into words
    fn tokenize(&self, text: &str) -> Vec<String> {
        text.unicode_words()
            .map(|w| w.to_lowercase())
            .filter(|w| {
                w.len() >= self.min_word_length
                    && !STOPWORDS.contains(w.as_str())
                    && w.chars().all(|c| c.is_alphabetic())
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

    /// Extract keywords using TF-IDF with corpus statistics (for future corpus-wide TF-IDF)
    #[allow(dead_code)]
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
        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

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
        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Return top N keywords
        candidates.truncate(self.max_keywords);
        candidates
    }

    /// Get tokens from text (useful for corpus building)
    #[allow(dead_code)]
    pub fn get_tokens(&self, text: &str) -> Vec<String> {
        self.tokenize(text)
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
        let tokens = extractor.tokenize("Das ist ein Test für kurze Wörter");

        // Should filter out words shorter than 4 chars
        assert!(!tokens.contains(&"ist".to_string()));
        assert!(!tokens.contains(&"ein".to_string()));
        assert!(!tokens.contains(&"für".to_string()));

        // Should keep longer words
        assert!(tokens.contains(&"test".to_string()));
        assert!(tokens.contains(&"kurze".to_string()));
        assert!(tokens.contains(&"wörter".to_string()));
    }

    #[test]
    fn test_extract_simple() {
        let extractor = TfIdfExtractor::new().with_max_keywords(5);
        let text = "Die Regierung plant neue Gesetze. Die Regierung will die Wirtschaft stärken. \
                    Die Regierung hat sich mit der Opposition getroffen. \
                    Regierung und Opposition diskutieren über Gesetze.";

        let keywords = extractor.extract_simple(text);

        // "regierung" should be a top keyword (appears 4 times)
        assert!(!keywords.is_empty());
        let top_term = &keywords[0].term;
        assert_eq!(top_term, "regierung");
    }

    #[test]
    fn test_corpus_stats_idf() {
        let mut stats = CorpusStats::new();

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
        let mut stats = CorpusStats::new();
        stats.add_document(&["politik".to_string(), "regierung".to_string(), "gesetz".to_string()]);
        stats.add_document(&["politik".to_string(), "wahl".to_string(), "partei".to_string()]);
        stats.add_document(&["wirtschaft".to_string(), "handel".to_string(), "export".to_string()]);

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
}
