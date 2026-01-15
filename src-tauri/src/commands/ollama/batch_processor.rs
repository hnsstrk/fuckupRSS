//! Batch processing commands for article analysis

use crate::embedding_worker;
use crate::ollama::{DiscordianAnalysis, OllamaClient, OllamaError};
use crate::text_analysis::{
    record_correction, BiasWeights, CategoryMatcher, CorrectionRecord, CorrectionType, CorpusStats,
    TfIdfExtractor,
};
use crate::{classify_by_keywords, extract_keywords};
use crate::AppState;
use futures::{stream, StreamExt};
use log::{debug, info};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{Emitter, Manager, State, Window};

use super::data_persistence::{
    generate_and_save_article_embedding, recalculate_keyword_weights, save_article_categories,
    save_article_categories_with_source, save_article_keywords_and_network,
    save_article_keywords_with_source,
};
use super::helpers::{
    determine_category_sources, determine_keyword_sources, get_ai_concurrency, get_locale_from_db,
    get_num_ctx_setting, merge_keywords, validate_and_merge_categories,
};
use super::types::{
    BatchArticle, BatchProgress, BatchResult, FailedCount, HopelessCount, UnprocessedCount,
};

/// Shared context for batch processing - loaded once before processing starts
struct BatchContext {
    bias_weights: BiasWeights,
    corpus_stats: Option<CorpusStats>,
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

/// Get count of failed articles (attempted but not hopeless)
#[tauri::command]
pub fn get_failed_count(state: State<AppState>) -> Result<FailedCount, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let count: i64 = db
        .conn()
        .query_row(
            r#"SELECT COUNT(*) FROM fnords
               WHERE analysis_attempts > 0
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

    // === LLM ANALYSIS ===
    let analysis_result = client
        .discordian_analysis_with_stats(model, &title, &content, locale, &stat_keywords, &stat_categories)
        .await;

    match analysis_result {
        Ok(analysis_with_rejections) => {
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
            }

            let analysis: DiscordianAnalysis = analysis_with_rejections.clone().into();

            // Save analysis to DB
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

                let merged_categories =
                    validate_and_merge_categories(&analysis.categories, local_categories);
                let categories_with_source =
                    determine_category_sources(&merged_categories, &stat_categories);
                let categories_saved =
                    save_article_categories_with_source(db.conn(), fnord_id, &categories_with_source);

                let merged_keywords = merge_keywords(&analysis.keywords, local_keywords.clone(), 15);
                let keywords_with_source = determine_keyword_sources(&merged_keywords, &stat_keywords);
                let (_tags_saved, tag_ids) = save_article_keywords_with_source(
                    db.conn(),
                    fnord_id,
                    &keywords_with_source,
                    &categories_saved,
                    article.article_date.as_deref(),
                );

                recalculate_keyword_weights(db.conn(), &tag_ids);

                if let Err(e) = CorpusStats::update_db_with_document(db.conn(), &document_tokens) {
                    debug!("Failed to update corpus stats: {}", e);
                }
            }

            // Generate article embedding
            if let Err(e) =
                generate_and_save_article_embedding(client, &state.db, fnord_id, &title, &content).await
            {
                debug!("Failed to generate embedding for article {}: {}", fnord_id, e);
            }

            (true, None)
        }
        Err(e) => {
            let db = match state.db.lock() {
                Ok(db) => db,
                Err(e) => return (false, Some(format!("Database lock failed: {}", e))),
            };

            let new_attempts = article.attempts + 1;

            let (error_msg, is_hopeless) = match &e {
                OllamaError::JsonParseError { message, .. } => {
                    if new_attempts >= 3 {
                        (
                            format!("JSON parse error after {} attempts: {}", new_attempts, message),
                            true,
                        )
                    } else {
                        (message.clone(), false)
                    }
                }
                other => {
                    if new_attempts >= 3 {
                        (
                            format!("Analysis failed after {} attempts: {}", new_attempts, other),
                            true,
                        )
                    } else {
                        (other.to_string(), false)
                    }
                }
            };

            let _ = db.conn().execute(
                r#"UPDATE fnords SET
                    analysis_attempts = ?1,
                    analysis_error = ?2,
                    analysis_hopeless = ?3
                WHERE id = ?4"#,
                rusqlite::params![new_attempts, &error_msg, is_hopeless, fnord_id],
            );

            // Fallback: Use statistical + local extraction
            let combined_keywords: Vec<String> = stat_keywords
                .into_iter()
                .chain(local_keywords.into_iter())
                .take(15)
                .collect();

            let categories_saved = save_article_categories(db.conn(), fnord_id, &local_categories);
            let _ = save_article_keywords_and_network(
                db.conn(),
                fnord_id,
                &combined_keywords,
                &categories_saved,
                article.article_date.as_deref(),
            );

            let status_msg = if is_hopeless {
                format!("Marked hopeless: {}", error_msg)
            } else {
                format!("Attempt {}/3 failed, will retry: {}", new_attempts, error_msg)
            };
            (false, Some(status_msg))
        }
    }
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

    info!("Starting batch processing with concurrency: {}", concurrency);

    // Load shared context ONCE before batch processing starts
    let batch_context = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let bias_weights = BiasWeights::load_from_db(db.conn()).unwrap_or_default();
        let corpus_stats = CorpusStats::load_from_db(db.conn()).ok();
        BatchContext {
            bias_weights,
            corpus_stats,
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
        return Ok(BatchResult {
            processed: 0,
            succeeded: 0,
            failed: 0,
        });
    }

    state.batch_cancel.store(false, Ordering::SeqCst);
    state.batch_running.store(true, Ordering::SeqCst);

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
    for (_, _, _, success, _) in &results {
        if *success {
            succeeded += 1;
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
                Some(&window.app_handle()),
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
