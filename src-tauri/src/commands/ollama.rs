use crate::embedding_worker;
use crate::ollama::{
    BiasAnalysis, DiscordianAnalysis, OllamaClient, OllamaError, DEFAULT_ANALYSIS_PROMPT, DEFAULT_SUMMARY_PROMPT,
    RECOMMENDED_MAIN_MODEL, RECOMMENDED_EMBEDDING_MODEL, get_language_for_locale, DEFAULT_NUM_CTX,
};
use crate::db::Database;
use crate::{extract_keywords, classify_by_keywords, normalize_keyword, find_canonical_keyword, SEPHIROTH_CATEGORIES};
use crate::AppState;
use log::info;
use std::sync::atomic::Ordering;
use tauri::{Emitter, Manager, State, Window};
use futures::{stream, StreamExt};

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
fn save_article_keywords_and_network(
    conn: &rusqlite::Connection,
    fnord_id: i64,
    keywords: &[String],
    categories_saved: &[String],
    article_date: Option<&str>,
) -> (Vec<String>, Vec<i64>) {
    let mut tags_saved = Vec::new();
    let mut tag_ids: Vec<i64> = Vec::new();
    let mut new_keyword_ids: Vec<i64> = Vec::new();

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
        let keyword = match normalize_keyword(keyword) {
            Some(k) => k,
            None => continue,
        };

        let canonical = find_canonical_keyword(&keyword);
        let store_keyword = canonical.unwrap_or(&keyword);

        let existing_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM immanentize WHERE LOWER(name) = LOWER(?)",
                [store_keyword],
                |row| row.get(0),
            )
            .ok();

        let is_new_keyword = existing_id.is_none();
        let is_new_for_article = existing_id
            .map(|id| !existing_tag_ids.contains(&id))
            .unwrap_or(true);

        if is_new_for_article {
            if existing_id.is_some() {
                conn.execute(
                    r#"UPDATE immanentize SET
                           count = count + 1,
                           article_count = article_count + 1,
                           last_used = CURRENT_TIMESTAMP
                       WHERE LOWER(name) = LOWER(?1)"#,
                    [store_keyword],
                )
                .ok();
            } else {
                conn.execute(
                    r#"INSERT INTO immanentize (name, count, article_count, first_seen, last_used, is_canonical)
                       VALUES (?1, 1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, TRUE)"#,
                    [store_keyword],
                )
                .ok();
            }
        } else {
            conn.execute(
                r#"UPDATE immanentize SET
                       count = count + 1,
                       last_used = CURRENT_TIMESTAMP
                   WHERE LOWER(name) = LOWER(?1)"#,
                [store_keyword],
            )
            .ok();
        }

        if let Ok(tag_id) = conn.query_row::<i64, _, _>(
            "SELECT id FROM immanentize WHERE LOWER(name) = LOWER(?)",
            [store_keyword],
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

            if is_new_keyword {
                new_keyword_ids.push(tag_id);
            }

            tags_saved.push(keyword.to_string());
            tag_ids.push(tag_id);
        }
    }

    for keyword_id in &new_keyword_ids {
        conn.execute(
            r#"INSERT OR IGNORE INTO embedding_queue (immanentize_id, priority, queued_at)
               VALUES (?1, 0, CURRENT_TIMESTAMP)"#,
            [keyword_id],
        )
        .ok();
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

fn merge_keywords(llm_keywords: &[String], local_keywords: Vec<String>, max_count: usize) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut merged = Vec::new();

    for kw in llm_keywords.iter().chain(local_keywords.iter()) {
        let normalized = kw.to_lowercase();
        if !normalized.is_empty() && normalized.len() >= 2 && seen.insert(normalized) {
            merged.push(kw.clone());
            if merged.len() >= max_count {
                break;
            }
        }
    }

    merged
}

fn validate_and_merge_categories(
    llm_categories: &[String],
    local_categories: Vec<String>,
) -> Vec<String> {
    let valid_llm: Vec<String> = llm_categories
        .iter()
        .filter(|c| SEPHIROTH_CATEGORIES.iter().any(|s| s.to_lowercase() == c.to_lowercase()))
        .cloned()
        .collect();

    if valid_llm.is_empty() {
        local_categories
    } else {
        let mut seen = std::collections::HashSet::new();
        valid_llm
            .into_iter()
            .chain(local_categories)
            .filter(|c| seen.insert(c.to_lowercase()))
            .take(5)
            .collect()
    }
}

/// Get num_ctx setting from database, falling back to DEFAULT_NUM_CTX
fn get_num_ctx_setting(db: &Database) -> u32 {
    db.conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'ollama_num_ctx'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_NUM_CTX)
}

/// Create OllamaClient with num_ctx from settings
fn create_ollama_client(db: &Database) -> OllamaClient {
    let num_ctx = get_num_ctx_setting(db);
    OllamaClient::with_context(None, num_ctx)
}

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
    let loaded = get_loaded_models().await?;
    let loaded_names: Vec<&str> = loaded.models.iter().map(|m| m.name.as_str()).collect();

    for model in &loaded.models {
        if model.name != main_model && model.name != embedding_model {
            let _ = unload_model(model.name.clone()).await;
        }
    }

    if !loaded_names.contains(&embedding_model.as_str()) {
        load_model(embedding_model).await?;
    }

    if !loaded_names.contains(&main_model.as_str()) {
        load_model(main_model).await?;
    }

    get_loaded_models().await
}

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

#[tauri::command]
pub async fn generate_summary(
    state: State<'_, AppState>,
    fnord_id: i64,
    model: String,
) -> Result<SummaryResponse, String> {
    let locale = get_locale_from_db(&state);
    let prompt_template = get_summary_prompt(&state, &locale);

    let (client, content): (OllamaClient, String) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let client = create_ollama_client(&db);
        let content = db.conn()
            .query_row(
                "SELECT COALESCE(content_full, '') FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        (client, content)
    };

    if content.is_empty() {
        return Ok(SummaryResponse {
            fnord_id,
            success: false,
            summary: None,
            error: Some("No content available".to_string()),
        });
    }

    match client.summarize_with_prompt(&model, &content, &prompt_template).await {
        Ok(summary) => {
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

#[tauri::command]
pub async fn analyze_article(
    state: State<'_, AppState>,
    fnord_id: i64,
    model: String,
) -> Result<AnalysisResponse, String> {
    let locale = get_locale_from_db(&state);
    let prompt_template = get_analysis_prompt(&state, &locale);

    let (client, title, content): (OllamaClient, String, String) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let client = create_ollama_client(&db);
        let (title, content) = db.conn()
            .query_row(
                "SELECT title, COALESCE(content_full, '') FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        (client, title, content)
    };

    if content.is_empty() {
        return Ok(AnalysisResponse {
            fnord_id,
            success: false,
            analysis: None,
            error: Some("No content available".to_string()),
        });
    }

    match client.analyze_bias_with_prompt(&model, &title, &content, &prompt_template).await {
        Ok(analysis) => {
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

#[tauri::command]
pub async fn process_article(
    state: State<'_, AppState>,
    fnord_id: i64,
    model: String,
) -> Result<(SummaryResponse, AnalysisResponse), String> {
    let locale = get_locale_from_db(&state);
    let summary_prompt_template = get_summary_prompt(&state, &locale);
    let analysis_prompt_template = get_analysis_prompt(&state, &locale);

    let (client, title, content): (OllamaClient, String, String) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let client = create_ollama_client(&db);
        let (title, content) = db.conn()
            .query_row(
                "SELECT title, COALESCE(content_full, '') FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        (client, title, content)
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

    let model_clone = model.clone();
    let content_clone = content.clone();
    let summary_future = client.summarize_with_prompt(&model, &content, &summary_prompt_template);
    let analysis_future = client.analyze_bias_with_prompt(&model_clone, &title, &content_clone, &analysis_prompt_template);

    let (summary_result, analysis_result) = tokio::join!(summary_future, analysis_future);

    let summary_response = match summary_result {
        Ok(summary) => {
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

#[tauri::command]
pub async fn process_article_discordian(
    state: State<'_, AppState>,
    fnord_id: i64,
    model: String,
) -> Result<DiscordianResponse, String> {
    let locale = get_locale_from_db(&state);

    let (client, title, content, article_date): (OllamaClient, String, String, Option<String>) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let client = create_ollama_client(&db);
        let (title, content, article_date) = db.conn()
            .query_row(
                "SELECT title, COALESCE(content_full, ''), DATE(COALESCE(published_at, fetched_at)) FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| e.to_string())?;
        (client, title, content, article_date)
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

    let local_keywords = extract_keywords(&title, &content, 10);
    let local_categories = classify_by_keywords(&local_keywords);

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

            let merged_categories = validate_and_merge_categories(&analysis.categories, local_categories);
            let categories_saved = save_article_categories(db.conn(), fnord_id, &merged_categories);

            let merged_keywords = merge_keywords(&analysis.keywords, local_keywords, 15);
            let (tags_saved, tag_ids) = save_article_keywords_and_network(
                db.conn(),
                fnord_id,
                &merged_keywords,
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
        Err(e) => {
            let db = state.db.lock().map_err(|e| e.to_string())?;

            let categories_saved = save_article_categories(db.conn(), fnord_id, &local_categories);
            let (tags_saved, tag_ids) = save_article_keywords_and_network(
                db.conn(),
                fnord_id,
                &local_keywords,
                &categories_saved,
                article_date.as_deref(),
            );
            recalculate_keyword_weights(db.conn(), &tag_ids);

            Ok(DiscordianResponse {
                fnord_id,
                success: false,
                analysis: None,
                categories_saved,
                tags_saved,
                error: Some(format!("LLM failed, used local extraction: {}", e)),
            })
        }
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

#[derive(serde::Serialize)]
pub struct HopelessCount {
    pub count: i64,
}

#[derive(serde::Serialize)]
pub struct FailedCount {
    pub count: i64,
}

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
               AND content_full IS NOT NULL AND content_full != ''
               AND (analysis_hopeless IS NULL OR analysis_hopeless = FALSE)"#,
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(UnprocessedCount { total, with_content })
}

// ------------------------------------------------------------
// PARALLEL BATCH PROCESSING IMPL
// ------------------------------------------------------------

fn get_ai_concurrency(state: &AppState) -> usize {
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return 1,
    };
    let val: String = db.conn()
        .query_row("SELECT value FROM settings WHERE key = 'ai_parallelism'", [], |row| row.get(0))
        .unwrap_or_else(|_| "1".to_string());
    
    val.parse().unwrap_or(1).clamp(1, 10)
}

struct BatchArticle {
    fnord_id: i64,
    title: String,
    content: String,
    article_date: Option<String>,
    attempts: i64,
    previous_error: Option<String>,
}

async fn process_single_article(
    client: &OllamaClient,
    state: &AppState,
    model: &str,
    locale: &str,
    article: BatchArticle,
) -> (bool, Option<String>) {
    let fnord_id = article.fnord_id;

    if article.content.is_empty() {
        return (false, Some("No content".to_string()));
    }

    let local_keywords = extract_keywords(&article.title, &article.content, 10);
    let local_categories = classify_by_keywords(&local_keywords);

    let analysis_result = client
        .discordian_analysis_with_retry(model, &article.title, &article.content, locale, article.previous_error.as_deref())
        .await;

    match analysis_result {
        Ok(analysis) => {
            let db = match state.db.lock() {
                Ok(db) => db,
                Err(e) => return (false, Some(format!("Database lock failed: {}", e))),
            };

            let update_result = db.conn().execute(
                r#"UPDATE fnords SET
                    summary = ?1,
                    political_bias = ?2,
                    sachlichkeit = ?3,
                    article_type = ?4,
                    processed_at = CURRENT_TIMESTAMP,
                    analysis_attempts = 0,
                    analysis_error = NULL
                WHERE id = ?5"#,
                (
                    &analysis.summary,
                    analysis.political_bias,
                    analysis.sachlichkeit,
                    &analysis.article_type,
                    fnord_id,
                ),
            );

            if let Err(e) = update_result {
                return (false, Some(format!("DB update failed: {}", e)));
            }

            let merged_categories = validate_and_merge_categories(&analysis.categories, local_categories);
            let categories_saved = save_article_categories(db.conn(), fnord_id, &merged_categories);

            let merged_keywords = merge_keywords(&analysis.keywords, local_keywords, 15);
            let (_tags_saved, _tag_ids) = save_article_keywords_and_network(
                db.conn(),
                fnord_id,
                &merged_keywords,
                &categories_saved,
                article.article_date.as_deref(),
            );

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
                    if new_attempts >= 2 {
                        (format!("JSON parse error after {} attempts: {}", new_attempts, message), true)
                    } else {
                        (message.clone(), false)
                    }
                }
                other => {
                    if new_attempts >= 2 {
                        (format!("Analysis failed after {} attempts: {}", new_attempts, other), true)
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

            if !local_categories.is_empty() || !local_keywords.is_empty() {
                let categories_saved = save_article_categories(db.conn(), fnord_id, &local_categories);
                let _ = save_article_keywords_and_network(
                    db.conn(),
                    fnord_id,
                    &local_keywords,
                    &categories_saved,
                    article.article_date.as_deref(),
                );
            }

            let status_msg = if is_hopeless {
                format!("Marked hopeless: {}", error_msg)
            } else {
                format!("Attempt {}/2 failed, will retry: {}", new_attempts, error_msg)
            };
            (false, Some(status_msg))
        }
    }
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
    let locale = get_locale_from_db(&state);
    let concurrency = get_ai_concurrency(&state);
    
    info!("Starting batch processing with concurrency: {}", concurrency);

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
                   AND content_full IS NOT NULL AND content_full != ''
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
                   AND content_full IS NOT NULL AND content_full != ''
                   AND (analysis_hopeless IS NULL OR analysis_hopeless = FALSE)
                   ORDER BY published_at DESC"#.to_string(),
        };

        let mut stmt = db.conn().prepare(&query).map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
             Ok(BatchArticle {
                 fnord_id: row.get(0)?,
                 title: row.get(1)?,
                 content: row.get(2)?,
                 article_date: row.get(3)?,
                 attempts: row.get(4)?,
                 previous_error: row.get(5)?,
             })
        }).map_err(|e| e.to_string())?;

        (rows.filter_map(|r| r.ok()).collect(), num_ctx)
    };

    let total = articles.len() as i64;
    
    // Check if empty
    if total == 0 {
        return Ok(BatchResult { processed: 0, succeeded: 0, failed: 0 });
    }

    state.batch_cancel.store(false, Ordering::SeqCst);
    state.batch_running.store(true, Ordering::SeqCst);

    let _ = window.emit("batch-progress", BatchProgress {
        current: 0,
        total,
        fnord_id: 0,
        title: format!("Starting batch ({} parallel)...", concurrency),
        success: true,
        error: None,
    });

    // Create stream for parallel processing
    let stream = stream::iter(articles.into_iter().enumerate());
    
    let results = stream.map(|(idx, article)| {
        let title = article.title.clone();
        let fnord_id = article.fnord_id;
        let model = model.clone();
        let locale = locale.clone();
        let state = state.clone();
        let window = window.clone();
        
        async move {
            if state.batch_cancel.load(Ordering::SeqCst) {
                return (idx, title, fnord_id, false, Some("Cancelled".to_string()));
            }

            // Re-create client in each future with configured num_ctx
            let client = OllamaClient::with_context(None, num_ctx);
            
            let (success, error) = process_single_article(&client, &state, &model, &locale, article).await;
            
            // Emit progress immediately
             let _ = window.emit("batch-progress", BatchProgress {
                current: (idx + 1) as i64,
                total,
                fnord_id,
                title: title.clone(),
                success,
                error: error.clone(),
            });
            
            (idx, title, fnord_id, success, error)
        }
    })
    .buffer_unordered(concurrency) // Run 'concurrency' futures in parallel
    .collect::<Vec<_>>()
    .await;
    
    // Calculate stats
    let mut succeeded = 0;
    let mut failed = 0;
    for (_, _, _, success, _) in &results {
        if *success { succeeded += 1; } else { failed += 1; }
    }

    // Embeddings queue processing (after batch)
    if succeeded > 0 && !state.batch_cancel.load(Ordering::SeqCst) {
        let queue_size = {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            db.conn().query_row("SELECT COUNT(*) FROM embedding_queue", [], |row| row.get::<_, i64>(0)).unwrap_or(0)
        };

        if queue_size > 0 {
             let _ = window.emit("embedding-progress", embedding_worker::EmbeddingProgress {
                queue_size, total: queue_size, processed: 0, failed: 0, is_processing: true
            });

            let _ = embedding_worker::process_embedding_queue(
                state.db.clone(),
                Some(&window.app_handle()),
                queue_size,
                Some(queue_size),
            ).await;

             let _ = window.emit("embedding-progress", embedding_worker::EmbeddingProgress {
                queue_size: 0, total: queue_size, processed: queue_size, failed: 0, is_processing: false
            });
        }
    }

    state.batch_running.store(false, Ordering::SeqCst);

    Ok(BatchResult {
        processed: total,
        succeeded,
        failed,
    })
}

#[tauri::command]
pub fn cancel_batch(state: State<AppState>) -> Result<(), String> {
    state.batch_cancel.store(true, Ordering::SeqCst);
    Ok(())
}

// ============================================================
// MODEL MANAGEMENT
// ============================================================

#[tauri::command]
pub async fn pull_model(window: Window, model: String) -> Result<ModelPullResult, String> {
    let client = OllamaClient::new(None);

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

#[tauri::command]
pub fn get_default_prompts() -> DefaultPrompts {
    DefaultPrompts {
        summary_prompt: DEFAULT_SUMMARY_PROMPT.to_string(),
        analysis_prompt: DEFAULT_ANALYSIS_PROMPT.to_string(),
    }
}

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

#[derive(serde::Serialize)]
pub struct ResetForReprocessingResult {
    pub reset_count: i64,
}

#[tauri::command]
pub fn reset_articles_for_reprocessing(
    state: State<AppState>,
    only_with_content: Option<bool>,
) -> Result<ResetForReprocessingResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let only_with_content = only_with_content.unwrap_or(true);

    let sql = if only_with_content {
        r#"UPDATE fnords SET processed_at = NULL
           WHERE content_full IS NOT NULL AND content_full != ''"#
    } else {
        "UPDATE fnords SET processed_at = NULL"
    };

    let reset_count = db.conn().execute(sql, []).map_err(|e| e.to_string())? as i64;

    Ok(ResetForReprocessingResult { reset_count })
}
