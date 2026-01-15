//! Shared helper functions for Ollama commands

use crate::db::Database;
use crate::ollama::{
    get_language_for_locale, OllamaClient, DEFAULT_ANALYSIS_PROMPT, DEFAULT_NUM_CTX,
    DEFAULT_SUMMARY_PROMPT,
};
use crate::AppState;
use crate::SEPHIROTH_CATEGORIES;
use std::collections::{HashMap, HashSet};
use tauri::State;

use super::types::{CategoryWithSource, KeywordWithSource};

// ============================================================
// DATABASE SETTINGS HELPERS
// ============================================================

/// Get num_ctx setting from database, falling back to DEFAULT_NUM_CTX
pub fn get_num_ctx_setting(db: &Database) -> u32 {
    db.conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'ollama_num_ctx'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_NUM_CTX)
}

/// Create OllamaClient with num_ctx from settings
pub fn create_ollama_client(db: &Database) -> OllamaClient {
    let num_ctx = get_num_ctx_setting(db);
    OllamaClient::with_context(None, num_ctx)
}

/// Get locale from database settings
pub fn get_locale_from_db(state: &State<'_, AppState>) -> String {
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return "de".to_string(),
    };
    db.conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'locale'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "de".to_string())
}

/// Get AI concurrency setting from database
pub fn get_ai_concurrency(state: &AppState) -> usize {
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return 1,
    };
    let val: String = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'ai_parallelism'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "1".to_string());

    val.parse().unwrap_or(1).clamp(1, 10)
}

// ============================================================
// PROMPT HELPERS
// ============================================================

/// Get summary prompt from database or default
pub fn get_summary_prompt(state: &State<'_, AppState>, locale: &str) -> String {
    let language = get_language_for_locale(locale);
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return DEFAULT_SUMMARY_PROMPT.replace("{language}", language),
    };

    let custom_prompt: Option<String> = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'summary_prompt'",
            [],
            |row| row.get(0),
        )
        .ok();

    match custom_prompt {
        Some(prompt) => prompt.replace("{language}", language),
        None => DEFAULT_SUMMARY_PROMPT.replace("{language}", language),
    }
}

/// Get analysis prompt from database or default
pub fn get_analysis_prompt(state: &State<'_, AppState>, locale: &str) -> String {
    let language = get_language_for_locale(locale);
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return DEFAULT_ANALYSIS_PROMPT.replace("{language}", language),
    };

    let custom_prompt: Option<String> = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'analysis_prompt'",
            [],
            |row| row.get(0),
        )
        .ok();

    match custom_prompt {
        Some(prompt) => prompt.replace("{language}", language),
        None => DEFAULT_ANALYSIS_PROMPT.replace("{language}", language),
    }
}

// ============================================================
// KEYWORD/CATEGORY MERGE HELPERS
// ============================================================

/// Merge LLM keywords with local extraction, deduplicating by lowercase
pub fn merge_keywords(
    llm_keywords: &[String],
    local_keywords: Vec<String>,
    max_count: usize,
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();

    for kw in llm_keywords.iter().chain(local_keywords.iter()) {
        let normalized = kw.to_lowercase();
        if !normalized.is_empty() && normalized.len() >= 2 && seen.insert(normalized) {
            merged.push(kw.clone());
            if merged.len() >= max_count {
                break;
            }
        }
    }

    merged
}

/// Validate and merge LLM categories with local extraction
pub fn validate_and_merge_categories(
    llm_categories: &[String],
    local_categories: Vec<String>,
) -> Vec<String> {
    let valid_llm: Vec<String> = llm_categories
        .iter()
        .filter(|c| {
            SEPHIROTH_CATEGORIES
                .iter()
                .any(|s| s.to_lowercase() == c.to_lowercase())
        })
        .cloned()
        .collect();

    if valid_llm.is_empty() {
        local_categories
    } else {
        let mut seen = HashSet::new();
        valid_llm
            .into_iter()
            .chain(local_categories)
            .filter(|c| seen.insert(c.to_lowercase()))
            .take(5)
            .collect()
    }
}

/// Determine source for each keyword by comparing with statistical suggestions
pub fn determine_keyword_sources(
    final_keywords: &[String],
    stat_keywords: &[String],
) -> Vec<KeywordWithSource> {
    let stat_lower: HashSet<String> = stat_keywords.iter().map(|k| k.to_lowercase()).collect();

    final_keywords
        .iter()
        .map(|k| {
            let source = if stat_lower.contains(&k.to_lowercase()) {
                "statistical"
            } else {
                "ai"
            };
            KeywordWithSource {
                name: k.clone(),
                source: source.to_string(),
                confidence: if source == "statistical" { 0.8 } else { 1.0 },
            }
        })
        .collect()
}

/// Determine source for each category by comparing with statistical suggestions
pub fn determine_category_sources(
    final_categories: &[String],
    stat_categories: &[(String, f64)],
) -> Vec<CategoryWithSource> {
    let stat_map: HashMap<String, f64> = stat_categories
        .iter()
        .map(|(name, conf)| (name.to_lowercase(), *conf))
        .collect();

    final_categories
        .iter()
        .map(|c| {
            let lower = c.to_lowercase();
            if let Some(&conf) = stat_map.get(&lower) {
                CategoryWithSource {
                    name: c.clone(),
                    source: "statistical".to_string(),
                    confidence: conf,
                }
            } else {
                CategoryWithSource {
                    name: c.clone(),
                    source: "ai".to_string(),
                    confidence: 1.0,
                }
            }
        })
        .collect()
}
