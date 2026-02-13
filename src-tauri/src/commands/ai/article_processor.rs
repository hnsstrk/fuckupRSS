//! Single article processing commands (summary, analysis, discordian)

use crate::ai_provider::{AiTextProvider, EmbeddingProvider};
use crate::ollama::DiscordianAnalysis;
use crate::text_analysis::{
    record_correction, BiasWeights, CategoryMatcher, CorpusStats, CorrectionRecord, CorrectionType,
    TfIdfExtractor,
};
use crate::AppState;
use crate::{classify_by_keywords, extract_keywords};
use log::{info, warn};
use std::sync::Arc;
use std::time::Instant;
use tauri::State;

use super::data_persistence::{
    generate_and_save_article_embedding, recalculate_keyword_weights, save_article_categories,
    save_article_categories_with_source, save_article_keywords_and_network,
    save_article_keywords_with_source,
};
use super::helpers::{
    analyze_bias_via_provider, create_embedding_provider_from_db, create_text_provider,
    determine_category_sources, determine_keyword_sources, discordian_analysis_via_provider,
    get_analysis_prompt, get_discordian_prompt, get_locale_from_db, get_summary_prompt,
    log_generation_cost, merge_keywords, summarize_via_provider, validate_and_merge_categories,
};
use super::types::{AnalysisResponse, DiscordianResponse, SummaryResponse};

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

/// Generate a summary for an article
#[tauri::command]
pub async fn generate_summary(
    state: State<'_, AppState>,
    fnord_id: i64,
    model: String,
) -> Result<SummaryResponse, String> {
    let locale = get_locale_from_db(&state);
    let prompt_template = get_summary_prompt(&state, &locale);

    let (provider, effective_model, content): (Arc<dyn AiTextProvider>, String, String) = {
        let db = state.db_conn()?;
        let (provider, provider_model) = create_text_provider(&db);
        let content = db
            .conn()
            .query_row(
                "SELECT COALESCE(content_full, '') FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        // Use the provider's configured model for OpenAI (frontend sends Ollama model names)
        let effective_model = crate::ai_provider::resolve_effective_model(
            provider.provider_name(),
            &model,
            &provider_model,
        );
        (provider, effective_model, content)
    };

    if content.is_empty() {
        return Ok(SummaryResponse {
            fnord_id,
            success: false,
            summary: None,
            error: Some("No content available".to_string()),
        });
    }

    match summarize_via_provider(
        provider.as_ref(),
        &effective_model,
        &content,
        &prompt_template,
    )
    .await
    {
        Ok((summary, usage)) => {
            let db = state.db_conn()?;
            log_generation_cost(
                db.conn(),
                provider.provider_name(),
                &effective_model,
                &usage,
            );
            db.conn()
                .execute(
                    "UPDATE fnords SET summary = ?1, processed_at = CURRENT_TIMESTAMP WHERE id = ?2",
                    (&summary, fnord_id),
                )
                .map_err(|e| e.to_string())?;

            Ok(SummaryResponse {
                fnord_id,
                success: true,
                summary: Some(summary),
                error: None,
            })
        }
        Err(e) => Ok(SummaryResponse {
            fnord_id,
            success: false,
            summary: None,
            error: Some(e.to_string()),
        }),
    }
}

/// Analyze an article for bias
#[tauri::command]
pub async fn analyze_article(
    state: State<'_, AppState>,
    fnord_id: i64,
    model: String,
) -> Result<AnalysisResponse, String> {
    let locale = get_locale_from_db(&state);
    let prompt_template = get_analysis_prompt(&state, &locale);

    let (provider, effective_model, title, content): (
        Arc<dyn AiTextProvider>,
        String,
        String,
        String,
    ) = {
        let db = state.db_conn()?;
        let (provider, provider_model) = create_text_provider(&db);
        let (title, content) = db
            .conn()
            .query_row(
                "SELECT title, COALESCE(content_full, '') FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        let effective_model = crate::ai_provider::resolve_effective_model(
            provider.provider_name(),
            &model,
            &provider_model,
        );
        (provider, effective_model, title, content)
    };

    if content.is_empty() {
        return Ok(AnalysisResponse {
            fnord_id,
            success: false,
            analysis: None,
            error: Some("No content available".to_string()),
        });
    }

    match analyze_bias_via_provider(
        provider.as_ref(),
        &effective_model,
        &title,
        &content,
        &prompt_template,
    )
    .await
    {
        Ok((analysis, usage)) => {
            let db = state.db_conn()?;
            log_generation_cost(
                db.conn(),
                provider.provider_name(),
                &effective_model,
                &usage,
            );
            db.conn()
                .execute(
                    r#"UPDATE fnords SET
                        political_bias = ?1,
                        sachlichkeit = ?2,
                        processed_at = CURRENT_TIMESTAMP
                    WHERE id = ?3"#,
                    (analysis.political_bias, analysis.sachlichkeit, fnord_id),
                )
                .map_err(|e| e.to_string())?;

            Ok(AnalysisResponse {
                fnord_id,
                success: true,
                analysis: Some(analysis),
                error: None,
            })
        }
        Err(e) => Ok(AnalysisResponse {
            fnord_id,
            success: false,
            analysis: None,
            error: Some(e.to_string()),
        }),
    }
}

/// Process an article (summary + bias analysis)
#[tauri::command]
pub async fn process_article(
    state: State<'_, AppState>,
    fnord_id: i64,
    model: String,
) -> Result<(SummaryResponse, AnalysisResponse), String> {
    let locale = get_locale_from_db(&state);
    let summary_prompt_template = get_summary_prompt(&state, &locale);
    let analysis_prompt_template = get_analysis_prompt(&state, &locale);

    let (provider, effective_model, title, content): (
        Arc<dyn AiTextProvider>,
        String,
        String,
        String,
    ) = {
        let db = state.db_conn()?;
        let (provider, provider_model) = create_text_provider(&db);
        let (title, content) = db
            .conn()
            .query_row(
                "SELECT title, COALESCE(content_full, '') FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        let effective_model = crate::ai_provider::resolve_effective_model(
            provider.provider_name(),
            &model,
            &provider_model,
        );
        (provider, effective_model, title, content)
    };

    if content.is_empty() {
        return Ok((
            SummaryResponse {
                fnord_id,
                success: false,
                summary: None,
                error: Some("No content available".to_string()),
            },
            AnalysisResponse {
                fnord_id,
                success: false,
                analysis: None,
                error: Some("No content available".to_string()),
            },
        ));
    }

    let model_clone = effective_model.clone();
    let content_clone = content.clone();
    let summary_prompt = summary_prompt_template.clone();
    let analysis_prompt = analysis_prompt_template.clone();
    let provider_ref = provider.clone();

    let summary_future = summarize_via_provider(
        provider.as_ref(),
        &effective_model,
        &content,
        &summary_prompt,
    );
    let analysis_future = analyze_bias_via_provider(
        provider_ref.as_ref(),
        &model_clone,
        &title,
        &content_clone,
        &analysis_prompt,
    );

    let (summary_result, analysis_result) = tokio::join!(summary_future, analysis_future);

    let summary_response = match summary_result {
        Ok((summary, usage)) => {
            let db = state.db_conn()?;
            log_generation_cost(
                db.conn(),
                provider.provider_name(),
                &effective_model,
                &usage,
            );
            let _ = db.conn().execute(
                "UPDATE fnords SET summary = ?1 WHERE id = ?2",
                (&summary, fnord_id),
            );
            SummaryResponse {
                fnord_id,
                success: true,
                summary: Some(summary),
                error: None,
            }
        }
        Err(e) => SummaryResponse {
            fnord_id,
            success: false,
            summary: None,
            error: Some(e.to_string()),
        },
    };

    let analysis_response = match analysis_result {
        Ok((analysis, usage)) => {
            let db = state.db_conn()?;
            log_generation_cost(
                db.conn(),
                provider.provider_name(),
                &effective_model,
                &usage,
            );
            let _ = db.conn().execute(
                r#"UPDATE fnords SET
                    political_bias = ?1,
                    sachlichkeit = ?2,
                    processed_at = CURRENT_TIMESTAMP
                WHERE id = ?3"#,
                (analysis.political_bias, analysis.sachlichkeit, fnord_id),
            );
            AnalysisResponse {
                fnord_id,
                success: true,
                analysis: Some(analysis),
                error: None,
            }
        }
        Err(e) => AnalysisResponse {
            fnord_id,
            success: false,
            analysis: None,
            error: Some(e.to_string()),
        },
    };

    Ok((summary_response, analysis_response))
}

/// Full Discordian analysis (summary, bias, categories, keywords)
#[tauri::command]
pub async fn process_article_discordian(
    state: State<'_, AppState>,
    fnord_id: i64,
    model: String,
) -> Result<DiscordianResponse, String> {
    let locale = get_locale_from_db(&state);
    let custom_discordian_prompt = get_discordian_prompt(&state);

    // Step 1: Load article content, bias weights, and corpus stats
    #[allow(clippy::type_complexity)]
    let (
        provider,
        effective_model,
        embedding_provider,
        title,
        content,
        article_date,
        bias_weights,
        corpus_stats,
    ): (
        Arc<dyn AiTextProvider>,
        String,
        Arc<dyn EmbeddingProvider>,
        String,
        String,
        Option<String>,
        BiasWeights,
        Option<CorpusStats>,
    ) = {
        let db = state.db_conn()?;
        let (provider, provider_model) = create_text_provider(&db);
        let embedding_provider = create_embedding_provider_from_db(&db);
        let (title, content, article_date) = db
            .conn()
            .query_row(
                "SELECT title, COALESCE(content_full, ''), DATE(COALESCE(published_at, fetched_at)) FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| e.to_string())?;
        let bias = BiasWeights::load_from_db(db.conn()).unwrap_or_default();
        let corpus = CorpusStats::load_from_db(db.conn()).ok();
        let effective_model = crate::ai_provider::resolve_effective_model(
            provider.provider_name(),
            &model,
            &provider_model,
        );
        (
            provider,
            effective_model,
            embedding_provider,
            title,
            content,
            article_date,
            bias,
            corpus,
        )
    };

    if content.is_empty() {
        return Ok(DiscordianResponse {
            fnord_id,
            success: false,
            analysis: None,
            categories_saved: vec![],
            tags_saved: vec![],
            error: Some("No content available".to_string()),
        });
    }

    // Step 2: Run statistical pre-analysis
    let text_for_analysis = format!("{} {}", title, content);

    // TF-IDF keyword extraction with bias weights
    let extractor = TfIdfExtractor::new().with_max_keywords(30);
    let mut keyword_candidates: Vec<(String, f64)> = extractor
        .extract_smart(&text_for_analysis, corpus_stats.as_ref())
        .into_iter()
        .map(|kc| {
            let adjusted_score = bias_weights.apply_to_keyword(&kc.term, kc.score);
            (kc.term, adjusted_score)
        })
        .collect();

    keyword_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    keyword_candidates.truncate(15);
    let stat_keywords: Vec<String> = keyword_candidates
        .into_iter()
        .map(|(term, _)| term)
        .collect();

    // Extract tokens for corpus stats update
    let document_tokens = extractor.get_tokens(&text_for_analysis);

    // Category matching with bias
    let matcher = CategoryMatcher::new().with_max_categories(5);
    let stat_categories: Vec<(String, f64)> = matcher
        .score_categories(&text_for_analysis, Some(&bias_weights))
        .into_iter()
        .map(|cs| (cs.name.clone(), cs.confidence))
        .collect();

    // Local extraction fallback
    let local_keywords = extract_keywords(&title, &content, 10);
    let local_categories = classify_by_keywords(&local_keywords);

    // Step 3: Run LLM analysis with statistical context
    info!(
        "[LLM] Single article analysis for \"{}\" (ID: {})",
        truncate_str(&title, 60),
        fnord_id
    );
    let llm_start = Instant::now();

    match discordian_analysis_via_provider(
        provider.as_ref(),
        &effective_model,
        &title,
        &content,
        &locale,
        &stat_keywords,
        &stat_categories,
        custom_discordian_prompt.as_deref(),
    )
    .await
    {
        Ok((analysis_with_rejections, usage)) => {
            // Log cost with a brief DB lock
            {
                let db = state.db_conn()?;
                log_generation_cost(
                    db.conn(),
                    provider.provider_name(),
                    &effective_model,
                    &usage,
                );
            }
            let duration = llm_start.elapsed();
            info!(
                "[LLM] Single article completed \"{}\" (ID: {}) in {:.2}s",
                truncate_str(&title, 50),
                fnord_id,
                duration.as_secs_f64()
            );
            // Step 4: Learn from LLM rejections
            {
                let db = state.db_conn()?;

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
                        let matching_terms: Vec<String> =
                            stat_keywords.iter().take(5).cloned().collect();

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

            // Save to database
            {
                let db = state.db_conn()?;

                db.conn()
                    .execute(
                        r#"UPDATE fnords SET
                            summary = ?1,
                            political_bias = ?2,
                            sachlichkeit = ?3,
                            processed_at = CURRENT_TIMESTAMP
                        WHERE id = ?4"#,
                        (
                            &analysis.summary,
                            analysis.political_bias,
                            analysis.sachlichkeit,
                            fnord_id,
                        ),
                    )
                    .map_err(|e| e.to_string())?;
            }

            let (categories_saved, tags_saved) = {
                let db = state.db_conn()?;

                let merged_categories =
                    validate_and_merge_categories(&analysis.categories, local_categories);
                let categories_with_source =
                    determine_category_sources(&merged_categories, &stat_categories);
                let categories_saved = save_article_categories_with_source(
                    db.conn(),
                    fnord_id,
                    &categories_with_source,
                );

                let merged_keywords =
                    merge_keywords(&analysis.keywords, local_keywords.clone(), 15);
                let keywords_with_source =
                    determine_keyword_sources(&merged_keywords, &stat_keywords);
                let (tags_saved, tag_ids) = save_article_keywords_with_source(
                    db.conn(),
                    fnord_id,
                    &keywords_with_source,
                    &categories_saved,
                    article_date.as_deref(),
                );

                recalculate_keyword_weights(db.conn(), &tag_ids);
                (categories_saved, tags_saved)
            };

            // Generate article embedding
            if let Err(e) = generate_and_save_article_embedding(
                embedding_provider.as_ref(),
                &state.db,
                fnord_id,
                &title,
                &content,
            )
            .await
            {
                warn!(
                    "Failed to generate embedding for article {}: {}",
                    fnord_id, e
                );
            }

            // Update corpus stats
            {
                let db = state.db_conn()?;
                if let Err(e) = CorpusStats::update_db_with_document(db.conn(), &document_tokens) {
                    warn!("Failed to update corpus stats: {}", e);
                }
            }

            Ok(DiscordianResponse {
                fnord_id,
                success: true,
                analysis: Some(analysis),
                categories_saved,
                tags_saved,
                error: None,
            })
        }
        Err(e) => {
            let duration = llm_start.elapsed();
            warn!(
                "[LLM] Single article FAILED \"{}\" (ID: {}) after {:.2}s: {}",
                truncate_str(&title, 50),
                fnord_id,
                duration.as_secs_f64(),
                e
            );

            // Fallback: Use statistical + local extraction
            let (categories_saved, tags_saved) = {
                let db = state.db_conn()?;

                let combined_keywords: Vec<String> = stat_keywords
                    .into_iter()
                    .chain(local_keywords.into_iter())
                    .take(15)
                    .collect();

                let categories_saved =
                    save_article_categories(db.conn(), fnord_id, &local_categories);
                let (tags_saved, tag_ids) = save_article_keywords_and_network(
                    db.conn(),
                    fnord_id,
                    &combined_keywords,
                    &categories_saved,
                    article_date.as_deref(),
                );
                recalculate_keyword_weights(db.conn(), &tag_ids);

                (categories_saved, tags_saved)
            }; // Lock is dropped here

            // Generate article embedding even if LLM analysis failed
            // This ensures articles processed with fallback methods still get embeddings for similarity search
            if let Err(embed_err) = generate_and_save_article_embedding(
                embedding_provider.as_ref(),
                &state.db,
                fnord_id,
                &title,
                &content,
            )
            .await
            {
                warn!(
                    "Failed to generate embedding for article {} in fallback mode: {}",
                    fnord_id, embed_err
                );
            }

            Ok(DiscordianResponse {
                fnord_id,
                success: false,
                analysis: None,
                categories_saved,
                tags_saved,
                error: Some(format!(
                    "LLM failed, used statistical + local extraction: {}",
                    e
                )),
            })
        }
    }
}
