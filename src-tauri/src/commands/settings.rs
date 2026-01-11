use crate::ollama::{DEFAULT_NUM_CTX, RECOMMENDED_EMBEDDING_MODEL};
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
    /// Embedding model for keyword similarity (e.g., "snowflake-arctic-embed2")
    pub embedding_model: String,
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

    let embedding_model = settings_map
        .get("embedding_model")
        .cloned()
        .unwrap_or_else(|| RECOMMENDED_EMBEDDING_MODEL.to_string());

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
        embedding_model,
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

/// Get the configured embedding model from the database
/// Returns the default if not configured
pub fn get_embedding_model_from_db(conn: &rusqlite::Connection) -> String {
    conn.query_row(
        "SELECT value FROM settings WHERE key = 'embedding_model'",
        [],
        |row| row.get(0),
    )
    .unwrap_or_else(|_| RECOMMENDED_EMBEDDING_MODEL.to_string())
}

/// Detects if the system prefers dark mode
/// Platform-specific detection:
/// - macOS: Uses `defaults read -g AppleInterfaceStyle`
/// - Linux: Uses gsettings (GNOME/GTK), kreadconfig5 (KDE), environment variables
#[tauri::command]
pub fn get_system_theme() -> String {
    // macOS detection
    #[cfg(target_os = "macos")]
    {
        // On macOS, AppleInterfaceStyle is "Dark" when dark mode is enabled
        // The key doesn't exist when light mode is active (command returns exit code 1)
        if let Ok(output) = Command::new("defaults")
            .args(["read", "-g", "AppleInterfaceStyle"])
            .output()
        {
            // Check if command succeeded (exit code 0) and returned "Dark"
            if output.status.success() {
                let result = String::from_utf8_lossy(&output.stdout);
                if result.trim().eq_ignore_ascii_case("dark") {
                    return "dark".to_string();
                }
            }
        }
        // If the command fails (light mode) or returns something else
        return "light".to_string();
    }

    // Linux detection
    #[cfg(target_os = "linux")]
    {
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

        // Default to dark for Linux
        return "dark".to_string();
    }

    // Fallback for other platforms (Windows, etc.)
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        "dark".to_string()
    }
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
