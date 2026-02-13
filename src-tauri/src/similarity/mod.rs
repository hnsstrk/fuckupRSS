//! Centralized similarity calculation module
//!
//! Provides string-based, embedding-based, and hybrid similarity methods
//! for keyword matching and synonym detection.

pub mod embedding;
pub mod hybrid;
pub mod string;

#[cfg(test)]
mod tests;

pub use hybrid::{find_similar_hybrid, SimilarityMethod, SimilarityOptions};
