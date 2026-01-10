mod categories;
mod commands;
mod db;
mod embedding_worker;
mod embeddings;
mod keywords;
mod logging;
mod ollama;
mod retrieval;
mod sync;

pub use categories::{classify_by_keywords, CategoryClassifier, SEPHIROTH_CATEGORIES};
pub use embedding_worker::EmbeddingWorker;
pub use keywords::{extract_keywords, normalize_keyword, normalize_and_dedupe_keywords, find_canonical_keyword, KeywordExtractor, Language};
pub use logging::LogLevel;

use db::Database;
use log::info;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub batch_cancel: AtomicBool,
    pub batch_running: Arc<AtomicBool>,
    pub embedding_worker: Arc<EmbeddingWorker>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::Webview),
                ])
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            info!("fuckupRSS starting up...");
            // Initialize database
            let db = Database::new(app.handle())?;

            // Seed development data in debug mode
            #[cfg(debug_assertions)]
            db.seed_dev_data()?;

            let db = Arc::new(Mutex::new(db));
            let embedding_worker = Arc::new(EmbeddingWorker::new());
            let batch_running = Arc::new(AtomicBool::new(false));

            // Queue existing keywords without embeddings for processing
            if let Ok(queued) = embedding_worker::queue_keywords_without_embeddings(&db) {
                if queued > 0 {
                    info!("Queued {} keywords for embedding generation", queued);
                }
            }

            // Start background embedding worker
            embedding_worker::start_background_worker(
                db.clone(),
                embedding_worker.clone(),
                app.handle().clone(),
                batch_running.clone(),
            );

            app.manage(AppState {
                db,
                batch_cancel: AtomicBool::new(false),
                batch_running,
                embedding_worker,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pentacles::get_pentacles,
            commands::pentacles::add_pentacle,
            commands::pentacles::delete_pentacle,
            commands::fnords::get_fnords,
            commands::fnords::get_fnord,
            commands::fnords::get_fnords_count,
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
            commands::ollama::get_hopeless_count,
            commands::ollama::get_failed_count,
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
            commands::ollama::reset_articles_for_reprocessing,
            // Similar Articles & Semantic Search (Phase 3)
            commands::ollama::find_similar_articles,
            commands::ollama::get_article_embedding_stats,
            commands::ollama::generate_article_embeddings_batch,
            commands::ollama::semantic_search,
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
            commands::settings::get_system_theme,
            commands::settings::get_log_levels,
            commands::settings::set_log_level,
            // Hardware Profiles
            commands::profiles::get_hardware_profiles,
            commands::profiles::save_hardware_profile,
            commands::profiles::delete_hardware_profile,
            commands::profiles::apply_hardware_profile,
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
            commands::immanentize::merge_synonym_keywords,
            commands::immanentize::cleanup_garbage_keywords,
            // Quality Score System
            commands::immanentize::calculate_keyword_quality_scores,
            commands::immanentize::get_low_quality_keywords,
            commands::immanentize::auto_prune_low_quality,
            // Embedding-based Synonym Detection
            commands::immanentize::find_similar_keywords,
            commands::immanentize::find_synonym_candidates,
            commands::immanentize::merge_keyword_pair,
            commands::immanentize::dismiss_synonym_pair,
            commands::immanentize::get_cooccurring_keywords,
            // Embedding Queue
            commands::embedding::get_embedding_queue_status,
            commands::embedding::process_embedding_queue_now,
            commands::embedding::queue_missing_embeddings,
            commands::embedding::get_embedding_queue_details,
            commands::embedding::calculate_neighbor_similarities,
            // OPML Import/Export
            commands::opml::parse_opml_preview,
            commands::opml::import_opml,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, _event| {
            // App lifecycle events handled by Tauri defaults
        });
}
