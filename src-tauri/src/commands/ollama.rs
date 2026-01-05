use crate::ollama::{
    BiasAnalysis, DiscordianAnalysis, OllamaClient, DEFAULT_ANALYSIS_PROMPT, DEFAULT_SUMMARY_PROMPT,
    RECOMMENDED_MAIN_MODEL, RECOMMENDED_EMBEDDING_MODEL, get_language_for_locale,
};
use crate::AppState;
use tauri::{Emitter, State, Window};

#[derive(serde::Serialize)]
pub struct OllamaStatus {
    pub available: bool,
    pub models: Vec<String>,
    pub recommended_main: String,
    pub recommended_embedding: String,
    pub has_recommended_main: bool,
    pub has_recommended_embedding: bool,
}

#[derive(serde::Serialize)]
pub struct ModelPullResult {
    pub success: bool,
    pub model: String,
    pub status: Option<String>,
    pub error: Option<String>,
}

#[derive(serde::Serialize)]
pub struct PromptTemplates {
    pub summary_prompt: String,
    pub analysis_prompt: String,
}

#[derive(serde::Serialize)]
pub struct DefaultPrompts {
    pub summary_prompt: String,
    pub analysis_prompt: String,
}

#[derive(serde::Serialize)]
pub struct SummaryResponse {
    pub fnord_id: i64,
    pub success: bool,
    pub summary: Option<String>,
    pub error: Option<String>,
}

#[derive(serde::Serialize)]
pub struct AnalysisResponse {
    pub fnord_id: i64,
    pub success: bool,
    pub analysis: Option<BiasAnalysis>,
    pub error: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct DiscordianResponse {
    pub fnord_id: i64,
    pub success: bool,
    pub analysis: Option<DiscordianAnalysis>,
    pub categories_saved: Vec<String>,
    pub tags_saved: Vec<String>,
    pub error: Option<String>,
}

/// Check if Ollama is available and list models
#[tauri::command]
pub async fn check_ollama() -> Result<OllamaStatus, String> {
    let client = OllamaClient::new(None);

    match client.list_models().await {
        Ok(models) => {
            let model_names: Vec<String> = models.into_iter().map(|m| m.name).collect();

            // Check if recommended models are installed
            let has_recommended_main = model_names.iter().any(|m| {
                m == RECOMMENDED_MAIN_MODEL || m.starts_with(&format!("{}:", RECOMMENDED_MAIN_MODEL.split(':').next().unwrap_or("")))
            });
            let has_recommended_embedding = model_names.iter().any(|m| {
                m == RECOMMENDED_EMBEDDING_MODEL || m.starts_with(&format!("{}:", RECOMMENDED_EMBEDDING_MODEL))
            });

            Ok(OllamaStatus {
                available: true,
                models: model_names,
                recommended_main: RECOMMENDED_MAIN_MODEL.to_string(),
                recommended_embedding: RECOMMENDED_EMBEDDING_MODEL.to_string(),
                has_recommended_main,
                has_recommended_embedding,
            })
        }
        Err(_) => Ok(OllamaStatus {
            available: false,
            models: vec![],
            recommended_main: RECOMMENDED_MAIN_MODEL.to_string(),
            recommended_embedding: RECOMMENDED_EMBEDDING_MODEL.to_string(),
            has_recommended_main: false,
            has_recommended_embedding: false,
        }),
    }
}

/// Helper to get locale from settings
fn get_locale_from_db(state: &State<'_, AppState>) -> String {
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return "de".to_string(),
    };
    db.conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'locale'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "de".to_string())
}

/// Helper to get custom prompt from settings or use default with language
fn get_summary_prompt(state: &State<'_, AppState>, locale: &str) -> String {
    let language = get_language_for_locale(locale);
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return DEFAULT_SUMMARY_PROMPT.replace("{language}", language),
    };

    let custom_prompt: Option<String> = db.conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'summary_prompt'",
            [],
            |row| row.get(0),
        )
        .ok();

    match custom_prompt {
        Some(prompt) => prompt.replace("{language}", language),
        None => DEFAULT_SUMMARY_PROMPT.replace("{language}", language),
    }
}

fn get_analysis_prompt(state: &State<'_, AppState>, locale: &str) -> String {
    let language = get_language_for_locale(locale);
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return DEFAULT_ANALYSIS_PROMPT.replace("{language}", language),
    };

    let custom_prompt: Option<String> = db.conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'analysis_prompt'",
            [],
            |row| row.get(0),
        )
        .ok();

    match custom_prompt {
        Some(prompt) => prompt.replace("{language}", language),
        None => DEFAULT_ANALYSIS_PROMPT.replace("{language}", language),
    }
}

/// Generate summary for an article
#[tauri::command]
pub async fn generate_summary(
    state: State<'_, AppState>,
    fnord_id: i64,
    model: String,
) -> Result<SummaryResponse, String> {
    let client = OllamaClient::new(None);
    let locale = get_locale_from_db(&state);
    let prompt_template = get_summary_prompt(&state, &locale);

    // Get article content from database
    let content: String = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.conn()
            .query_row(
                "SELECT COALESCE(content_full, content_raw, '') FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?
    };

    if content.is_empty() {
        return Ok(SummaryResponse {
            fnord_id,
            success: false,
            summary: None,
            error: Some("No content available".to_string()),
        });
    }

    // Generate summary with locale-aware prompt
    match client.summarize_with_prompt(&model, &content, &prompt_template).await {
        Ok(summary) => {
            // Store summary in database
            let db = state.db.lock().map_err(|e| e.to_string())?;
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

/// Analyze article for bias and objectivity
#[tauri::command]
pub async fn analyze_article(
    state: State<'_, AppState>,
    fnord_id: i64,
    model: String,
) -> Result<AnalysisResponse, String> {
    let client = OllamaClient::new(None);
    let locale = get_locale_from_db(&state);
    let prompt_template = get_analysis_prompt(&state, &locale);

    // Get article title and content from database
    let (title, content): (String, String) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.conn()
            .query_row(
                "SELECT title, COALESCE(content_full, content_raw, '') FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?
    };

    if content.is_empty() {
        return Ok(AnalysisResponse {
            fnord_id,
            success: false,
            analysis: None,
            error: Some("No content available".to_string()),
        });
    }

    // Analyze article with locale-aware prompt
    match client.analyze_bias_with_prompt(&model, &title, &content, &prompt_template).await {
        Ok(analysis) => {
            // Store analysis in database
            let db = state.db.lock().map_err(|e| e.to_string())?;
            db.conn()
                .execute(
                    r#"UPDATE fnords SET
                        political_bias = ?1,
                        sachlichkeit = ?2,
                        article_type = ?3,
                        processed_at = CURRENT_TIMESTAMP
                    WHERE id = ?4"#,
                    (
                        analysis.political_bias,
                        analysis.sachlichkeit,
                        &analysis.article_type,
                        fnord_id,
                    ),
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

/// Generate summary and analysis for an article (combined, parallel)
#[tauri::command]
pub async fn process_article(
    state: State<'_, AppState>,
    fnord_id: i64,
    model: String,
) -> Result<(SummaryResponse, AnalysisResponse), String> {
    let client = OllamaClient::new(None);
    let locale = get_locale_from_db(&state);
    let summary_prompt_template = get_summary_prompt(&state, &locale);
    let analysis_prompt_template = get_analysis_prompt(&state, &locale);

    // Get article content once
    let (title, content): (String, String) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.conn()
            .query_row(
                "SELECT title, COALESCE(content_full, content_raw, '') FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?
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

    // Run both API calls in parallel
    let model_clone = model.clone();
    let content_clone = content.clone();
    let summary_future = client.summarize_with_prompt(&model, &content, &summary_prompt_template);
    let analysis_future = client.analyze_bias_with_prompt(&model_clone, &title, &content_clone, &analysis_prompt_template);

    let (summary_result, analysis_result) = tokio::join!(summary_future, analysis_future);

    // Process results
    let summary_response = match summary_result {
        Ok(summary) => {
            // Store summary
            let db = state.db.lock().map_err(|e| e.to_string())?;
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
        Ok(analysis) => {
            // Store analysis
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let _ = db.conn().execute(
                r#"UPDATE fnords SET
                    political_bias = ?1,
                    sachlichkeit = ?2,
                    article_type = ?3,
                    processed_at = CURRENT_TIMESTAMP
                WHERE id = ?4"#,
                (
                    analysis.political_bias,
                    analysis.sachlichkeit,
                    &analysis.article_type,
                    fnord_id,
                ),
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

/// Full Discordian Analysis: Summary + Bias + Categories + Keywords in one call
#[tauri::command]
pub async fn process_article_discordian(
    state: State<'_, AppState>,
    fnord_id: i64,
    model: String,
) -> Result<DiscordianResponse, String> {
    let client = OllamaClient::new(None);
    let locale = get_locale_from_db(&state);

    // Get article content
    let (title, content): (String, String) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.conn()
            .query_row(
                "SELECT title, COALESCE(content_full, content_raw, '') FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?
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

    // Run Discordian Analysis
    match client.discordian_analysis(&model, &title, &content, &locale).await {
        Ok(analysis) => {
            let db = state.db.lock().map_err(|e| e.to_string())?;

            // Store summary and bias analysis
            db.conn()
                .execute(
                    r#"UPDATE fnords SET
                        summary = ?1,
                        political_bias = ?2,
                        sachlichkeit = ?3,
                        article_type = ?4,
                        processed_at = CURRENT_TIMESTAMP
                    WHERE id = ?5"#,
                    (
                        &analysis.summary,
                        analysis.political_bias,
                        analysis.sachlichkeit,
                        &analysis.article_type,
                        fnord_id,
                    ),
                )
                .map_err(|e| e.to_string())?;

            // Store categories (Sephiroth)
            let mut categories_saved = Vec::new();
            db.conn()
                .execute("DELETE FROM fnord_sephiroth WHERE fnord_id = ?", [fnord_id])
                .ok();

            for cat_name in &analysis.categories {
                if let Ok(sephiroth_id) = db.conn().query_row::<i64, _, _>(
                    "SELECT id FROM sephiroth WHERE LOWER(name) = LOWER(?)",
                    [cat_name],
                    |row| row.get(0),
                ) {
                    db.conn()
                        .execute(
                            r#"INSERT OR IGNORE INTO fnord_sephiroth
                               (fnord_id, sephiroth_id, confidence, source, assigned_at)
                               VALUES (?, ?, 1.0, 'ai', CURRENT_TIMESTAMP)"#,
                            rusqlite::params![fnord_id, sephiroth_id],
                        )
                        .ok();

                    // Update article count
                    db.conn()
                        .execute(
                            "UPDATE sephiroth SET article_count = (SELECT COUNT(*) FROM fnord_sephiroth WHERE sephiroth_id = ?) WHERE id = ?",
                            rusqlite::params![sephiroth_id, sephiroth_id],
                        )
                        .ok();

                    categories_saved.push(cat_name.clone());
                }
            }

            // Store keywords (Immanentize)
            let mut tags_saved = Vec::new();
            db.conn()
                .execute("DELETE FROM fnord_immanentize WHERE fnord_id = ?", [fnord_id])
                .ok();

            for keyword in &analysis.keywords {
                let keyword = keyword.trim();
                if keyword.is_empty() {
                    continue;
                }

                // Upsert tag
                db.conn()
                    .execute(
                        r#"INSERT INTO immanentize (name, count, last_used)
                           VALUES (?, 1, CURRENT_TIMESTAMP)
                           ON CONFLICT(name) DO UPDATE SET
                               count = count + 1,
                               last_used = CURRENT_TIMESTAMP"#,
                        [keyword],
                    )
                    .ok();

                // Get tag ID and link
                if let Ok(tag_id) = db.conn().query_row::<i64, _, _>(
                    "SELECT id FROM immanentize WHERE name = ?",
                    [keyword],
                    |row| row.get(0),
                ) {
                    db.conn()
                        .execute(
                            "INSERT OR IGNORE INTO fnord_immanentize (fnord_id, immanentize_id) VALUES (?, ?)",
                            rusqlite::params![fnord_id, tag_id],
                        )
                        .ok();
                    tags_saved.push(keyword.to_string());
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
        Err(e) => Ok(DiscordianResponse {
            fnord_id,
            success: false,
            analysis: None,
            categories_saved: vec![],
            tags_saved: vec![],
            error: Some(e.to_string()),
        }),
    }
}

// ============================================================
// BATCH PROCESSING (Fnord Processing)
// ============================================================

#[derive(serde::Serialize, Clone)]
pub struct BatchProgress {
    pub current: i64,
    pub total: i64,
    pub fnord_id: i64,
    pub title: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(serde::Serialize)]
pub struct BatchResult {
    pub processed: i64,
    pub succeeded: i64,
    pub failed: i64,
}

#[derive(serde::Serialize)]
pub struct UnprocessedCount {
    pub total: i64,
    pub with_content: i64,
}

/// Get count of unprocessed articles
#[tauri::command]
pub fn get_unprocessed_count(state: State<AppState>) -> Result<UnprocessedCount, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let total: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnords WHERE processed_at IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let with_content: i64 = db
        .conn()
        .query_row(
            r#"SELECT COUNT(*) FROM fnords
               WHERE processed_at IS NULL
               AND (content_full IS NOT NULL OR content_raw IS NOT NULL)"#,
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(UnprocessedCount { total, with_content })
}

/// Process all unprocessed articles in batch
/// Emits 'batch-progress' events to the window
#[tauri::command]
pub async fn process_batch(
    window: Window,
    state: State<'_, AppState>,
    model: String,
    limit: Option<i64>,
) -> Result<BatchResult, String> {
    let client = OllamaClient::new(None);
    let batch_limit = limit.unwrap_or(100);

    // Get locale and prompts once for the batch
    let locale = get_locale_from_db(&state);
    let summary_prompt = get_summary_prompt(&state, &locale);
    let analysis_prompt = get_analysis_prompt(&state, &locale);

    // Get unprocessed articles with content
    let articles: Vec<(i64, String, String)> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .conn()
            .prepare(
                r#"SELECT id, title, COALESCE(content_full, content_raw, '') as content
                   FROM fnords
                   WHERE processed_at IS NULL
                   AND (content_full IS NOT NULL OR content_raw IS NOT NULL)
                   ORDER BY published_at DESC
                   LIMIT ?1"#,
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([batch_limit], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|e| e.to_string())?;

        rows.filter_map(|r| r.ok()).collect()
    };

    let total = articles.len() as i64;
    let mut succeeded: i64 = 0;
    let mut failed: i64 = 0;

    for (idx, (fnord_id, title, content)) in articles.into_iter().enumerate() {
        let current = (idx + 1) as i64;

        if content.is_empty() {
            failed += 1;
            let _ = window.emit(
                "batch-progress",
                BatchProgress {
                    current,
                    total,
                    fnord_id,
                    title: title.clone(),
                    success: false,
                    error: Some("No content".to_string()),
                },
            );
            continue;
        }

        // Generate summary with locale-aware prompt
        let summary_result = client.summarize_with_prompt(&model, &content, &summary_prompt).await;

        // Analyze bias with locale-aware prompt
        let analysis_result = client.analyze_bias_with_prompt(&model, &title, &content, &analysis_prompt).await;

        // Store results
        let (success, error) = match (&summary_result, &analysis_result) {
            (Ok(summary), Ok(analysis)) => {
                let db = state.db.lock().map_err(|e| e.to_string())?;
                let update_result = db.conn().execute(
                    r#"UPDATE fnords SET
                        summary = ?1,
                        political_bias = ?2,
                        sachlichkeit = ?3,
                        article_type = ?4,
                        processed_at = CURRENT_TIMESTAMP
                    WHERE id = ?5"#,
                    (
                        summary,
                        analysis.political_bias,
                        analysis.sachlichkeit,
                        &analysis.article_type,
                        fnord_id,
                    ),
                );
                match update_result {
                    Ok(_) => {
                        succeeded += 1;
                        (true, None)
                    }
                    Err(e) => {
                        failed += 1;
                        (false, Some(e.to_string()))
                    }
                }
            }
            (Err(e), _) => {
                failed += 1;
                (false, Some(format!("Summary: {}", e)))
            }
            (_, Err(e)) => {
                failed += 1;
                (false, Some(format!("Analyse: {}", e)))
            }
        };

        // Emit progress event
        let _ = window.emit(
            "batch-progress",
            BatchProgress {
                current,
                total,
                fnord_id,
                title,
                success,
                error,
            },
        );
    }

    Ok(BatchResult {
        processed: total,
        succeeded,
        failed,
    })
}

// ============================================================
// MODEL MANAGEMENT
// ============================================================

/// Pull/download a model from Ollama
#[tauri::command]
pub async fn pull_model(window: Window, model: String) -> Result<ModelPullResult, String> {
    let client = OllamaClient::new(None);

    // Emit start event
    let _ = window.emit("model-pull-start", &model);

    match client.pull_model(&model).await {
        Ok(status) => {
            let _ = window.emit("model-pull-complete", &model);
            Ok(ModelPullResult {
                success: true,
                model,
                status: Some(status),
                error: None,
            })
        }
        Err(e) => {
            let _ = window.emit("model-pull-error", &model);
            Ok(ModelPullResult {
                success: false,
                model,
                status: None,
                error: Some(e.to_string()),
            })
        }
    }
}

// ============================================================
// PROMPT TEMPLATES
// ============================================================

/// Get default prompt templates
#[tauri::command]
pub fn get_default_prompts() -> DefaultPrompts {
    DefaultPrompts {
        summary_prompt: DEFAULT_SUMMARY_PROMPT.to_string(),
        analysis_prompt: DEFAULT_ANALYSIS_PROMPT.to_string(),
    }
}

/// Get current prompt templates (from DB or defaults)
#[tauri::command]
pub fn get_prompts(state: State<AppState>) -> Result<PromptTemplates, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let summary_prompt: Option<String> = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'summary_prompt'",
            [],
            |row| row.get(0),
        )
        .ok();

    let analysis_prompt: Option<String> = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'analysis_prompt'",
            [],
            |row| row.get(0),
        )
        .ok();

    Ok(PromptTemplates {
        summary_prompt: summary_prompt.unwrap_or_else(|| DEFAULT_SUMMARY_PROMPT.to_string()),
        analysis_prompt: analysis_prompt.unwrap_or_else(|| DEFAULT_ANALYSIS_PROMPT.to_string()),
    })
}

/// Save prompt templates to DB
#[tauri::command]
pub fn set_prompts(
    state: State<AppState>,
    summary_prompt: String,
    analysis_prompt: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('summary_prompt', ?1)",
            [&summary_prompt],
        )
        .map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('analysis_prompt', ?1)",
            [&analysis_prompt],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Reset prompts to defaults
#[tauri::command]
pub fn reset_prompts(state: State<AppState>) -> Result<PromptTemplates, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute("DELETE FROM settings WHERE key = 'summary_prompt'", [])
        .map_err(|e| e.to_string())?;

    db.conn()
        .execute("DELETE FROM settings WHERE key = 'analysis_prompt'", [])
        .map_err(|e| e.to_string())?;

    Ok(PromptTemplates {
        summary_prompt: DEFAULT_SUMMARY_PROMPT.to_string(),
        analysis_prompt: DEFAULT_ANALYSIS_PROMPT.to_string(),
    })
}
