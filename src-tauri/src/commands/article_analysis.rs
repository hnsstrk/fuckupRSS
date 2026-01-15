//! Article Keywords & Categories Management Commands
//!
//! Commands for managing article keywords and categories with source tracking,
//! statistical analysis, and bias learning.

use crate::text_analysis::{
    BiasWeights, BiasStats, CategoryMatcher, CorrectionRecord, CorrectionType,
    TfIdfExtractor, record_correction as bias_record_correction, get_bias_stats as bias_get_stats,
    load_user_stopwords,
};
use crate::{find_canonical_keyword, AppState};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

// ============================================================
// DATA STRUCTURES
// ============================================================

/// Article keyword with source tracking
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArticleKeyword {
    pub id: i64,
    pub name: String,
    pub source: String, // 'ai', 'statistical', 'manual'
    pub confidence: f64,
}

/// Article category with source tracking
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArticleCategory {
    pub sephiroth_id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub source: String, // 'ai', 'manual'
    pub confidence: f64,
    pub parent_id: Option<i64>,
    pub parent_name: Option<String>,
    pub parent_color: Option<String>,
}

/// Statistical analysis result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatisticalAnalysis {
    pub keyword_candidates: Vec<KeywordCandidateResult>,
    pub category_scores: Vec<CategoryScoreResult>,
}

/// Keyword candidate from statistical analysis
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeywordCandidateResult {
    pub term: String,
    pub score: f64,
    pub frequency: u32,
}

/// Category score from statistical analysis
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoryScoreResult {
    pub sephiroth_id: i64,
    pub name: String,
    pub score: f64,
    pub confidence: f64,
    pub matching_terms: Vec<String>,
}

/// Correction input from frontend
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CorrectionInput {
    pub fnord_id: i64,
    pub correction_type: String, // 'keyword_added', 'keyword_removed', 'category_added', 'category_removed'
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    /// For category corrections: the terms that matched this category (from statistical analysis)
    #[serde(default)]
    pub matching_terms: Vec<String>,
    /// For category corrections: the category ID
    #[serde(default)]
    pub category_id: Option<i64>,
}

// ============================================================
// ARTICLE KEYWORDS
// ============================================================

/// Get keywords for an article with source information and source-weighted confidence
#[tauri::command]
pub fn get_article_keywords(
    state: State<AppState>,
    fnord_id: i64,
) -> Result<Vec<ArticleKeyword>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Load bias weights to apply source weighting
    let bias_weights = BiasWeights::load_from_db(db.conn()).unwrap_or_default();

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                i.id,
                i.name,
                COALESCE(fi.source, 'ai') as source,
                COALESCE(fi.confidence, 1.0) as confidence
            FROM fnord_immanentize fi
            JOIN immanentize i ON i.id = fi.immanentize_id
            WHERE fi.fnord_id = ?
            ORDER BY fi.confidence DESC, i.name ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let keywords = stmt
        .query_map([fnord_id], |row| {
            let source: String = row.get(2)?;
            let base_confidence: f64 = row.get(3)?;
            // Apply source weight to confidence
            let weighted_confidence = bias_weights.apply_source_weight(&source, base_confidence);
            Ok(ArticleKeyword {
                id: row.get(0)?,
                name: row.get(1)?,
                source,
                confidence: weighted_confidence,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(keywords)
}

/// Add a keyword to an article (manual)
#[tauri::command]
pub fn add_article_keyword(
    state: State<AppState>,
    fnord_id: i64,
    keyword: String,
) -> Result<ArticleKeyword, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let keyword_normalized = keyword.trim().to_lowercase();

    if keyword_normalized.len() < 2 || keyword_normalized.len() > 100 {
        return Err("Keyword must be 2-100 characters".to_string());
    }

    // Get or create the keyword in immanentize table
    let keyword_id: i64 = db
        .conn()
        .query_row(
            "SELECT id FROM immanentize WHERE name = ?",
            [&keyword_normalized],
            |row| row.get(0),
        )
        .or_else(|_| {
            // Create new keyword
            db.conn().execute(
                "INSERT INTO immanentize (name, count, article_count, first_seen, last_used) VALUES (?, 1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
                [&keyword_normalized],
            )?;
            Ok(db.conn().last_insert_rowid())
        })
        .map_err(|e: rusqlite::Error| e.to_string())?;

    // Add to article with source='manual'
    db.conn()
        .execute(
            "INSERT OR REPLACE INTO fnord_immanentize (fnord_id, immanentize_id, source, confidence)
             VALUES (?, ?, 'manual', 1.0)",
            params![fnord_id, keyword_id],
        )
        .map_err(|e| e.to_string())?;

    // Update article_count for the keyword
    db.conn()
        .execute(
            "UPDATE immanentize SET
                article_count = (SELECT COUNT(DISTINCT fnord_id) FROM fnord_immanentize WHERE immanentize_id = ?),
                last_used = CURRENT_TIMESTAMP
             WHERE id = ?",
            params![keyword_id, keyword_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(ArticleKeyword {
        id: keyword_id,
        name: keyword_normalized,
        source: "manual".to_string(),
        confidence: 1.0,
    })
}

/// Remove a keyword from an article
#[tauri::command]
pub fn remove_article_keyword(
    state: State<AppState>,
    fnord_id: i64,
    keyword_id: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "DELETE FROM fnord_immanentize WHERE fnord_id = ? AND immanentize_id = ?",
            params![fnord_id, keyword_id],
        )
        .map_err(|e| e.to_string())?;

    // Update article_count for the keyword
    db.conn()
        .execute(
            "UPDATE immanentize SET
                article_count = (SELECT COUNT(DISTINCT fnord_id) FROM fnord_immanentize WHERE immanentize_id = ?)
             WHERE id = ?",
            params![keyword_id, keyword_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================
// ARTICLE CATEGORIES
// ============================================================

/// Get categories for an article with source information
#[tauri::command]
pub fn get_article_categories_detailed(
    state: State<AppState>,
    fnord_id: i64,
) -> Result<Vec<ArticleCategory>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Load bias weights to apply source weighting
    let bias_weights = BiasWeights::load_from_db(db.conn()).unwrap_or_default();

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                s.id,
                s.name,
                s.icon,
                COALESCE(s.color, p.color) as color,
                COALESCE(fs.source, 'ai') as source,
                COALESCE(fs.confidence, 1.0) as confidence,
                s.parent_id,
                p.name as parent_name,
                p.color as parent_color
            FROM fnord_sephiroth fs
            JOIN sephiroth s ON s.id = fs.sephiroth_id
            LEFT JOIN sephiroth p ON p.id = s.parent_id
            WHERE fs.fnord_id = ?
            ORDER BY fs.confidence DESC, s.name ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let categories = stmt
        .query_map([fnord_id], |row| {
            let source: String = row.get(4)?;
            let base_confidence: f64 = row.get(5)?;
            // Apply source weight to confidence
            let weighted_confidence = bias_weights.apply_source_weight(&source, base_confidence);
            Ok(ArticleCategory {
                sephiroth_id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                color: row.get(3)?,
                source,
                confidence: weighted_confidence,
                parent_id: row.get(6)?,
                parent_name: row.get(7)?,
                parent_color: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(categories)
}

/// Update categories for an article (replaces all categories)
#[tauri::command]
pub fn update_article_categories(
    state: State<AppState>,
    fnord_id: i64,
    categories: Vec<i64>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Remove all existing categories
    db.conn()
        .execute("DELETE FROM fnord_sephiroth WHERE fnord_id = ?", [fnord_id])
        .map_err(|e| e.to_string())?;

    // Add new categories with source='manual'
    for sephiroth_id in categories {
        db.conn()
            .execute(
                "INSERT INTO fnord_sephiroth (fnord_id, sephiroth_id, source, confidence, assigned_at)
                 VALUES (?, ?, 'manual', 1.0, CURRENT_TIMESTAMP)",
                params![fnord_id, sephiroth_id],
            )
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Add a single category to an article
#[tauri::command]
pub fn add_article_category(
    state: State<AppState>,
    fnord_id: i64,
    sephiroth_id: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "INSERT OR REPLACE INTO fnord_sephiroth (fnord_id, sephiroth_id, source, confidence, assigned_at)
             VALUES (?, ?, 'manual', 1.0, CURRENT_TIMESTAMP)",
            params![fnord_id, sephiroth_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Remove a category from an article
#[tauri::command]
pub fn remove_article_category(
    state: State<AppState>,
    fnord_id: i64,
    sephiroth_id: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "DELETE FROM fnord_sephiroth WHERE fnord_id = ? AND sephiroth_id = ?",
            params![fnord_id, sephiroth_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================
// STATISTICAL ANALYSIS
// ============================================================

/// Perform statistical analysis on article text
#[tauri::command]
pub fn analyze_article_statistical(
    state: State<AppState>,
    fnord_id: i64,
) -> Result<StatisticalAnalysis, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get article content
    let content: String = db
        .conn()
        .query_row(
            "SELECT COALESCE(content_full, content_raw, '') FROM fnords WHERE id = ?",
            [fnord_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if content.is_empty() {
        return Ok(StatisticalAnalysis {
            keyword_candidates: Vec::new(),
            category_scores: Vec::new(),
        });
    }

    // Load bias weights
    let bias = BiasWeights::load_from_db(db.conn()).unwrap_or_default();

    // Load user stopwords
    let user_stopwords = load_user_stopwords(db.conn()).unwrap_or_default();

    // TF-IDF keyword extraction with user stopwords
    let extractor = TfIdfExtractor::new().with_max_keywords(15);
    let keyword_candidates: Vec<KeywordCandidateResult> = extractor
        .extract_simple_with_stopwords(&content, &user_stopwords)
        .into_iter()
        .map(|kc| {
            let adjusted_score = bias.apply_to_keyword(&kc.term, kc.score);
            // Normalize to canonical form if available
            let term = find_canonical_keyword(&kc.term)
                .map(|s| s.to_string())
                .unwrap_or(kc.term);
            KeywordCandidateResult {
                term,
                score: adjusted_score,
                frequency: kc.frequency,
            }
        })
        .collect();

    // Category matching
    let matcher = CategoryMatcher::new().with_max_categories(5);
    let category_scores: Vec<CategoryScoreResult> = matcher
        .score_categories(&content, Some(&bias))
        .into_iter()
        .map(|cs| CategoryScoreResult {
            sephiroth_id: cs.sephiroth_id,
            name: cs.name,
            score: cs.score,
            confidence: cs.confidence,
            matching_terms: cs.matching_terms,
        })
        .collect();

    Ok(StatisticalAnalysis {
        keyword_candidates,
        category_scores,
    })
}

// ============================================================
// BATCH STATISTICAL ANALYSIS
// ============================================================

/// Result of batch statistical analysis
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchStatisticalResult {
    pub processed: usize,
    pub total: usize,
    pub errors: Vec<String>,
}

/// Count unprocessed articles (no LLM analysis yet) that have full content
#[tauri::command]
pub fn get_unprocessed_statistical_count(state: State<AppState>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let count: i64 = db
        .conn()
        .query_row(
            r#"
            SELECT COUNT(*)
            FROM fnords
            WHERE processed_at IS NULL
              AND content_full IS NOT NULL
              AND content_full != ''
            "#,
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(count)
}

/// Progress event for statistical analysis
#[derive(Debug, Serialize, Clone)]
pub struct StatisticalProgress {
    pub current: i64,
    pub total: i64,
    pub fnord_id: i64,
    pub title: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Run statistical analysis on all unprocessed articles
/// Extracts keywords and categories using TF-IDF and word matching
#[tauri::command]
pub async fn process_statistical_batch(
    window: tauri::Window,
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<BatchStatisticalResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Load bias weights once
    let bias = BiasWeights::load_from_db(db.conn()).unwrap_or_default();

    // Load user stopwords once
    let user_stopwords = load_user_stopwords(db.conn()).unwrap_or_default();

    // Get unprocessed articles with full content (including title for progress)
    let limit_val = limit.unwrap_or(10000);
    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT id, title, COALESCE(content_full, '') as content
            FROM fnords
            WHERE processed_at IS NULL
              AND content_full IS NOT NULL
              AND content_full != ''
            ORDER BY published_at DESC
            LIMIT ?
            "#,
        )
        .map_err(|e| e.to_string())?;

    let articles: Vec<(i64, String, String)> = stmt
        .query_map([limit_val], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let total = articles.len() as i64;
    let mut processed = 0usize;
    let mut errors = Vec::new();

    // Emit initial progress
    let _ = window.emit("statistical-progress", StatisticalProgress {
        current: 0,
        total,
        fnord_id: 0,
        title: "Starting...".to_string(),
        success: true,
        error: None,
    });

    // Create extractors
    let extractor = TfIdfExtractor::new().with_max_keywords(10);
    let matcher = CategoryMatcher::new().with_max_categories(3);

    for (idx, (fnord_id, title, content)) in articles.into_iter().enumerate() {
        if content.is_empty() {
            continue;
        }

        // Extract keywords
        let keywords = extractor.extract_simple_with_stopwords(&content, &user_stopwords);

        // Match categories
        let categories = matcher.score_categories(&content, Some(&bias));

        // Store keywords with source='statistical'
        for kc in keywords.iter().take(10) {
            let adjusted_score = bias.apply_to_keyword(&kc.term, kc.score);
            if adjusted_score < 0.05 {
                continue; // Skip very low scoring keywords
            }

            // Normalize keyword to canonical form if available
            // e.g., "european" -> "Europäische Union", "russian" -> "Russland"
            let keyword_name = find_canonical_keyword(&kc.term)
                .map(|s| s.to_string())
                .unwrap_or_else(|| kc.term.clone());

            // Get or create keyword
            let keyword_id: i64 = db
                .conn()
                .query_row(
                    "SELECT id FROM immanentize WHERE name = ?",
                    [&keyword_name],
                    |row| row.get(0),
                )
                .or_else(|_| {
                    db.conn().execute(
                        "INSERT INTO immanentize (name, count, article_count, first_seen, last_used) VALUES (?, 1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
                        [&keyword_name],
                    )?;
                    Ok::<i64, rusqlite::Error>(db.conn().last_insert_rowid())
                })
                .unwrap_or(0);

            if keyword_id > 0 {
                // Insert with source='statistical', only if not already present
                let _ = db.conn().execute(
                    "INSERT OR IGNORE INTO fnord_immanentize (fnord_id, immanentize_id, source, confidence)
                     VALUES (?, ?, 'statistical', ?)",
                    params![fnord_id, keyword_id, adjusted_score.min(1.0)],
                );

                // Update article_count
                let _ = db.conn().execute(
                    "UPDATE immanentize SET
                        article_count = (SELECT COUNT(DISTINCT fnord_id) FROM fnord_immanentize WHERE immanentize_id = ?),
                        last_used = CURRENT_TIMESTAMP
                     WHERE id = ?",
                    params![keyword_id, keyword_id],
                );
            }
        }

        // Store categories with source='statistical' (only subcategories, level=1)
        for cs in categories.iter().take(3) {
            if cs.confidence < 0.3 {
                continue; // Skip low-confidence categories
            }

            // Check if this is a subcategory (level=1)
            let is_subcategory: bool = db
                .conn()
                .query_row(
                    "SELECT level = 1 FROM sephiroth WHERE id = ?",
                    [cs.sephiroth_id],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if !is_subcategory {
                continue;
            }

            // Insert with source='ai' (statistical categories use same source as AI for now)
            let _ = db.conn().execute(
                "INSERT OR IGNORE INTO fnord_sephiroth (fnord_id, sephiroth_id, source, confidence, assigned_at)
                 VALUES (?, ?, 'ai', ?, CURRENT_TIMESTAMP)",
                params![fnord_id, cs.sephiroth_id, cs.confidence],
            );
        }

        processed += 1;

        // Emit progress every article (fast enough for statistical analysis)
        let _ = window.emit("statistical-progress", StatisticalProgress {
            current: (idx + 1) as i64,
            total,
            fnord_id,
            title: title.clone(),
            success: true,
            error: None,
        });
    }

    // Emit completion
    let _ = window.emit("statistical-progress", StatisticalProgress {
        current: total,
        total,
        fnord_id: 0,
        title: "Complete".to_string(),
        success: true,
        error: None,
    });

    Ok(BatchStatisticalResult {
        processed,
        total: total as usize,
        errors,
    })
}

// ============================================================
// BIAS LEARNING
// ============================================================

/// Record a user correction for bias learning
#[tauri::command]
pub fn record_correction(state: State<AppState>, correction: CorrectionInput) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let correction_type = match correction.correction_type.as_str() {
        "keyword_added" => CorrectionType::KeywordAdded,
        "keyword_removed" => CorrectionType::KeywordRemoved,
        "category_added" => CorrectionType::CategoryAdded,
        "category_removed" => CorrectionType::CategoryRemoved,
        _ => return Err(format!("Unknown correction type: {}", correction.correction_type)),
    };

    let record = CorrectionRecord {
        fnord_id: correction.fnord_id,
        correction_type,
        old_value: correction.old_value,
        new_value: correction.new_value,
        matching_terms: correction.matching_terms,
        category_id: correction.category_id,
    };

    bias_record_correction(db.conn(), &record)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get bias statistics
#[tauri::command]
pub fn get_bias_stats(state: State<AppState>) -> Result<BiasStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    bias_get_stats(db.conn()).map_err(|e| e.to_string())
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correction_type_parsing() {
        let input = CorrectionInput {
            fnord_id: 1,
            correction_type: "keyword_added".to_string(),
            old_value: None,
            new_value: Some("test".to_string()),
            matching_terms: Vec::new(),
            category_id: None,
        };

        assert_eq!(input.correction_type, "keyword_added");
    }

    #[test]
    fn test_correction_with_category_data() {
        let input = CorrectionInput {
            fnord_id: 1,
            correction_type: "category_removed".to_string(),
            old_value: Some("Politik".to_string()),
            new_value: None,
            matching_terms: vec!["regierung".to_string(), "minister".to_string()],
            category_id: Some(201),
        };

        assert_eq!(input.correction_type, "category_removed");
        assert_eq!(input.matching_terms.len(), 2);
        assert_eq!(input.category_id, Some(201));
    }
}
