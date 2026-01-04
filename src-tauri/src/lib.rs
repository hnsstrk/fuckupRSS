mod commands;
mod db;

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
