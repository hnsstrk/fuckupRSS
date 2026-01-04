mod commands;
mod db;
mod ollama;
mod retrieval;
mod sync;

use db::Database;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<Database>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize database
            let db = Database::new(app.handle())?;

            // Seed development data in debug mode
            #[cfg(debug_assertions)]
            db.seed_dev_data()?;

            app.manage(AppState { db: Mutex::new(db) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pentacles::get_pentacles,
            commands::pentacles::add_pentacle,
            commands::pentacles::delete_pentacle,
            commands::fnords::get_fnords,
            commands::fnords::get_fnord,
            commands::fnords::update_fnord_status,
            commands::fnords::get_changed_fnords,
            commands::fnords::acknowledge_changes,
            commands::fnords::get_fnord_revisions,
            commands::fnords::get_changed_count,
            commands::fnords::reset_all_changes,
            commands::sync::sync_all_feeds,
            commands::sync::sync_feed,
            commands::retrieval::fetch_full_content,
            commands::retrieval::fetch_truncated_articles,
            commands::ollama::check_ollama,
            commands::ollama::generate_summary,
            commands::ollama::analyze_article,
            commands::ollama::process_article,
            commands::ollama::get_unprocessed_count,
            commands::ollama::process_batch,
            commands::ollama::pull_model,
            commands::ollama::get_default_prompts,
            commands::ollama::get_prompts,
            commands::ollama::set_prompts,
            commands::ollama::reset_prompts,
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::settings::get_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
