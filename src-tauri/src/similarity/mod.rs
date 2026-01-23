//! Centralized similarity calculation module
//!
//! Provides string-based, embedding-based, and hybrid similarity methods
//! for keyword matching and synonym detection.

pub mod string;
pub mod embedding;
pub mod hybrid;

#[cfg(test)]
mod tests;

pub use string::{
    token_set_ratio,
    calculate_string_similarity,
    calculate_abbreviation_score,
    calculate_exact_token_match_score,
};

pub use embedding::{
    cosine_similarity,
    blob_to_embedding,
};

pub use hybrid::{
    find_similar_hybrid,
    SimilarityOptions,
    SimilarityMethod,
    SimilarKeywordResult,
};
