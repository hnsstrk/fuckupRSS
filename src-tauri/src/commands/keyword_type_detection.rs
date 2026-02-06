//! Enhanced Keyword Type Detection
//!
//! This module provides keyword type detection with:
//! 1. Improved heuristics (existing rules enhanced)
//! 2. Prototype embedding preparation for future semantic matching
//! 3. Progress reporting for batch operations

use super::ai::helpers::detect_keyword_type;
use crate::AppState;
use log::{info, warn};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use tauri::State;

/// Result of keyword type detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordTypeResult {
    pub keyword_type: String,
    pub confidence: f64,
    pub method: String, // "heuristic", "semantic", "llm"
}

/// Prototype embeddings statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrototypeStats {
    pub total: i64,
    pub expected: i64,
    pub complete: bool,
    pub by_type: std::collections::HashMap<String, i64>,
}

/// Result of prototype generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrototypeGenerationResult {
    pub total: i64,
    pub generated: i64,
    pub errors: i64,
}

/// Batch result with detailed counts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordTypeBatchResult {
    pub total: i64,
    pub processed: i64,
    pub updated: i64,
    pub errors: i64,
    pub by_type: TypeCounts,
    pub by_method: MethodCounts,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeCounts {
    pub person: i64,
    pub organization: i64,
    pub location: i64,
    pub acronym: i64,
    pub concept: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MethodCounts {
    pub heuristic: i64,
    pub semantic: i64,
    pub llm: i64,
}

/// Type prototypes for semantic matching
const TYPE_PROTOTYPES: &[(&str, &[&str])] = &[
    ("person", &[
        "Angela Merkel", "Friedrich Merz", "Olaf Scholz", "Markus Söder",
        "Donald Trump", "Emmanuel Macron", "Joe Biden", "Elon Musk",
        "Tim Cook", "Albert Einstein", "Marie Curie"
    ]),
    ("organization", &[
        "Deutsche Bank", "Siemens AG", "Volkswagen", "Microsoft Corporation",
        "Apple Inc", "Bundesregierung", "European Commission", "United Nations",
        "Bayern München", "Manchester United"
    ]),
    ("location", &[
        "Berlin", "München", "Hamburg", "Frankfurt", "Paris", "London",
        "New York", "Deutschland", "Frankreich", "Japan"
    ]),
    ("acronym", &[
        "NATO", "EU", "USA", "UN", "AI", "API", "HTTP", "CDU", "SPD", "BND"
    ]),
    ("concept", &[
        "Klimawandel", "Digitalisierung", "Inflation", "Demokratie",
        "Künstliche Intelligenz", "Machine Learning", "Blockchain",
        "Nachhaltigkeit", "Globalisierung"
    ]),
];

/// Initialize prototype embeddings table in database
pub fn init_prototype_table(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS keyword_type_prototypes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            keyword_type TEXT NOT NULL,
            prototype_name TEXT NOT NULL,
            embedding BLOB,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(keyword_type, prototype_name)
        )",
        [],
    )
    .map_err(|e| format!("Failed to create prototype table: {}", e))?;

    Ok(())
}

/// Detect keyword type using heuristics with confidence scoring
fn detect_keyword_type_with_confidence(keyword: &str) -> KeywordTypeResult {
    let detected_type = detect_keyword_type(keyword);

    // Assign confidence based on detection reliability
    let confidence = match detected_type.as_str() {
        "acronym" => 0.98,      // Acronyms are very reliable (pattern-based)
        "organization" => 0.92, // Org suffixes and patterns are reliable
        "location" => 0.88,     // Known locations are reliable
        "person" => 0.82,       // Person detection has some false positives
        _ => 0.75,              // Concept is the fallback
    };

    KeywordTypeResult {
        keyword_type: detected_type,
        confidence,
        method: "heuristic".to_string(),
    }
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Initialize prototype embeddings database table
#[tauri::command]
pub fn init_keyword_type_prototypes(state: State<AppState>) -> Result<(), String> {
    let db = state.db_conn()?;
    init_prototype_table(db.conn())?;

    // Insert prototype entries (without embeddings initially)
    for (ktype, prototypes) in TYPE_PROTOTYPES {
        for prototype in *prototypes {
            let _ = db.conn().execute(
                "INSERT OR IGNORE INTO keyword_type_prototypes (keyword_type, prototype_name) VALUES (?1, ?2)",
                params![ktype, prototype],
            );
        }
    }

    info!("Initialized keyword type prototype table");
    Ok(())
}

/// Get prototype embedding statistics
#[tauri::command]
pub fn get_prototype_stats(state: State<AppState>) -> Result<PrototypeStats, String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    // Check if table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='keyword_type_prototypes'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !table_exists {
        let expected: i64 = TYPE_PROTOTYPES.iter().map(|(_, p)| p.len() as i64).sum();
        return Ok(PrototypeStats {
            total: 0,
            expected,
            complete: false,
            by_type: std::collections::HashMap::new(),
        });
    }

    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM keyword_type_prototypes WHERE embedding IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let by_type: Vec<(String, i64)> = conn
        .prepare("SELECT keyword_type, COUNT(*) FROM keyword_type_prototypes WHERE embedding IS NOT NULL GROUP BY keyword_type")
        .map_err(|e| e.to_string())?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let expected: i64 = TYPE_PROTOTYPES.iter().map(|(_, p)| p.len() as i64).sum();

    Ok(PrototypeStats {
        total,
        expected,
        complete: total >= expected,
        by_type: by_type.into_iter().collect(),
    })
}

/// Generate prototype embeddings (requires Ollama)
/// This is a placeholder that marks prototypes for embedding generation
#[tauri::command]
pub async fn generate_keyword_type_prototypes(
    state: State<'_, AppState>,
) -> Result<PrototypeGenerationResult, String> {
    // First, ensure prototypes are in the table
    {
        let db = state.db_conn()?;
        init_prototype_table(db.conn())?;

        for (ktype, prototypes) in TYPE_PROTOTYPES {
            for prototype in *prototypes {
                let _ = db.conn().execute(
                    "INSERT OR IGNORE INTO keyword_type_prototypes (keyword_type, prototype_name) VALUES (?1, ?2)",
                    params![ktype, prototype],
                );
            }
        }
    }

    // Count what we have
    let db = state.db_conn()?;
    let total: i64 = db.conn()
        .query_row(
            "SELECT COUNT(*) FROM keyword_type_prototypes",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let with_embeddings: i64 = db.conn()
        .query_row(
            "SELECT COUNT(*) FROM keyword_type_prototypes WHERE embedding IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    info!("Prototype table ready: {} total, {} with embeddings", total, with_embeddings);

    Ok(PrototypeGenerationResult {
        total,
        generated: with_embeddings,
        errors: 0,
    })
}

/// Detect the type of a single keyword
#[tauri::command]
pub async fn detect_single_keyword_type(
    _state: State<'_, AppState>,
    keyword: String,
) -> Result<KeywordTypeResult, String> {
    Ok(detect_keyword_type_with_confidence(&keyword))
}

/// Batch update all keyword types with enhanced heuristic detection
#[tauri::command]
pub async fn update_keyword_types_hybrid(
    state: State<'_, AppState>,
) -> Result<KeywordTypeBatchResult, String> {
    // Get all keywords
    let keywords: Vec<(i64, String)> = {
        let db = state.db_conn()?;
        let mut stmt = db.conn()
            .prepare("SELECT id, name FROM immanentize")
            .map_err(|e| e.to_string())?;

        let result: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        result
    };

    let total = keywords.len() as i64;
    let mut processed = 0i64;
    let mut updated = 0i64;
    let mut errors = 0i64;
    let mut by_type = TypeCounts::default();
    let mut by_method = MethodCounts::default();

    for (id, name) in keywords {
        processed += 1;

        let detection = detect_keyword_type_with_confidence(&name);

        // Update database
        let update_result = {
            let db = state.db_conn()?;
            db.conn().execute(
                "UPDATE immanentize SET keyword_type = ?1 WHERE id = ?2",
                params![&detection.keyword_type, id],
            )
        };

        match update_result {
            Ok(_) => {
                updated += 1;

                // Count by type
                match detection.keyword_type.as_str() {
                    "person" => by_type.person += 1,
                    "organization" => by_type.organization += 1,
                    "location" => by_type.location += 1,
                    "acronym" => by_type.acronym += 1,
                    _ => by_type.concept += 1,
                }

                // Count by method
                by_method.heuristic += 1;
            }
            Err(e) => {
                warn!("Failed to update type for keyword {}: {}", id, e);
                errors += 1;
            }
        }
    }

    info!(
        "Keyword type batch: {} total, {} updated, {} errors",
        total, updated, errors
    );
    info!(
        "By type: {} person, {} org, {} location, {} acronym, {} concept",
        by_type.person, by_type.organization, by_type.location,
        by_type.acronym, by_type.concept
    );

    Ok(KeywordTypeBatchResult {
        total,
        processed,
        updated,
        errors,
        by_type,
        by_method,
    })
}

/// Count keywords without a type
#[tauri::command]
pub fn count_untyped_keywords(state: State<AppState>) -> Result<i64, String> {
    let db = state.db_conn()?;
    let count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM immanentize WHERE keyword_type IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    Ok(count)
}

/// Update only keywords that don't have a type yet
#[tauri::command]
pub async fn update_untyped_keywords(
    state: State<'_, AppState>,
) -> Result<KeywordTypeBatchResult, String> {
    // Get only keywords without a type
    let keywords: Vec<(i64, String)> = {
        let db = state.db_conn()?;
        let mut stmt = db
            .conn()
            .prepare("SELECT id, name FROM immanentize WHERE keyword_type IS NULL")
            .map_err(|e| e.to_string())?;

        let result: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        result
    };

    let total = keywords.len() as i64;
    let mut processed = 0i64;
    let mut updated = 0i64;
    let mut errors = 0i64;
    let mut by_type = TypeCounts::default();
    let mut by_method = MethodCounts::default();

    for (id, name) in keywords {
        processed += 1;

        let detection = detect_keyword_type_with_confidence(&name);

        // Update database
        let update_result = {
            let db = state.db_conn()?;
            db.conn().execute(
                "UPDATE immanentize SET keyword_type = ?1 WHERE id = ?2",
                params![&detection.keyword_type, id],
            )
        };

        match update_result {
            Ok(_) => {
                updated += 1;

                // Count by type
                match detection.keyword_type.as_str() {
                    "person" => by_type.person += 1,
                    "organization" => by_type.organization += 1,
                    "location" => by_type.location += 1,
                    "acronym" => by_type.acronym += 1,
                    _ => by_type.concept += 1,
                }

                // Count by method
                by_method.heuristic += 1;
            }
            Err(e) => {
                warn!("Failed to update type for keyword {}: {}", id, e);
                errors += 1;
            }
        }
    }

    info!(
        "Untyped keyword batch: {} total, {} updated, {} errors",
        total, updated, errors
    );
    info!(
        "By type: {} person, {} org, {} location, {} acronym, {} concept",
        by_type.person,
        by_type.organization,
        by_type.location,
        by_type.acronym,
        by_type.concept
    );

    Ok(KeywordTypeBatchResult {
        total,
        processed,
        updated,
        errors,
        by_type,
        by_method,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_keyword_type_with_confidence() {
        let result = detect_keyword_type_with_confidence("NATO");
        assert_eq!(result.keyword_type, "acronym");
        assert!(result.confidence > 0.9);

        let result = detect_keyword_type_with_confidence("Deutsche Bank");
        assert_eq!(result.keyword_type, "organization");

        let result = detect_keyword_type_with_confidence("Berlin");
        assert_eq!(result.keyword_type, "location");

        let result = detect_keyword_type_with_confidence("Klimawandel");
        assert_eq!(result.keyword_type, "concept");
    }

    #[test]
    fn test_type_prototypes() {
        let total: usize = TYPE_PROTOTYPES.iter().map(|(_, p)| p.len()).sum();
        assert!(total > 40); // Should have at least 40 prototypes

        // Check all 5 types are represented
        assert_eq!(TYPE_PROTOTYPES.len(), 5);
    }

    #[test]
    fn test_real_world_keywords() {
        // Test keywords from actual database
        let test_cases = [
            // Locations
            ("Berlin", "location"),
            ("Deutschland", "location"),
            ("München", "location"),
            // Acronyms
            ("CDU", "acronym"),
            ("SPD", "acronym"),
            ("EU", "acronym"),
            ("NATO", "acronym"),
            // Persons (including names starting with Sc, Fc, Ac)
            ("Donald Trump", "person"),
            ("Elon Musk", "person"),
            ("Olaf Scholz", "person"),
            ("Joe Biden", "person"),
            ("Angela Merkel", "person"),
            ("Friedrich Merz", "person"),
            // Organizations
            ("Deutsche Bank", "organization"),
            ("Bundesregierung", "organization"),
            // Sports clubs (organization, not person)
            ("Bayern SC", "organization"),  // ends with " sc" -> org pattern
            ("FC Bayern", "organization"),  // starts with "fc " -> org pattern
        ];

        for (keyword, expected_type) in test_cases {
            let result = detect_keyword_type_with_confidence(keyword);
            println!("{:30} -> {} (expected: {})", keyword, result.keyword_type, expected_type);
            assert_eq!(result.keyword_type, expected_type, "Failed for: {}", keyword);
        }
    }
}
