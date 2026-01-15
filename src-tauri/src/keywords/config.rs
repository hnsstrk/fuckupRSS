//! Centralized configuration for keyword extraction
//!
//! This module provides a single source of truth for keyword extraction parameters
//! to ensure consistency across the codebase.

/// Central configuration for keyword extraction
#[derive(Debug, Clone)]
pub struct KeywordConfig {
    /// Maximum number of keywords to extract
    pub max_keywords: usize,
    /// Minimum word length to consider
    pub min_word_length: usize,
    /// Whether to use stemming
    pub use_stemming: bool,
    /// Maximum categories to assign
    pub max_categories: usize,
    /// Confidence threshold for statistical keywords
    pub statistical_confidence: f64,
    /// Confidence multiplier for compound keyword parts
    pub compound_confidence_factor: f64,
}

impl Default for KeywordConfig {
    fn default() -> Self {
        Self::standard()
    }
}

impl KeywordConfig {
    /// Standard configuration for article processing
    pub fn standard() -> Self {
        Self {
            max_keywords: 15,
            min_word_length: 3,
            use_stemming: true,
            max_categories: 5,
            statistical_confidence: 0.8,
            compound_confidence_factor: 0.8,
        }
    }

    /// Configuration for batch processing (more candidates for filtering)
    pub fn batch_processing() -> Self {
        Self {
            max_keywords: 30,
            min_word_length: 3,
            use_stemming: true,
            max_categories: 5,
            statistical_confidence: 0.8,
            compound_confidence_factor: 0.8,
        }
    }

    /// Configuration for statistical-only analysis
    pub fn statistical_analysis() -> Self {
        Self {
            max_keywords: 15,
            min_word_length: 3,
            use_stemming: true,
            max_categories: 5,
            statistical_confidence: 0.8,
            compound_confidence_factor: 0.8,
        }
    }

    /// Configuration for local/fallback extraction
    pub fn local_extraction() -> Self {
        Self {
            max_keywords: 10,
            min_word_length: 3,
            use_stemming: false,
            max_categories: 5,
            statistical_confidence: 0.8,
            compound_confidence_factor: 0.8,
        }
    }

    /// Builder method: set max keywords
    pub fn with_max_keywords(mut self, max: usize) -> Self {
        self.max_keywords = max;
        self
    }

    /// Builder method: set min word length
    pub fn with_min_word_length(mut self, len: usize) -> Self {
        self.min_word_length = len;
        self
    }

    /// Builder method: set stemming
    pub fn with_stemming(mut self, enabled: bool) -> Self {
        self.use_stemming = enabled;
        self
    }

    /// Builder method: set max categories
    pub fn with_max_categories(mut self, max: usize) -> Self {
        self.max_categories = max;
        self
    }
}

// Default values as constants for documentation and reference
pub mod defaults {
    /// Default maximum keywords for standard processing
    pub const MAX_KEYWORDS_STANDARD: usize = 15;
    /// Maximum keywords for batch processing (pre-filtering)
    pub const MAX_KEYWORDS_BATCH: usize = 30;
    /// Maximum keywords for local extraction
    pub const MAX_KEYWORDS_LOCAL: usize = 10;
    /// Minimum word length
    pub const MIN_WORD_LENGTH: usize = 3;
    /// Maximum categories per article
    pub const MAX_CATEGORIES: usize = 5;
    /// Default confidence for statistical keywords
    pub const STATISTICAL_CONFIDENCE: f64 = 0.8;
    /// Confidence multiplier for compound keyword parts
    pub const COMPOUND_CONFIDENCE_FACTOR: f64 = 0.8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = KeywordConfig::default();
        assert_eq!(config.max_keywords, 15);
        assert_eq!(config.min_word_length, 3);
        assert!(config.use_stemming);
    }

    #[test]
    fn test_batch_config() {
        let config = KeywordConfig::batch_processing();
        assert_eq!(config.max_keywords, 30);
    }

    #[test]
    fn test_builder_pattern() {
        let config = KeywordConfig::standard()
            .with_max_keywords(20)
            .with_stemming(false);
        assert_eq!(config.max_keywords, 20);
        assert!(!config.use_stemming);
    }
}
