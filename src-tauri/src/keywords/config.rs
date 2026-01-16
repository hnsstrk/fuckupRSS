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

    // === MMR Diversification Options ===
    /// Whether to use MMR (Maximal Marginal Relevance) for diversification
    pub use_mmr: bool,
    /// Lambda parameter for MMR (0.0-1.0, lower = more diversity)
    pub mmr_lambda: f64,

    // === TRISUM Multi-Centrality Options ===
    /// Whether to use TRISUM instead of standard TextRank
    pub use_trisum: bool,
    /// Weight for PageRank component in TRISUM
    pub trisum_pagerank_weight: f64,
    /// Weight for Eigenvector centrality in TRISUM
    pub trisum_eigenvector_weight: f64,
    /// Weight for Betweenness centrality in TRISUM
    pub trisum_betweenness_weight: f64,

    // === Levenshtein Deduplication Options ===
    /// Maximum Levenshtein distance for near-duplicate detection
    pub levenshtein_max_distance: usize,
}

impl Default for KeywordConfig {
    fn default() -> Self {
        Self::standard()
    }
}

impl KeywordConfig {
    /// Standard configuration for article processing
    /// Uses full pipeline: MMR for diversity, TRISUM for multi-centrality, and Levenshtein for deduplication
    pub fn standard() -> Self {
        Self {
            max_keywords: 15,
            min_word_length: 3,
            use_stemming: true,
            max_categories: 5,
            statistical_confidence: 0.8,
            compound_confidence_factor: 0.8,
            // MMR enabled by default for better diversity
            use_mmr: true,
            mmr_lambda: 0.6,
            // TRISUM enabled by default for multi-centrality keyword extraction
            use_trisum: true,
            trisum_pagerank_weight: 0.4,
            trisum_eigenvector_weight: 0.35,
            trisum_betweenness_weight: 0.25,
            // Levenshtein deduplication with distance 2
            levenshtein_max_distance: 2,
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
            use_mmr: true,
            mmr_lambda: 0.5, // Slightly more diversity for batch
            use_trisum: true, // Use TRISUM for better quality in batch
            trisum_pagerank_weight: 0.4,
            trisum_eigenvector_weight: 0.35,
            trisum_betweenness_weight: 0.25,
            levenshtein_max_distance: 2,
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
            use_mmr: true,
            mmr_lambda: 0.6,
            use_trisum: false,
            trisum_pagerank_weight: 0.4,
            trisum_eigenvector_weight: 0.35,
            trisum_betweenness_weight: 0.25,
            levenshtein_max_distance: 2,
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
            use_mmr: false, // Keep simple for fallback
            mmr_lambda: 0.6,
            use_trisum: false,
            trisum_pagerank_weight: 0.4,
            trisum_eigenvector_weight: 0.35,
            trisum_betweenness_weight: 0.25,
            levenshtein_max_distance: 2,
        }
    }

    /// Configuration optimized for high diversity (unique keywords)
    pub fn high_diversity() -> Self {
        Self {
            max_keywords: 15,
            min_word_length: 3,
            use_stemming: true,
            max_categories: 5,
            statistical_confidence: 0.8,
            compound_confidence_factor: 0.8,
            use_mmr: true,
            mmr_lambda: 0.3, // Low lambda = high diversity
            use_trisum: true, // Use TRISUM for multi-centrality
            trisum_pagerank_weight: 0.3,
            trisum_eigenvector_weight: 0.3,
            trisum_betweenness_weight: 0.4, // Emphasize bridge terms
            levenshtein_max_distance: 3, // Stricter deduplication
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

    /// Builder method: enable/disable MMR diversification
    pub fn with_mmr(mut self, enabled: bool) -> Self {
        self.use_mmr = enabled;
        self
    }

    /// Builder method: set MMR lambda (0.0-1.0, lower = more diversity)
    pub fn with_mmr_lambda(mut self, lambda: f64) -> Self {
        self.mmr_lambda = lambda.clamp(0.0, 1.0);
        self
    }

    /// Builder method: enable/disable TRISUM
    pub fn with_trisum(mut self, enabled: bool) -> Self {
        self.use_trisum = enabled;
        self
    }

    /// Builder method: set TRISUM weights
    pub fn with_trisum_weights(
        mut self,
        pagerank: f64,
        eigenvector: f64,
        betweenness: f64,
    ) -> Self {
        self.trisum_pagerank_weight = pagerank;
        self.trisum_eigenvector_weight = eigenvector;
        self.trisum_betweenness_weight = betweenness;
        self
    }

    /// Builder method: set Levenshtein max distance
    pub fn with_levenshtein_distance(mut self, distance: usize) -> Self {
        self.levenshtein_max_distance = distance;
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

    // === MMR Defaults ===
    /// Whether MMR is enabled by default
    pub const USE_MMR: bool = true;
    /// Default MMR lambda (balancing relevance vs diversity)
    pub const MMR_LAMBDA: f64 = 0.6;

    // === TRISUM Defaults ===
    /// Whether TRISUM is enabled by default
    pub const USE_TRISUM: bool = true;
    /// Default TRISUM PageRank weight
    pub const TRISUM_PAGERANK_WEIGHT: f64 = 0.4;
    /// Default TRISUM Eigenvector weight
    pub const TRISUM_EIGENVECTOR_WEIGHT: f64 = 0.35;
    /// Default TRISUM Betweenness weight
    pub const TRISUM_BETWEENNESS_WEIGHT: f64 = 0.25;

    // === Levenshtein Defaults ===
    /// Default maximum Levenshtein distance for deduplication
    pub const LEVENSHTEIN_MAX_DISTANCE: usize = 2;
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
        assert!(config.use_mmr);
        assert!(config.use_trisum); // TRISUM is now enabled by default
        assert_eq!(config.levenshtein_max_distance, 2);
    }

    #[test]
    fn test_batch_config() {
        let config = KeywordConfig::batch_processing();
        assert_eq!(config.max_keywords, 30);
        assert!(config.use_mmr);
        assert!(config.use_trisum); // TRISUM enabled for batch
    }

    #[test]
    fn test_builder_pattern() {
        let config = KeywordConfig::standard()
            .with_max_keywords(20)
            .with_stemming(false);
        assert_eq!(config.max_keywords, 20);
        assert!(!config.use_stemming);
    }

    #[test]
    fn test_mmr_builder() {
        let config = KeywordConfig::standard()
            .with_mmr(true)
            .with_mmr_lambda(0.4);
        assert!(config.use_mmr);
        assert!((config.mmr_lambda - 0.4).abs() < 1e-6);
    }

    #[test]
    fn test_trisum_builder() {
        let config = KeywordConfig::standard()
            .with_trisum(true)
            .with_trisum_weights(0.3, 0.4, 0.3);
        assert!(config.use_trisum);
        assert!((config.trisum_pagerank_weight - 0.3).abs() < 1e-6);
        assert!((config.trisum_eigenvector_weight - 0.4).abs() < 1e-6);
        assert!((config.trisum_betweenness_weight - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_levenshtein_builder() {
        let config = KeywordConfig::standard()
            .with_levenshtein_distance(3);
        assert_eq!(config.levenshtein_max_distance, 3);
    }

    #[test]
    fn test_high_diversity_config() {
        let config = KeywordConfig::high_diversity();
        assert!(config.use_mmr);
        assert!(config.use_trisum);
        assert!(config.mmr_lambda < 0.5); // Lower lambda for more diversity
        assert_eq!(config.levenshtein_max_distance, 3); // Stricter dedup
    }

    #[test]
    fn test_mmr_lambda_clamping() {
        let config = KeywordConfig::standard().with_mmr_lambda(1.5);
        assert!((config.mmr_lambda - 1.0).abs() < 1e-6);

        let config2 = KeywordConfig::standard().with_mmr_lambda(-0.5);
        assert!(config2.mmr_lambda >= 0.0);
    }
}
