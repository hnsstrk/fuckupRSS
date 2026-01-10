use crate::ollama::DEFAULT_NUM_CTX;
use crate::{AppState, LogLevel};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub locale: String,
    pub theme: String,
    pub show_terminology_tooltips: bool,
    pub log_level: String,
    /// Ollama context length (num_ctx) - affects VRAM usage and speed
    pub ollama_num_ctx: u32,
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

    let log_level = settings_map
        .get("logLevel")
        .cloned()
        .unwrap_or_else(|| {
            // Default to debug in dev, info in release
            if cfg!(debug_assertions) {
                "debug".to_string()
            } else {
                "info".to_string()
            }
        });

    let ollama_num_ctx = settings_map
        .get("ollama_num_ctx")
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_NUM_CTX);

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
        log_level,
        ollama_num_ctx,
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

/// Detects if the system prefers dark mode
/// Uses multiple methods to detect dark mode on Linux:
/// 1. GNOME/GTK color-scheme setting
/// 2. GTK theme name containing "dark"
/// 3. KDE color scheme
#[tauri::command]
pub fn get_system_theme() -> String {
    // Method 1: Check GNOME/GTK color-scheme (works for most GTK-based desktops)
    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        let result = String::from_utf8_lossy(&output.stdout);
        if result.contains("dark") {
            return "dark".to_string();
        }
        if result.contains("light") || result.contains("default") {
            return "light".to_string();
        }
    }

    // Method 2: Check GTK theme name
    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "gtk-theme"])
        .output()
    {
        let result = String::from_utf8_lossy(&output.stdout).to_lowercase();
        if result.contains("dark") {
            return "dark".to_string();
        }
    }

    // Method 3: Check KDE color scheme (Plasma)
    if let Ok(output) = Command::new("kreadconfig5")
        .args(["--group", "General", "--key", "ColorScheme"])
        .output()
    {
        let result = String::from_utf8_lossy(&output.stdout).to_lowercase();
        if result.contains("dark") {
            return "dark".to_string();
        }
    }

    // Method 4: Check environment variable
    if let Ok(theme) = std::env::var("GTK_THEME") {
        if theme.to_lowercase().contains("dark") {
            return "dark".to_string();
        }
    }

    // Default to dark (most users of this app probably prefer dark mode)
    "dark".to_string()
}

/// Get available log levels
#[tauri::command]
pub fn get_log_levels() -> Vec<&'static str> {
    vec!["error", "warn", "info", "debug", "trace"]
}

/// Set the log level at runtime
/// Note: This changes the log level for the current session
/// To persist, also call set_setting with key="logLevel"
#[tauri::command]
pub fn set_log_level(level: String) -> Result<(), String> {
    let log_level = LogLevel::from(level.as_str());
    info!("Setting log level to: {}", log_level);

    // Note: tauri-plugin-log doesn't support runtime level changes
    // This is mainly for frontend logging coordination
    // The actual Rust log level is set at startup
    debug!("Log level change requested (effective for frontend)");

    Ok(())
}
