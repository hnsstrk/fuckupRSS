mod categories;
mod commands;
mod db;
mod keywords;
mod ollama;
mod retrieval;
mod sync;

pub use categories::{classify_by_keywords, CategoryClassifier, SEPHIROTH_CATEGORIES};
pub use keywords::{extract_keywords, normalize_keyword, normalize_and_dedupe_keywords, KeywordExtractor, Language};

use db::Database;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<Database>,
    pub batch_cancel: AtomicBool,
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

            app.manage(AppState {
                db: Mutex::new(db),
                batch_cancel: AtomicBool::new(false),
            });
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
            commands::fnords::get_fnord_stats,
            commands::sync::sync_all_feeds,
            commands::sync::sync_feed,
            commands::retrieval::fetch_full_content,
            commands::retrieval::fetch_truncated_articles,
            commands::ollama::check_ollama,
            commands::ollama::generate_summary,
            commands::ollama::analyze_article,
            commands::ollama::process_article,
            commands::ollama::process_article_discordian,
            commands::ollama::get_unprocessed_count,
            commands::ollama::process_batch,
            commands::ollama::cancel_batch,
            commands::ollama::pull_model,
            commands::ollama::get_default_prompts,
            commands::ollama::get_prompts,
            commands::ollama::set_prompts,
            commands::ollama::reset_prompts,
            commands::ollama::get_loaded_models,
            commands::ollama::load_model,
            commands::ollama::unload_model,
            commands::ollama::ensure_models_loaded,
            commands::categories::get_all_categories,
            commands::categories::get_article_categories,
            commands::categories::set_article_categories,
            commands::tags::get_all_tags,
            commands::tags::get_article_tags,
            commands::tags::add_article_tag,
            commands::tags::remove_article_tag,
            commands::tags::set_article_tags,
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::settings::get_setting,
            // Immanentize Network
            commands::immanentize::get_keywords,
            commands::immanentize::get_keyword,
            commands::immanentize::get_keyword_neighbors,
            commands::immanentize::get_keyword_categories,
            commands::immanentize::get_category_keywords,
            commands::immanentize::get_trending_keywords,
            commands::immanentize::get_network_stats,
            commands::immanentize::search_keywords,
            // Graph & Trend Visualization
            commands::immanentize::get_keyword_trend,
            commands::immanentize::get_network_graph,
            commands::immanentize::get_trending_comparison,
            commands::immanentize::get_keyword_articles,
            // Keyword Maintenance
            commands::immanentize::prune_keywords,
            commands::immanentize::get_keyword_health,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
