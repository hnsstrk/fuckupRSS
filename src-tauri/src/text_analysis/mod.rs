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

// Export public API
pub use tfidf::{TfIdfExtractor, CorpusStats};
pub use category_matcher::CategoryMatcher;
pub use bias::{BiasWeights, CorrectionRecord, CorrectionType, BiasStats, record_correction, get_bias_stats};
#[allow(unused_imports)]
pub use stopwords::{
    STOPWORDS, load_user_stopwords, add_user_stopword, remove_user_stopword,
    get_all_stopwords, get_stopword_stats, StopwordStats, is_stopword,
};
