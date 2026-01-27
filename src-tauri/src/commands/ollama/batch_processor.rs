//! Batch processing commands for article analysis

use crate::embedding_worker;
use crate::keywords::{
    cluster_articles, get_representatives, calculate_savings,
    ArticleForClustering, ClusterConfig, ClusteringResult,
};
use crate::ollama::{DiscordianAnalysis, OllamaClient, OllamaError};
use crate::text_analysis::{
    record_correction, BiasWeights, CategoryMatcher, CorrectionRecord, CorrectionType, CorpusStats,
    TfIdfExtractor,
};
use crate::{classify_by_keywords, extract_keywords};
use crate::AppState;
use futures::{stream, StreamExt};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{Emitter, Manager, State, Window};

/// Global counter for tracking active parallel LLM requests
static ACTIVE_LLM_REQUESTS: AtomicUsize = AtomicUsize::new(0);

use super::data_persistence::{
    generate_and_save_article_embedding, recalculate_keyword_weights, save_article_categories,
    save_article_categories_with_source, save_article_keywords_and_network,
    save_article_keywords_with_source,
};
use super::helpers::{
    check_analysis_cache, compute_content_hash, derive_categories_from_keywords,
    determine_category_sources, determine_keyword_sources, get_ai_concurrency, get_locale_from_db,
    get_num_ctx_setting, merge_categories_stat_primary, merge_keywords, store_analysis_cache,
};
use super::types::{
    BatchArticle, BatchProgress, BatchResult, FailedCount, HopelessCount, UnprocessedCount,
};

/// Type alias for articles with optional embeddings (for clustering)
type ArticleWithEmbedding = (BatchArticle, Option<Vec<f32>>);

/// Shared context for batch processing - loaded once before processing starts
struct BatchContext {
    bias_weights: BiasWeights,
    corpus_stats: Option<CorpusStats>,
    /// Custom discordian prompt (None = use default)
    discordian_prompt: Option<String>,
}

/// Configuration for cluster-based batch processing
///
/// Cluster-based batch processing - implemented but intentionally dormant.
///
/// This feature groups similar articles using embeddings and processes only
/// cluster representatives via LLM, then transfers keywords to cluster members.
///
/// **Status:** Implemented but not exposed to frontend.
/// **Trade-off:** Speed (~30-50% fewer LLM calls) vs. accuracy (cluster transfers use confidence=0.85).
/// **To enable:** Register in invoke_handler (lib.rs), add frontend UI, write integration tests.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ClusterBatchConfig {
    /// Whether to use clustering optimization
    pub use_clustering: bool,
    /// Minimum articles to enable clustering (below this, process all)
    pub min_articles_for_clustering: usize,
    /// Clustering configuration
    pub cluster_config: ClusterConfig,
}

impl Default for ClusterBatchConfig {
    fn default() -> Self {
        Self {
            use_clustering: true,
            min_articles_for_clustering: 10,
            cluster_config: ClusterConfig {
                distance_threshold: 0.4, // Cosine distance
                min_cluster_size: 2,     // Need at least 2 to benefit
                max_clusters: 0,         // Unlimited
            },
        }
    }
}

/// Result of cluster-optimized batch processing
///
/// Part of the dormant cluster-based batch processing feature.
/// See [`ClusterBatchConfig`] for full documentation on status and trade-offs.
#[derive(Debug, Clone, serde::Serialize)]
#[allow(dead_code)]
pub struct ClusterBatchResult {
    /// Standard batch result
    pub processed: i64,
    pub succeeded: i64,
    pub failed: i64,
    /// Clustering statistics
    pub clusters_found: usize,
    pub llm_calls_saved: usize,
    pub savings_percentage: f64,
}

/// Load articles with embeddings for clustering
///
/// Part of the dormant cluster-based batch processing feature.
/// See [`ClusterBatchConfig`] for full documentation on status and trade-offs.
#[allow(dead_code)]
fn load_articles_for_clustering(
    state: &AppState,
    limit: Option<i64>,
) -> Result<Vec<ArticleWithEmbedding>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let query = match limit {
        Some(n) => format!(
            r#"SELECT f.id, f.title, f.content_full,
                      DATE(COALESCE(f.published_at, f.fetched_at)) as article_date,
                      COALESCE(f.analysis_attempts, 0) as attempts,
                      f.analysis_error,
                      f.embedding
               FROM fnords f
               WHERE f.processed_at IS NULL
               AND f.content_full IS NOT NULL AND LENGTH(f.content_full) >= 100
               AND (f.analysis_hopeless IS NULL OR f.analysis_hopeless = FALSE)
               ORDER BY f.published_at DESC
               LIMIT {}"#,
            n
        ),
        None => r#"SELECT f.id, f.title, f.content_full,
                      DATE(COALESCE(f.published_at, f.fetched_at)) as article_date,
                      COALESCE(f.analysis_attempts, 0) as attempts,
                      f.analysis_error,
                      f.embedding
               FROM fnords f
               WHERE f.processed_at IS NULL
               AND f.content_full IS NOT NULL AND LENGTH(f.content_full) >= 100
               AND (f.analysis_hopeless IS NULL OR f.analysis_hopeless = FALSE)
               ORDER BY f.published_at DESC"#
            .to_string(),
    };

    let mut stmt = db.conn().prepare(&query).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let embedding: Option<Vec<u8>> = row.get(6)?;
            let embedding_f32: Option<Vec<f32>> = embedding.map(|bytes| {
                bytes
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect()
            });

            Ok((
                BatchArticle {
                    fnord_id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    article_date: row.get(3)?,
                    attempts: row.get(4)?,
                    previous_error: row.get(5)?,
                },
                embedding_f32,
            ))
        })
        .map_err(|e| e.to_string())?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Apply clustering to articles and return optimized processing plan
///
/// Part of the dormant cluster-based batch processing feature.
/// See [`ClusterBatchConfig`] for full documentation on status and trade-offs.
#[allow(dead_code)]
fn apply_clustering(
    articles: Vec<(BatchArticle, Option<Vec<f32>>)>,
    config: &ClusterBatchConfig,
) -> (ClusteringResult, HashMap<i64, BatchArticle>) {
    // Build article map for later lookup
    let article_map: HashMap<i64, BatchArticle> = articles
        .iter()
        .map(|(a, _)| (a.fnord_id, a.clone()))
        .collect();

    // Only articles with embeddings can be clustered
    let clusterable: Vec<ArticleForClustering> = articles
        .iter()
        .filter_map(|(article, embedding)| {
            embedding.as_ref().map(|emb| ArticleForClustering {
                id: article.fnord_id,
                title: article.title.clone(),
                embedding: emb.clone(),
                summary: None,
                is_processed: false,
            })
        })
        .collect();

    let non_clusterable_ids: Vec<i64> = articles
        .iter()
        .filter(|(_, emb)| emb.is_none())
        .map(|(a, _)| a.fnord_id)
        .collect();

    if clusterable.len() < config.min_articles_for_clustering {
        // Not enough articles for clustering, return all as unclustered
        let result = ClusteringResult {
            clusters: vec![],
            unclustered_ids: articles.iter().map(|(a, _)| a.fnord_id).collect(),
            total_articles: articles.len(),
            representatives_count: articles.len(),
        };
        return (result, article_map);
    }

    // Perform clustering
    let mut clustering_result = cluster_articles(clusterable, &config.cluster_config);

    // Add non-clusterable articles to unclustered list
    clustering_result.unclustered_ids.extend(non_clusterable_ids);
    clustering_result.representatives_count =
        clustering_result.clusters.len() + clustering_result.unclustered_ids.len();

    (clustering_result, article_map)
}

/// Get count of hopeless articles (failed 3+ times)
#[tauri::command]
pub fn get_hopeless_count(state: State<AppState>) -> Result<HopelessCount, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnords WHERE analysis_hopeless = TRUE",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(HopelessCount { count })
}

/// Get count of failed articles (attempted but not processed successfully, not hopeless)
#[tauri::command]
pub fn get_failed_count(state: State<AppState>) -> Result<FailedCount, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let count: i64 = db
        .conn()
        .query_row(
            r#"SELECT COUNT(*) FROM fnords
               WHERE analysis_attempts > 0
               AND processed_at IS NULL
               AND (analysis_hopeless IS NULL OR analysis_hopeless = FALSE)"#,
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(FailedCount { count })
}

/// Get count of unprocessed articles
#[tauri::command]
pub fn get_unprocessed_count(state: State<AppState>) -> Result<UnprocessedCount, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let total: i64 = db
        .conn()
        .query_row(
            r#"SELECT COUNT(*) FROM fnords
               WHERE processed_at IS NULL
               AND (analysis_hopeless IS NULL OR analysis_hopeless = FALSE)"#,
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let with_content: i64 = db
        .conn()
        .query_row(
            r#"SELECT COUNT(*) FROM fnords
               WHERE processed_at IS NULL
               AND content_full IS NOT NULL AND LENGTH(content_full) >= 100
               AND (analysis_hopeless IS NULL OR analysis_hopeless = FALSE)"#,
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(UnprocessedCount { total, with_content })
}

/// Get failed articles (attempted but not processed successfully, not hopeless)
#[tauri::command]
pub fn get_failed_articles(
    state: State<AppState>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<super::types::AnalysisStatusArticle>, String> {
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"SELECT
                f.id, f.title, f.pentacle_id, p.title as pentacle_title,
                f.summary, f.published_at, f.status, f.analysis_attempts, f.analysis_error
            FROM fnords f
            LEFT JOIN pentacles p ON p.id = f.pentacle_id
            WHERE f.analysis_attempts > 0
              AND f.processed_at IS NULL
              AND (f.analysis_hopeless IS NULL OR f.analysis_hopeless = FALSE)
            ORDER BY f.analysis_attempts DESC, f.published_at DESC
            LIMIT ? OFFSET ?"#,
        )
        .map_err(|e| e.to_string())?;

    let articles: Vec<super::types::AnalysisStatusArticle> = stmt
        .query_map([limit, offset], |row| {
            Ok(super::types::AnalysisStatusArticle {
                id: row.get(0)?,
                title: row.get(1)?,
                pentacle_id: row.get(2)?,
                pentacle_title: row.get(3)?,
                summary: row.get(4)?,
                published_at: row.get(5)?,
                status: row.get(6)?,
                analysis_attempts: row.get(7)?,
                last_error: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(articles)
}

/// Get hopeless articles (failed 3+ times, marked as hopeless)
#[tauri::command]
pub fn get_hopeless_articles(
    state: State<AppState>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<super::types::AnalysisStatusArticle>, String> {
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"SELECT
                f.id, f.title, f.pentacle_id, p.title as pentacle_title,
                f.summary, f.published_at, f.status, f.analysis_attempts, f.analysis_error
            FROM fnords f
            LEFT JOIN pentacles p ON p.id = f.pentacle_id
            WHERE f.analysis_hopeless = TRUE
            ORDER BY f.analysis_attempts DESC, f.published_at DESC
            LIMIT ? OFFSET ?"#,
        )
        .map_err(|e| e.to_string())?;

    let articles: Vec<super::types::AnalysisStatusArticle> = stmt
        .query_map([limit, offset], |row| {
            Ok(super::types::AnalysisStatusArticle {
                id: row.get(0)?,
                title: row.get(1)?,
                pentacle_id: row.get(2)?,
                pentacle_title: row.get(3)?,
                summary: row.get(4)?,
                published_at: row.get(5)?,
                status: row.get(6)?,
                analysis_attempts: row.get(7)?,
                last_error: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(articles)
}

/// Process a single article with full statistical + LLM pipeline
async fn process_single_article(
    client: &OllamaClient,
    state: &AppState,
    model: &str,
    locale: &str,
    article: BatchArticle,
    batch_context: &Arc<BatchContext>,
) -> (bool, Option<String>) {
    let fnord_id = article.fnord_id;
    let title = article.title.clone();
    let content = article.content.clone();

    if content.is_empty() {
        return (false, Some("No content".to_string()));
    }

    // === STATISTICAL PRE-ANALYSIS ===
    let text_for_analysis = format!("{} {}", title, content);

    let extractor = TfIdfExtractor::new().with_max_keywords(30);
    let mut keyword_candidates: Vec<(String, f64)> = extractor
        .extract_smart(&text_for_analysis, batch_context.corpus_stats.as_ref())
        .into_iter()
        .map(|kc| {
            let adjusted_score = batch_context.bias_weights.apply_to_keyword(&kc.term, kc.score);
            (kc.term, adjusted_score)
        })
        .collect();

    keyword_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    keyword_candidates.truncate(15);
    let stat_keywords: Vec<String> = keyword_candidates.into_iter().map(|(term, _)| term).collect();

    let document_tokens = extractor.get_tokens(&text_for_analysis);

    let matcher = CategoryMatcher::new().with_max_categories(5);
    let stat_categories: Vec<(String, f64)> = matcher
        .score_categories(&text_for_analysis, Some(&batch_context.bias_weights))
        .into_iter()
        .map(|cs| (cs.name.clone(), cs.confidence))
        .collect();

    let local_keywords = extract_keywords(&title, &content, 10);
    let local_categories = classify_by_keywords(&local_keywords);

    // === CACHE CHECK ===
    let content_hash = compute_content_hash(&title, &content);
    let cached_result = {
        let db = match state.db.lock() {
            Ok(db) => db,
            Err(_) => return (false, Some("Database lock failed".to_string())),
        };
        check_analysis_cache(db.conn(), &content_hash)
    };

    // Use cached result if available, otherwise call LLM
    let (analysis, _from_cache): (DiscordianAnalysis, bool) = if let Some(cached) = cached_result {
        debug!("[LLM] Cache hit for article {}: \"{}\"", fnord_id, &title[..title.len().min(50)]);
        (
            DiscordianAnalysis {
                summary: cached.summary,
                categories: cached.categories,
                keywords: cached.keywords,
                political_bias: cached.political_bias,
                sachlichkeit: cached.sachlichkeit,
            },
            true,
        )
    } else {
        // === LLM ANALYSIS ===
        // Track active parallel requests
        let active_before = ACTIVE_LLM_REQUESTS.fetch_add(1, Ordering::SeqCst);
        let active_count = active_before + 1;
        let llm_start = Instant::now();

        info!(
            "[LLM] Starting analysis for \"{}\" (ID: {}) [{} parallel active]",
            &title[..title.len().min(60)],
            fnord_id,
            active_count
        );

        let analysis_result = client
            .discordian_analysis_with_stats_custom(
                model,
                &title,
                &content,
                locale,
                &stat_keywords,
                &stat_categories,
                batch_context.discordian_prompt.as_deref(),
            )
            .await;

        // Decrement active counter after request completes
        let active_after = ACTIVE_LLM_REQUESTS.fetch_sub(1, Ordering::SeqCst);
        let duration = llm_start.elapsed();

        match analysis_result {
            Ok(analysis_with_rejections) => {
                info!(
                    "[LLM] Completed \"{}\" (ID: {}) in {:.2}s [{} parallel remaining]",
                    &title[..title.len().min(50)],
                    fnord_id,
                    duration.as_secs_f64(),
                    active_after.saturating_sub(1)
                );

                // === LEARN FROM LLM REJECTIONS ===
                {
                    let db = match state.db.lock() {
                        Ok(db) => db,
                        Err(e) => return (false, Some(format!("Database lock failed: {}", e))),
                    };

                    for rejected_kw in &analysis_with_rejections.rejected_keywords {
                        let _ = record_correction(
                            db.conn(),
                            &CorrectionRecord {
                                fnord_id,
                                correction_type: CorrectionType::KeywordRemoved,
                                old_value: Some(rejected_kw.clone()),
                                new_value: None,
                                matching_terms: vec![],
                                category_id: None,
                            },
                        );
                    }

                    for rejected_cat in &analysis_with_rejections.rejected_categories {
                        let cat_id: Option<i64> = db
                            .conn()
                            .query_row(
                                "SELECT id FROM sephiroth WHERE LOWER(name) = LOWER(?1)",
                                [rejected_cat],
                                |row| row.get(0),
                            )
                            .ok();

                        if let Some(cat_id) = cat_id {
                            let matching_terms: Vec<String> = stat_keywords.iter().take(5).cloned().collect();
                            let _ = record_correction(
                                db.conn(),
                                &CorrectionRecord {
                                    fnord_id,
                                    correction_type: CorrectionType::CategoryRemoved,
                                    old_value: Some(rejected_cat.clone()),
                                    new_value: None,
                                    matching_terms,
                                    category_id: Some(cat_id),
                                },
                            );
                        }
                    }

                    // Store in cache for future use
                    let llm_analysis: DiscordianAnalysis = analysis_with_rejections.clone().into();
                    let _ = store_analysis_cache(
                        db.conn(),
                        &content_hash,
                        &llm_analysis.summary,
                        &llm_analysis.categories,
                        &llm_analysis.keywords,
                        llm_analysis.political_bias,
                        llm_analysis.sachlichkeit,
                    );
                }

                (analysis_with_rejections.into(), false)
            }
            Err(e) => {
                warn!(
                    "[LLM] FAILED \"{}\" (ID: {}) after {:.2}s: {} [{} parallel remaining]",
                    &title[..title.len().min(50)],
                    fnord_id,
                    duration.as_secs_f64(),
                    e,
                    active_after.saturating_sub(1)
                );

                // Handle LLM error (same as before)
                let error_msg = e.to_string();
                let is_json_error = matches!(e, OllamaError::JsonParseError { .. });

                {
                    let db = match state.db.lock() {
                        Ok(db) => db,
                        Err(e) => return (false, Some(format!("Database lock failed: {}", e))),
                    };

                    let attempts: i32 = db
                        .conn()
                        .query_row(
                            "SELECT COALESCE(analysis_attempts, 0) FROM fnords WHERE id = ?1",
                            [fnord_id],
                            |row| row.get(0),
                        )
                        .unwrap_or(0);

                    let new_attempts = attempts + 1;
                    let is_hopeless = new_attempts >= 3;

                    let _ = db.conn().execute(
                        r#"UPDATE fnords SET
                            analysis_attempts = ?1,
                            analysis_error = ?2,
                            analysis_hopeless = ?3
                        WHERE id = ?4"#,
                        rusqlite::params![new_attempts, &error_msg, is_hopeless, fnord_id],
                    );
                }

                if is_json_error {
                    warn!("JSON parse error for article {}: {}", fnord_id, error_msg);
                }

                return (false, Some(error_msg));
            }
        }
    };

    // Save analysis to DB (either from cache or LLM)
    {
        let db = match state.db.lock() {
            Ok(db) => db,
            Err(e) => return (false, Some(format!("Database lock failed: {}", e))),
        };

        let update_result = db.conn().execute(
            r#"UPDATE fnords SET
                summary = ?1,
                political_bias = ?2,
                sachlichkeit = ?3,
                processed_at = CURRENT_TIMESTAMP,
                analysis_attempts = 0,
                analysis_error = NULL
            WHERE id = ?4"#,
            (&analysis.summary, analysis.political_bias, analysis.sachlichkeit, fnord_id),
        );

                if let Err(e) = update_result {
                    return (false, Some(format!("DB update failed: {}", e)));
                }

                // Use statistical categories as PRIMARY source (more reliable than LLM)
                let merged_categories = merge_categories_stat_primary(
                    &stat_categories,
                    &analysis.categories,
                    local_categories.clone(),
                    0.2, // min confidence threshold for statistical categories
                );

                // Save keywords FIRST (so immanentize_sephiroth links are established)
                let merged_keywords = merge_keywords(&analysis.keywords, local_keywords.clone(), 15);
                let keywords_with_source = determine_keyword_sources(&merged_keywords, &stat_keywords);

                // Use initial merged categories for keyword-category associations
                let initial_categories_with_source =
                    determine_category_sources(&merged_categories, &stat_categories);
                let initial_categories_saved = initial_categories_with_source
                    .iter()
                    .map(|c| c.name.clone())
                    .collect::<Vec<_>>();

                let (_tags_saved, tag_ids) = save_article_keywords_with_source(
                    db.conn(),
                    fnord_id,
                    &keywords_with_source,
                    &initial_categories_saved,
                    article.article_date.as_deref(),
                );

                recalculate_keyword_weights(db.conn(), &tag_ids);

                // Derive additional categories from keyword network
                // This uses the immanentize_sephiroth associations to find categories
                // that are commonly associated with this article's keywords
                let keyword_names: Vec<String> = keywords_with_source
                    .iter()
                    .map(|kw| kw.name.clone())
                    .collect();
                let network_categories =
                    derive_categories_from_keywords(db.conn(), &keyword_names, 0.15, 2);

                // Merge network categories with existing merged categories
                // Network categories are treated as 'statistical' source
                let mut final_categories = merged_categories.clone();
                let seen: std::collections::HashSet<String> = final_categories
                    .iter()
                    .map(|c| c.to_lowercase())
                    .collect();

                // Also extend stat_categories with network categories for source determination
                let mut stat_cats_extended = stat_categories.clone();
                for (cat_name, weight) in &network_categories {
                    if !seen.contains(&cat_name.to_lowercase()) {
                        final_categories.push(cat_name.clone());
                    }
                    if !stat_cats_extended.iter().any(|(n, _)| n.to_lowercase() == cat_name.to_lowercase()) {
                        stat_cats_extended.push((cat_name.clone(), *weight));
                    }
                }
                final_categories.truncate(5); // Keep max 5 categories

                let categories_with_source =
                    determine_category_sources(&final_categories, &stat_cats_extended);
                let _categories_saved =
                    save_article_categories_with_source(db.conn(), fnord_id, &categories_with_source);

        if let Err(e) = CorpusStats::update_db_with_document(db.conn(), &document_tokens) {
            debug!("Failed to update corpus stats: {}", e);
        }
    }

    // NOTE: Article embeddings are generated at the END of the batch
    // to avoid constant model switching between LLM and embedding model.
    // See the "Generate article embeddings" section after the main loop.

    (true, None)
}

/// Process all unprocessed articles in batch
#[tauri::command]
pub async fn process_batch(
    window: Window,
    state: State<'_, AppState>,
    model: String,
    limit: Option<i64>,
) -> Result<BatchResult, String> {
    let locale = get_locale_from_db(&state);
    let concurrency = get_ai_concurrency(&state);

    info!("Starting batch processing: model={}, limit={:?}, concurrency={}", model, limit, concurrency);

    // Load shared context ONCE before batch processing starts
    let batch_context = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let bias_weights = BiasWeights::load_from_db(db.conn()).unwrap_or_default();
        let corpus_stats = match CorpusStats::load_from_db(db.conn()) {
            Ok(stats) => Some(stats),
            Err(e) => {
                warn!("Failed to load corpus stats, using fallback TF-IDF: {}", e);
                None
            }
        };

        // Load custom discordian prompt from settings (if set)
        let discordian_prompt: Option<String> = match db.conn().query_row(
            "SELECT value FROM settings WHERE key = 'discordian_prompt'",
            [],
            |row| row.get(0),
        ) {
            Ok(prompt) => Some(prompt),
            Err(rusqlite::Error::QueryReturnedNoRows) => None, // Expected when not set
            Err(e) => {
                warn!("Failed to load custom discordian prompt: {}", e);
                None
            }
        };

        BatchContext {
            bias_weights,
            corpus_stats,
            discordian_prompt,
        }
    };

    let (articles, num_ctx): (Vec<BatchArticle>, u32) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let num_ctx = get_num_ctx_setting(&db);

        let query = match limit {
            Some(n) => format!(
                r#"SELECT id, title, content_full,
                          DATE(COALESCE(published_at, fetched_at)) as article_date,
                          COALESCE(analysis_attempts, 0) as attempts,
                          analysis_error
                   FROM fnords
                   WHERE processed_at IS NULL
                   AND content_full IS NOT NULL AND LENGTH(content_full) >= 100
                   AND (analysis_hopeless IS NULL OR analysis_hopeless = FALSE)
                   ORDER BY published_at DESC
                   LIMIT {}"#,
                n
            ),
            None => r#"SELECT id, title, content_full,
                          DATE(COALESCE(published_at, fetched_at)) as article_date,
                          COALESCE(analysis_attempts, 0) as attempts,
                          analysis_error
                   FROM fnords
                   WHERE processed_at IS NULL
                   AND content_full IS NOT NULL AND LENGTH(content_full) >= 100
                   AND (analysis_hopeless IS NULL OR analysis_hopeless = FALSE)
                   ORDER BY published_at DESC"#
                .to_string(),
        };

        let mut stmt = db.conn().prepare(&query).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(BatchArticle {
                    fnord_id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    article_date: row.get(3)?,
                    attempts: row.get(4)?,
                    previous_error: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;

        (rows.filter_map(|r| r.ok()).collect(), num_ctx)
    };

    let total = articles.len() as i64;

    if total == 0 {
        info!("[LLM] Batch: No articles to process");
        return Ok(BatchResult {
            processed: 0,
            succeeded: 0,
            failed: 0,
        });
    }

    state.batch_cancel.store(false, Ordering::SeqCst);
    state.batch_running.store(true, Ordering::SeqCst);

    info!(
        "[LLM] Batch starting: {} articles with {} parallel workers (model: {}, num_ctx: {})",
        total, concurrency, model, num_ctx
    );
    let batch_start_time = Instant::now();

    let _ = window.emit(
        "batch-progress",
        BatchProgress {
            current: 0,
            total,
            fnord_id: 0,
            title: format!("Starting batch ({} parallel)...", concurrency),
            success: true,
            error: None,
        },
    );

    let stream = stream::iter(articles.into_iter().enumerate());
    let batch_context = Arc::new(batch_context);

    let results = stream
        .map(|(idx, article)| {
            let title = article.title.clone();
            let fnord_id = article.fnord_id;
            let model = model.clone();
            let locale = locale.clone();
            let state = state.clone();
            let window = window.clone();
            let batch_context = batch_context.clone();

            async move {
                if state.batch_cancel.load(Ordering::SeqCst) {
                    return (idx, title, fnord_id, false, Some("Cancelled".to_string()));
                }

                let (ctx_multiplier, adjusted_num_ctx) = match article.attempts {
                    0 => (1.0, num_ctx),
                    1 => (1.5, ((num_ctx as f64) * 1.5) as u32),
                    _ => (2.0, num_ctx * 2),
                };

                if article.attempts > 0 {
                    info!(
                        "Retry {}/3 for article {}: using {}x context (num_ctx={})",
                        article.attempts + 1,
                        fnord_id,
                        ctx_multiplier,
                        adjusted_num_ctx
                    );
                }

                let client = OllamaClient::with_context(None, adjusted_num_ctx);

                let (success, error) =
                    process_single_article(&client, &state, &model, &locale, article, &batch_context)
                        .await;

                let _ = window.emit(
                    "batch-progress",
                    BatchProgress {
                        current: (idx + 1) as i64,
                        total,
                        fnord_id,
                        title: title.clone(),
                        success,
                        error: error.clone(),
                    },
                );

                (idx, title, fnord_id, success, error)
            }
        })
        .buffer_unordered(concurrency)
        .collect::<Vec<_>>()
        .await;

    let mut succeeded = 0;
    let mut failed = 0;
    let mut successful_fnord_ids: Vec<i64> = Vec::new();
    for (_, _, fnord_id, success, _) in &results {
        if *success {
            succeeded += 1;
            successful_fnord_ids.push(*fnord_id);
        } else {
            failed += 1;
        }
    }

    let batch_duration = batch_start_time.elapsed();
    let avg_time_per_article = if succeeded > 0 {
        batch_duration.as_secs_f64() / succeeded as f64
    } else {
        0.0
    };

    info!(
        "[LLM] Batch LLM analysis complete: {}/{} succeeded, {} failed in {:.1}s ({:.2}s avg/article)",
        succeeded, total, failed, batch_duration.as_secs_f64(), avg_time_per_article
    );

    // Process embedding queue after batch
    if succeeded > 0 && !state.batch_cancel.load(Ordering::SeqCst) {
        let queue_size = {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            db.conn()
                .query_row("SELECT COUNT(*) FROM embedding_queue", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap_or(0)
        };

        if queue_size > 0 {
            let _ = window.emit(
                "embedding-progress",
                embedding_worker::EmbeddingProgress {
                    queue_size,
                    total: queue_size,
                    processed: 0,
                    failed: 0,
                    is_processing: true,
                },
            );

            let _ = embedding_worker::process_embedding_queue(
                state.db.clone(),
                Some(window.app_handle()),
                queue_size,
                Some(queue_size),
            )
            .await;

            let _ = window.emit(
                "embedding-progress",
                embedding_worker::EmbeddingProgress {
                    queue_size: 0,
                    total: queue_size,
                    processed: queue_size,
                    failed: 0,
                    is_processing: false,
                },
            );
        }

        // Generate article embeddings AFTER all LLM analysis is complete
        // This avoids constant model switching between LLM and embedding model
        if !successful_fnord_ids.is_empty() && !state.batch_cancel.load(Ordering::SeqCst) {
            info!(
                "[Embedding] Starting article embeddings for {} articles...",
                successful_fnord_ids.len()
            );
            let embedding_start = Instant::now();

            let _ = window.emit(
                "batch-progress",
                BatchProgress {
                    current: total,
                    total,
                    fnord_id: 0,
                    title: format!("Generating {} article embeddings...", successful_fnord_ids.len()),
                    success: true,
                    error: None,
                },
            );

            // Load articles for embedding generation
            let articles_for_embedding: Vec<(i64, String, String)> = {
                let db = state.db.lock().map_err(|e| e.to_string())?;
                // Query each article individually to avoid lifetime issues
                let mut result = Vec::new();
                for &fnord_id in &successful_fnord_ids {
                    if let Ok((title, content)) = db.conn().query_row(
                        "SELECT title, COALESCE(content_full, '') FROM fnords WHERE id = ? AND embedding IS NULL",
                        [fnord_id],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                    ) {
                        result.push((fnord_id, title, content));
                    }
                }
                result
            };

            // Generate embeddings in parallel (uses embedding model, not LLM)
            // Use the same concurrency as LLM processing for consistency
            let embedding_client = Arc::new(OllamaClient::new(None));
            let embedding_concurrency = concurrency.max(4); // At least 4 for embeddings (faster than LLM)

            // Check for cancellation before starting parallel embedding generation
            let cancelled = state.batch_cancel.load(Ordering::SeqCst);
            if cancelled {
                info!("Embedding generation cancelled");
            } else {
                let embedding_results = stream::iter(articles_for_embedding)
                    .map(|(fnord_id, title, content)| {
                        let client = Arc::clone(&embedding_client);
                        let db = state.db.clone();
                        async move {
                            let result = generate_and_save_article_embedding(
                                &client,
                                &db,
                                fnord_id,
                                &title,
                                &content,
                            )
                            .await;
                            (fnord_id, result)
                        }
                    })
                    .buffer_unordered(embedding_concurrency)
                    .collect::<Vec<_>>()
                    .await;

                // Log any failed embeddings
                let mut embed_succeeded = 0;
                for (fnord_id, result) in &embedding_results {
                    match result {
                        Ok(_) => embed_succeeded += 1,
                        Err(e) => {
                            debug!("[Embedding] Failed for article {}: {}", fnord_id, e);
                        }
                    }
                }

                let embed_duration = embedding_start.elapsed();
                info!(
                    "[Embedding] Completed: {}/{} succeeded in {:.1}s",
                    embed_succeeded,
                    embedding_results.len(),
                    embed_duration.as_secs_f64()
                );
            }
        }
    }

    state.batch_running.store(false, Ordering::SeqCst);

    // Final batch summary
    let total_duration = batch_start_time.elapsed();
    info!(
        "[LLM] Batch COMPLETE: {} articles processed ({} succeeded, {} failed) in {:.1}s total",
        total, succeeded, failed, total_duration.as_secs_f64()
    );

    // Trigger WAL checkpoint if we processed a significant number of articles
    if succeeded >= 100 {
        if let Ok(db) = state.db.lock() {
            match db.conn().execute("PRAGMA wal_checkpoint(PASSIVE)", []) {
                Ok(_) => {
                    info!(
                        "WAL checkpoint triggered after processing {} articles",
                        succeeded
                    );
                }
                Err(e) => {
                    warn!("WAL checkpoint failed after batch processing: {}", e);
                }
            }
        }
    }

    Ok(BatchResult {
        processed: total,
        succeeded,
        failed,
    })
}

/// Cancel the running batch
#[tauri::command]
pub fn cancel_batch(state: State<AppState>) -> Result<(), String> {
    state.batch_cancel.store(true, Ordering::SeqCst);
    Ok(())
}

/// Transfer keywords from representative to cluster members
///
/// Part of the dormant cluster-based batch processing feature.
/// See [`ClusterBatchConfig`] for full documentation on status and trade-offs.
#[allow(dead_code)]
async fn transfer_keywords_to_cluster_members(
    state: &AppState,
    cluster: &crate::keywords::ArticleCluster,
    keywords: &[String],
    categories: &[i64],
) -> Result<usize, String> {
    let mut transferred = 0;

    for &article_id in &cluster.article_ids {
        // Skip the representative (already processed)
        if article_id == cluster.representative_id {
            continue;
        }

        // Acquire lock for this article only, use transaction for atomicity
        {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let conn = db.conn();

            conn.execute("BEGIN TRANSACTION", [])
                .map_err(|e| e.to_string())?;

            let result: Result<(), String> = {
                // Save keywords for this article
                for keyword in keywords {
                    let _ = conn.execute(
                        r#"INSERT OR IGNORE INTO immanentize (name) VALUES (?1)"#,
                        [keyword],
                    );

                    let keyword_id: i64 = conn
                        .query_row(
                            "SELECT id FROM immanentize WHERE name = ?1",
                            [keyword],
                            |row| row.get(0),
                        )
                        .unwrap_or(0);

                    if keyword_id > 0 {
                        let _ = conn.execute(
                            r#"INSERT OR REPLACE INTO fnord_immanentize
                               (fnord_id, immanentize_id, source, confidence)
                               VALUES (?1, ?2, 'cluster', 0.85)"#,
                            (article_id, keyword_id),
                        );
                    }
                }

                // Save categories for this article
                for &category_id in categories {
                    let _ = conn.execute(
                        r#"INSERT OR REPLACE INTO fnord_sephiroth
                           (fnord_id, sephiroth_id, source, confidence)
                           VALUES (?1, ?2, 'cluster', 0.85)"#,
                        (article_id, category_id),
                    );
                }

                // Mark as processed (via cluster)
                let _ = conn.execute(
                    r#"UPDATE fnords SET
                       processed_at = CURRENT_TIMESTAMP,
                       analysis_attempts = 0,
                       analysis_error = 'Processed via cluster transfer'
                    WHERE id = ?1"#,
                    [article_id],
                );

                Ok(())
            };

            match result {
                Ok(()) => {
                    conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
                    transferred += 1;
                }
                Err(e) => {
                    let _ = conn.execute("ROLLBACK", []);
                    return Err(e);
                }
            }
        } // Lock released here

        // Yield for other tasks after processing each cluster member
        tokio::task::yield_now().await;
    }

    Ok(transferred)
}

/// Process batch with clustering optimization
///
/// This command groups similar articles together based on embeddings,
/// runs LLM analysis only on cluster representatives, and transfers
/// keywords to all cluster members.
///
/// Cluster-based batch processing - implemented but intentionally dormant.
///
/// **Status:** Implemented but not registered in invoke_handler (lib.rs).
/// **Trade-off:** Speed (~30-50% fewer LLM calls) vs. accuracy (cluster transfers use confidence=0.85).
/// **To enable:** Register in invoke_handler (lib.rs), add frontend UI, write integration tests.
#[tauri::command]
#[allow(dead_code)]
pub async fn process_batch_clustered(
    window: Window,
    state: State<'_, AppState>,
    model: String,
    limit: Option<i64>,
    use_clustering: Option<bool>,
) -> Result<ClusterBatchResult, String> {
    let locale = get_locale_from_db(&state);
    let concurrency = get_ai_concurrency(&state);
    let should_cluster = use_clustering.unwrap_or(true);

    info!(
        "Starting clustered batch processing with concurrency: {}, clustering: {}",
        concurrency, should_cluster
    );

    // Load batch context
    let batch_context = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let bias_weights = BiasWeights::load_from_db(db.conn()).unwrap_or_default();
        let corpus_stats = match CorpusStats::load_from_db(db.conn()) {
            Ok(stats) => Some(stats),
            Err(e) => {
                warn!("Failed to load corpus stats for clustered batch: {}", e);
                None
            }
        };

        // Load custom discordian prompt from settings (if set)
        let discordian_prompt: Option<String> = match db.conn().query_row(
            "SELECT value FROM settings WHERE key = 'discordian_prompt'",
            [],
            |row| row.get(0),
        ) {
            Ok(prompt) => Some(prompt),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => {
                warn!("Failed to load custom discordian prompt for clustered batch: {}", e);
                None
            }
        };

        BatchContext {
            bias_weights,
            corpus_stats,
            discordian_prompt,
        }
    };

    // Load articles with embeddings
    let articles_with_embeddings = load_articles_for_clustering(&state, limit)?;
    let total_articles = articles_with_embeddings.len();

    if total_articles == 0 {
        return Ok(ClusterBatchResult {
            processed: 0,
            succeeded: 0,
            failed: 0,
            clusters_found: 0,
            llm_calls_saved: 0,
            savings_percentage: 0.0,
        });
    }

    state.batch_cancel.store(false, Ordering::SeqCst);
    state.batch_running.store(true, Ordering::SeqCst);

    // Apply clustering if enabled
    let cluster_config = ClusterBatchConfig::default();
    let (clustering_result, article_map) = if should_cluster {
        apply_clustering(articles_with_embeddings, &cluster_config)
    } else {
        let article_map: HashMap<i64, BatchArticle> = articles_with_embeddings
            .iter()
            .map(|(a, _)| (a.fnord_id, a.clone()))
            .collect();
        let result = ClusteringResult {
            clusters: vec![],
            unclustered_ids: articles_with_embeddings.iter().map(|(a, _)| a.fnord_id).collect(),
            total_articles,
            representatives_count: total_articles,
        };
        (result, article_map)
    };

    let (saved, _total, savings_pct) = calculate_savings(&clustering_result);
    let representatives = get_representatives(&clustering_result);
    let total_to_process = representatives.len() as i64;

    info!(
        "Clustering: {} clusters found, {} representatives, {} LLM calls saved ({:.1}%)",
        clustering_result.clusters.len(),
        representatives.len(),
        saved,
        savings_pct
    );

    let _ = window.emit(
        "batch-progress",
        BatchProgress {
            current: 0,
            total: total_to_process,
            fnord_id: 0,
            title: format!(
                "Starting clustered batch: {} articles in {} clusters, {} to process...",
                total_articles,
                clustering_result.clusters.len(),
                representatives.len()
            ),
            success: true,
            error: None,
        },
    );

    // Get num_ctx setting
    let num_ctx = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        get_num_ctx_setting(&db)
    };

    // Process only representatives
    let articles_to_process: Vec<BatchArticle> = representatives
        .iter()
        .filter_map(|&id| article_map.get(&id).cloned())
        .collect();

    // Wrap in Arc first, then build cluster lookup
    let clustering_result = Arc::new(clustering_result);
    let batch_context = Arc::new(batch_context);

    // Build cluster lookup: representative_id -> cluster index
    let cluster_lookup: HashMap<i64, usize> = clustering_result
        .clusters
        .iter()
        .enumerate()
        .map(|(idx, c)| (c.representative_id, idx))
        .collect();
    let cluster_lookup = Arc::new(cluster_lookup);

    let stream = stream::iter(articles_to_process.into_iter().enumerate());

    let results = stream
        .map(|(idx, article)| {
            let title = article.title.clone();
            let fnord_id = article.fnord_id;
            let model = model.clone();
            let locale = locale.clone();
            let state = state.clone();
            let window = window.clone();
            let batch_context = batch_context.clone();
            let cluster_lookup = cluster_lookup.clone();
            let clustering_result = clustering_result.clone();

            async move {
                if state.batch_cancel.load(Ordering::SeqCst) {
                    return (idx, title, fnord_id, false, Some("Cancelled".to_string()), 0usize);
                }

                let (ctx_multiplier, adjusted_num_ctx) = match article.attempts {
                    0 => (1.0, num_ctx),
                    1 => (1.5, ((num_ctx as f64) * 1.5) as u32),
                    _ => (2.0, num_ctx * 2),
                };

                if article.attempts > 0 {
                    info!(
                        "Retry {}/3 for article {}: using {}x context (num_ctx={})",
                        article.attempts + 1,
                        fnord_id,
                        ctx_multiplier,
                        adjusted_num_ctx
                    );
                }

                let client = OllamaClient::with_context(None, adjusted_num_ctx);

                let (success, error) =
                    process_single_article(&client, &state, &model, &locale, article, &batch_context)
                        .await;

                // If this is a cluster representative, transfer keywords to members
                let mut transferred = 0;
                if success {
                    if let Some(&cluster_idx) = cluster_lookup.get(&fnord_id) {
                        if let Some(cluster) = clustering_result.clusters.get(cluster_idx) {
                            // Load keywords and categories for this article
                            let (keywords, categories) = {
                                let db = match state.db.lock() {
                                    Ok(db) => db,
                                    Err(e) => {
                                        warn!("Failed to acquire DB lock for cluster transfer: {}", e);
                                        return (idx, title, fnord_id, success, error, 0);
                                    }
                                };

                                let keywords: Vec<String> = match db.conn().prepare(
                                    r#"SELECT i.name FROM immanentize i
                                       JOIN fnord_immanentize fi ON fi.immanentize_id = i.id
                                       WHERE fi.fnord_id = ?1"#
                                ) {
                                    Ok(mut stmt) => {
                                        match stmt.query_map([fnord_id], |row| row.get(0)) {
                                            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                                            Err(e) => {
                                                warn!("Failed to query keywords for article {}: {}", fnord_id, e);
                                                vec![]
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to prepare keyword query for article {}: {}", fnord_id, e);
                                        vec![]
                                    }
                                };

                                let categories: Vec<i64> = match db.conn().prepare(
                                    r#"SELECT sephiroth_id FROM fnord_sephiroth
                                       WHERE fnord_id = ?1"#
                                ) {
                                    Ok(mut stmt) => {
                                        match stmt.query_map([fnord_id], |row| row.get(0)) {
                                            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                                            Err(e) => {
                                                warn!("Failed to query categories for article {}: {}", fnord_id, e);
                                                vec![]
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to prepare category query for article {}: {}", fnord_id, e);
                                        vec![]
                                    }
                                };

                                (keywords, categories)
                            };

                            if !keywords.is_empty() || !categories.is_empty() {
                                match transfer_keywords_to_cluster_members(&state, cluster, &keywords, &categories).await {
                                    Ok(n) => {
                                        transferred = n;
                                        info!(
                                            "Transferred {} keywords to {} cluster members",
                                            keywords.len(),
                                            transferred
                                        );
                                    }
                                    Err(e) => {
                                        warn!("Failed to transfer keywords to cluster: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }

                let _ = window.emit(
                    "batch-progress",
                    BatchProgress {
                        current: (idx + 1) as i64,
                        total: total_to_process,
                        fnord_id,
                        title: title.clone(),
                        success,
                        error: error.clone(),
                    },
                );

                (idx, title, fnord_id, success, error, transferred)
            }
        })
        .buffer_unordered(concurrency)
        .collect::<Vec<_>>()
        .await;

    let mut succeeded = 0;
    let mut failed = 0;
    let mut total_transferred = 0;
    for (_, _, _, success, _, transferred) in &results {
        if *success {
            succeeded += 1;
            total_transferred += transferred;
        } else {
            failed += 1;
        }
    }

    // Process embedding queue after batch
    if succeeded > 0 && !state.batch_cancel.load(Ordering::SeqCst) {
        let queue_size = {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            db.conn()
                .query_row("SELECT COUNT(*) FROM embedding_queue", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap_or(0)
        };

        if queue_size > 0 {
            let _ = window.emit(
                "embedding-progress",
                embedding_worker::EmbeddingProgress {
                    queue_size,
                    total: queue_size,
                    processed: 0,
                    failed: 0,
                    is_processing: true,
                },
            );

            let _ = embedding_worker::process_embedding_queue(
                state.db.clone(),
                Some(window.app_handle()),
                queue_size,
                Some(queue_size),
            )
            .await;

            let _ = window.emit(
                "embedding-progress",
                embedding_worker::EmbeddingProgress {
                    queue_size: 0,
                    total: queue_size,
                    processed: queue_size,
                    failed: 0,
                    is_processing: false,
                },
            );
        }
    }

    state.batch_running.store(false, Ordering::SeqCst);

    Ok(ClusterBatchResult {
        processed: total_articles as i64,
        succeeded: succeeded + total_transferred as i64,
        failed,
        clusters_found: clustering_result.clusters.len(),
        llm_calls_saved: saved,
        savings_percentage: savings_pct,
    })
}
