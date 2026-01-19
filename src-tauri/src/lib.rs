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
mod text_analysis;

pub use categories::{classify_by_keywords, CategoryClassifier, SEPHIROTH_CATEGORIES};
pub use embedding_worker::EmbeddingWorker;
pub use keywords::{
    extract_keywords, normalize_keyword, normalize_and_dedupe_keywords,
    find_canonical_keyword, find_canonical_keyword_with_db, load_dynamic_synonyms,
    split_compound_keyword, expand_compound_keywords,
    KeywordExtractor, Language,
    // Unified keyword types
    KeywordSource, KeywordWithMetadata, ExtractedKeywordCandidate, ArticleKeywordRef,
    // Centralized configuration
    KeywordConfig, keyword_defaults,
};
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

            // Load dynamic synonyms from database
            {
                let db_guard = db.lock().expect("DB lock failed");
                if let Ok(count) = keywords::load_dynamic_synonyms(db_guard.conn()) {
                    if count > 0 {
                        info!("Loaded {} dynamic synonyms from database", count);
                    }
                }
            }

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
            commands::fnords::get_subcategory_stats,
            // Extended Fnord Statistics (Plan 4)
            commands::fnords::get_article_timeline,
            commands::fnords::get_greyface_index,
            commands::fnords::get_top_keywords_stats,
            commands::fnords::get_feed_activity,
            commands::fnords::get_bias_heatmap,
            commands::fnords::get_keyword_cloud,
            commands::sync::sync_all_feeds,
            commands::sync::sync_feed,
            commands::retrieval::fetch_full_content,
            commands::retrieval::fetch_truncated_articles,
            commands::retrieval::refetch_short_articles,
            commands::retrieval::get_short_content_stats,
            // Model Management
            commands::ollama::model_management::check_ollama,
            commands::ollama::model_management::get_loaded_models,
            commands::ollama::model_management::load_model,
            commands::ollama::model_management::unload_model,
            commands::ollama::model_management::ensure_models_loaded,
            commands::ollama::model_management::pull_model,
            // Article Processing
            commands::ollama::article_processor::generate_summary,
            commands::ollama::article_processor::analyze_article,
            commands::ollama::article_processor::process_article,
            commands::ollama::article_processor::process_article_discordian,
            // Batch Processing
            commands::ollama::batch_processor::get_unprocessed_count,
            commands::ollama::batch_processor::get_hopeless_count,
            commands::ollama::batch_processor::get_failed_count,
            commands::ollama::batch_processor::get_failed_articles,
            commands::ollama::batch_processor::get_hopeless_articles,
            commands::ollama::batch_processor::process_batch,
            commands::ollama::batch_processor::cancel_batch,
            // Prompts
            commands::ollama::prompts::get_default_prompts,
            commands::ollama::prompts::get_prompts,
            commands::ollama::prompts::set_prompts,
            commands::ollama::prompts::reset_prompts,
            commands::ollama::prompts::reset_articles_for_reprocessing,
            // Similar Articles & Semantic Search (Phase 3)
            commands::ollama::similarity::find_similar_articles,
            commands::ollama::similarity::get_article_embedding_stats,
            commands::ollama::similarity::generate_article_embeddings_batch,
            commands::ollama::similarity::semantic_search,
            commands::categories::get_all_categories,
            commands::categories::get_main_categories,
            commands::categories::get_subcategories,
            commands::categories::get_categories_with_stats,
            commands::categories::get_subcategory_names,
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
            // Manual Keyword Management
            commands::immanentize::create_keyword,
            commands::immanentize::delete_keyword,
            commands::immanentize::rename_keyword,
            // Learning System: Auto-merge similar keywords
            commands::immanentize::auto_merge_similar_keywords,
            // Keyword Type Batch Update
            commands::immanentize::update_keyword_types,
            // Keyword Cleanup (stopwords, seeds, type detection)
            commands::immanentize::cleanup_keywords,
            // Embedding Queue
            commands::embedding::get_embedding_queue_status,
            commands::embedding::process_embedding_queue_now,
            commands::embedding::queue_missing_embeddings,
            commands::embedding::get_embedding_queue_details,
            commands::embedding::calculate_neighbor_similarities,
            // OPML Import/Export
            commands::opml::parse_opml_preview,
            commands::opml::import_opml,
            commands::opml::export_opml,
            // Operation Mindfuck (Bias Mirror)
            commands::mindfuck::get_reading_profile,
            commands::mindfuck::get_blind_spots,
            commands::mindfuck::get_counter_perspectives,
            commands::mindfuck::get_reading_trends,
            // Recommendations (Operation Mindfuck - Personalized)
            commands::recommendations::get_recommendations,
            commands::recommendations::save_article,
            commands::recommendations::unsave_article,
            commands::recommendations::hide_recommendation,
            commands::recommendations::get_saved_articles,
            commands::recommendations::get_recommendation_stats,
            // Article Analysis (Statistical Keywords/Categories)
            commands::article_analysis::get_article_keywords,
            commands::article_analysis::add_article_keyword,
            commands::article_analysis::remove_article_keyword,
            commands::article_analysis::get_article_categories_detailed,
            commands::article_analysis::update_article_categories,
            commands::article_analysis::add_article_category,
            commands::article_analysis::remove_article_category,
            commands::article_analysis::analyze_article_statistical,
            commands::article_analysis::get_unprocessed_statistical_count,
            commands::article_analysis::process_statistical_batch,
            commands::article_analysis::record_correction,
            commands::article_analysis::get_bias_stats,
            commands::article_analysis::get_similar_keywords,
            commands::article_analysis::get_keyword_suggestions_from_network,
            commands::article_analysis::score_keywords_semantically,
            // Stopword Management
            commands::stopwords::get_user_stopwords,
            commands::stopwords::get_system_stopwords,
            commands::stopwords::get_all_stopwords_list,
            commands::stopwords::add_stopword,
            commands::stopwords::add_stopwords_batch,
            commands::stopwords::remove_stopword,
            commands::stopwords::get_stopwords_stats,
            commands::stopwords::is_stopword_check,
            commands::stopwords::search_stopwords,
            commands::stopwords::clear_user_stopwords,
            commands::stopwords::reset_stopwords,
            commands::stopwords::restore_system_stopwords,
            commands::stopwords::export_stopwords,
            commands::stopwords::import_stopwords,
            // Keyword Type Detection (Semantic)
            commands::keyword_type_detection::init_keyword_type_prototypes,
            commands::keyword_type_detection::generate_keyword_type_prototypes,
            commands::keyword_type_detection::detect_single_keyword_type,
            commands::keyword_type_detection::update_keyword_types_hybrid,
            commands::keyword_type_detection::count_untyped_keywords,
            commands::keyword_type_detection::update_untyped_keywords,
            commands::keyword_type_detection::get_prototype_stats,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, _event| {
            // App lifecycle events handled by Tauri defaults
        });
}
