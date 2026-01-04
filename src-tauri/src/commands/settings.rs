use crate::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub locale: String,
    pub theme: String,
    pub show_terminology_tooltips: bool,
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;

    let settings_map: HashMap<String, String> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(Settings {
        locale: settings_map
            .get("locale")
            .cloned()
            .unwrap_or_else(|| "de".to_string()),
        theme: settings_map
            .get("theme")
            .cloned()
            .unwrap_or_else(|| "mocha".to_string()),
        show_terminology_tooltips: settings_map
            .get("showTerminologyTooltips")
            .map(|v| v == "true")
            .unwrap_or(true),
    })
}

#[tauri::command]
pub fn set_setting(state: State<AppState>, key: String, value: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            (&key, &value),
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_setting(state: State<AppState>, key: String) -> Result<Option<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let result = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [&key],
            |row| row.get(0),
        )
        .ok();

    Ok(result)
}
