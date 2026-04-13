//! Prompt template management commands

use crate::ollama::{
    DEFAULT_ANALYSIS_PROMPT, DEFAULT_DISCORDIAN_PROMPT_WITH_STATS, DEFAULT_SUMMARY_PROMPT,
    DEFAULT_THEME_REPORT_PROMPT, DEFAULT_THEME_VALIDATION_PROMPT,
};
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
        discordian_prompt: DEFAULT_DISCORDIAN_PROMPT_WITH_STATS.to_string(),
        theme_validation_prompt: DEFAULT_THEME_VALIDATION_PROMPT.to_string(),
        theme_report_prompt: DEFAULT_THEME_REPORT_PROMPT.to_string(),
    }
}

/// Get current prompts (custom or default)
#[tauri::command]
pub fn get_prompts(state: State<AppState>) -> Result<PromptTemplates, String> {
    let db = state.db_conn()?;

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

    let discordian_prompt: Option<String> = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'discordian_prompt'",
            [],
            |row| row.get(0),
        )
        .ok();

    let theme_validation_prompt: Option<String> = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'theme_validation_prompt'",
            [],
            |row| row.get(0),
        )
        .ok();

    let theme_report_prompt: Option<String> = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'theme_report_prompt'",
            [],
            |row| row.get(0),
        )
        .ok();

    Ok(PromptTemplates {
        summary_prompt: summary_prompt.unwrap_or_else(|| DEFAULT_SUMMARY_PROMPT.to_string()),
        analysis_prompt: analysis_prompt.unwrap_or_else(|| DEFAULT_ANALYSIS_PROMPT.to_string()),
        discordian_prompt: discordian_prompt
            .unwrap_or_else(|| DEFAULT_DISCORDIAN_PROMPT_WITH_STATS.to_string()),
        theme_validation_prompt: theme_validation_prompt
            .unwrap_or_else(|| DEFAULT_THEME_VALIDATION_PROMPT.to_string()),
        theme_report_prompt: theme_report_prompt
            .unwrap_or_else(|| DEFAULT_THEME_REPORT_PROMPT.to_string()),
    })
}

/// Set custom prompts
#[tauri::command]
pub fn set_prompts(
    state: State<AppState>,
    summary_prompt: String,
    analysis_prompt: String,
    discordian_prompt: String,
    theme_validation_prompt: String,
    theme_report_prompt: String,
) -> Result<(), String> {
    let db = state.db_conn()?;

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

    db.conn()
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('discordian_prompt', ?1)",
            [&discordian_prompt],
        )
        .map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('theme_validation_prompt', ?1)",
            [&theme_validation_prompt],
        )
        .map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('theme_report_prompt', ?1)",
            [&theme_report_prompt],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Reset prompts to default values
#[tauri::command]
pub fn reset_prompts(state: State<AppState>) -> Result<PromptTemplates, String> {
    let db = state.db_conn()?;

    db.conn()
        .execute("DELETE FROM settings WHERE key = 'summary_prompt'", [])
        .map_err(|e| e.to_string())?;

    db.conn()
        .execute("DELETE FROM settings WHERE key = 'analysis_prompt'", [])
        .map_err(|e| e.to_string())?;

    db.conn()
        .execute("DELETE FROM settings WHERE key = 'discordian_prompt'", [])
        .map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "DELETE FROM settings WHERE key = 'theme_validation_prompt'",
            [],
        )
        .map_err(|e| e.to_string())?;

    db.conn()
        .execute("DELETE FROM settings WHERE key = 'theme_report_prompt'", [])
        .map_err(|e| e.to_string())?;

    Ok(PromptTemplates {
        summary_prompt: DEFAULT_SUMMARY_PROMPT.to_string(),
        analysis_prompt: DEFAULT_ANALYSIS_PROMPT.to_string(),
        discordian_prompt: DEFAULT_DISCORDIAN_PROMPT_WITH_STATS.to_string(),
        theme_validation_prompt: DEFAULT_THEME_VALIDATION_PROMPT.to_string(),
        theme_report_prompt: DEFAULT_THEME_REPORT_PROMPT.to_string(),
    })
}

/// Reset articles for reprocessing.
/// Clears processed_at, analysis_hopeless, analysis_attempts, and analysis_error.
#[tauri::command]
pub fn reset_articles_for_reprocessing(
    state: State<AppState>,
    only_with_content: Option<bool>,
) -> Result<ResetForReprocessingResult, String> {
    let db = state.db_conn()?;
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
