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

pub use stopwords::STOPWORDS;
pub use tfidf::{TfIdfExtractor, KeywordCandidate, CorpusStats};
pub use category_matcher::{CategoryMatcher, CategoryScore, WeightedTerm};
pub use bias::{BiasWeights, CorrectionRecord, CorrectionType, BiasStats, record_correction, get_bias_stats};
