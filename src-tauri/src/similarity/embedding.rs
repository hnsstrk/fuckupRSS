//! Embedding-based similarity functions
//!
//! Provides vector similarity metrics for semantic comparison.

/// Convert a blob (byte array) to a vector of f32 embeddings.
pub fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
    if blob.is_empty() {
        return vec![];
    }
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Convert a vector of f32 embeddings to a blob (byte array).
pub fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

/// Calculate cosine similarity between two embedding vectors.
///
/// Returns a value between -1.0 and 1.0, where:
/// - 1.0 means identical direction
/// - 0.0 means orthogonal (unrelated)
/// - -1.0 means opposite direction
///
/// For normalized embeddings, this is equivalent to dot product.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| (*x as f64) * (*y as f64)).sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

/// Calculate Euclidean distance between two embedding vectors.
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
