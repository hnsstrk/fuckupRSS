//! Centralized similarity calculation module
//!
//! Provides string-based, embedding-based, and hybrid similarity methods
//! for keyword matching and synonym detection.

pub mod string;
pub mod embedding;
pub mod hybrid;

#[cfg(test)]
mod tests;

pub use hybrid::{
    find_similar_hybrid,
    SimilarityOptions,
    SimilarityMethod,
};
