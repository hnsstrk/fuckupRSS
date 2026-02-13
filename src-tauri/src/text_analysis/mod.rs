//! Text Analysis Module
//!
//! This module provides statistical text analysis capabilities:
//! - TF-IDF based keyword extraction
//! - Category matching via word frequency analysis
//! - Bias weight management for learning from user corrections

mod bias;
mod category_matcher;
pub mod keyword_seeds;
mod stopwords;
mod tfidf;

// Export public API
pub use bias::{
    get_bias_stats, record_correction, BiasStats, BiasWeights, CorrectionRecord, CorrectionType,
};
pub use category_matcher::CategoryMatcher;
pub use keyword_seeds::{seed_known_keywords, update_types_from_seeds};
#[allow(unused_imports)]
pub use stopwords::{
    add_user_stopword, count_all_stopwords, count_system_stopwords, count_user_stopwords,
    get_all_stopwords, get_stopword_stats, is_stopword, load_all_db_stopwords,
    load_system_stopwords, load_user_stopwords, remove_stopword, remove_user_stopword,
    StopwordStats, STOPWORDS,
};
pub use tfidf::{CorpusStats, TfIdfExtractor};
// Re-export known keyword collections and lookup function for external use
#[allow(unused_imports)] // Public API for keyword type lookup and seeding
pub use keyword_seeds::{
    get_known_keyword_type, KNOWN_ACRONYMS, KNOWN_LOCATIONS, KNOWN_ORGANIZATIONS, KNOWN_PERSONS,
};
