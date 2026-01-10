use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HardwareProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub ai_parallelism: usize,
}

const DEFAULT_PROFILES_JSON: &str = r#"[
    {
        "id": "default",
        "name": "Standard (Kompatibel)",
        "description": "Sichere Einstellung für alle Systeme. Ein Artikel nach dem anderen.",
        "ai_parallelism": 1
    },
    {
        "id": "rtx3080ti",
        "name": "Desktop (RTX 3080 Ti)",
        "description": "Optimiert für 12GB VRAM. Verarbeitet 4 Artikel gleichzeitig.",
        "ai_parallelism": 4
    },
    {
        "id": "m4pro",
        "name": "MacBook Pro M4",
        "description": "Maximale Leistung dank Unified Memory. Verarbeitet 8 Artikel gleichzeitig.",
        "ai_parallelism": 8
    }
]"#;

pub fn get_default_profiles() -> Vec<HardwareProfile> {
    serde_json::from_str(DEFAULT_PROFILES_JSON).unwrap_or_default()
}

#[tauri::command]
pub fn get_hardware_profiles(state: State<AppState>) -> Result<Vec<HardwareProfile>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    
    // Read from settings "hardware_profiles"
    // If not set, return defaults (and maybe save them? No, keep it clean)
    let json: Option<String> = db.conn()
        .query_row("SELECT value FROM settings WHERE key = 'hardware_profiles'", [], |row| row.get(0))
        .ok();

    let profiles = if let Some(json_str) = json {
        serde_json::from_str(&json_str).unwrap_or_else(|_| get_default_profiles())
    } else {
        get_default_profiles()
    };
    
    Ok(profiles)
}

#[tauri::command]
pub fn save_hardware_profile(state: State<AppState>, profile: HardwareProfile) -> Result<Vec<HardwareProfile>, String> {
    let mut profiles = get_hardware_profiles(state.clone())?;
    
    // Update or add
    if let Some(idx) = profiles.iter().position(|p| p.id == profile.id) {
        profiles[idx] = profile;
    } else {
        profiles.push(profile);
    }
    
    let json = serde_json::to_string(&profiles).map_err(|e| e.to_string())?;
    
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.conn()
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('hardware_profiles', ?1)",
            [&json],
        )
        .map_err(|e| e.to_string())?;
        
    Ok(profiles)
}

#[tauri::command]
pub fn delete_hardware_profile(state: State<AppState>, profile_id: String) -> Result<Vec<HardwareProfile>, String> {
    let mut profiles = get_hardware_profiles(state.clone())?;
    
    // Prevent deleting built-in defaults if we want enforce them? 
    // Let's allow deletion but maybe user wants them back.
    
    profiles.retain(|p| p.id != profile_id);
    
    let json = serde_json::to_string(&profiles).map_err(|e| e.to_string())?;
    
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.conn()
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('hardware_profiles', ?1)",
            [&json],
        )
        .map_err(|e| e.to_string())?;
        
    Ok(profiles)
}

#[tauri::command]
pub fn apply_hardware_profile(state: State<AppState>, profile_id: String) -> Result<(), String> {
    let profiles = get_hardware_profiles(state.clone())?;
    
    if let Some(profile) = profiles.into_iter().find(|p| p.id == profile_id) {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        
        // Save parallelism setting
        db.conn()
            .execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES ('ai_parallelism', ?1)",
                [profile.ai_parallelism.to_string()],
            )
            .map_err(|e| e.to_string())?;
            
        // Save current profile ID for UI
         db.conn()
            .execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES ('active_hardware_profile', ?1)",
                [profile.id],
            )
            .map_err(|e| e.to_string())?;
    } else {
        return Err("Profile not found".to_string());
    }
    
    Ok(())
}
