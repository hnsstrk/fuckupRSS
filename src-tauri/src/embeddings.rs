//! Shared embedding utilities for vector operations.
//!
//! This module provides common functions for:
//! - Converting between embedding vectors (f32) and binary blobs for storage
//! - Calculating cosine similarity between embeddings

/// Convert an embedding vector to a binary blob for SQLite storage.
/// Each f32 is stored as 4 bytes in little-endian format.
pub fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Convert a binary blob back to an embedding vector.
/// Assumes little-endian f32 format.
pub fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Calculate cosine similarity between two embedding vectors.
/// Returns 0.0 for empty or mismatched vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    (dot / (norm_a * norm_b)) as f64
}

/// Calculate cosine similarity directly from two embedding blobs.
/// More efficient than converting to Vec<f32> first when working with DB data.
/// Returns None for empty or mismatched blobs.
pub fn cosine_similarity_from_blobs(blob_a: &[u8], blob_b: &[u8]) -> Option<f64> {
    if blob_a.len() != blob_b.len() || blob_a.is_empty() {
        return None;
    }

    let dim = blob_a.len() / 4; // f32 = 4 bytes
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;

    for i in 0..dim {
        let offset = i * 4;
        let a = f32::from_le_bytes([
            blob_a[offset],
            blob_a[offset + 1],
            blob_a[offset + 2],
            blob_a[offset + 3],
        ]) as f64;
        let b = f32::from_le_bytes([
            blob_b[offset],
            blob_b[offset + 1],
            blob_b[offset + 2],
            blob_b[offset + 3],
        ]) as f64;

        dot += a * b;
        norm_a += a * a;
        norm_b += b * b;
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return None;
    }

    Some(dot / (norm_a.sqrt() * norm_b.sqrt()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_to_blob() {
        let embedding = vec![1.0f32, 2.0, 3.0];
        let blob = embedding_to_blob(&embedding);
        assert_eq!(blob.len(), 12); // 3 * 4 bytes
    }

    #[test]
    fn test_blob_to_embedding() {
        let embedding = vec![1.0f32, 2.0, 3.0];
        let blob = embedding_to_blob(&embedding);
        let restored = blob_to_embedding(&blob);
        assert_eq!(embedding, restored);
    }

    #[test]
    fn test_roundtrip() {
        let original = vec![0.5f32, -0.3, 0.8, 0.0, -1.0];
        let blob = embedding_to_blob(&original);
        let restored = blob_to_embedding(&blob);
        assert_eq!(original, restored);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![1.0f32, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![0.0f32, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![-1.0f32, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_mismatched() {
        let a = vec![1.0f32, 0.0];
        let b = vec![1.0f32, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_from_blobs() {
        let a = vec![1.0f32, 2.0, 3.0];
        let b = vec![4.0f32, 5.0, 6.0];
        let blob_a = embedding_to_blob(&a);
        let blob_b = embedding_to_blob(&b);

        let sim_direct = cosine_similarity(&a, &b);
        let sim_blob = cosine_similarity_from_blobs(&blob_a, &blob_b).unwrap();

        assert!((sim_direct - sim_blob).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_from_blobs_empty() {
        let blob_a: Vec<u8> = vec![];
        let blob_b: Vec<u8> = vec![];
        assert_eq!(cosine_similarity_from_blobs(&blob_a, &blob_b), None);
    }
}
