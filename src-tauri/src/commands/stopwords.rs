//! Stopword Management Commands
//!
//! Commands for managing stopwords that filter out unwanted terms
//! from keyword extraction. Supports both system stopwords (from txt files)
//! and user-defined stopwords.

use crate::db::{reset_stopwords_to_default, restore_default_stopwords};
use crate::text_analysis::{
    add_user_stopword, get_stopword_stats, load_all_db_stopwords, remove_user_stopword, STOPWORDS,
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
    pub source: Option<String>,
    pub language: Option<String>,
}

/// Stopword statistics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StopwordStatsResponse {
    pub system_count: i64,
    pub user_count: i64,
    pub builtin_count: usize,
    pub total_count: i64,
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
    let db = state.db_conn()?;

    let mut stmt = db
        .conn()
        .prepare("SELECT word, created_at, source, language FROM stopwords WHERE source = 'user' ORDER BY word ASC")
        .map_err(|e| e.to_string())?;

    let stopwords = stmt
        .query_map([], |row| {
            Ok(UserStopword {
                word: row.get(0)?,
                added_at: row.get(1)?,
                source: row.get(2)?,
                language: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(stopwords)
}

/// Get all system stopwords
#[tauri::command]
pub fn get_system_stopwords(state: State<AppState>) -> Result<Vec<UserStopword>, String> {
    let db = state.db_conn()?;

    let mut stmt = db
        .conn()
        .prepare("SELECT word, created_at, source, language FROM stopwords WHERE source = 'system' ORDER BY language, word ASC")
        .map_err(|e| e.to_string())?;

    let stopwords = stmt
        .query_map([], |row| {
            Ok(UserStopword {
                word: row.get(0)?,
                added_at: row.get(1)?,
                source: row.get(2)?,
                language: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(stopwords)
}

/// Get all stopwords (system + user)
#[tauri::command]
pub fn get_all_stopwords_list(state: State<AppState>) -> Result<Vec<UserStopword>, String> {
    let db = state.db_conn()?;

    let mut stmt = db
        .conn()
        .prepare("SELECT word, created_at, source, language FROM stopwords ORDER BY word ASC")
        .map_err(|e| e.to_string())?;

    let stopwords = stmt
        .query_map([], |row| {
            Ok(UserStopword {
                word: row.get(0)?,
                added_at: row.get(1)?,
                source: row.get(2)?,
                language: row.get(3)?,
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
    let db = state.db_conn()?;

    let word_trimmed = word.trim().to_lowercase();
    if word_trimmed.is_empty() || word_trimmed.len() < 2 {
        return Err("Stopword must be at least 2 characters".to_string());
    }

    // Check if already exists in database (system or user)
    let all_stopwords = load_all_db_stopwords(db.conn()).map_err(|e| e.to_string())?;
    if all_stopwords.contains(&word_trimmed) {
        return Ok(false); // Already exists
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
    let db = state.db_conn()?;

    // Load existing stopwords once for efficiency
    let existing = load_all_db_stopwords(db.conn()).map_err(|e| e.to_string())?;

    let mut added = 0i64;
    let mut skipped = 0i64;

    for word in words {
        let word_trimmed = word.trim().to_lowercase();
        if word_trimmed.is_empty() || word_trimmed.len() < 2 {
            skipped += 1;
            continue;
        }

        // Skip if already exists in database
        if existing.contains(&word_trimmed) {
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
    let db = state.db_conn()?;

    remove_user_stopword(db.conn(), &word).map_err(|e| e.to_string())
}

/// Get stopword statistics
#[tauri::command]
pub fn get_stopwords_stats(state: State<AppState>) -> Result<StopwordStatsResponse, String> {
    let db = state.db_conn()?;

    let stats = get_stopword_stats(db.conn()).map_err(|e| e.to_string())?;

    Ok(StopwordStatsResponse {
        system_count: stats.system_count,
        user_count: stats.user_count,
        builtin_count: stats.builtin_count,
        total_count: stats.total_count,
    })
}

/// Check if a word is a stopword (system or user-defined)
#[tauri::command]
pub fn is_stopword_check(state: State<AppState>, word: String) -> Result<bool, String> {
    let db = state.db_conn()?;

    let word_lower = word.trim().to_lowercase();

    // Check against all database stopwords
    let all_stopwords = load_all_db_stopwords(db.conn()).map_err(|e| e.to_string())?;

    // If DB is empty, fall back to static stopwords
    if all_stopwords.is_empty() {
        return Ok(STOPWORDS.contains(word_lower.as_str()));
    }

    Ok(all_stopwords.contains(&word_lower))
}

/// Search stopwords (system and user from database)
#[tauri::command]
pub fn search_stopwords(
    state: State<AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<StopwordSearchResult>, String> {
    let db = state.db_conn()?;
    let limit = limit.unwrap_or(50);
    let query_lower = query.trim().to_lowercase();

    if query_lower.is_empty() {
        return Ok(vec![]);
    }

    // Search stopwords from database
    let mut stmt = db
        .conn()
        .prepare("SELECT word, source FROM stopwords WHERE word LIKE ? ORDER BY word ASC LIMIT ?")
        .map_err(|e| e.to_string())?;

    let pattern = format!("%{}%", query_lower);
    let results = stmt
        .query_map(rusqlite::params![pattern, limit as i64], |row| {
            let word: String = row.get(0)?;
            let source: String = row.get(1)?;
            Ok(StopwordSearchResult {
                word,
                is_builtin: source == "system",
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(results)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StopwordSearchResult {
    pub word: String,
    pub is_builtin: bool,
}

/// Clear all user stopwords (keeps system stopwords)
#[tauri::command]
pub fn clear_user_stopwords(state: State<AppState>) -> Result<i64, String> {
    let db = state.db_conn()?;

    let deleted = db
        .conn()
        .execute("DELETE FROM stopwords WHERE source = 'user'", [])
        .map_err(|e| e.to_string())?;

    Ok(deleted as i64)
}

/// Reset all stopwords to system defaults (removes user stopwords and re-seeds from txt files)
#[tauri::command]
pub fn reset_stopwords(state: State<AppState>) -> Result<ResetStopwordsResult, String> {
    let db = state.db_conn()?;

    let restored =
        reset_stopwords_to_default(db.conn()).map_err(|e: rusqlite::Error| e.to_string())?;

    Ok(ResetStopwordsResult {
        restored_count: restored as i64,
    })
}

/// Restore missing system stopwords (adds any missing from txt files without removing user stopwords)
#[tauri::command]
pub fn restore_system_stopwords(state: State<AppState>) -> Result<ResetStopwordsResult, String> {
    let db = state.db_conn()?;

    let restored =
        restore_default_stopwords(db.conn()).map_err(|e: rusqlite::Error| e.to_string())?;

    Ok(ResetStopwordsResult {
        restored_count: restored as i64,
    })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResetStopwordsResult {
    pub restored_count: i64,
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

/// Export user stopwords to JSON format (only user-added, not system stopwords)
#[tauri::command]
pub fn export_stopwords(state: State<AppState>) -> Result<String, String> {
    let db = state.db_conn()?;

    let mut stmt = db
        .conn()
        .prepare("SELECT word FROM stopwords WHERE source = 'user' ORDER BY word ASC")
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

    let db = state.db_conn()?;

    // Load existing stopwords once for efficiency
    let existing = load_all_db_stopwords(db.conn()).map_err(|e| e.to_string())?;

    let mut imported = 0i64;
    let mut skipped = 0i64;
    let total = backup.stopwords.len() as i64;

    for word in backup.stopwords {
        let word_trimmed = word.trim().to_lowercase();
        if word_trimmed.is_empty() || word_trimmed.len() < 2 {
            skipped += 1;
            continue;
        }

        // Skip if already exists in database (system or user)
        if existing.contains(&word_trimmed) {
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
