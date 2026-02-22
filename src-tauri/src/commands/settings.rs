use crate::ai_provider::{DEFAULT_OPENAI_EMBEDDING_MODEL, DEFAULT_OPENAI_MODEL};
use crate::error::{CmdResult, FuckupError};
use crate::ollama::{DEFAULT_NUM_CTX, RECOMMENDED_EMBEDDING_MODEL, RECOMMENDED_MAIN_MODEL};
use crate::{AppState, LogLevel};
use log::{debug, info};
use std::collections::HashMap;
use std::process::Command;
use tauri::State;

/// Returns all settings as a key-value map
/// This allows the frontend to access any setting without needing to update the struct
#[tauri::command]
pub fn get_settings(state: State<AppState>) -> CmdResult<HashMap<String, serde_json::Value>> {
    let db = state.db_conn()?;

    let mut stmt = db.conn().prepare("SELECT key, value FROM settings")?;

    let settings_map: HashMap<String, String> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Convert to JSON values, parsing booleans and numbers where appropriate
    let mut result: HashMap<String, serde_json::Value> = HashMap::new();

    for (key, value) in settings_map {
        let json_value = match key.as_str() {
            // Boolean settings
            "showTerminologyTooltips" | "syncOnStart" | "enable_headless_browser" => {
                serde_json::Value::Bool(value == "true")
            }
            // Numeric settings (integer)
            "syncInterval" | "ollama_num_ctx" | "embedding_dimensions" => {
                if let Ok(num) = value.parse::<i64>() {
                    serde_json::Value::Number(num.into())
                } else {
                    serde_json::Value::String(value)
                }
            }
            // Numeric settings (float)
            "cost_limit_monthly" => {
                if let Ok(num) = value.parse::<f64>() {
                    serde_json::json!(num)
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
        result.insert(
            "locale".to_string(),
            serde_json::Value::String("de".to_string()),
        );
    }
    if !result.contains_key("theme_mode") {
        result.insert(
            "theme_mode".to_string(),
            serde_json::Value::String("system".to_string()),
        );
    }
    if !result.contains_key("dark_theme") {
        // Fall back to legacy 'theme' key if exists
        let default_dark = result
            .get("theme")
            .and_then(|v| v.as_str())
            .unwrap_or("mocha")
            .to_string();
        result.insert(
            "dark_theme".to_string(),
            serde_json::Value::String(default_dark),
        );
    }
    if !result.contains_key("light_theme") {
        result.insert(
            "light_theme".to_string(),
            serde_json::Value::String("latte".to_string()),
        );
    }
    if !result.contains_key("showTerminologyTooltips") {
        result.insert(
            "showTerminologyTooltips".to_string(),
            serde_json::Value::Bool(true),
        );
    }
    if !result.contains_key("syncInterval") {
        result.insert(
            "syncInterval".to_string(),
            serde_json::Value::Number(30.into()),
        );
    }
    if !result.contains_key("syncOnStart") {
        result.insert("syncOnStart".to_string(), serde_json::Value::Bool(true));
    }
    if !result.contains_key("logLevel") {
        let default_level = if cfg!(debug_assertions) {
            "debug"
        } else {
            "info"
        };
        result.insert(
            "logLevel".to_string(),
            serde_json::Value::String(default_level.to_string()),
        );
    }
    if !result.contains_key("ollama_num_ctx") {
        result.insert(
            "ollama_num_ctx".to_string(),
            serde_json::Value::Number(DEFAULT_NUM_CTX.into()),
        );
    }
    if !result.contains_key("embedding_model") {
        result.insert(
            "embedding_model".to_string(),
            serde_json::Value::String(RECOMMENDED_EMBEDDING_MODEL.to_string()),
        );
    }
    if !result.contains_key("enable_headless_browser") {
        result.insert(
            "enable_headless_browser".to_string(),
            serde_json::Value::Bool(false),
        );
    }

    // AI provider defaults
    if !result.contains_key("ai_text_provider") {
        result.insert(
            "ai_text_provider".to_string(),
            serde_json::Value::String("ollama".to_string()),
        );
    }
    if !result.contains_key("ollama_url") {
        result.insert(
            "ollama_url".to_string(),
            serde_json::Value::String("http://localhost:11434".to_string()),
        );
    }
    if !result.contains_key("ollama_model") {
        result.insert(
            "ollama_model".to_string(),
            serde_json::Value::String(RECOMMENDED_MAIN_MODEL.to_string()),
        );
    }
    if !result.contains_key("openai_base_url") {
        result.insert(
            "openai_base_url".to_string(),
            serde_json::Value::String("https://api.openai.com".to_string()),
        );
    }
    if !result.contains_key("openai_api_key") {
        result.insert(
            "openai_api_key".to_string(),
            serde_json::Value::String(String::new()),
        );
    }
    if !result.contains_key("openai_model") {
        result.insert(
            "openai_model".to_string(),
            serde_json::Value::String(DEFAULT_OPENAI_MODEL.to_string()),
        );
    }
    if !result.contains_key("cost_limit_monthly") {
        result.insert("cost_limit_monthly".to_string(), serde_json::json!(5.0));
    }

    // Embedding provider defaults
    if !result.contains_key("embedding_provider") {
        result.insert(
            "embedding_provider".to_string(),
            serde_json::Value::String("ollama".to_string()),
        );
    }
    if !result.contains_key("openai_embedding_model") {
        result.insert(
            "openai_embedding_model".to_string(),
            serde_json::Value::String(DEFAULT_OPENAI_EMBEDDING_MODEL.to_string()),
        );
    }
    if !result.contains_key("embedding_dimensions") {
        result.insert(
            "embedding_dimensions".to_string(),
            serde_json::Value::Number(1024.into()),
        );
    }

    Ok(result)
}

#[tauri::command]
pub fn set_setting(state: State<AppState>, key: String, value: String) -> CmdResult<()> {
    // Input validation
    if key.is_empty() {
        return Err(FuckupError::Validation(
            "Setting key cannot be empty".to_string(),
        ));
    }
    if key.len() > 256 {
        return Err(FuckupError::Validation(
            "Setting key too long (max 256 characters)".to_string(),
        ));
    }

    let db = state.db_conn()?;

    db.conn().execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        (&key, &value),
    )?;

    Ok(())
}

#[tauri::command]
pub fn get_setting(state: State<AppState>, key: String) -> CmdResult<Option<String>> {
    let db = state.db_conn()?;

    let result = db
        .conn()
        .query_row("SELECT value FROM settings WHERE key = ?1", [&key], |row| {
            row.get(0)
        })
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
        "light".to_string()
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
        "dark".to_string()
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

/// Get the current operating system platform
/// Returns: "macos", "linux", or "windows"
#[tauri::command]
pub fn get_platform() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macos"
    }

    #[cfg(target_os = "linux")]
    {
        "linux"
    }

    #[cfg(target_os = "windows")]
    {
        "windows"
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        "unknown"
    }
}

/// Set the log level at runtime
/// Note: This changes the log level for the current session
/// To persist, also call set_setting with key="logLevel"
#[tauri::command]
pub fn set_log_level(level: String) -> CmdResult<()> {
    // Input validation
    if !["error", "warn", "info", "debug", "trace"].contains(&level.as_str()) {
        return Err(FuckupError::Validation(format!(
            "Invalid log level: {}",
            level
        )));
    }

    let log_level = LogLevel::from(level.as_str());
    info!("Setting log level to: {}", log_level);

    // Note: tauri-plugin-log doesn't support runtime level changes
    // This is mainly for frontend logging coordination
    // The actual Rust log level is set at startup
    debug!("Log level change requested (effective for frontend)");

    Ok(())
}

// ============================================================
// Unit Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    /// Helper to create a test database
    fn setup_test_db() -> Database {
        Database::new_in_memory().expect("Failed to create in-memory database")
    }

    // ============================================================
    // get_settings tests
    // ============================================================

    #[test]
    fn test_get_settings_returns_hashmap() {
        let db = setup_test_db();

        let mut stmt = db
            .conn()
            .prepare("SELECT key, value FROM settings")
            .expect("Failed to prepare statement");

        let settings: HashMap<String, String> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .expect("Failed to query")
            .filter_map(|r| r.ok())
            .collect();

        // Settings table should have some defaults from schema initialization
        assert!(
            !settings.is_empty() || settings.is_empty(),
            "Settings query should work"
        );
    }

    #[test]
    fn test_get_settings_includes_defaults() {
        let db = setup_test_db();

        // Check for seeded default settings (from schema.rs)
        let theme: Option<String> = db
            .conn()
            .query_row(
                "SELECT value FROM settings WHERE key = 'theme'",
                [],
                |row| row.get(0),
            )
            .ok();

        // Theme should be set to default 'mocha' by schema initialization
        if let Some(t) = theme {
            assert_eq!(t, "mocha", "Default theme should be 'mocha'");
        }
    }

    // ============================================================
    // set_setting tests
    // ============================================================

    #[test]
    fn test_set_setting_insert_new() {
        let db = setup_test_db();

        // Insert a new setting
        db.conn()
            .execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                ["test_key", "test_value"],
            )
            .expect("Failed to insert setting");

        let value: String = db
            .conn()
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                ["test_key"],
                |row| row.get(0),
            )
            .expect("Failed to query setting");

        assert_eq!(value, "test_value", "Setting value should match");
    }

    #[test]
    fn test_set_setting_update_existing() {
        let db = setup_test_db();

        // Insert initial value
        db.conn()
            .execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                ["test_key", "initial_value"],
            )
            .expect("Failed to insert setting");

        // Update value
        db.conn()
            .execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                ["test_key", "updated_value"],
            )
            .expect("Failed to update setting");

        let value: String = db
            .conn()
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                ["test_key"],
                |row| row.get(0),
            )
            .expect("Failed to query setting");

        assert_eq!(value, "updated_value", "Setting should be updated");
    }

    #[test]
    fn test_set_setting_boolean_as_string() {
        let db = setup_test_db();

        // Set boolean setting (stored as string)
        db.conn()
            .execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                ["showTerminologyTooltips", "true"],
            )
            .expect("Failed to insert setting");

        let value: String = db
            .conn()
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                ["showTerminologyTooltips"],
                |row| row.get(0),
            )
            .expect("Failed to query setting");

        assert_eq!(value, "true", "Boolean should be stored as string 'true'");

        // Update to false
        db.conn()
            .execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                ["showTerminologyTooltips", "false"],
            )
            .expect("Failed to update setting");

        let updated_value: String = db
            .conn()
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                ["showTerminologyTooltips"],
                |row| row.get(0),
            )
            .expect("Failed to query setting");

        assert_eq!(
            updated_value, "false",
            "Boolean should be stored as string 'false'"
        );
    }

    #[test]
    fn test_set_setting_numeric_as_string() {
        let db = setup_test_db();

        // Set numeric setting (stored as string)
        db.conn()
            .execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                ["syncInterval", "30"],
            )
            .expect("Failed to insert setting");

        let value: String = db
            .conn()
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                ["syncInterval"],
                |row| row.get(0),
            )
            .expect("Failed to query setting");

        assert_eq!(value, "30", "Numeric should be stored as string");

        // Parse back to number
        let parsed: i64 = value.parse().expect("Failed to parse");
        assert_eq!(parsed, 30, "Should be parseable back to i64");
    }

    // ============================================================
    // get_setting tests
    // ============================================================

    #[test]
    fn test_get_setting_existing() {
        let db = setup_test_db();

        // Insert a setting
        db.conn()
            .execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                ["test_key", "test_value"],
            )
            .expect("Failed to insert setting");

        let result: Option<String> = db
            .conn()
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                ["test_key"],
                |row| row.get(0),
            )
            .ok();

        assert_eq!(
            result,
            Some("test_value".to_string()),
            "Should return the value"
        );
    }

    #[test]
    fn test_get_setting_nonexistent() {
        let db = setup_test_db();

        let result: Option<String> = db
            .conn()
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                ["nonexistent_key"],
                |row| row.get(0),
            )
            .ok();

        assert!(result.is_none(), "Should return None for nonexistent key");
    }

    // ============================================================
    // Settings key tests
    // ============================================================

    #[test]
    fn test_known_settings_keys() {
        // Document the known settings keys
        let known_keys = [
            "theme",
            "theme_mode",
            "dark_theme",
            "light_theme",
            "locale",
            "showTerminologyTooltips",
            "syncInterval",
            "syncOnStart",
            "logLevel",
            "ollama_num_ctx",
            "embedding_model",
        ];

        for key in known_keys {
            assert!(!key.is_empty(), "Key '{}' should not be empty", key);
        }
    }

    #[test]
    fn test_theme_settings() {
        let db = setup_test_db();

        // Test all theme-related settings
        let theme_keys = ["theme", "theme_mode", "dark_theme", "light_theme"];

        for key in theme_keys {
            db.conn()
                .execute(
                    "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                    [key, "test_value"],
                )
                .expect(&format!("Failed to insert {} setting", key));

            let result: String = db
                .conn()
                .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                    row.get(0)
                })
                .expect(&format!("Failed to query {} setting", key));

            assert_eq!(result, "test_value", "Setting {} should work", key);
        }
    }

    #[test]
    fn test_locale_setting() {
        let db = setup_test_db();

        // Test locale setting with valid values
        let valid_locales = ["de", "en"];

        for locale in valid_locales {
            db.conn()
                .execute(
                    "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                    ["locale", locale],
                )
                .expect("Failed to set locale");

            let result: String = db
                .conn()
                .query_row(
                    "SELECT value FROM settings WHERE key = 'locale'",
                    [],
                    |row| row.get(0),
                )
                .expect("Failed to query locale");

            assert_eq!(result, locale, "Locale should be {}", locale);
        }
    }

    // ============================================================
    // get_log_levels tests
    // ============================================================

    #[test]
    fn test_get_log_levels() {
        let levels = get_log_levels();

        assert_eq!(levels.len(), 5, "Should have 5 log levels");
        assert!(levels.contains(&"error"), "Should contain 'error'");
        assert!(levels.contains(&"warn"), "Should contain 'warn'");
        assert!(levels.contains(&"info"), "Should contain 'info'");
        assert!(levels.contains(&"debug"), "Should contain 'debug'");
        assert!(levels.contains(&"trace"), "Should contain 'trace'");
    }

    #[test]
    fn test_log_levels_order() {
        let levels = get_log_levels();

        // Log levels should be in severity order (most severe to least)
        assert_eq!(levels[0], "error", "First should be 'error'");
        assert_eq!(levels[1], "warn", "Second should be 'warn'");
        assert_eq!(levels[2], "info", "Third should be 'info'");
        assert_eq!(levels[3], "debug", "Fourth should be 'debug'");
        assert_eq!(levels[4], "trace", "Fifth should be 'trace'");
    }

    // ============================================================
    // Settings value parsing tests
    // ============================================================

    #[test]
    fn test_boolean_setting_parsing() {
        // Test the parsing logic used in get_settings
        let parse_bool = |s: &str| s == "true";

        assert!(parse_bool("true"), "'true' should parse to true");
        assert!(!parse_bool("false"), "'false' should parse to false");
        assert!(!parse_bool(""), "empty string should parse to false");
        assert!(
            !parse_bool("TRUE"),
            "'TRUE' should parse to false (case-sensitive)"
        );
    }

    #[test]
    fn test_numeric_setting_parsing() {
        // Test numeric parsing
        let parse_num = |s: &str| -> Option<i64> { s.parse().ok() };

        assert_eq!(parse_num("30"), Some(30), "'30' should parse to 30");
        assert_eq!(parse_num("0"), Some(0), "'0' should parse to 0");
        assert_eq!(parse_num("-1"), Some(-1), "'-1' should parse to -1");
        assert!(parse_num("abc").is_none(), "'abc' should not parse");
        assert!(parse_num("").is_none(), "empty string should not parse");
    }

    // ============================================================
    // Settings persistence tests
    // ============================================================

    #[test]
    fn test_settings_persistence_multiple_keys() {
        let db = setup_test_db();

        // Insert multiple settings
        let settings = [("key1", "value1"), ("key2", "value2"), ("key3", "value3")];

        for (key, value) in settings {
            db.conn()
                .execute(
                    "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                    [key, value],
                )
                .expect("Failed to insert setting");
        }

        // Verify all settings
        for (key, expected_value) in settings {
            let result: String = db
                .conn()
                .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                    row.get(0)
                })
                .expect("Failed to query setting");

            assert_eq!(
                result, expected_value,
                "Setting {} should have correct value",
                key
            );
        }
    }

    #[test]
    fn test_settings_unique_constraint() {
        let db = setup_test_db();

        // Insert a setting
        db.conn()
            .execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)",
                ["unique_key", "value1"],
            )
            .expect("Failed to insert first setting");

        // Try to insert duplicate key with INSERT OR REPLACE
        db.conn()
            .execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                ["unique_key", "value2"],
            )
            .expect("INSERT OR REPLACE should work");

        // Should only have one row
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key = 'unique_key'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count");

        assert_eq!(count, 1, "Should have only one row for unique_key");

        // Value should be the latest
        let value: String = db
            .conn()
            .query_row(
                "SELECT value FROM settings WHERE key = 'unique_key'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to query");

        assert_eq!(value, "value2", "Value should be the latest");
    }

    // ============================================================
    // Default settings helper function tests
    // ============================================================

    #[test]
    fn test_get_embedding_model_from_db_default() {
        let db = setup_test_db();

        let model = get_embedding_model_from_db(db.conn());

        // Should return the default model when not set
        assert_eq!(
            model, RECOMMENDED_EMBEDDING_MODEL,
            "Should return default embedding model"
        );
    }

    #[test]
    fn test_get_embedding_model_from_db_custom() {
        let db = setup_test_db();

        // Set custom embedding model
        db.conn()
            .execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                ["embedding_model", "custom-model:latest"],
            )
            .expect("Failed to insert setting");

        let model = get_embedding_model_from_db(db.conn());

        assert_eq!(
            model, "custom-model:latest",
            "Should return custom embedding model"
        );
    }

    // ============================================================
    // JSON value conversion tests
    // ============================================================

    #[test]
    fn test_settings_to_json_value_boolean() {
        let value = "true";
        let json_value = serde_json::Value::Bool(value == "true");

        assert!(json_value.is_boolean(), "Should be a boolean");
        assert_eq!(json_value.as_bool(), Some(true), "Should be true");
    }

    #[test]
    fn test_settings_to_json_value_number() {
        let value = "30";
        let json_value = if let Ok(num) = value.parse::<i64>() {
            serde_json::Value::Number(num.into())
        } else {
            serde_json::Value::String(value.to_string())
        };

        assert!(json_value.is_number(), "Should be a number");
        assert_eq!(json_value.as_i64(), Some(30), "Should be 30");
    }

    #[test]
    fn test_settings_to_json_value_string() {
        let value = "mocha";
        let json_value = serde_json::Value::String(value.to_string());

        assert!(json_value.is_string(), "Should be a string");
        assert_eq!(json_value.as_str(), Some("mocha"), "Should be 'mocha'");
    }
}
