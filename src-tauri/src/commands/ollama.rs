use crate::ollama::{
    BiasAnalysis, DiscordianAnalysis, OllamaClient, DEFAULT_ANALYSIS_PROMPT, DEFAULT_SUMMARY_PROMPT,
    RECOMMENDED_MAIN_MODEL, RECOMMENDED_EMBEDDING_MODEL, get_language_for_locale,
};
use crate::AppState;
use std::sync::atomic::Ordering;
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

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct LoadedModel {
    pub name: String,
    pub size: u64,
    pub size_vram: u64,
    pub parameter_size: String,
}

#[derive(serde::Serialize)]
pub struct LoadedModelsResponse {
    pub models: Vec<LoadedModel>,
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

// ============================================================
// HELPER FUNCTIONS FOR CATEGORY/KEYWORD SAVING
// ============================================================

/// Save categories (Sephiroth) for an article and update article counts
/// Returns the list of successfully saved category names
fn save_article_categories(
    conn: &rusqlite::Connection,
    fnord_id: i64,
    categories: &[String],
) -> Vec<String> {
    let mut saved = Vec::new();

    conn.execute("DELETE FROM fnord_sephiroth WHERE fnord_id = ?", [fnord_id])
        .ok();

    for cat_name in categories {
        if let Ok(sephiroth_id) = conn.query_row::<i64, _, _>(
            "SELECT id FROM sephiroth WHERE LOWER(name) = LOWER(?)",
            [cat_name],
            |row| row.get(0),
        ) {
            conn.execute(
                r#"INSERT OR IGNORE INTO fnord_sephiroth
                   (fnord_id, sephiroth_id, confidence, source, assigned_at)
                   VALUES (?, ?, 1.0, 'ai', CURRENT_TIMESTAMP)"#,
                rusqlite::params![fnord_id, sephiroth_id],
            )
            .ok();

            conn.execute(
                "UPDATE sephiroth SET article_count = (SELECT COUNT(*) FROM fnord_sephiroth WHERE sephiroth_id = ?) WHERE id = ?",
                rusqlite::params![sephiroth_id, sephiroth_id],
            )
            .ok();

            saved.push(cat_name.clone());
        }
    }

    saved
}

/// Save keywords (Immanentize) for an article and update the keyword network
/// Returns the list of saved keyword names and tag IDs
fn save_article_keywords_and_network(
    conn: &rusqlite::Connection,
    fnord_id: i64,
    keywords: &[String],
    categories_saved: &[String],
    article_date: Option<&str>,
) -> (Vec<String>, Vec<i64>) {
    let mut tags_saved = Vec::new();
    let mut tag_ids: Vec<i64> = Vec::new();

    let existing_tag_ids: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT immanentize_id FROM fnord_immanentize WHERE fnord_id = ?")
            .unwrap();
        stmt.query_map([fnord_id], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    };

    conn.execute("DELETE FROM fnord_immanentize WHERE fnord_id = ?", [fnord_id])
        .ok();

    for keyword in keywords {
        let keyword = keyword.trim();
        if keyword.is_empty() {
            continue;
        }

        let existing_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM immanentize WHERE name = ?",
                [keyword],
                |row| row.get(0),
            )
            .ok();

        let is_new_for_article = existing_id
            .map(|id| !existing_tag_ids.contains(&id))
            .unwrap_or(true);

        if is_new_for_article {
            conn.execute(
                r#"INSERT INTO immanentize (name, count, article_count, first_seen, last_used)
                   VALUES (?1, 1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                   ON CONFLICT(name) DO UPDATE SET
                       count = count + 1,
                       article_count = article_count + 1,
                       last_used = CURRENT_TIMESTAMP"#,
                [keyword],
            )
            .ok();
        } else {
            conn.execute(
                r#"UPDATE immanentize SET
                       count = count + 1,
                       last_used = CURRENT_TIMESTAMP
                   WHERE name = ?1"#,
                [keyword],
            )
            .ok();
        }

        if let Ok(tag_id) = conn.query_row::<i64, _, _>(
            "SELECT id FROM immanentize WHERE name = ?",
            [keyword],
            |row| row.get(0),
        ) {
            conn.execute(
                "INSERT OR IGNORE INTO fnord_immanentize (fnord_id, immanentize_id) VALUES (?, ?)",
                rusqlite::params![fnord_id, tag_id],
            )
            .ok();

            if let Some(date) = article_date {
                conn.execute(
                    r#"INSERT INTO immanentize_daily (immanentize_id, date, count)
                       VALUES (?1, ?2, 1)
                       ON CONFLICT(immanentize_id, date) DO UPDATE SET count = count + 1"#,
                    rusqlite::params![tag_id, date],
                )
                .ok();
            }

            tags_saved.push(keyword.to_string());
            tag_ids.push(tag_id);
        }
    }

    for tag_id in &tag_ids {
        for cat_name in categories_saved {
            if let Ok(sephiroth_id) = conn.query_row::<i64, _, _>(
                "SELECT id FROM sephiroth WHERE LOWER(name) = LOWER(?)",
                [cat_name],
                |row| row.get(0),
            ) {
                conn.execute(
                    r#"INSERT INTO immanentize_sephiroth
                       (immanentize_id, sephiroth_id, weight, article_count, first_seen, updated_at)
                       VALUES (?1, ?2, 1.0, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                       ON CONFLICT(immanentize_id, sephiroth_id) DO UPDATE SET
                           article_count = article_count + 1,
                           updated_at = CURRENT_TIMESTAMP"#,
                    rusqlite::params![tag_id, sephiroth_id],
                )
                .ok();
            }
        }
    }

    for i in 0..tag_ids.len() {
        for j in (i + 1)..tag_ids.len() {
            let (id_a, id_b) = if tag_ids[i] < tag_ids[j] {
                (tag_ids[i], tag_ids[j])
            } else {
                (tag_ids[j], tag_ids[i])
            };

            conn.execute(
                r#"INSERT INTO immanentize_neighbors
                   (immanentize_id_a, immanentize_id_b, cooccurrence, first_seen, last_seen)
                   VALUES (?1, ?2, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                   ON CONFLICT(immanentize_id_a, immanentize_id_b) DO UPDATE SET
                       cooccurrence = cooccurrence + 1,
                       last_seen = CURRENT_TIMESTAMP"#,
                rusqlite::params![id_a, id_b],
            )
            .ok();
        }
    }

    (tags_saved, tag_ids)
}

fn recalculate_keyword_weights(conn: &rusqlite::Connection, tag_ids: &[i64]) {
    for tag_id in tag_ids {
        conn.execute(
            r#"UPDATE immanentize_sephiroth
               SET weight = CAST(article_count AS REAL) / (
                   SELECT MAX(article_count) FROM immanentize_sephiroth
                   WHERE immanentize_id = ?1
               )
               WHERE immanentize_id = ?1"#,
            [tag_id],
        )
        .ok();
    }

    conn.execute(
        r#"UPDATE immanentize_neighbors
           SET combined_weight = CAST(cooccurrence AS REAL) / (
               SELECT MAX(cooccurrence) FROM immanentize_neighbors
           )
           WHERE immanentize_id_a IN (SELECT value FROM json_each(?1))
              OR immanentize_id_b IN (SELECT value FROM json_each(?1))"#,
        [serde_json::to_string(&tag_ids).unwrap_or_default()],
    )
    .ok();
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

/// Get currently loaded models in Ollama VRAM
#[tauri::command]
pub async fn get_loaded_models() -> Result<LoadedModelsResponse, String> {
    let client = reqwest_new::Client::new();

    let response = client
        .get("http://localhost:11434/api/ps")
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;

    if !response.status().is_success() {
        return Err("Failed to get loaded models".to_string());
    }

    #[derive(serde::Deserialize)]
    struct PsResponse {
        models: Option<Vec<PsModel>>,
    }

    #[derive(serde::Deserialize)]
    struct PsModel {
        name: String,
        size: u64,
        size_vram: u64,
        details: PsModelDetails,
    }

    #[derive(serde::Deserialize)]
    struct PsModelDetails {
        parameter_size: String,
    }

    let ps_response: PsResponse = response.json().await.map_err(|e| e.to_string())?;

    let models = ps_response.models.unwrap_or_default()
        .into_iter()
        .map(|m| LoadedModel {
            name: m.name,
            size: m.size,
            size_vram: m.size_vram,
            parameter_size: m.details.parameter_size,
        })
        .collect();

    Ok(LoadedModelsResponse { models })
}

/// Load a model into Ollama VRAM and keep it loaded indefinitely
#[tauri::command]
pub async fn load_model(model: String) -> Result<bool, String> {
    let client = reqwest_new::Client::new();

    // Use keep_alive: -1 to keep the model loaded indefinitely
    // For embedding models, use /api/embeddings endpoint
    // For generation models, use /api/generate endpoint

    // Try embedding first (works for nomic-embed-text)
    let embed_body = serde_json::json!({
        "model": model,
        "prompt": "test",
        "keep_alive": -1
    });

    let response = client
        .post("http://localhost:11434/api/embeddings")
        .json(&embed_body)
        .send()
        .await;

    if let Ok(resp) = response {
        if resp.status().is_success() {
            return Ok(true);
        }
    }

    // If embedding fails, try generate (for LLM models)
    let gen_body = serde_json::json!({
        "model": model,
        "prompt": "",
        "keep_alive": -1
    });

    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&gen_body)
        .send()
        .await
        .map_err(|e| format!("Failed to load model: {}", e))?;

    Ok(response.status().is_success())
}

/// Unload a model from Ollama VRAM
#[tauri::command]
pub async fn unload_model(model: String) -> Result<bool, String> {
    let client = reqwest_new::Client::new();

    let body = format!(
        r#"{{"model":"{}","prompt":"","keep_alive":0}}"#,
        model
    );

    let response: reqwest_new::Response = client
        .post("http://localhost:11434/api/generate")
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e: reqwest_new::Error| format!("Failed to unload model: {}", e))?;

    Ok(response.status().is_success())
}

/// Ensure only the specified models are loaded (main + embedding)
#[tauri::command]
pub async fn ensure_models_loaded(main_model: String, embedding_model: String) -> Result<LoadedModelsResponse, String> {
    // First, get currently loaded models
    let loaded = get_loaded_models().await?;
    let loaded_names: Vec<&str> = loaded.models.iter().map(|m| m.name.as_str()).collect();

    // Unload models that aren't needed
    for model in &loaded.models {
        if model.name != main_model && model.name != embedding_model {
            let _ = unload_model(model.name.clone()).await;
        }
    }

    // Load embedding model first (smaller, faster)
    if !loaded_names.contains(&embedding_model.as_str()) {
        load_model(embedding_model).await?;
    }

    // Load main model
    if !loaded_names.contains(&main_model.as_str()) {
        load_model(main_model).await?;
    }

    // Return updated list
    get_loaded_models().await
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

    // Get article content and date
    let (title, content, article_date): (String, String, Option<String>) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.conn()
            .query_row(
                "SELECT title, COALESCE(content_full, content_raw, ''), DATE(COALESCE(published_at, fetched_at)) FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
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

    match client.discordian_analysis(&model, &title, &content, &locale).await {
        Ok(analysis) => {
            let db = state.db.lock().map_err(|e| e.to_string())?;

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

            let categories_saved = save_article_categories(db.conn(), fnord_id, &analysis.categories);

            let (tags_saved, tag_ids) = save_article_keywords_and_network(
                db.conn(),
                fnord_id,
                &analysis.keywords,
                &categories_saved,
                article_date.as_deref(),
            );

            recalculate_keyword_weights(db.conn(), &tag_ids);

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

/// Process all unprocessed articles in batch using Discordian Analysis
/// Emits 'batch-progress' events to the window
#[tauri::command]
pub async fn process_batch(
    window: Window,
    state: State<'_, AppState>,
    model: String,
    limit: Option<i64>,
) -> Result<BatchResult, String> {
    let client = OllamaClient::new(None);

    // Get locale for the batch
    let locale = get_locale_from_db(&state);

    // Get unprocessed articles with content (including date for trend tracking)
    let articles: Vec<(i64, String, String, Option<String>)> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        // Use limit if provided, otherwise process all
        let query = match limit {
            Some(n) => format!(
                r#"SELECT id, title, COALESCE(content_full, content_raw, '') as content,
                          DATE(COALESCE(published_at, fetched_at)) as article_date
                   FROM fnords
                   WHERE processed_at IS NULL
                   AND (content_full IS NOT NULL OR content_raw IS NOT NULL)
                   ORDER BY published_at DESC
                   LIMIT {}"#,
                n
            ),
            None => r#"SELECT id, title, COALESCE(content_full, content_raw, '') as content,
                          DATE(COALESCE(published_at, fetched_at)) as article_date
                   FROM fnords
                   WHERE processed_at IS NULL
                   AND (content_full IS NOT NULL OR content_raw IS NOT NULL)
                   ORDER BY published_at DESC"#.to_string(),
        };

        let mut stmt = db.conn().prepare(&query).map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| e.to_string())?;

        rows.filter_map(|r| r.ok()).collect()
    };

    let total = articles.len() as i64;
    let mut succeeded: i64 = 0;
    let mut failed: i64 = 0;

    // Reset cancel flag at start
    state.batch_cancel.store(false, Ordering::SeqCst);

    // Emit initial progress event so UI knows batch has started
    if total > 0 {
        let _ = window.emit(
            "batch-progress",
            BatchProgress {
                current: 0,
                total,
                fnord_id: 0,
                title: "Starting...".to_string(),
                success: true,
                error: None,
            },
        );
    }

    for (idx, (fnord_id, title, content, article_date)) in articles.into_iter().enumerate() {
        // Check for cancellation
        if state.batch_cancel.load(Ordering::SeqCst) {
            let _ = window.emit(
                "batch-progress",
                BatchProgress {
                    current: (idx + 1) as i64,
                    total,
                    fnord_id: 0,
                    title: "Cancelled".to_string(),
                    success: false,
                    error: Some("Batch cancelled by user".to_string()),
                },
            );
            break;
        }

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

        // Use Discordian Analysis for full processing (summary + bias + categories + keywords)
        let analysis_result = client.discordian_analysis(&model, &title, &content, &locale).await;

        let (success, error) = match analysis_result {
            Ok(analysis) => {
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
                        &analysis.summary,
                        analysis.political_bias,
                        analysis.sachlichkeit,
                        &analysis.article_type,
                        fnord_id,
                    ),
                );

                if update_result.is_err() {
                    failed += 1;
                    (false, Some("DB update failed".to_string()))
                } else {
                    let categories_saved = save_article_categories(db.conn(), fnord_id, &analysis.categories);

                    let (_tags_saved, _tag_ids) = save_article_keywords_and_network(
                        db.conn(),
                        fnord_id,
                        &analysis.keywords,
                        &categories_saved,
                        article_date.as_deref(),
                    );

                    succeeded += 1;
                    (true, None)
                }
            }
            Err(e) => {
                failed += 1;
                (false, Some(e.to_string()))
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

/// Cancel ongoing batch processing
#[tauri::command]
pub fn cancel_batch(state: State<AppState>) -> Result<(), String> {
    state.batch_cancel.store(true, Ordering::SeqCst);
    Ok(())
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
