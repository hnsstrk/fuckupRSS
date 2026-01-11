use crate::ollama::{DEFAULT_NUM_CTX, RECOMMENDED_EMBEDDING_MODEL};
use crate::{AppState, LogLevel};
use log::{debug, info};
use std::collections::HashMap;
use std::process::Command;
use tauri::State;

/// Returns all settings as a key-value map
/// This allows the frontend to access any setting without needing to update the struct
#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<HashMap<String, serde_json::Value>, String> {
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

    // Convert to JSON values, parsing booleans and numbers where appropriate
    let mut result: HashMap<String, serde_json::Value> = HashMap::new();

    for (key, value) in settings_map {
        let json_value = match key.as_str() {
            // Boolean settings
            "showTerminologyTooltips" | "syncOnStart" => {
                serde_json::Value::Bool(value == "true")
            }
            // Numeric settings
            "syncInterval" | "ollama_num_ctx" => {
                if let Ok(num) = value.parse::<i64>() {
                    serde_json::Value::Number(num.into())
                } else {
                    serde_json::Value::String(value)
                }
            }
            // String settings (theme_mode, dark_theme, light_theme, locale, etc.)
            _ => serde_json::Value::String(value),
        };
        result.insert(key, json_value);
    }

    // Add defaults for missing settings
    if !result.contains_key("locale") {
        result.insert("locale".to_string(), serde_json::Value::String("de".to_string()));
    }
    if !result.contains_key("theme_mode") {
        result.insert("theme_mode".to_string(), serde_json::Value::String("system".to_string()));
    }
    if !result.contains_key("dark_theme") {
        // Fall back to legacy 'theme' key if exists
        let default_dark = result
            .get("theme")
            .and_then(|v| v.as_str())
            .unwrap_or("mocha")
            .to_string();
        result.insert("dark_theme".to_string(), serde_json::Value::String(default_dark));
    }
    if !result.contains_key("light_theme") {
        result.insert("light_theme".to_string(), serde_json::Value::String("latte".to_string()));
    }
    if !result.contains_key("showTerminologyTooltips") {
        result.insert("showTerminologyTooltips".to_string(), serde_json::Value::Bool(true));
    }
    if !result.contains_key("syncInterval") {
        result.insert("syncInterval".to_string(), serde_json::Value::Number(30.into()));
    }
    if !result.contains_key("syncOnStart") {
        result.insert("syncOnStart".to_string(), serde_json::Value::Bool(true));
    }
    if !result.contains_key("logLevel") {
        let default_level = if cfg!(debug_assertions) { "debug" } else { "info" };
        result.insert("logLevel".to_string(), serde_json::Value::String(default_level.to_string()));
    }
    if !result.contains_key("ollama_num_ctx") {
        result.insert("ollama_num_ctx".to_string(), serde_json::Value::Number(DEFAULT_NUM_CTX.into()));
    }
    if !result.contains_key("embedding_model") {
        result.insert("embedding_model".to_string(), serde_json::Value::String(RECOMMENDED_EMBEDDING_MODEL.to_string()));
    }

    Ok(result)
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
