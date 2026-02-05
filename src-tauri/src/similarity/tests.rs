//! Integration tests for the similarity module

use super::embedding::cosine_similarity;
use super::string::{
    calculate_abbreviation_score, calculate_string_similarity, token_set_ratio,
};

#[test]
fn test_trump_donald_trump_is_name_variant() {
    let score = token_set_ratio("Trump", "Donald Trump");
    assert!(
        score >= 1.0,
        "Expected token_set_ratio >= 1.0 for name variant, got {}",
        score
    );
}

#[test]
fn test_trump_donald_trump_string_similarity() {
    let score = calculate_string_similarity("Trump", "Donald Trump");
    assert!(
        score >= 0.9,
        "Expected string_similarity >= 0.9 for name variant, got {}",
        score
    );
}

#[test]
fn test_eu_european_union_abbreviation() {
    let score = calculate_abbreviation_score("EU", "European Union");
    assert!(
        score >= 0.9,
        "Expected abbreviation_score >= 0.9, got {}",
        score
    );
}

#[test]
fn test_usa_united_states() {
    let score = calculate_abbreviation_score("USA", "United States of America");
    assert!(
        score >= 0.8,
        "Expected abbreviation_score >= 0.8, got {}",
        score
    );
}

#[test]
fn test_different_names_low_similarity() {
    let score = calculate_string_similarity("Angela Merkel", "Donald Trump");
    assert!(
        score < 0.5,
        "Expected string_similarity < 0.5 for different names, got {}",
        score
    );
}

#[test]
fn test_cosine_similarity_basic() {
    let a = vec![1.0_f32, 0.0, 0.0];
    let b = vec![1.0_f32, 0.0, 0.0];
    let sim = cosine_similarity(&a, &b);
    assert!((sim - 1.0).abs() < 0.01, "Expected 1.0, got {}", sim);
}

#[test]
fn test_embedding_blob_roundtrip() {
    use super::embedding::{blob_to_embedding, embedding_to_blob};

    let original = vec![0.5_f32, -0.3, 0.8, 1.0];
    let blob = embedding_to_blob(&original);
    let restored = blob_to_embedding(&blob);

    assert_eq!(original.len(), restored.len());
    for (a, b) in original.iter().zip(restored.iter()) {
        assert!((a - b).abs() < 0.0001);
    }
}
