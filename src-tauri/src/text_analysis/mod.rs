//! Text Analysis Module
//!
//! This module provides statistical text analysis capabilities:
//! - TF-IDF based keyword extraction
//! - Category matching via word frequency analysis
//! - Bias weight management for learning from user corrections

mod stopwords;
mod tfidf;
mod category_matcher;
mod bias;
pub mod keyword_seeds;

// Export public API
pub use tfidf::{TfIdfExtractor, CorpusStats};
pub use category_matcher::CategoryMatcher;
pub use bias::{BiasWeights, CorrectionRecord, CorrectionType, BiasStats, record_correction, get_bias_stats};
#[allow(unused_imports)]
pub use stopwords::{
    STOPWORDS, load_user_stopwords, load_system_stopwords, load_all_db_stopwords,
    add_user_stopword, remove_user_stopword, remove_stopword,
    get_all_stopwords, get_stopword_stats, StopwordStats, is_stopword,
    count_user_stopwords, count_system_stopwords, count_all_stopwords,
};
pub use keyword_seeds::{
    seed_known_keywords, update_types_from_seeds,
};
// Re-export known keyword collections and lookup function for external use
#[allow(unused_imports)] // Public API for keyword type lookup and seeding
pub use keyword_seeds::{
    get_known_keyword_type, KNOWN_PERSONS, KNOWN_ORGANIZATIONS, KNOWN_LOCATIONS, KNOWN_ACRONYMS,
};
