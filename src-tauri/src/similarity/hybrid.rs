//! Hybrid similarity combining string and embedding methods
//!
//! Provides a unified API for finding similar keywords using
//! configurable combination of string-based and embedding-based methods.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::embedding::{blob_to_embedding, cosine_similarity};
use super::string::{calculate_string_similarity, token_set_ratio};

/// Configuration options for similarity search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityOptions {
    /// Method to use for similarity calculation
    #[serde(default)]
    pub method: SimilarityMethod,
    /// Minimum string similarity threshold (0.0-1.0)
    #[serde(default = "default_string_threshold")]
    pub string_threshold: f64,
    /// Minimum embedding similarity threshold (0.0-1.0)
    #[serde(default = "default_embedding_threshold")]
    pub embedding_threshold: f64,
    /// Maximum number of results to return
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Whether to include name variants (e.g., "Trump" for "Donald Trump")
    #[serde(default = "default_true")]
    pub include_name_variants: bool,
}

fn default_string_threshold() -> f64 {
    0.6
}
fn default_embedding_threshold() -> f64 {
    0.7
}
fn default_limit() -> i64 {
    20
}
fn default_true() -> bool {
    true
}

impl Default for SimilarityOptions {
    fn default() -> Self {
        Self {
            method: SimilarityMethod::Hybrid,
            string_threshold: 0.6,
            embedding_threshold: 0.7,
            limit: 20,
            include_name_variants: true,
        }
    }
}

/// Method for calculating similarity
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SimilarityMethod {
    /// Only use string-based similarity (Token Set Ratio, Levenshtein, etc.)
    String,
    /// Only use embedding-based similarity (Cosine similarity)
    Embedding,
    /// Combine both methods (recommended)
    #[default]
    Hybrid,
}

/// Result of similarity search
#[derive(Debug, Clone, Serialize)]
pub struct SimilarKeywordResult {
    pub id: i64,
    pub name: String,
    pub string_similarity: f64,
    pub embedding_similarity: f64,
    pub combined_score: f64,
    pub is_name_variant: bool,
    pub is_abbreviation: bool,
    pub article_count: i64,
}

/// Find similar keywords using hybrid string + embedding similarity.
///
/// This is the main entry point for similarity search, combining:
/// - Token Set Ratio for name variants
/// - String similarity for lexical matches
/// - Embedding similarity for semantic matches
pub fn find_similar_hybrid(
    conn: &Connection,
    keyword_id: i64,
    options: &SimilarityOptions,
) -> Result<Vec<SimilarKeywordResult>, String> {
    // Get target keyword name and embedding
    let (target_name, target_embedding): (String, Option<Vec<u8>>) = conn
        .query_row(
            "SELECT name, embedding FROM immanentize WHERE id = ?",
            [keyword_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("Failed to get target keyword: {}", e))?;

    let target_embedding_vec = target_embedding
        .as_ref()
        .map(|blob| blob_to_embedding(blob))
        .unwrap_or_default();

    // Load all keywords with their embeddings
    let all_keywords: Vec<(i64, String, Option<Vec<u8>>, i64)> = conn
        .prepare(
            r#"SELECT id, name, embedding, COALESCE(article_count, 0)
               FROM immanentize
               WHERE id != ? AND is_canonical = TRUE"#,
        )
        .map_err(|e| e.to_string())?
        .query_map([keyword_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let target_name_lower = target_name.to_lowercase();
    let mut results: Vec<SimilarKeywordResult> = Vec::new();

    for (id, name, embedding_blob, article_count) in all_keywords {
        let name_lower = name.to_lowercase();

        // Calculate string similarity
        let string_sim = calculate_string_similarity(&target_name, &name);
        let token_set = token_set_ratio(&target_name_lower, &name_lower);

        // Check for name variants (e.g., "Trump" is subset of "Donald Trump")
        let is_name_variant = token_set >= 1.0;

        // Check for abbreviations
        let is_abbreviation =
            super::string::calculate_abbreviation_score(&target_name_lower, &name_lower) > 0.7;

        // Calculate embedding similarity
        let embedding_sim = if options.method != SimilarityMethod::String {
            if let Some(blob) = &embedding_blob {
                let emb = blob_to_embedding(blob);
                if !emb.is_empty() && !target_embedding_vec.is_empty() {
                    cosine_similarity(&target_embedding_vec, &emb)
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Calculate combined score based on method
        let combined_score = match options.method {
            SimilarityMethod::String => string_sim,
            SimilarityMethod::Embedding => embedding_sim,
            SimilarityMethod::Hybrid => {
                if is_name_variant {
                    // Name variants get maximum score
                    1.0
                } else if is_abbreviation {
                    // Abbreviations: trust string similarity more
                    string_sim * 0.7 + embedding_sim * 0.3
                } else if string_sim > 0.8 {
                    // High string similarity
                    string_sim * 0.6 + embedding_sim * 0.4
                } else {
                    // Balance both
                    string_sim * 0.4 + embedding_sim * 0.6
                }
            }
        };

        // Apply filters
        let passes_filter = match options.method {
            SimilarityMethod::String => string_sim >= options.string_threshold,
            SimilarityMethod::Embedding => embedding_sim >= options.embedding_threshold,
            SimilarityMethod::Hybrid => {
                // Name variants always pass if enabled
                (options.include_name_variants && is_name_variant)
                    || string_sim >= options.string_threshold
                    || embedding_sim >= options.embedding_threshold
            }
        };

        if passes_filter {
            results.push(SimilarKeywordResult {
                id,
                name,
                string_similarity: string_sim,
                embedding_similarity: embedding_sim,
                combined_score,
                is_name_variant,
                is_abbreviation,
                article_count,
            });
        }
    }

    // Sort by combined score (name variants first)
    results.sort_by(|a, b| {
        // Name variants always first
        match (a.is_name_variant, b.is_name_variant) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                // Then abbreviations
                match (a.is_abbreviation, b.is_abbreviation) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => {
                        // Then by combined score
                        b.combined_score
                            .partial_cmp(&a.combined_score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    }
                }
            }
        }
    });

    results.truncate(options.limit as usize);

    Ok(results)
}

#[cfg(test)]
mod hybrid_tests {
    use super::*;

    #[test]
    fn test_similarity_options_default() {
        let opts = SimilarityOptions::default();
        assert_eq!(opts.method, SimilarityMethod::Hybrid);
        assert!((opts.string_threshold - 0.6).abs() < 0.01);
        assert!((opts.embedding_threshold - 0.7).abs() < 0.01);
        assert_eq!(opts.limit, 20);
        assert!(opts.include_name_variants);
    }
}
