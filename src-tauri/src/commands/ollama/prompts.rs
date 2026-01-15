//! Prompt template management commands

use crate::ollama::{DEFAULT_ANALYSIS_PROMPT, DEFAULT_SUMMARY_PROMPT};
use crate::AppState;
use log::info;
use tauri::State;

use super::types::{DefaultPrompts, PromptTemplates, ResetForReprocessingResult};

/// Get default prompts (hardcoded)
#[tauri::command]
pub fn get_default_prompts() -> DefaultPrompts {
    DefaultPrompts {
        summary_prompt: DEFAULT_SUMMARY_PROMPT.to_string(),
        analysis_prompt: DEFAULT_ANALYSIS_PROMPT.to_string(),
    }
}

/// Get current prompts (custom or default)
#[tauri::command]
pub fn get_prompts(state: State<AppState>) -> Result<PromptTemplates, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let summary_prompt: Option<String> = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'summary_prompt'",
            [],
            |row| row.get(0),
        )
        .ok();

    let analysis_prompt: Option<String> = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'analysis_prompt'",
            [],
            |row| row.get(0),
        )
        .ok();

    Ok(PromptTemplates {
        summary_prompt: summary_prompt.unwrap_or_else(|| DEFAULT_SUMMARY_PROMPT.to_string()),
        analysis_prompt: analysis_prompt.unwrap_or_else(|| DEFAULT_ANALYSIS_PROMPT.to_string()),
    })
}

/// Set custom prompts
#[tauri::command]
pub fn set_prompts(
    state: State<AppState>,
    summary_prompt: String,
    analysis_prompt: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('summary_prompt', ?1)",
            [&summary_prompt],
        )
        .map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('analysis_prompt', ?1)",
            [&analysis_prompt],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Reset prompts to default values
#[tauri::command]
pub fn reset_prompts(state: State<AppState>) -> Result<PromptTemplates, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute("DELETE FROM settings WHERE key = 'summary_prompt'", [])
        .map_err(|e| e.to_string())?;

    db.conn()
        .execute("DELETE FROM settings WHERE key = 'analysis_prompt'", [])
        .map_err(|e| e.to_string())?;

    Ok(PromptTemplates {
        summary_prompt: DEFAULT_SUMMARY_PROMPT.to_string(),
        analysis_prompt: DEFAULT_ANALYSIS_PROMPT.to_string(),
    })
}

/// Reset articles for reprocessing.
/// Clears processed_at, analysis_hopeless, analysis_attempts, and analysis_error.
#[tauri::command]
pub fn reset_articles_for_reprocessing(
    state: State<AppState>,
    only_with_content: Option<bool>,
) -> Result<ResetForReprocessingResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let only_with_content = only_with_content.unwrap_or(true);

    let sql = if only_with_content {
        r#"UPDATE fnords SET
           processed_at = NULL,
           analysis_hopeless = FALSE,
           analysis_attempts = 0,
           analysis_error = NULL
           WHERE content_full IS NOT NULL AND content_full != ''"#
    } else {
        r#"UPDATE fnords SET
           processed_at = NULL,
           analysis_hopeless = FALSE,
           analysis_attempts = 0,
           analysis_error = NULL"#
    };

    let reset_count = db.conn().execute(sql, []).map_err(|e| e.to_string())? as i64;
    info!(
        "Reset {} articles for reprocessing (hopeless flags cleared)",
        reset_count
    );

    Ok(ResetForReprocessingResult { reset_count })
}
