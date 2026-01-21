//! Article Keywords & Categories Management Commands
//!
//! Commands for managing article keywords and categories with source tracking,
//! statistical analysis, and bias learning.

use crate::text_analysis::{
    BiasWeights, BiasStats, CategoryMatcher, CorrectionRecord, CorrectionType,
    TfIdfExtractor, record_correction as bias_record_correction, get_bias_stats as bias_get_stats,
    load_user_stopwords,
};
use crate::{find_canonical_keyword_with_db, AppState};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

// ============================================================
// HELPER FUNCTIONS
// ============================================================

/// Capitalize keyword for proper noun form (German style)
/// Single words get capitalized first letter, multi-word phrases get each word capitalized
fn capitalize_keyword(keyword: &str) -> String {
    let trimmed = keyword.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // If it's all uppercase (acronym), keep it
    if trimmed.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
        return trimmed.to_string();
    }

    // Capitalize first letter of each word
    trimmed
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let first_upper: String = first.to_uppercase().collect();
                    first_upper + chars.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ============================================================
// DATA STRUCTURES
// ============================================================

/// Keyword type for entity classification
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum KeywordTypeInfo {
    Concept,
    Person,
    Organization,
    Location,
    Acronym,
}

impl Default for KeywordTypeInfo {
    fn default() -> Self {
        Self::Concept
    }
}

/// Article keyword with source tracking and advanced metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArticleKeyword {
    pub id: i64,
    pub name: String,
    pub source: String, // 'ai', 'statistical', 'manual'
    pub confidence: f64,
    // Advanced extraction metadata
    pub keyword_type: KeywordTypeInfo,
    pub extraction_methods: Vec<String>,
    pub quality_score: Option<f64>,
    pub semantic_score: Option<f64>,
}

/// Infer keyword type from name patterns
fn infer_keyword_type(name: &str) -> KeywordTypeInfo {
    let words: Vec<&str> = name.split_whitespace().collect();

    // Check for acronyms (all uppercase, 2-6 chars)
    if words.len() == 1 {
        let word = words[0];
        if word.len() >= 2 && word.len() <= 6 && word.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
            return KeywordTypeInfo::Acronym;
        }
    }

    // Check for organization patterns
    let org_indicators = [
        "gmbh", "ag", "inc", "ltd", "corp", "kg", "e.v.", "se",
        "ministerium", "ministry", "bundesamt", "behörde", "agency",
        "bank", "verband", "stiftung", "foundation", "institute", "institut",
        "universität", "university", "partei", "party",
    ];
    let lower = name.to_lowercase();
    for indicator in org_indicators {
        if lower.ends_with(indicator) || lower.contains(&format!(" {}", indicator)) {
            return KeywordTypeInfo::Organization;
        }
    }

    // Check for person names (2-3 capitalized words)
    if (words.len() == 2 || words.len() == 3)
        && words.iter().all(|w| {
            w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                && w.len() >= 2
                && w.len() <= 15
                && !org_indicators.iter().any(|ind| w.to_lowercase().contains(ind))
        })
    {
        // Additional check: not a known location or organization prefix
        let not_location = !["New", "Los", "San", "Las", "Den", "Der", "Die"].contains(&words[0]);
        if not_location {
            return KeywordTypeInfo::Person;
        }
    }

    // Check for location patterns
    let location_indicators = [
        "stadt", "city", "land", "country", "region", "province", "state",
        "republic", "republik", "kingdom", "königreich",
    ];
    for indicator in location_indicators {
        if lower.contains(indicator) {
            return KeywordTypeInfo::Location;
        }
    }

    KeywordTypeInfo::Concept
}

/// Infer extraction methods from source
fn infer_extraction_methods(source: &str) -> Vec<String> {
    match source {
        "ai" => vec!["ai".to_string()],
        "statistical" => vec!["tfidf".to_string(), "yake".to_string(), "rake".to_string()],
        "manual" => vec!["manual".to_string()],
        _ => vec![source.to_string()],
    }
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

/// Get keywords for an article with source information, source-weighted confidence, and advanced metadata
#[tauri::command]
pub fn get_article_keywords(
    state: State<AppState>,
    fnord_id: i64,
) -> Result<Vec<ArticleKeyword>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Load bias weights to apply source weighting
    let bias_weights = BiasWeights::load_from_db(db.conn()).unwrap_or_default();

    // Get article embedding for semantic score calculation
    let article_embedding: Option<Vec<u8>> = db
        .conn()
        .query_row(
            "SELECT embedding FROM fnords WHERE id = ?",
            [fnord_id],
            |row| row.get(0),
        )
        .ok();

    // Query keywords with their embeddings for semantic score calculation
    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                i.id,
                i.name,
                COALESCE(fi.source, 'ai') as source,
                COALESCE(fi.confidence, 1.0) as confidence,
                i.quality_score,
                i.embedding
            FROM fnord_immanentize fi
            JOIN immanentize i ON i.id = fi.immanentize_id
            WHERE fi.fnord_id = ?
            ORDER BY fi.confidence DESC, i.name ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let keywords = stmt
        .query_map([fnord_id], |row| {
            let name: String = row.get(1)?;
            let source: String = row.get(2)?;
            let base_confidence: f64 = row.get(3)?;
            let quality_score: Option<f64> = row.get(4)?;
            let keyword_embedding: Option<Vec<u8>> = row.get(5)?;

            // Apply source weight to confidence
            let weighted_confidence = bias_weights.apply_source_weight(&source, base_confidence);

            // Infer keyword type from name patterns
            let keyword_type = infer_keyword_type(&name);

            // Infer extraction methods from source
            let extraction_methods = infer_extraction_methods(&source);

            // Calculate semantic score from embeddings if both are available
            let semantic_score = match (&article_embedding, &keyword_embedding) {
                (Some(article_emb), Some(kw_emb)) => {
                    let similarity = cosine_similarity_blob(article_emb, kw_emb);
                    // Only return score if it's meaningful (not zero due to dimension mismatch)
                    if similarity > 0.0 {
                        Some(similarity)
                    } else {
                        None
                    }
                }
                _ => None,
            };

            Ok(ArticleKeyword {
                id: row.get(0)?,
                name,
                source,
                confidence: weighted_confidence,
                keyword_type,
                extraction_methods,
                quality_score,
                semantic_score,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(keywords)
}

/// Add a keyword to an article (manual)
/// Uses the learning system: checks SYNONYM_GROUPS, then existing keywords (case-insensitive)
#[tauri::command]
pub fn add_article_keyword(
    state: State<AppState>,
    fnord_id: i64,
    keyword: String,
) -> Result<ArticleKeyword, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let keyword_trimmed = keyword.trim();

    if keyword_trimmed.len() < 2 || keyword_trimmed.len() > 100 {
        return Err("Keyword must be 2-100 characters".to_string());
    }

    // Step 1: Check SYNONYM_GROUPS and dynamic DB synonyms for canonical form
    let keyword_name = find_canonical_keyword_with_db(keyword_trimmed)
        .unwrap_or_else(|| keyword_trimmed.to_string());

    // Step 2: Try to find existing keyword (learning from existing data)
    // Priority: exact match > case-insensitive match
    let (keyword_id, final_name): (i64, String) = db
        .conn()
        .query_row(
            "SELECT id, name FROM immanentize WHERE name = ?",
            [&keyword_name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .or_else(|_| {
            // Try case-insensitive match
            db.conn().query_row(
                "SELECT id, name FROM immanentize WHERE LOWER(name) = LOWER(?) LIMIT 1",
                [&keyword_name],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
        })
        .or_else(|_| {
            // Create new keyword with proper capitalization
            let normalized_name = capitalize_keyword(&keyword_name);
            db.conn().execute(
                "INSERT INTO immanentize (name, count, article_count, first_seen, last_used) VALUES (?, 1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
                [&normalized_name],
            )?;
            Ok::<(i64, String), rusqlite::Error>((db.conn().last_insert_rowid(), normalized_name))
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

    // Record correction for bias learning - user added this keyword
    let _ = bias_record_correction(
        db.conn(),
        &CorrectionRecord {
            fnord_id,
            correction_type: CorrectionType::KeywordAdded,
            old_value: None,
            new_value: Some(final_name.clone()),
            matching_terms: Vec::new(),
            category_id: None,
        },
    );

    // Get quality score from DB if available
    let quality_score: Option<f64> = db
        .conn()
        .query_row("SELECT quality_score FROM immanentize WHERE id = ?", [keyword_id], |row| row.get(0))
        .ok();

    Ok(ArticleKeyword {
        id: keyword_id,
        name: final_name.clone(),
        source: "manual".to_string(),
        confidence: 1.0,
        keyword_type: infer_keyword_type(&final_name),
        extraction_methods: vec!["manual".to_string()],
        quality_score,
        semantic_score: None,
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

    // Get keyword name before deleting for bias learning
    let keyword_name: Option<String> = db
        .conn()
        .query_row("SELECT name FROM immanentize WHERE id = ?", [keyword_id], |row| row.get(0))
        .ok();

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

    // Record correction for bias learning - user removed this keyword
    if let Some(name) = keyword_name {
        let _ = bias_record_correction(
            db.conn(),
            &CorrectionRecord {
                fnord_id,
                correction_type: CorrectionType::KeywordRemoved,
                old_value: Some(name),
                new_value: None,
                matching_terms: Vec::new(),
                category_id: None,
            },
        );
    }

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

    // Get category name for bias learning
    let category_name: String = db
        .conn()
        .query_row("SELECT name FROM sephiroth WHERE id = ?", [sephiroth_id], |row| row.get(0))
        .unwrap_or_else(|_| sephiroth_id.to_string());

    db.conn()
        .execute(
            "INSERT OR REPLACE INTO fnord_sephiroth (fnord_id, sephiroth_id, source, confidence, assigned_at)
             VALUES (?, ?, 'manual', 1.0, CURRENT_TIMESTAMP)",
            params![fnord_id, sephiroth_id],
        )
        .map_err(|e| e.to_string())?;

    // Record correction for bias learning
    let _ = bias_record_correction(
        db.conn(),
        &CorrectionRecord {
            fnord_id,
            correction_type: CorrectionType::CategoryAdded,
            old_value: None,
            new_value: Some(category_name),
            matching_terms: Vec::new(),
            category_id: Some(sephiroth_id),
        },
    );

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

    // Get category name for bias learning
    let category_name: String = db
        .conn()
        .query_row("SELECT name FROM sephiroth WHERE id = ?", [sephiroth_id], |row| row.get(0))
        .unwrap_or_else(|_| sephiroth_id.to_string());

    db.conn()
        .execute(
            "DELETE FROM fnord_sephiroth WHERE fnord_id = ? AND sephiroth_id = ?",
            params![fnord_id, sephiroth_id],
        )
        .map_err(|e| e.to_string())?;

    // Record correction for bias learning
    let _ = bias_record_correction(
        db.conn(),
        &CorrectionRecord {
            fnord_id,
            correction_type: CorrectionType::CategoryRemoved,
            old_value: Some(category_name),
            new_value: None,
            matching_terms: Vec::new(),
            category_id: Some(sephiroth_id),
        },
    );

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
            // Normalize to canonical form if available (static + dynamic synonyms)
            let term = find_canonical_keyword_with_db(&kc.term)
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
    // Phase 1: Load articles and config (short lock)
    let (articles, bias, user_stopwords) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        // Load bias weights
        let bias = BiasWeights::load_from_db(db.conn()).unwrap_or_default();

        // Load user stopwords
        let user_stopwords = load_user_stopwords(db.conn()).unwrap_or_default();

        // Get unprocessed articles with full content
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

        (articles, bias, user_stopwords)
    }; // Lock released here!

    let total = articles.len() as i64;
    let mut processed = 0usize;
    let mut errors = Vec::new();

    // Emit initial progress (no lock needed)
    let _ = window.emit("statistical-progress", StatisticalProgress {
        current: 0,
        total,
        fnord_id: 0,
        title: "Starting...".to_string(),
        success: true,
        error: None,
    });

    // Create extractors (no DB needed)
    let extractor = TfIdfExtractor::new().with_max_keywords(10);
    let matcher = CategoryMatcher::new().with_max_categories(3);

    // Phase 2: Process each article with individual lock acquisition
    for (idx, (fnord_id, title, content)) in articles.into_iter().enumerate() {
        if content.is_empty() {
            continue;
        }

        // CPU-bound extraction (no lock needed)
        let keywords = extractor.extract_simple_with_stopwords(&content, &user_stopwords);
        let categories = matcher.score_categories(&content, Some(&bias));

        // Prepare keyword data before acquiring lock
        let mut keyword_ops: Vec<(String, f64)> = Vec::new();
        for kc in keywords.iter().take(10) {
            let adjusted_score = bias.apply_to_keyword(&kc.term, kc.score);
            if adjusted_score < 0.05 {
                continue;
            }
            let keyword_name = find_canonical_keyword_with_db(&kc.term)
                .unwrap_or_else(|| kc.term.clone());
            keyword_ops.push((keyword_name, adjusted_score));
        }

        // Prepare category data
        let category_ops: Vec<(i64, f64)> = categories
            .iter()
            .take(3)
            .filter(|cs| cs.confidence >= 0.3)
            .map(|cs| (cs.sephiroth_id, cs.confidence))
            .collect();

        // Phase 3: Database operations with transaction (short lock per article)
        let article_result = {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let conn = db.conn();

            // Use transaction for atomicity
            conn.execute("BEGIN TRANSACTION", []).map_err(|e| e.to_string())?;

            let mut success = true;
            let mut error_msg: Option<String> = None;

            // Store keywords
            for (keyword_name, adjusted_score) in &keyword_ops {
                // Find or create keyword
                let keyword_id: i64 = conn
                    .query_row(
                        "SELECT id FROM immanentize WHERE name = ?",
                        [keyword_name],
                        |row| row.get(0),
                    )
                    .or_else(|_| {
                        conn.query_row(
                            "SELECT id FROM immanentize WHERE LOWER(name) = LOWER(?) ORDER BY name = name COLLATE NOCASE DESC LIMIT 1",
                            [keyword_name],
                            |row| row.get(0),
                        )
                    })
                    .or_else(|_| {
                        let normalized_name = capitalize_keyword(keyword_name);
                        conn.execute(
                            "INSERT INTO immanentize (name, count, article_count, first_seen, last_used) VALUES (?, 1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
                            [&normalized_name],
                        )?;
                        Ok::<i64, rusqlite::Error>(conn.last_insert_rowid())
                    })
                    .unwrap_or(0);

                if keyword_id > 0 {
                    let _ = conn.execute(
                        "INSERT OR IGNORE INTO fnord_immanentize (fnord_id, immanentize_id, source, confidence)
                         VALUES (?, ?, 'statistical', ?)",
                        params![fnord_id, keyword_id, adjusted_score.min(1.0)],
                    );

                    let _ = conn.execute(
                        "UPDATE immanentize SET
                            article_count = (SELECT COUNT(DISTINCT fnord_id) FROM fnord_immanentize WHERE immanentize_id = ?),
                            last_used = CURRENT_TIMESTAMP
                         WHERE id = ?",
                        params![keyword_id, keyword_id],
                    );
                }
            }

            // Store categories (only subcategories)
            for (sephiroth_id, confidence) in &category_ops {
                let is_subcategory: bool = conn
                    .query_row(
                        "SELECT level = 1 FROM sephiroth WHERE id = ?",
                        [sephiroth_id],
                        |row| row.get(0),
                    )
                    .unwrap_or(false);

                if is_subcategory {
                    let _ = conn.execute(
                        "INSERT OR IGNORE INTO fnord_sephiroth (fnord_id, sephiroth_id, source, confidence, assigned_at)
                         VALUES (?, ?, 'statistical', ?, CURRENT_TIMESTAMP)",
                        params![fnord_id, sephiroth_id, confidence],
                    );
                }
            }

            // CRITICAL: Mark article as processed to prevent re-processing
            if let Err(e) = conn.execute(
                "UPDATE fnords SET processed_at = CURRENT_TIMESTAMP WHERE id = ?",
                [fnord_id],
            ) {
                success = false;
                error_msg = Some(format!("Failed to mark as processed: {}", e));
            }

            // Commit or rollback
            if success {
                if let Err(e) = conn.execute("COMMIT", []) {
                    let _ = conn.execute("ROLLBACK", []);
                    success = false;
                    error_msg = Some(format!("Commit failed: {}", e));
                }
            } else {
                let _ = conn.execute("ROLLBACK", []);
            }

            (success, error_msg)
        }; // Lock released here!

        let (success, error_msg) = article_result;
        if success {
            processed += 1;
        } else if let Some(ref err) = error_msg {
            errors.push(format!("{}: {}", title, err));
        }

        // Emit progress (no lock held)
        let _ = window.emit("statistical-progress", StatisticalProgress {
            current: (idx + 1) as i64,
            total,
            fnord_id,
            title: title.clone(),
            success,
            error: error_msg.clone(),
        });

        // Yield to allow other tasks (embedding worker, UI) to run
        tokio::task::yield_now().await;
    }

    // Emit completion (no lock needed)
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
// SIMILAR KEYWORDS (from Immanentize Network)
// ============================================================

/// Similar keyword from the network
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimilarKeywordInfo {
    pub id: i64,
    pub name: String,
    pub similarity: f64,
    pub cooccurrence: i64,
}

/// Get similar keywords for a given keyword from the Immanentize network
#[tauri::command]
pub fn get_similar_keywords(
    state: State<AppState>,
    keyword_id: i64,
    limit: Option<i64>,
) -> Result<Vec<SimilarKeywordInfo>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let limit_val = limit.unwrap_or(5);

    // Optimized query: First find neighbors via index, then lookup keyword info
    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                i.id,
                i.name,
                COALESCE(n.embedding_similarity, 0.0) as similarity,
                COALESCE(n.cooccurrence, 0) as cooccurrence
            FROM immanentize_neighbors n
            JOIN immanentize i ON i.id = CASE
                WHEN n.immanentize_id_a = ?1 THEN n.immanentize_id_b
                ELSE n.immanentize_id_a
            END
            WHERE n.immanentize_id_a = ?1 OR n.immanentize_id_b = ?1
            ORDER BY n.combined_weight DESC, n.cooccurrence DESC
            LIMIT ?2
            "#,
        )
        .map_err(|e| e.to_string())?;

    let similar = stmt
        .query_map(params![keyword_id, limit_val], |row| {
            Ok(SimilarKeywordInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                similarity: row.get(2)?,
                cooccurrence: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(similar)
}

/// Get suggested keywords for an article based on assigned keywords' network
#[tauri::command]
pub fn get_keyword_suggestions_from_network(
    state: State<AppState>,
    fnord_id: i64,
    limit: Option<i64>,
) -> Result<Vec<SimilarKeywordInfo>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let limit_val = limit.unwrap_or(5);

    // Get neighbors of assigned keywords that are not already assigned
    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                i.id,
                i.name,
                MAX(COALESCE(n.embedding_similarity, 0.0)) as max_similarity,
                SUM(COALESCE(n.cooccurrence, 0)) as total_cooccurrence
            FROM fnord_immanentize fi
            JOIN immanentize_neighbors n ON (
                n.immanentize_id_a = fi.immanentize_id OR n.immanentize_id_b = fi.immanentize_id
            )
            JOIN immanentize i ON (
                i.id = CASE
                    WHEN n.immanentize_id_a = fi.immanentize_id THEN n.immanentize_id_b
                    ELSE n.immanentize_id_a
                END
            )
            WHERE fi.fnord_id = ?
              AND i.id NOT IN (SELECT immanentize_id FROM fnord_immanentize WHERE fnord_id = ?)
            GROUP BY i.id, i.name
            ORDER BY total_cooccurrence DESC, max_similarity DESC
            LIMIT ?
            "#,
        )
        .map_err(|e| e.to_string())?;

    let suggestions = stmt
        .query_map(params![fnord_id, fnord_id, limit_val], |row| {
            Ok(SimilarKeywordInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                similarity: row.get(2)?,
                cooccurrence: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(suggestions)
}

// ============================================================
// SEMANTIC KEYWORD SCORING
// ============================================================

/// Result of semantic keyword scoring
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SemanticKeywordScore {
    pub keyword: String,
    pub base_score: f64,
    pub semantic_score: f64,
    pub combined_score: f64,
}

/// Calculate semantic scores for keyword candidates using embeddings
/// This compares keyword embeddings against the article embedding
#[tauri::command]
pub async fn score_keywords_semantically(
    state: State<'_, AppState>,
    fnord_id: i64,
    keywords: Vec<String>,
    semantic_weight: Option<f64>,
) -> Result<Vec<SemanticKeywordScore>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let weight = semantic_weight.unwrap_or(0.3);

    // Get article embedding
    let article_embedding: Option<Vec<u8>> = db
        .conn()
        .query_row(
            "SELECT embedding FROM fnords WHERE id = ?",
            [fnord_id],
            |row| row.get(0),
        )
        .ok();

    if article_embedding.is_none() {
        // No article embedding, return base scores only
        return Ok(keywords
            .into_iter()
            .map(|kw| SemanticKeywordScore {
                keyword: kw,
                base_score: 0.5,
                semantic_score: 0.0,
                combined_score: 0.5,
            })
            .collect());
    }

    let article_emb = article_embedding.unwrap();

    // Get keyword embeddings and calculate similarities
    let mut results = Vec::new();
    for keyword in keywords {
        let keyword_embedding: Option<Vec<u8>> = db
            .conn()
            .query_row(
                "SELECT embedding FROM immanentize WHERE name = ? OR LOWER(name) = LOWER(?)",
                params![&keyword, &keyword],
                |row| row.get(0),
            )
            .ok();

        let semantic_score = if let Some(kw_emb) = keyword_embedding {
            cosine_similarity_blob(&article_emb, &kw_emb)
        } else {
            0.0
        };

        let base_score = 0.5; // Default base score, could be enhanced
        let combined = (base_score * (1.0 - weight)) + (semantic_score * weight);

        results.push(SemanticKeywordScore {
            keyword,
            base_score,
            semantic_score,
            combined_score: combined,
        });
    }

    // Sort by combined score descending
    results.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap_or(std::cmp::Ordering::Equal));

    Ok(results)
}

/// Calculate cosine similarity between two embedding blobs
fn cosine_similarity_blob(a: &[u8], b: &[u8]) -> f64 {
    // Embeddings are stored as f32 arrays (1024 dimensions for snowflake-arctic-embed2)
    if a.len() != b.len() || a.len() % 4 != 0 {
        return 0.0;
    }

    let dim = a.len() / 4;
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;

    for i in 0..dim {
        let offset = i * 4;
        let val_a = f32::from_le_bytes([a[offset], a[offset + 1], a[offset + 2], a[offset + 3]]) as f64;
        let val_b = f32::from_le_bytes([b[offset], b[offset + 1], b[offset + 2], b[offset + 3]]) as f64;

        dot += val_a * val_b;
        norm_a += val_a * val_a;
        norm_b += val_b * val_b;
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a.sqrt() * norm_b.sqrt())
}

// ============================================================
// CATEGORY MAINTENANCE
// ============================================================

/// Result of category fix operation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoryFixResult {
    /// Number of articles that were fixed
    pub fixed_count: i64,
    /// Categories that were added (name -> count)
    pub categories_added: std::collections::HashMap<String, i64>,
    /// Total articles scanned
    pub total_scanned: i64,
}

/// Fix category assignments by deriving categories from keyword associations
///
/// Algorithm:
/// 1. For each article with keywords but potentially missing categories
/// 2. Get all keywords assigned to the article (fnord_immanentize)
/// 3. For each keyword, get its category associations (immanentize_sephiroth)
/// 4. Weight categories by:
///    - The keyword-category weight
///    - A specificity factor (1/num_categories for the keyword - specific keywords count more)
/// 5. Sum weighted scores per category
/// 6. Add categories that exceed a threshold and aren't already assigned
#[tauri::command]
pub fn fix_category_assignments(
    state: State<AppState>,
) -> Result<CategoryFixResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.conn();

    // Configuration
    let min_score_threshold = 0.15; // Minimum aggregated score to add a category
    let min_keywords_for_category = 2; // Minimum keywords supporting a category
    let max_keyword_categories = 6; // Ignore keywords with more categories (too unspecific)

    // Count total articles with keywords
    let total_scanned: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT fnord_id) FROM fnord_immanentize",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let mut categories_added: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut fixed_count = 0i64;

    // Get all subcategories (level = 1)
    let mut category_stmt = conn
        .prepare("SELECT id, name FROM sephiroth WHERE level = 1")
        .map_err(|e| e.to_string())?;

    let categories: Vec<(i64, String)> = category_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // For each article with keywords, calculate category scores from keyword network
    // SQL approach: aggregate category scores from keywords in a single query per category
    for (sephiroth_id, category_name) in &categories {
        // Find articles where:
        // - They have keywords
        // - Those keywords are associated with this category
        // - The article doesn't already have this category
        // - The aggregated weighted score exceeds our threshold
        let sql = r#"
            WITH keyword_category_scores AS (
                -- For each article's keyword, get the category association
                SELECT
                    fi.fnord_id,
                    fi.immanentize_id,
                    ims.weight as category_weight,
                    -- Specificity: keywords with fewer categories are more reliable
                    1.0 / NULLIF(
                        (SELECT COUNT(*) FROM immanentize_sephiroth
                         WHERE immanentize_id = fi.immanentize_id),
                        0
                    ) as specificity
                FROM fnord_immanentize fi
                JOIN immanentize_sephiroth ims ON ims.immanentize_id = fi.immanentize_id
                WHERE ims.sephiroth_id = ?1
                -- Only consider keywords that aren't too generic
                AND (SELECT COUNT(*) FROM immanentize_sephiroth
                     WHERE immanentize_id = fi.immanentize_id) <= ?4
            ),
            article_scores AS (
                -- Aggregate scores per article
                SELECT
                    fnord_id,
                    SUM(category_weight * specificity) as total_score,
                    COUNT(DISTINCT immanentize_id) as supporting_keywords
                FROM keyword_category_scores
                GROUP BY fnord_id
                HAVING total_score >= ?2
                   AND supporting_keywords >= ?3
            )
            -- Insert for articles that don't have this category yet
            INSERT OR IGNORE INTO fnord_sephiroth (fnord_id, sephiroth_id, confidence, source, assigned_at)
            SELECT
                a.fnord_id,
                ?1,
                MIN(a.total_score, 1.0),
                'statistical',
                datetime('now')
            FROM article_scores a
            WHERE a.fnord_id NOT IN (
                SELECT fnord_id FROM fnord_sephiroth WHERE sephiroth_id = ?1
            )
        "#;

        let affected = conn
            .execute(
                sql,
                params![
                    sephiroth_id,
                    min_score_threshold,
                    min_keywords_for_category,
                    max_keyword_categories
                ],
            )
            .unwrap_or(0) as i64;

        if affected > 0 {
            categories_added.insert(category_name.clone(), affected);
            fixed_count += affected;
        }
    }

    // Update article_count for affected categories
    conn.execute_batch(
        r#"
        UPDATE sephiroth SET article_count = (
            SELECT COUNT(DISTINCT fnord_id) FROM fnord_sephiroth WHERE sephiroth_id = sephiroth.id
        );
        "#,
    )
    .ok();

    Ok(CategoryFixResult {
        fixed_count,
        categories_added,
        total_scanned,
    })
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
