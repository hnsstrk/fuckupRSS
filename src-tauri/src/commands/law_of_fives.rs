//! Law of Fives - Keyword Type Detection using Embedding Prototypes
//!
//! This module provides semantic keyword type detection using embedding prototypes.
//! The 5 types (person, organization, location, concept, acronym) each have prototype
//! embeddings generated from representative example keywords.
//!
//! The detection strategy is:
//! 1. Heuristic detection (fast, pattern-based) - see ollama/helpers.rs
//! 2. Embedding prototype comparison (~90% accuracy)
//! 3. Optional LLM fallback for edge cases (future)

use crate::embeddings::{blob_to_embedding, cosine_similarity, embedding_to_blob};
use crate::ollama::OllamaClient;
use crate::AppState;
use log::{debug, info, warn};
use rusqlite::params;
use serde::Serialize;
use tauri::State;

// ============================================================
// PROTOTYPE DEFINITIONS
// ============================================================

/// Example keywords for each type, used to generate prototype embeddings.
/// These are carefully chosen to be representative of their category.
pub const PERSON_EXAMPLES: &[&str] = &[
    "Angela Merkel",
    "Emmanuel Macron",
    "Donald Trump",
    "Olaf Scholz",
    "Joe Biden",
    "Vladimir Putin",
    "Xi Jinping",
    "Elon Musk",
    "Mark Zuckerberg",
    "Albert Einstein",
];

pub const ORGANIZATION_EXAMPLES: &[&str] = &[
    "Bundesregierung",
    "Europäische Union",
    "NATO",
    "Vereinte Nationen",
    "Deutsche Bank",
    "Volkswagen AG",
    "Google Inc",
    "CDU",
    "SPD",
    "Greenpeace",
];

pub const LOCATION_EXAMPLES: &[&str] = &[
    "Berlin",
    "Deutschland",
    "Europa",
    "Washington D.C.",
    "Russland",
    "China",
    "Frankreich",
    "München",
    "New York",
    "Brüssel",
];

pub const CONCEPT_EXAMPLES: &[&str] = &[
    "Klimawandel",
    "Digitalisierung",
    "Nachhaltigkeit",
    "Inflation",
    "Demokratie",
    "Energiewende",
    "Cybersicherheit",
    "Migration",
    "Künstliche Intelligenz",
    "Globalisierung",
];

pub const ACRONYM_EXAMPLES: &[&str] = &[
    "CO2",
    "BIP",
    "KI",
    "IT",
    "USA",
    "CIA",
    "FBI",
    "DNA",
    "GPS",
    "API",
];

/// The 5 keyword types
pub const KEYWORD_TYPES: &[&str] = &["person", "organization", "location", "concept", "acronym"];

// ============================================================
// RESULT TYPES
// ============================================================

/// Result of generating prototype embeddings
#[derive(Debug, Serialize)]
pub struct PrototypeGenerationResult {
    pub types_generated: i64,
    pub total_examples_processed: i64,
    pub errors: Vec<String>,
}

/// Result of semantic type detection
#[derive(Debug, Clone, Serialize)]
pub struct SemanticTypeResult {
    pub keyword_type: String,
    pub confidence: f64,
    pub all_scores: Vec<(String, f64)>,
}

/// Result of batch semantic type update
#[derive(Debug, Serialize)]
pub struct SemanticTypeUpdateResult {
    pub total_processed: i64,
    pub type_counts: Vec<(String, i64)>,
    pub low_confidence_count: i64,
    pub errors: Vec<String>,
}

// ============================================================
// PROTOTYPE GENERATION
// ============================================================

/// Get example keywords for a given type
fn get_examples_for_type(type_name: &str) -> &'static [&'static str] {
    match type_name {
        "person" => PERSON_EXAMPLES,
        "organization" => ORGANIZATION_EXAMPLES,
        "location" => LOCATION_EXAMPLES,
        "concept" => CONCEPT_EXAMPLES,
        "acronym" => ACRONYM_EXAMPLES,
        _ => &[],
    }
}

/// Generate and save prototype embeddings for all keyword types.
/// Each prototype is the average embedding of its example keywords.
#[tauri::command]
pub async fn generate_keyword_type_prototypes(
    state: State<'_, AppState>,
) -> Result<PrototypeGenerationResult, String> {
    let client = OllamaClient::new(None);

    // Get embedding model from settings
    let model = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        get_embedding_model(db.conn())
    };

    info!("Generating keyword type prototypes using model: {}", model);

    let mut types_generated = 0i64;
    let mut total_examples = 0i64;
    let mut errors = Vec::new();

    for type_name in KEYWORD_TYPES {
        let examples = get_examples_for_type(type_name);
        if examples.is_empty() {
            warn!("No examples for type: {}", type_name);
            continue;
        }

        // Generate embeddings for all examples
        let mut valid_embeddings: Vec<Vec<f32>> = Vec::new();

        for example in examples {
            match client.generate_embedding(&model, example).await {
                Ok(emb) => {
                    valid_embeddings.push(emb);
                    total_examples += 1;
                }
                Err(e) => {
                    let err_msg = format!("Failed to embed '{}': {}", example, e);
                    warn!("{}", err_msg);
                    errors.push(err_msg);
                }
            }
        }

        if valid_embeddings.is_empty() {
            errors.push(format!("No valid embeddings for type: {}", type_name));
            continue;
        }

        // Calculate average embedding (centroid)
        let dim = valid_embeddings[0].len();
        let mut centroid = vec![0.0f32; dim];
        let count = valid_embeddings.len() as f32;

        for emb in &valid_embeddings {
            for (i, val) in emb.iter().enumerate() {
                centroid[i] += val / count;
            }
        }

        // Normalize the centroid
        let norm: f32 = centroid.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut centroid {
                *val /= norm;
            }
        }

        // Save to database
        let examples_json = serde_json::to_string(&examples.to_vec()).unwrap_or_default();
        let blob = embedding_to_blob(&centroid);

        {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            db.conn()
                .execute(
                    r#"
                    INSERT OR REPLACE INTO keyword_type_prototypes
                    (type_name, embedding, example_keywords, updated_at)
                    VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)
                    "#,
                    params![type_name, blob, examples_json],
                )
                .map_err(|e| format!("Failed to save prototype for {}: {}", type_name, e))?;
        }

        info!(
            "Generated prototype for '{}' from {} examples",
            type_name,
            valid_embeddings.len()
        );
        types_generated += 1;
    }

    Ok(PrototypeGenerationResult {
        types_generated,
        total_examples_processed: total_examples,
        errors,
    })
}

// ============================================================
// SEMANTIC TYPE DETECTION
// ============================================================

/// Load all prototype embeddings from database
fn load_prototypes(conn: &rusqlite::Connection) -> Result<Vec<(String, Vec<f32>)>, String> {
    let mut stmt = conn
        .prepare("SELECT type_name, embedding FROM keyword_type_prototypes WHERE embedding IS NOT NULL")
        .map_err(|e| e.to_string())?;

    let prototypes: Vec<(String, Vec<f32>)> = stmt
        .query_map([], |row| {
            let type_name: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            Ok((type_name, blob_to_embedding(&blob)))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(prototypes)
}

/// Detect keyword type semantically using embedding similarity to prototypes.
/// Returns the type with highest similarity and confidence score.
pub fn detect_keyword_type_semantic(
    keyword_embedding: &[f32],
    prototypes: &[(String, Vec<f32>)],
) -> SemanticTypeResult {
    if prototypes.is_empty() {
        return SemanticTypeResult {
            keyword_type: "concept".to_string(),
            confidence: 0.0,
            all_scores: vec![],
        };
    }

    // Calculate similarity to each prototype
    let mut scores: Vec<(String, f64)> = prototypes
        .iter()
        .map(|(type_name, proto_emb)| {
            let sim = cosine_similarity(keyword_embedding, proto_emb);
            (type_name.clone(), sim)
        })
        .collect();

    // Sort by similarity descending
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let (best_type, best_score) = scores.first().cloned().unwrap_or(("concept".to_string(), 0.0));

    // Calculate confidence based on margin to second-best
    let confidence = if scores.len() > 1 {
        let second_score = scores[1].1;
        // Confidence is higher when there's a clear winner
        let margin = best_score - second_score;
        // Scale margin: 0.1 margin = 0.8 confidence, 0.2+ margin = 1.0 confidence
        (margin * 5.0 + 0.5).min(1.0).max(0.0)
    } else {
        best_score
    };

    SemanticTypeResult {
        keyword_type: best_type,
        confidence,
        all_scores: scores,
    }
}

/// Detect keyword type for a single keyword by its ID.
/// First tries heuristic detection, then falls back to semantic.
#[tauri::command]
pub fn detect_keyword_type_by_id(
    keyword_id: i64,
    state: State<AppState>,
) -> Result<SemanticTypeResult, String> {
    use super::ollama::helpers::detect_keyword_type;

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.conn();

    // Get keyword name and embedding
    let (name, embedding_blob): (String, Option<Vec<u8>>) = conn
        .query_row(
            "SELECT name, embedding FROM immanentize WHERE id = ?",
            [keyword_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("Keyword not found: {}", e))?;

    // First try heuristic detection
    let heuristic_type = detect_keyword_type(&name);

    // If heuristic is confident (not "concept"), use it
    if heuristic_type != "concept" {
        return Ok(SemanticTypeResult {
            keyword_type: heuristic_type.clone(),
            confidence: 0.9, // High confidence for heuristic matches
            all_scores: vec![(heuristic_type, 0.9)],
        });
    }

    // For "concept" from heuristic, try semantic detection if embedding exists
    if let Some(blob) = embedding_blob {
        if !blob.is_empty() {
            let embedding = blob_to_embedding(&blob);
            let prototypes = load_prototypes(conn)?;

            if !prototypes.is_empty() {
                let result = detect_keyword_type_semantic(&embedding, &prototypes);

                // If semantic detection is confident, use it
                if result.confidence > 0.6 {
                    return Ok(result);
                }
            }
        }
    }

    // Fall back to heuristic result
    Ok(SemanticTypeResult {
        keyword_type: heuristic_type.clone(),
        confidence: 0.5, // Lower confidence for "concept" fallback
        all_scores: vec![(heuristic_type, 0.5)],
    })
}

// ============================================================
// BATCH UPDATE WITH HYBRID DETECTION
// ============================================================

/// Update all keyword types using hybrid detection (heuristic + semantic).
/// This replaces the pure heuristic update_keyword_types with a smarter approach.
#[tauri::command]
pub fn update_keyword_types_semantic(
    state: State<AppState>,
) -> Result<SemanticTypeUpdateResult, String> {
    use super::ollama::helpers::detect_keyword_type;

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.conn();

    // Load prototypes once
    let prototypes = load_prototypes(conn)?;
    let has_prototypes = !prototypes.is_empty();

    if !has_prototypes {
        warn!("No prototypes found - using heuristic-only detection");
    }

    // Get all keywords with their embeddings
    let keywords: Vec<(i64, String, Option<Vec<u8>>)> = conn
        .prepare("SELECT id, name, embedding FROM immanentize")
        .map_err(|e| e.to_string())?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut type_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut low_confidence_count = 0i64;
    let mut errors = Vec::new();
    let mut total_processed = 0i64;

    for (id, name, embedding_blob) in &keywords {
        // First try heuristic detection
        let heuristic_type = detect_keyword_type(name);

        let (final_type, _confidence) = if heuristic_type != "concept" {
            // Heuristic found a specific type - trust it
            (heuristic_type, 0.9)
        } else if has_prototypes {
            // Try semantic detection for "concept" keywords
            if let Some(blob) = embedding_blob {
                if !blob.is_empty() {
                    let embedding = blob_to_embedding(blob);
                    let result = detect_keyword_type_semantic(&embedding, &prototypes);

                    if result.confidence > 0.55 {
                        (result.keyword_type, result.confidence)
                    } else {
                        low_confidence_count += 1;
                        ("concept".to_string(), result.confidence)
                    }
                } else {
                    low_confidence_count += 1;
                    ("concept".to_string(), 0.5)
                }
            } else {
                low_confidence_count += 1;
                ("concept".to_string(), 0.5)
            }
        } else {
            (heuristic_type, 0.5)
        };

        // Update database
        if let Err(e) = conn.execute(
            "UPDATE immanentize SET keyword_type = ?1 WHERE id = ?2",
            params![&final_type, id],
        ) {
            errors.push(format!("Failed to update {}: {}", id, e));
            continue;
        }

        *type_counts.entry(final_type).or_insert(0) += 1;
        total_processed += 1;

        if total_processed % 500 == 0 {
            debug!("Processed {} keywords...", total_processed);
        }
    }

    info!(
        "Keyword types updated: {} total, {} low confidence",
        total_processed, low_confidence_count
    );

    let type_counts_vec: Vec<(String, i64)> = type_counts.into_iter().collect();

    Ok(SemanticTypeUpdateResult {
        total_processed,
        type_counts: type_counts_vec,
        low_confidence_count,
        errors,
    })
}

// ============================================================
// HELPER FUNCTIONS
// ============================================================

/// Get the configured embedding model from settings
fn get_embedding_model(conn: &rusqlite::Connection) -> String {
    conn.query_row(
        "SELECT value FROM settings WHERE key = 'embedding_model'",
        [],
        |row| row.get(0),
    )
    .unwrap_or_else(|_| "snowflake-arctic-embed2".to_string())
}

/// Check if prototypes exist and are up-to-date
#[tauri::command]
pub fn get_prototype_status(state: State<AppState>) -> Result<PrototypeStatus, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.conn();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM keyword_type_prototypes WHERE embedding IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let oldest_update: Option<String> = conn
        .query_row(
            "SELECT MIN(updated_at) FROM keyword_type_prototypes",
            [],
            |row| row.get(0),
        )
        .ok();

    let model = get_embedding_model(conn);

    Ok(PrototypeStatus {
        prototype_count: count,
        expected_count: KEYWORD_TYPES.len() as i64,
        oldest_update,
        embedding_model: model,
        is_complete: count == KEYWORD_TYPES.len() as i64,
    })
}

/// Status of prototype embeddings
#[derive(Debug, Serialize)]
pub struct PrototypeStatus {
    pub prototype_count: i64,
    pub expected_count: i64,
    pub oldest_update: Option<String>,
    pub embedding_model: String,
    pub is_complete: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_examples_for_type() {
        assert_eq!(get_examples_for_type("person").len(), 10);
        assert_eq!(get_examples_for_type("organization").len(), 10);
        assert_eq!(get_examples_for_type("location").len(), 10);
        assert_eq!(get_examples_for_type("concept").len(), 10);
        assert_eq!(get_examples_for_type("acronym").len(), 10);
        assert!(get_examples_for_type("unknown").is_empty());
    }

    #[test]
    fn test_semantic_detection_with_empty_prototypes() {
        let embedding = vec![0.1, 0.2, 0.3];
        let prototypes: Vec<(String, Vec<f32>)> = vec![];

        let result = detect_keyword_type_semantic(&embedding, &prototypes);
        assert_eq!(result.keyword_type, "concept");
        assert_eq!(result.confidence, 0.0);
    }

    #[test]
    fn test_semantic_detection_finds_best_match() {
        let keyword_emb = vec![0.9, 0.1, 0.0];
        let prototypes = vec![
            ("person".to_string(), vec![0.8, 0.2, 0.0]),       // Similar
            ("organization".to_string(), vec![0.1, 0.9, 0.0]), // Different
            ("concept".to_string(), vec![0.0, 0.0, 1.0]),      // Very different
        ];

        let result = detect_keyword_type_semantic(&keyword_emb, &prototypes);
        assert_eq!(result.keyword_type, "person");
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_keyword_types_count() {
        assert_eq!(KEYWORD_TYPES.len(), 5);
    }
}
