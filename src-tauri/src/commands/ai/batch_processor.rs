//! Batch processing commands for article analysis

use crate::ai_provider::AiTextProvider;
use crate::embedding_worker;
use crate::extract_keywords;
#[cfg(feature = "clustering")]
use crate::keywords::{
    calculate_savings, cluster_articles, get_representatives, ArticleForClustering, ClusterConfig,
    ClusteringResult,
};
use crate::text_analysis::{
    record_correction, BiasWeights, CategoryMatcher, CorpusStats, CorrectionRecord, CorrectionType,
    TfIdfExtractor,
};
use crate::AppState;
use futures::stream::{self, StreamExt};
use log::{debug, info, warn};
use rusqlite::Connection;
#[cfg(feature = "clustering")]
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager, State, Window};

/// Safely truncate a string to a maximum byte length, respecting UTF-8 char boundaries.
/// Returns a slice that ends at a valid char boundary.
fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // Find the largest valid char boundary <= max_bytes
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

use super::data_persistence::{
    generate_and_save_article_embedding, recalculate_keyword_weights,
    save_article_categories_with_source, save_article_keywords_with_source,
};
use super::helpers::{
    check_analysis_cache, compute_content_hash, determine_keyword_sources,
    discordian_analysis_via_provider, get_embedding_provider_config, get_locale_from_db,
    get_num_ctx_setting, log_generation_cost, merge_categories_stat_primary, merge_keywords,
    store_analysis_cache, TokenUsage,
};
#[cfg(feature = "clustering")]
use super::helpers::{create_text_provider, get_provider_config};
// CorrectionType is already imported from text_analysis
use super::types::{
    BatchArticle, BatchProgress, BatchResult, FailedCount, HopelessCount, UnprocessedCount,
};
use crate::categories::classify_by_keywords;
use crate::commands::ai::helpers::{derive_categories_from_keywords, determine_category_sources};
use crate::ollama::{DiscordianAnalysis, DiscordianAnalysisWithRejections};

/// Type alias for articles with optional embeddings (for clustering)
#[cfg(feature = "clustering")]
type ArticleWithEmbedding = (BatchArticle, Option<Vec<f32>>);

/// Shared context for batch processing - loaded once before processing starts
struct BatchContext {
    bias_weights: BiasWeights,
    corpus_stats: Option<CorpusStats>,
    /// Custom discordian prompt (None = use default)
    discordian_prompt: Option<String>,
}

impl BatchContext {
    pub fn new(conn: &Connection) -> Result<Self, rusqlite::Error> {
        let bias_weights = BiasWeights::load_from_db(conn).unwrap_or_default();
        let corpus_stats = match CorpusStats::load_from_db(conn) {
            Ok(stats) => Some(stats),
            Err(e) => {
                warn!("Failed to load corpus stats, using fallback TF-IDF: {}", e);
                None
            }
        };

        // Load custom discordian prompt from settings (if set)
        let discordian_prompt: Option<String> = match conn.query_row(
            "SELECT value FROM settings WHERE key = 'discordian_prompt'",
            [],
            |row: &rusqlite::Row| row.get(0),
        ) {
            Ok(prompt) => Some(prompt),
            Err(rusqlite::Error::QueryReturnedNoRows) => None, // Expected when not set
            Err(e) => {
                warn!("Failed to load custom discordian prompt: {}", e);
                None
            }
        };

        Ok(Self {
            bias_weights,
            corpus_stats,
            discordian_prompt,
        })
    }
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
/// **To enable:** Enable `clustering` feature flag in Cargo.toml, register in invoke_handler (lib.rs), add frontend UI, write integration tests.
#[cfg(feature = "clustering")]
#[derive(Debug, Clone)]
pub struct ClusterBatchConfig {
    /// Whether to use clustering optimization
    pub use_clustering: bool,
    /// Minimum articles to enable clustering (below this, process all)
    pub min_articles_for_clustering: usize,
    /// Clustering configuration
    pub cluster_config: ClusterConfig,
}

#[cfg(feature = "clustering")]
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
#[cfg(feature = "clustering")]
#[derive(Debug, Clone, serde::Serialize)]
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
#[cfg(feature = "clustering")]
fn load_articles_for_clustering(
    state: &AppState,
    limit: Option<i64>,
) -> Result<Vec<ArticleWithEmbedding>, String> {
    let db = state.db_conn()?;

    let query = r#"SELECT f.id, f.title, f.content_full,
                      DATE(COALESCE(f.published_at, f.fetched_at)) as article_date,
                      COALESCE(f.analysis_attempts, 0) as attempts,
                      f.analysis_error,
                      f.embedding
               FROM fnords f
               WHERE f.processed_at IS NULL
               AND f.content_full IS NOT NULL AND LENGTH(f.content_full) >= 100
               AND (f.analysis_hopeless IS NULL OR f.analysis_hopeless = FALSE)
               ORDER BY f.published_at DESC
               LIMIT ?1"#;

    // SQLite: LIMIT -1 means unlimited
    let limit_param = limit.unwrap_or(-1);

    let mut stmt = to_cmd_err!(db.conn().prepare(query));
    let rows = stmt
        .query_map([limit_param], |row| {
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
#[cfg(feature = "clustering")]
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
    clustering_result
        .unclustered_ids
        .extend(non_clusterable_ids);
    clustering_result.representatives_count =
        clustering_result.clusters.len() + clustering_result.unclustered_ids.len();

    (clustering_result, article_map)
}

/// Get count of hopeless articles (failed 3+ times)
#[tauri::command]
pub fn get_hopeless_count(state: State<AppState>) -> Result<HopelessCount, String> {
    let db = state.db_conn()?;

    let count: i64 = to_cmd_err!(db.conn().query_row(
        "SELECT COUNT(*) FROM fnords WHERE analysis_hopeless = TRUE",
        [],
        |row| row.get(0),
    ));

    Ok(HopelessCount { count })
}

/// Get count of failed articles (attempted but not processed successfully, not hopeless)
#[tauri::command]
pub fn get_failed_count(state: State<AppState>) -> Result<FailedCount, String> {
    let db = state.db_conn()?;

    let count: i64 = to_cmd_err!(db.conn().query_row(
        r#"SELECT COUNT(*) FROM fnords
               WHERE analysis_attempts > 0
               AND processed_at IS NULL
               AND (analysis_hopeless IS NULL OR analysis_hopeless = FALSE)"#,
        [],
        |row| row.get(0),
    ));

    Ok(FailedCount { count })
}

/// Get count of unprocessed articles
#[tauri::command]
pub fn get_unprocessed_count(state: State<AppState>) -> Result<UnprocessedCount, String> {
    let db = state.db_conn()?;

    let total: i64 = to_cmd_err!(db.conn().query_row(
        r#"SELECT COUNT(*) FROM fnords
               WHERE processed_at IS NULL
               AND (analysis_hopeless IS NULL OR analysis_hopeless = FALSE)"#,
        [],
        |row| row.get(0),
    ));

    let with_content: i64 = to_cmd_err!(db.conn().query_row(
        r#"SELECT COUNT(*) FROM fnords
               WHERE processed_at IS NULL
               AND content_full IS NOT NULL AND LENGTH(content_full) >= 100
               AND (analysis_hopeless IS NULL OR analysis_hopeless = FALSE)"#,
        [],
        |row| row.get(0),
    ));

    Ok(UnprocessedCount {
        total,
        with_content,
    })
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
    let db = state.db_conn()?;

    let mut stmt = to_cmd_err!(db.conn().prepare(
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
    ));

    let articles: Vec<super::types::AnalysisStatusArticle> =
        to_cmd_err!(stmt.query_map([limit, offset], |row| {
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
        }))
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
    let db = state.db_conn()?;

    let mut stmt = to_cmd_err!(db.conn().prepare(
        r#"SELECT
                f.id, f.title, f.pentacle_id, p.title as pentacle_title,
                f.summary, f.published_at, f.status, f.analysis_attempts, f.analysis_error
            FROM fnords f
            LEFT JOIN pentacles p ON p.id = f.pentacle_id
            WHERE f.analysis_hopeless = TRUE
            ORDER BY f.analysis_attempts DESC, f.published_at DESC
            LIMIT ? OFFSET ?"#,
    ));

    let articles: Vec<super::types::AnalysisStatusArticle> =
        to_cmd_err!(stmt.query_map([limit, offset], |row| {
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
        }))
        .filter_map(|r| r.ok())
        .collect();

    Ok(articles)
}

/// Process a single article with full statistical + LLM pipeline
async fn process_single_article(
    provider: &dyn AiTextProvider,
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
            let adjusted_score = batch_context
                .bias_weights
                .apply_to_keyword(&kc.term, kc.score);
            (kc.term, adjusted_score)
        })
        .collect();

    keyword_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    keyword_candidates.truncate(15);
    let stat_keywords: Vec<String> = keyword_candidates
        .into_iter()
        .map(|(term, _)| term)
        .collect();

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
        debug!(
            "[LLM] Cache hit for article {}: \"{}\"",
            fnord_id,
            truncate_str(&title, 50)
        );
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
        let llm_start = Instant::now();

        info!(
            "[LLM] Starting analysis for \"{}\" (ID: {})",
            truncate_str(&title, 60),
            fnord_id
        );

        let analysis_result: Result<
            (DiscordianAnalysisWithRejections, TokenUsage),
            crate::ai_provider::AiProviderError,
        > = discordian_analysis_via_provider(
            provider,
            model,
            &title,
            &content,
            locale,
            &stat_keywords,
            &stat_categories,
            batch_context.discordian_prompt.as_deref(),
        )
        .await;

        let duration = llm_start.elapsed();

        match analysis_result {
            Ok((analysis_with_rejections, usage)) => {
                let analysis_with_rejections: crate::ollama::DiscordianAnalysisWithRejections =
                    analysis_with_rejections;
                info!(
                    "[LLM] Completed \"{}\" (ID: {}) in {:.2}s",
                    truncate_str(&title, 50),
                    fnord_id,
                    duration.as_secs_f64()
                );

                // === LOG COST + LEARN FROM LLM REJECTIONS ===
                {
                    let db = match state.db.lock() {
                        Ok(db) => db,
                        Err(e) => return (false, Some(format!("Database lock failed: {}", e))),
                    };

                    // Log cost with same brief DB lock
                    log_generation_cost(db.conn(), provider.provider_name(), model, &usage);

                    let rejected_kws: &Vec<String> = &analysis_with_rejections.rejected_keywords;
                    for rejected_kw in rejected_kws {
                        let _ = record_correction(
                            db.conn(),
                            &CorrectionRecord {
                                fnord_id,
                                correction_type: CorrectionType::KeywordRemoved,
                                old_value: {
                                    let s: String = rejected_kw.to_string();
                                    let opt: Option<String> = Some(s);
                                    opt
                                },
                                new_value: None,
                                matching_terms: vec![],
                                category_id: None,
                            },
                        );
                    }

                    let rejected_cats: &Vec<String> = &analysis_with_rejections.rejected_categories;
                    for rejected_cat in rejected_cats {
                        let cat_id: Option<i64> = db
                            .conn()
                            .query_row(
                                "SELECT id FROM sephiroth WHERE LOWER(name) = LOWER(?1)",
                                rusqlite::params![rejected_cat],
                                |row: &rusqlite::Row| row.get(0),
                            )
                            .ok();

                        if let Some(cat_id) = cat_id {
                            let matching_terms: Vec<String> =
                                stat_keywords.iter().take(5).cloned().collect();
                            let _ = record_correction(
                                db.conn(),
                                &CorrectionRecord {
                                    fnord_id,
                                    correction_type: CorrectionType::CategoryRemoved,
                                    old_value: {
                                        let s: String = rejected_cat.clone();
                                        let opt: Option<String> = Some(s);
                                        opt
                                    },
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

                let llm_analysis_converted: DiscordianAnalysis = analysis_with_rejections.into();
                (llm_analysis_converted, false)
            }
            Err(e) => {
                let e: crate::ai_provider::AiProviderError = e;
                warn!(
                    "[LLM] FAILED \"{}\" (ID: {}) after {:.2}s: {}",
                    truncate_str(&title, 50),
                    fnord_id,
                    duration.as_secs_f64(),
                    e
                );

                let error_msg = e.to_string();
                let is_rate_limit = matches!(e, crate::ai_provider::AiProviderError::RateLimited);
                let is_json_error = matches!(
                    e,
                    crate::ai_provider::AiProviderError::JsonParseError { .. }
                );

                {
                    let db = match state.db.lock() {
                        Ok(db) => db,
                        Err(e) => return (false, Some(format!("Database lock failed: {}", e))),
                    };

                    if is_rate_limit {
                        // Rate limits are NOT article-specific failures.
                        // Store error message but do NOT increment attempts.
                        let _ = db.conn().execute(
                            r#"UPDATE fnords SET
                                analysis_error = ?1
                            WHERE id = ?2"#,
                            rusqlite::params![&error_msg, fnord_id],
                        );
                    } else {
                        // Actual article-specific failure: increment attempts
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
            (
                &analysis.summary,
                analysis.political_bias,
                analysis.sachlichkeit,
                fnord_id,
            ),
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
        let network_categories: Vec<(String, f64)> =
            derive_categories_from_keywords(db.conn(), &keyword_names, 0.15, 2);

        // Merge network categories with existing merged categories
        // Network categories are treated as 'statistical' source
        let mut final_categories = merged_categories.clone();
        let seen: std::collections::HashSet<String> = final_categories
            .iter()
            .map(|c: &String| c.to_lowercase())
            .collect();

        // Also extend stat_categories with network categories for source determination
        let mut stat_cats_extended = stat_categories.clone();
        for (cat_name, weight) in &network_categories {
            let cat_lower: String = (*cat_name).to_lowercase();
            if !seen.contains(&cat_lower) {
                final_categories.push(cat_name.clone());
            }
            if !stat_cats_extended
                .iter()
                .any(|(n, _)| n.to_lowercase() == cat_lower)
            {
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
    // Initialize Atomic counters
    let processed_count = Arc::new(AtomicUsize::new(0));
    let succeeded_count = Arc::new(AtomicUsize::new(0));
    let failed_count = Arc::new(AtomicUsize::new(0));

    // 1. Fetch unprocessed articles

    let articles = {
        let conn = state.db_conn().map_err(|e| e.to_string())?;

        let mut stmt = conn
            .conn()
            .prepare(
                r#"
            SELECT
                f.id,
                f.title,
                COALESCE(f.content_full, '') as content,
                DATE(COALESCE(f.published_at, f.fetched_at)) as article_date,
                f.analysis_error,
                COALESCE(f.analysis_attempts, 0) as attempts
            FROM fnords f
            WHERE f.processed_at IS NULL
               AND f.content_full IS NOT NULL
               AND LENGTH(f.content_full) >= 100
               AND (f.analysis_hopeless IS NULL OR f.analysis_hopeless = FALSE)
            ORDER BY f.published_at DESC
            LIMIT ?1
            "#,
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([limit.unwrap_or(-1)], |row| {
                Ok(BatchArticle {
                    fnord_id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    article_date: row.get(3)?,
                    previous_error: row.get(4)?,
                    attempts: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut articles = Vec::new();
        for article in rows {
            articles.push(article.map_err(|e| e.to_string())?);
        }
        articles
    };

    let total = articles.len() as i64;
    if total == 0 {
        return Ok(BatchResult {
            processed: 0,
            succeeded: 0,
            failed: 0,
            provider: "".to_string(),
            model: model.clone(),
        });
    }

    info!("[LLM] Found {} articles to process", total);
    let batch_start_time = Instant::now();

    // Reset cancel flag
    state.batch_cancel.store(false, Ordering::SeqCst);
    state.batch_running.store(true, Ordering::SeqCst);

    // Global rate limit pause mechanism:
    // When any task hits a rate limit, all tasks pause before their next request.
    // rate_limit_until stores a UNIX timestamp (seconds) until which to pause.
    let rate_limit_until = Arc::new(AtomicU64::new(0));
    let rate_limit_hit_count = Arc::new(AtomicUsize::new(0));

    // Get provider and configure model
    let (provider_config, effective_model) = {
        let db = state.db_conn().map_err(|e| e.to_string())?;
        let mut config = super::helpers::get_provider_config(&db, Some(&state.proxy_manager));

        // Only override Ollama model from frontend; OpenAI uses its configured model
        if matches!(
            config.provider_type,
            crate::ai_provider::ProviderType::Ollama
        ) {
            config.ollama_model = model.clone();
        }

        let config_model = match config.provider_type {
            crate::ai_provider::ProviderType::Ollama => config.ollama_model.clone(),
            crate::ai_provider::ProviderType::OpenAiCompatible => config.openai_model.clone(),
        };
        let effective = crate::ai_provider::resolve_effective_model(
            &format!("{:?}", config.provider_type),
            &model,
            &config_model,
        );

        (config, effective)
    };

    // Needed for logic below
    let _num_ctx = provider_config.ollama_num_ctx;
    let provider_name = format!("{:?}", provider_config.provider_type);

    let provider_for_batch: Arc<dyn crate::ai_provider::AiTextProvider> =
        crate::ai_provider::create_provider(&provider_config);
    let provider_config_for_retry = provider_config.clone();
    let locale = get_locale_from_db(&state);

    // Initialize batch context (keyword cache)
    let batch_context = {
        let conn = state.db_conn().map_err(|e| e.to_string())?;
        // BatchContext is defined in this file, so no crate::... prefix needed if not public in module
        // But if it was crate::commands::ai::analysis::BatchContext, the error said it's missing.
        // It is defined at line 57 in this file.
        BatchContext::new(conn.conn()).map_err(|e: rusqlite::Error| e.to_string())?
    };

    // Determine concurrency
    let suggested = provider_for_batch.suggested_concurrency();

    // Provider determines concurrency: Ollama uses ollama_concurrency setting,
    // OpenAI uses openai_concurrency setting
    let active_concurrency = if matches!(
        provider_config.provider_type,
        crate::ai_provider::ProviderType::OpenAiCompatible
    ) {
        // For OpenAI-compatible: use the separate openai_concurrency setting
        let openai_concurrency_setting: usize = if suggested > 1 {
            let db = state.db_conn().map_err(|e| e.to_string())?;
            super::helpers::get_setting(&db, "openai_concurrency", "20")
                .parse()
                .unwrap_or(20)
        } else {
            1
        };
        openai_concurrency_setting
    } else {
        // For Ollama: suggested already contains ollama_concurrency value
        suggested
    };

    info!(
        "[LLM] Parallel processing enabled: concurrency={} (provider suggested={})",
        active_concurrency, suggested
    );

    let batch_context = Arc::new(batch_context);
    // provider_ref is not needed as we clone the Arc for the stream

    // Create a generated stream of futures
    let results_stream = stream::iter(articles.into_iter().enumerate())
        .map(|(idx, article)| {
            let state = state.clone();
            let batch_cancel = state.batch_cancel.clone();
            let provider = provider_for_batch.clone();
            let window = window.clone();
            let effective_model = effective_model.clone();
            let locale = locale.clone();
            let batch_context = batch_context.clone();
            let provider_config_for_retry = provider_config_for_retry.clone();
            let provider_name = provider_name.clone();

            // Capture atomic counters
            let processed_count = processed_count.clone();
            let succeeded_count = succeeded_count.clone();
            let failed_count = failed_count.clone();

            // Rate limit pause mechanism
            let rate_limit_until = rate_limit_until.clone();
            let rate_limit_hit_count = rate_limit_hit_count.clone();

            async move {
                if batch_cancel.load(Ordering::SeqCst) {
                    return None;
                }

                // Check if we need to wait due to global rate limit pause
                let pause_until = rate_limit_until.load(Ordering::SeqCst);
                if pause_until > 0 {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    if now < pause_until {
                        let wait_secs = pause_until - now;
                        info!(
                            "[LLM] Rate limit pause: waiting {}s before processing article {}",
                            wait_secs, article.fnord_id
                        );
                        tokio::time::sleep(Duration::from_secs(wait_secs)).await;
                    }
                }

                let fnord_id = article.fnord_id;
                let title = article.title.clone();
                let attempts = article.attempts;

                // For retries with adjusted context (ONLY for Ollama)
                // OpenAI-compatible providers use the same provider for retries
                let retry_provider: Option<Arc<dyn AiTextProvider>> = if attempts > 0 {
                    use crate::ai_provider::{ProviderType, create_provider};

                    match provider_config_for_retry.provider_type {
                        ProviderType::Ollama => {
                            // Ollama: adjust num_ctx for retries
                            let db = state.db_conn().ok()?; // Handle DB error gracefully in async block
                            let num_ctx_setting = get_num_ctx_setting(&db); // Re-fetch or pass in? passing in was cleaner but let's re-fetch safely
                            // optimizing: pass num_ctx in closure? No, let's just use the one we computed earlier if we can pass it
                            // Re-calculating context multiplier logic:
                             let (ctx_multiplier, adjusted_num_ctx) = match attempts {
                                1 => (1.5, ((num_ctx_setting as f64) * 1.5) as u32),
                                _ => (2.0, num_ctx_setting * 2),
                            };

                            info!(
                                "Retry {}/3 for article {} (Ollama): using {}x context (num_ctx={})",
                                attempts + 1,
                                fnord_id,
                                ctx_multiplier,
                                adjusted_num_ctx
                            );

                            let mut retry_config = provider_config_for_retry.clone();
                            retry_config.ollama_num_ctx = adjusted_num_ctx;
                            Some(create_provider(&retry_config))
                        }
                        ProviderType::OpenAiCompatible => {
                             info!(
                                "Retry {}/3 for article {} (OpenAI-compatible): using same provider",
                                attempts + 1,
                                fnord_id
                            );
                            None
                        }
                    }
                } else {
                    None
                };

                let provider_ref: &dyn AiTextProvider = match &retry_provider {
                    Some(p) => p.as_ref(),
                    None => provider.as_ref(),
                };let (success, error) =
                    process_single_article(provider_ref, &state, &effective_model, &locale, article, &batch_context)
                        .await;

                // Detect rate limit errors and set global pause
                if let Some(ref err_msg) = error {
                    if err_msg.contains("Rate limit") {
                        let hits = rate_limit_hit_count.fetch_add(1, Ordering::SeqCst) + 1;
                        // Exponential backoff: 30s, 60s, 120s based on how many rate limits we've hit
                        let pause_secs = match hits {
                            1..=3 => 30,
                            4..=6 => 60,
                            _ => 120,
                        };
                        let until = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                            + pause_secs;
                        rate_limit_until.store(until, Ordering::SeqCst);
                        warn!(
                            "[LLM] Rate limit detected (hit #{}) - pausing all tasks for {}s",
                            hits, pause_secs
                        );
                    }
                }

                // Emit progress immediately from the task
                let _ = window.emit(
                    "batch-progress",
                    BatchProgress {
                        current: (idx + 1) as i64,
                        total,
                        fnord_id,
                        title: title.clone(),
                        success,
                        error: error.clone(),
                        provider: provider_name.clone(),
                        model: effective_model.clone(),
                    },
                );

                // Update counters
                processed_count.fetch_add(1, Ordering::Relaxed);
                if success {
                    succeeded_count.fetch_add(1, Ordering::Relaxed);
                } else {
                    failed_count.fetch_add(1, Ordering::Relaxed);
                }

                // Return result for collecting
                Some((fnord_id, success))
            }
        })
        .buffer_unordered(active_concurrency);

    // Drain the stream to execute futures and collect results
    let results: Vec<_> = results_stream.collect().await;

    // Convert atomics for subsequent logic
    let succeeded = succeeded_count.load(Ordering::Relaxed) as i64;
    let failed = failed_count.load(Ordering::Relaxed) as i64;

    // Create embedding provider
    let embedding_provider = {
        let db = state.db_conn().map_err(|e| e.to_string())?;
        let config = get_embedding_provider_config(&db, Some(&state.proxy_manager));
        crate::ai_provider::create_embedding_provider(&config)
    };

    // Collect successful IDs for embeddings
    let successful_fnord_ids: Vec<i64> = results
        .into_iter()
        .flatten()
        .filter_map(|(id, success)| if success { Some(id) } else { None })
        .collect();

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

    // Release batch_running BEFORE embedding generation to avoid model swapping.
    // The LLM model (ministral) can now be unloaded by Ollama, and the background
    // embedding worker will resume using the same embedding model (snowflake) as
    // the article embedding generation below - no model swapping needed.
    state.batch_running.store(false, Ordering::SeqCst);
    info!("[LLM] Batch flag released - embedding worker can resume, embeddings use same model (no swap)");

    // Explicitly unload LLM model to free VRAM for embedding model
    {
        let ollama_url = {
            let db = state.db_conn()?;
            super::helpers::get_setting(&db, "ollama_url", "http://localhost:11434")
        };
        let unload_client = crate::ollama::OllamaClient::new(Some(ollama_url));
        if let Err(e) = unload_client.unload_model(&effective_model).await {
            warn!("[LLM] Failed to unload model: {}", e);
        }
    }

    // Process embeddings (keyword queue + article embeddings)
    // Both use the embedding model (snowflake), so no model swapping occurs.
    if succeeded > 0 && !state.batch_cancel.load(Ordering::SeqCst) {
        let queue_size = {
            let db = state.db_conn()?;
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
                    title: format!(
                        "Generating {} article embeddings...",
                        successful_fnord_ids.len()
                    ),
                    success: true,
                    error: None,
                    provider: provider_name.clone(),
                    model: effective_model.clone(),
                },
            );

            // Load articles for embedding generation
            let articles_for_embedding: Vec<(i64, String, String)> = {
                let db = state.db_conn()?;
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

            // Generate embeddings via batch request (uses embedding model, not LLM)

            // Check for cancellation before starting embedding generation
            let cancelled = state.batch_cancel.load(Ordering::SeqCst);
            if cancelled {
                info!("Embedding generation cancelled");
            } else {
                let mut embed_succeeded = 0;
                let embed_total = articles_for_embedding.len();

                // Prepare batch: collect embedding texts
                let embedding_texts: Vec<String> = articles_for_embedding
                    .iter()
                    .map(|(_id, title, content)| {
                        let content_preview: String = content.chars().take(500).collect();
                        format!("{}\n\n{}", title, content_preview)
                    })
                    .collect();

                // Generate all embeddings in one batch request
                match embedding_provider
                    .generate_embeddings_batch(&embedding_texts)
                    .await
                {
                    Ok(embeddings) => {
                        let db = state.db_conn()?;
                        for (embedding, (fnord_id, _title, _content)) in
                            embeddings.iter().zip(articles_for_embedding.iter())
                        {
                            match crate::commands::ai::data_persistence::save_article_embedding(
                                db.conn(),
                                *fnord_id,
                                embedding,
                            ) {
                                Ok(_) => embed_succeeded += 1,
                                Err(e) => {
                                    debug!(
                                        "[Embedding] Failed to save for article {}: {}",
                                        fnord_id, e
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            "[Embedding] Batch embedding failed, falling back to sequential: {}",
                            e
                        );
                        for (fnord_id, title, content) in &articles_for_embedding {
                            if state.batch_cancel.load(Ordering::SeqCst) {
                                break;
                            }
                            match generate_and_save_article_embedding(
                                embedding_provider.as_ref(),
                                &state.db,
                                *fnord_id,
                                title,
                                content,
                            )
                            .await
                            {
                                Ok(_) => embed_succeeded += 1,
                                Err(e) => {
                                    debug!("[Embedding] Failed for article {}: {}", fnord_id, e);
                                }
                            }
                        }
                    }
                }

                let embed_duration = embedding_start.elapsed();
                info!(
                    "[Embedding] Completed: {}/{} succeeded in {:.1}s",
                    embed_succeeded,
                    embed_total,
                    embed_duration.as_secs_f64()
                );
            }
        }
    }

    // Final batch summary
    let processed_final = processed_count.load(Ordering::Relaxed) as i64;
    let succeeded_final = succeeded_count.load(Ordering::Relaxed) as i64;
    let failed_final = failed_count.load(Ordering::Relaxed) as i64;

    let total_duration = batch_start_time.elapsed();
    info!(
        "[LLM] Batch complete. Processed: {}/{}. Success: {}, Failed: {}. Time: {:.2}s",
        processed_final,
        total,
        succeeded_final,
        failed_final,
        total_duration.as_secs_f64()
    );

    // Trigger WAL checkpoint if many changes were made
    if succeeded_final >= 100 {
        if let Ok(db) = state.db.lock() {
            match db
                .conn()
                .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
                    let busy: i32 = row.get(0)?;
                    let log: i32 = row.get(1)?;
                    let checkpointed: i32 = row.get(2)?;
                    Ok((busy, log, checkpointed))
                }) {
                Ok((busy, log, checkpointed)) => {
                    info!(
                        "WAL checkpoint after processing {} articles: busy={}, log={}, checkpointed={}",
                        succeeded_final, busy, log, checkpointed
                    );
                }
                Err(e) => {
                    warn!("WAL checkpoint failed after batch processing: {}", e);
                }
            }
        }
    }

    Ok(BatchResult {
        processed: processed_final,
        succeeded: succeeded_final,
        failed: failed_final,
        provider: provider_name,
        model,
    })
}

/// Cancel the running batch
#[tauri::command]
pub fn cancel_batch(state: State<AppState>) -> Result<(), String> {
    state.batch_cancel.store(true, Ordering::SeqCst);
    Ok(())
}

/// Reset all failed (non-hopeless) articles so they can be retried from scratch.
///
/// This clears `analysis_attempts` and `analysis_error` for articles that previously
/// failed (e.g. due to provider misconfiguration) but were not yet marked hopeless.
#[tauri::command]
pub fn reset_failed_articles(state: State<AppState>) -> Result<i64, String> {
    let db = state.db_conn()?;

    let affected = to_cmd_err!(db.conn().execute(
        r#"UPDATE fnords SET
                analysis_attempts = 0,
                analysis_error = NULL
            WHERE analysis_attempts > 0
              AND processed_at IS NULL
              AND (analysis_hopeless IS NULL OR analysis_hopeless = FALSE)"#,
        [],
    ));

    info!("[LLM] Reset {} failed articles for retry", affected);
    Ok(affected as i64)
}

/// Reset all hopeless articles so they can be retried from scratch.
///
/// This clears `analysis_hopeless`, `analysis_attempts`, and `analysis_error`
/// for articles that were marked as hopeless after 3+ failures.
#[tauri::command]
pub fn reset_hopeless_articles(state: State<AppState>) -> Result<i64, String> {
    let db = state.db_conn()?;

    let affected = to_cmd_err!(db.conn().execute(
        r#"UPDATE fnords SET
                analysis_attempts = 0,
                analysis_error = NULL,
                analysis_hopeless = FALSE
            WHERE analysis_hopeless = TRUE"#,
        [],
    ));

    info!("[LLM] Reset {} hopeless articles for retry", affected);
    Ok(affected as i64)
}

/// Transfer keywords from representative to cluster members
///
/// Part of the dormant cluster-based batch processing feature.
/// See [`ClusterBatchConfig`] for full documentation on status and trade-offs.
#[cfg(feature = "clustering")]
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
            let db = state.db_conn()?;
            let conn = db.conn();

            conn.execute("BEGIN TRANSACTION", [])
                .map_err(|e| e.to_string())?;

            let result: Result<(), String> = (|| {
                // Batch-insert all keywords at once using prepared statements
                if !keywords.is_empty() {
                    let mut insert_kw_stmt = conn
                        .prepare_cached("INSERT OR IGNORE INTO immanentize (name) VALUES (?1)")
                        .map_err(|e| e.to_string())?;
                    for keyword in keywords {
                        let _ = insert_kw_stmt.execute([keyword]);
                    }

                    // Build a single query to resolve all keyword IDs at once
                    let placeholders: String =
                        keywords.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                    let select_sql = format!(
                        "SELECT id, name FROM immanentize WHERE name IN ({})",
                        placeholders
                    );
                    let mut select_stmt = conn.prepare(&select_sql).map_err(|e| e.to_string())?;
                    let params: Vec<&dyn rusqlite::ToSql> =
                        keywords.iter().map(|k| k as &dyn rusqlite::ToSql).collect();
                    let keyword_ids: Vec<i64> = select_stmt
                        .query_map(params.as_slice(), |row| row.get::<_, i64>(0))
                        .map_err(|e| e.to_string())?
                        .filter_map(|r| r.ok())
                        .collect();

                    // Batch-insert all keyword-article associations
                    let mut link_stmt = conn
                        .prepare_cached(
                            r#"INSERT OR REPLACE INTO fnord_immanentize
                               (fnord_id, immanentize_id, source, confidence)
                               VALUES (?1, ?2, 'cluster', 0.85)"#,
                        )
                        .map_err(|e| e.to_string())?;
                    for keyword_id in &keyword_ids {
                        let _ = link_stmt.execute((article_id, keyword_id));
                    }
                }

                // Batch-insert all categories using prepared statement
                if !categories.is_empty() {
                    let mut cat_stmt = conn
                        .prepare_cached(
                            r#"INSERT OR REPLACE INTO fnord_sephiroth
                               (fnord_id, sephiroth_id, source, confidence)
                               VALUES (?1, ?2, 'cluster', 0.85)"#,
                        )
                        .map_err(|e| e.to_string())?;
                    for &category_id in categories {
                        let _ = cat_stmt.execute((article_id, category_id));
                    }
                }

                // Mark as processed (via cluster)
                conn.execute(
                    r#"UPDATE fnords SET
                       processed_at = CURRENT_TIMESTAMP,
                       analysis_attempts = 0,
                       analysis_error = 'Processed via cluster transfer'
                    WHERE id = ?1"#,
                    [article_id],
                )
                .map_err(|e| e.to_string())?;

                Ok(())
            })();

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
/// **To enable:** Enable `clustering` feature flag, register in invoke_handler (lib.rs), add frontend UI.
#[cfg(feature = "clustering")]
#[tauri::command]
pub async fn process_batch_clustered(
    window: Window,
    state: State<'_, AppState>,
    model: String,
    limit: Option<i64>,
    use_clustering: Option<bool>,
) -> Result<ClusterBatchResult, String> {
    let locale = get_locale_from_db(&state);

    // Get provider config
    let provider_config = {
        let db = state.db_conn()?;
        get_provider_config(&db, Some(&state.proxy_manager))
    };
    let should_cluster = use_clustering.unwrap_or(true);

    info!(
        "Starting clustered batch processing, clustering: {}",
        should_cluster
    );

    // Load batch context
    let batch_context = {
        let db = state.db_conn()?;
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
                warn!(
                    "Failed to load custom discordian prompt for clustered batch: {}",
                    e
                );
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
            unclustered_ids: articles_with_embeddings
                .iter()
                .map(|(a, _)| a.fnord_id)
                .collect(),
            total_articles,
            representatives_count: total_articles,
        };
        (result, article_map)
    };

    let (saved, _total, savings_pct) = calculate_savings(&clustering_result);
    let representatives = get_representatives(&clustering_result);
    let total_to_process = representatives.len() as i64;

    // Get num_ctx setting and create provider BEFORE emitting events
    let num_ctx = {
        let db = state.db_conn()?;
        get_num_ctx_setting(&db)
    };

    let (provider_for_batch, provider_model) = {
        let db = state.db_conn()?;
        create_text_provider(&db, Some(&state.proxy_manager))
    };

    // Determine provider name for logging/events
    let provider_name = provider_for_batch.provider_name().to_string();

    // Override model from frontend parameter only for Ollama provider
    let effective_model =
        crate::ai_provider::resolve_effective_model(&provider_name, &model, &provider_model);

    // Provider config for retry logic
    let provider_config_for_retry = provider_config;

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
            provider: provider_name.clone(),
            model: effective_model.clone(),
        },
    );

    // Process only representatives
    let articles_to_process: Vec<BatchArticle> = representatives
        .iter()
        .filter_map(|&id| article_map.get(&id).cloned())
        .collect();

    let batch_context = Arc::new(batch_context);

    // Build cluster lookup: representative_id -> cluster index
    let cluster_lookup: HashMap<i64, usize> = clustering_result
        .clusters
        .iter()
        .enumerate()
        .map(|(idx, c)| (c.representative_id, idx))
        .collect();

    let mut succeeded: i64 = 0;
    let mut failed: i64 = 0;
    let mut total_transferred: usize = 0;

    // Process articles sequentially
    for (idx, article) in articles_to_process.into_iter().enumerate() {
        if state.batch_cancel.load(Ordering::SeqCst) {
            break;
        }

        let fnord_id = article.fnord_id;
        let title = article.title.clone();

        // For retries with adjusted context (ONLY for Ollama)
        // OpenAI-compatible providers use the same provider for retries
        let retry_provider: Option<Arc<dyn AiTextProvider>> = if article.attempts > 0 {
            use crate::ai_provider::{create_provider, ProviderType};

            match provider_config_for_retry.provider_type {
                ProviderType::Ollama => {
                    // Ollama: adjust num_ctx for retries
                    let (ctx_multiplier, adjusted_num_ctx) = match article.attempts {
                        1 => (1.5, ((num_ctx as f64) * 1.5) as u32),
                        _ => (2.0, num_ctx * 2),
                    };

                    info!(
                        "Retry {}/3 for article {} (Ollama): using {}x context (num_ctx={})",
                        article.attempts + 1,
                        fnord_id,
                        ctx_multiplier,
                        adjusted_num_ctx
                    );

                    let mut retry_config = provider_config_for_retry.clone();
                    retry_config.ollama_num_ctx = adjusted_num_ctx;
                    Some(create_provider(&retry_config))
                }
                ProviderType::OpenAiCompatible => {
                    // OpenAI-compatible: use same provider (no num_ctx adjustment)
                    info!(
                        "Retry {}/3 for article {} (OpenAI-compatible): using same provider",
                        article.attempts + 1,
                        fnord_id
                    );
                    None
                }
            }
        } else {
            None
        };

        let provider: &dyn AiTextProvider = match &retry_provider {
            Some(p) => p.as_ref(),
            None => provider_for_batch.as_ref(),
        };

        let (success, error) = process_single_article(
            provider,
            &state,
            &effective_model,
            &locale,
            article,
            &batch_context,
        )
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
                                if success {
                                    succeeded += 1;
                                } else {
                                    failed += 1;
                                }
                                continue;
                            }
                        };

                        let keywords: Vec<String> = match db.conn().prepare(
                            r#"SELECT i.name FROM immanentize i
                               JOIN fnord_immanentize fi ON fi.immanentize_id = i.id
                               WHERE fi.fnord_id = ?1"#,
                        ) {
                            Ok(mut stmt) => match stmt.query_map([fnord_id], |row| row.get(0)) {
                                Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                                Err(e) => {
                                    warn!(
                                        "Failed to query keywords for article {}: {}",
                                        fnord_id, e
                                    );
                                    vec![]
                                }
                            },
                            Err(e) => {
                                warn!(
                                    "Failed to prepare keyword query for article {}: {}",
                                    fnord_id, e
                                );
                                vec![]
                            }
                        };

                        let categories: Vec<i64> = match db.conn().prepare(
                            r#"SELECT sephiroth_id FROM fnord_sephiroth
                               WHERE fnord_id = ?1"#,
                        ) {
                            Ok(mut stmt) => match stmt.query_map([fnord_id], |row| row.get(0)) {
                                Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                                Err(e) => {
                                    warn!(
                                        "Failed to query categories for article {}: {}",
                                        fnord_id, e
                                    );
                                    vec![]
                                }
                            },
                            Err(e) => {
                                warn!(
                                    "Failed to prepare category query for article {}: {}",
                                    fnord_id, e
                                );
                                vec![]
                            }
                        };

                        (keywords, categories)
                    };

                    if !keywords.is_empty() || !categories.is_empty() {
                        match transfer_keywords_to_cluster_members(
                            &state,
                            cluster,
                            &keywords,
                            &categories,
                        )
                        .await
                        {
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
                provider: provider_name.clone(),
                model: effective_model.clone(),
            },
        );

        if success {
            succeeded += 1;
            total_transferred += transferred;
        } else {
            failed += 1;
        }
    }

    // Process embedding queue after batch
    if succeeded > 0 && !state.batch_cancel.load(Ordering::SeqCst) {
        let queue_size = {
            let db = state.db_conn()?;
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
