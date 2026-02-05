//! Embedding-based similarity functions
//!
//! Re-exports shared embedding utilities from `crate::embeddings` and provides
//! additional vector similarity metrics for semantic comparison.

pub use crate::embeddings::{blob_to_embedding, cosine_similarity};

// Re-exported for use in similarity tests (blob roundtrip test)
#[allow(unused_imports)]
pub use crate::embeddings::embedding_to_blob;

/// Calculate Euclidean distance between two embedding vectors.
#[allow(dead_code)] // Available for future use in clustering/similarity
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() {
        return f64::MAX;
    }

    a.iter()
        .zip(b.iter())
        .map(|(x, y)| ((*x as f64) - (*y as f64)).powi(2))
        .sum::<f64>()
        .sqrt()
}

#[cfg(test)]
mod embedding_tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &b).abs() < 0.01);
    }

    #[test]
    fn test_blob_roundtrip() {
        let original = vec![1.0_f32, 2.5, -3.14, 0.0];
        let blob = embedding_to_blob(&original);
        let restored = blob_to_embedding(&blob);
        assert_eq!(original.len(), restored.len());
        for (a, b) in original.iter().zip(restored.iter()) {
            assert!((a - b).abs() < 0.0001);
        }
    }
}
