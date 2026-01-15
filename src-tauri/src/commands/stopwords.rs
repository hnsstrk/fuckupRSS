//! Stopword Management Commands
//!
//! Commands for managing user-defined stopwords that filter out
//! unwanted terms from keyword extraction.

use crate::text_analysis::{
    add_user_stopword, get_stopword_stats, load_user_stopwords, remove_user_stopword, STOPWORDS,
};
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

// ============================================================
// DATA STRUCTURES
// ============================================================

/// Stopword with metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserStopword {
    pub word: String,
    pub added_at: Option<String>,
}

/// Stopword statistics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StopwordStatsResponse {
    pub builtin_count: usize,
    pub user_count: i64,
    pub total_count: usize,
}

/// Response for adding stopwords
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddStopwordsResult {
    pub added: i64,
    pub skipped: i64,
}

// ============================================================
// COMMANDS
// ============================================================

/// Get all user-defined stopwords
#[tauri::command]
pub fn get_user_stopwords(state: State<AppState>) -> Result<Vec<UserStopword>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare("SELECT word, added_at FROM user_stopwords ORDER BY word ASC")
        .map_err(|e| e.to_string())?;

    let stopwords = stmt
        .query_map([], |row| {
            Ok(UserStopword {
                word: row.get(0)?,
                added_at: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(stopwords)
}

/// Add a single stopword
#[tauri::command]
pub fn add_stopword(state: State<AppState>, word: String) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let word_trimmed = word.trim().to_lowercase();
    if word_trimmed.is_empty() || word_trimmed.len() < 2 {
        return Err("Stopword must be at least 2 characters".to_string());
    }

    // Check if already a builtin stopword
    if STOPWORDS.contains(word_trimmed.as_str()) {
        return Ok(false); // Already exists in builtin list
    }

    add_user_stopword(db.conn(), &word_trimmed).map_err(|e| e.to_string())?;

    Ok(true)
}

/// Add multiple stopwords at once
#[tauri::command]
pub fn add_stopwords_batch(
    state: State<AppState>,
    words: Vec<String>,
) -> Result<AddStopwordsResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut added = 0i64;
    let mut skipped = 0i64;

    for word in words {
        let word_trimmed = word.trim().to_lowercase();
        if word_trimmed.is_empty() || word_trimmed.len() < 2 {
            skipped += 1;
            continue;
        }

        // Skip if already a builtin stopword
        if STOPWORDS.contains(word_trimmed.as_str()) {
            skipped += 1;
            continue;
        }

        match add_user_stopword(db.conn(), &word_trimmed) {
            Ok(_) => added += 1,
            Err(_) => skipped += 1,
        }
    }

    Ok(AddStopwordsResult { added, skipped })
}

/// Remove a stopword
#[tauri::command]
pub fn remove_stopword(state: State<AppState>, word: String) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    remove_user_stopword(db.conn(), &word).map_err(|e| e.to_string())
}

/// Get stopword statistics
#[tauri::command]
pub fn get_stopwords_stats(state: State<AppState>) -> Result<StopwordStatsResponse, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let stats = get_stopword_stats(db.conn()).map_err(|e| e.to_string())?;

    Ok(StopwordStatsResponse {
        builtin_count: stats.builtin_count,
        user_count: stats.user_count,
        total_count: stats.total_count,
    })
}

/// Check if a word is a stopword (builtin or user-defined)
#[tauri::command]
pub fn is_stopword_check(state: State<AppState>, word: String) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let word_lower = word.trim().to_lowercase();

    // Check builtin first
    if STOPWORDS.contains(word_lower.as_str()) {
        return Ok(true);
    }

    // Check user stopwords
    let user_stopwords = load_user_stopwords(db.conn()).map_err(|e| e.to_string())?;

    Ok(user_stopwords.contains(&word_lower))
}

/// Search stopwords (both builtin and user)
#[tauri::command]
pub fn search_stopwords(
    state: State<AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<StopwordSearchResult>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(50);
    let query_lower = query.trim().to_lowercase();

    if query_lower.is_empty() {
        return Ok(vec![]);
    }

    let mut results = Vec::new();

    // Search builtin stopwords
    for word in STOPWORDS.iter() {
        if word.contains(&query_lower) {
            results.push(StopwordSearchResult {
                word: word.to_string(),
                is_builtin: true,
            });
        }
        if results.len() >= limit {
            break;
        }
    }

    // Search user stopwords
    if results.len() < limit {
        let user_stopwords = load_user_stopwords(db.conn()).map_err(|e| e.to_string())?;
        for word in user_stopwords {
            if word.contains(&query_lower) {
                results.push(StopwordSearchResult {
                    word,
                    is_builtin: false,
                });
            }
            if results.len() >= limit {
                break;
            }
        }
    }

    // Sort alphabetically
    results.sort_by(|a, b| a.word.cmp(&b.word));

    Ok(results)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StopwordSearchResult {
    pub word: String,
    pub is_builtin: bool,
}

/// Clear all user stopwords
#[tauri::command]
pub fn clear_user_stopwords(state: State<AppState>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let deleted = db
        .conn()
        .execute("DELETE FROM user_stopwords", [])
        .map_err(|e| e.to_string())?;

    Ok(deleted as i64)
}

// ============================================================
// BACKUP / RESTORE
// ============================================================

/// Stopword backup format
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StopwordBackup {
    pub stopwords: Vec<String>,
    pub exported_at: String,
}

/// Result of stopword import
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StopwordImportResult {
    pub imported: i64,
    pub skipped: i64,
    pub total: i64,
}

/// Export user stopwords to JSON format
#[tauri::command]
pub fn export_stopwords(state: State<AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare("SELECT word FROM user_stopwords ORDER BY word ASC")
        .map_err(|e| e.to_string())?;

    let stopwords: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let backup = StopwordBackup {
        stopwords,
        exported_at: chrono::Utc::now().to_rfc3339(),
    };

    serde_json::to_string_pretty(&backup).map_err(|e| e.to_string())
}

/// Import user stopwords from JSON format
#[tauri::command]
pub fn import_stopwords(
    state: State<AppState>,
    content: String,
) -> Result<StopwordImportResult, String> {
    let backup: StopwordBackup =
        serde_json::from_str(&content).map_err(|e| format!("Invalid JSON format: {}", e))?;

    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut imported = 0i64;
    let mut skipped = 0i64;
    let total = backup.stopwords.len() as i64;

    for word in backup.stopwords {
        let word_trimmed = word.trim().to_lowercase();
        if word_trimmed.is_empty() || word_trimmed.len() < 2 {
            skipped += 1;
            continue;
        }

        // Skip if already a builtin stopword
        if STOPWORDS.contains(word_trimmed.as_str()) {
            skipped += 1;
            continue;
        }

        match add_user_stopword(db.conn(), &word_trimmed) {
            Ok(_) => imported += 1,
            Err(_) => skipped += 1, // Already exists
        }
    }

    Ok(StopwordImportResult {
        imported,
        skipped,
        total,
    })
}
