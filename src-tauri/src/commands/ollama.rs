use crate::commands::settings::get_embedding_model_from_db;
use crate::embedding_worker;
use crate::embeddings::embedding_to_blob;
use crate::ollama::{
    BiasAnalysis, DiscordianAnalysis, DiscordianAnalysisWithRejections, OllamaClient, OllamaError,
    DEFAULT_ANALYSIS_PROMPT, DEFAULT_SUMMARY_PROMPT,
    RECOMMENDED_MAIN_MODEL, RECOMMENDED_EMBEDDING_MODEL, get_language_for_locale, DEFAULT_NUM_CTX,
};
use crate::text_analysis::{BiasWeights, TfIdfExtractor, CategoryMatcher, record_correction, CorrectionRecord, CorrectionType};
use crate::db::Database;
use crate::{extract_keywords, classify_by_keywords, normalize_keyword, find_canonical_keyword, SEPHIROTH_CATEGORIES};
use crate::AppState;
use log::{debug, info, warn};
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
// SOURCE TRACKING STRUCTURES
// ============================================================

/// Keyword with source tracking (statistical vs ai)
#[derive(Debug, Clone)]
pub struct KeywordWithSource {
    pub name: String,
    pub source: String, // 'ai', 'statistical'
    pub confidence: f64,
}

/// Category with source tracking (statistical vs ai)
#[derive(Debug, Clone)]
pub struct CategoryWithSource {
    pub name: String,
    pub source: String, // 'ai', 'statistical'
    pub confidence: f64,
}

/// Determine source for each keyword by comparing with statistical suggestions
fn determine_keyword_sources(
    final_keywords: &[String],
    stat_keywords: &[String],
) -> Vec<KeywordWithSource> {
    let stat_lower: std::collections::HashSet<String> = stat_keywords
        .iter()
        .map(|k| k.to_lowercase())
        .collect();

    final_keywords
        .iter()
        .map(|k| {
            let source = if stat_lower.contains(&k.to_lowercase()) {
                "statistical"
            } else {
                "ai"
            };
            KeywordWithSource {
                name: k.clone(),
                source: source.to_string(),
                confidence: if source == "statistical" { 0.8 } else { 1.0 },
            }
        })
        .collect()
}

/// Determine source for each category by comparing with statistical suggestions
fn determine_category_sources(
    final_categories: &[String],
    stat_categories: &[(String, f64)],
) -> Vec<CategoryWithSource> {
    let stat_map: std::collections::HashMap<String, f64> = stat_categories
        .iter()
        .map(|(name, conf)| (name.to_lowercase(), *conf))
        .collect();

    final_categories
        .iter()
        .map(|c| {
            let lower = c.to_lowercase();
            if let Some(&conf) = stat_map.get(&lower) {
                CategoryWithSource {
                    name: c.clone(),
                    source: "statistical".to_string(),
                    confidence: conf,
                }
            } else {
                CategoryWithSource {
                    name: c.clone(),
                    source: "ai".to_string(),
                    confidence: 1.0,
                }
            }
        })
        .collect()
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
    // Convert to CategoryWithSource with default 'ai' source
    let cats_with_source: Vec<CategoryWithSource> = categories
        .iter()
        .map(|c| CategoryWithSource {
            name: c.clone(),
            source: "ai".to_string(),
            confidence: 1.0,
        })
        .collect();
    save_article_categories_with_source(conn, fnord_id, &cats_with_source)
}

/// Save categories with source tracking (statistical vs ai)
fn save_article_categories_with_source(
    conn: &rusqlite::Connection,
    fnord_id: i64,
    categories: &[CategoryWithSource],
) -> Vec<String> {
    let mut saved = Vec::new();

    conn.execute("DELETE FROM fnord_sephiroth WHERE fnord_id = ?", [fnord_id])
        .ok();

    for cat in categories {
        if let Ok(sephiroth_id) = conn.query_row::<i64, _, _>(
            "SELECT id FROM sephiroth WHERE LOWER(name) = LOWER(?)",
            [&cat.name],
            |row| row.get(0),
        ) {
            conn.execute(
                r#"INSERT OR IGNORE INTO fnord_sephiroth
                   (fnord_id, sephiroth_id, confidence, source, assigned_at)
                   VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)"#,
                rusqlite::params![fnord_id, sephiroth_id, cat.confidence, &cat.source],
            )
            .ok();

            conn.execute(
                "UPDATE sephiroth SET article_count = (SELECT COUNT(*) FROM fnord_sephiroth WHERE sephiroth_id = ?) WHERE id = ?",
                rusqlite::params![sephiroth_id, sephiroth_id],
            )
            .ok();

            saved.push(cat.name.clone());
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
    // Convert to KeywordWithSource with default 'ai' source
    let kws_with_source: Vec<KeywordWithSource> = keywords
        .iter()
        .map(|k| KeywordWithSource {
            name: k.clone(),
            source: "ai".to_string(),
            confidence: 1.0,
        })
        .collect();
    save_article_keywords_with_source(conn, fnord_id, &kws_with_source, categories_saved, article_date)
}

/// Save keywords with source tracking (statistical vs ai)
fn save_article_keywords_with_source(
    conn: &rusqlite::Connection,
    fnord_id: i64,
    keywords: &[KeywordWithSource],
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

    for kw in keywords {
        let keyword = match normalize_keyword(&kw.name) {
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
            // Insert with source and confidence tracking
            conn.execute(
                "INSERT OR IGNORE INTO fnord_immanentize (fnord_id, immanentize_id, source, confidence) VALUES (?, ?, ?, ?)",
                rusqlite::params![fnord_id, tag_id, &kw.source, kw.confidence],
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

// ============================================================
// ARTICLE EMBEDDING FUNCTIONS (Phase 3)
// ============================================================

/// Save an article embedding to the database (fnords.embedding + vec_fnords)
fn save_article_embedding(
    conn: &rusqlite::Connection,
    fnord_id: i64,
    embedding: &[f32],
) -> Result<(), String> {
    let blob = embedding_to_blob(embedding);

    // Update the article with embedding
    conn.execute(
        "UPDATE fnords SET embedding = ?1, embedding_at = datetime('now') WHERE id = ?2",
        rusqlite::params![blob, fnord_id],
    )
    .map_err(|e| format!("Failed to save article embedding: {}", e))?;

    // Insert into vec0 virtual table for fast similarity search
    // Use REPLACE to handle re-embeddings
    conn.execute(
        "INSERT OR REPLACE INTO vec_fnords (fnord_id, embedding) VALUES (?1, ?2)",
        rusqlite::params![fnord_id, blob],
    )
    .map_err(|e| {
        warn!("Failed to update vec_fnords: {}", e);
        e.to_string()
    })
    .ok();

    Ok(())
}

/// Generate and save embedding for an article
/// Uses title + first ~500 chars of content for embedding
async fn generate_and_save_article_embedding(
    client: &OllamaClient,
    db: &std::sync::Arc<std::sync::Mutex<Database>>,
    fnord_id: i64,
    title: &str,
    content: &str,
) -> Result<(), String> {
    // Get embedding model from settings
    let model = {
        let db_guard = db.lock().map_err(|e| e.to_string())?;
        get_embedding_model_from_db(db_guard.conn())
    };

    // Create embedding text: title + truncated content
    // Use first ~500 chars of content for a compact but representative embedding
    let content_preview: String = content.chars().take(500).collect();
    let embedding_text = format!("{}\n\n{}", title, content_preview);

    // Generate embedding
    let embedding = client
        .generate_embedding(&model, &embedding_text)
        .await
        .map_err(|e| format!("Embedding generation failed: {}", e))?;

    // Save to database
    {
        let db_guard = db.lock().map_err(|e| e.to_string())?;
        save_article_embedding(db_guard.conn(), fnord_id, &embedding)?;
    }

    debug!("Generated embedding for article {}", fnord_id);
    Ok(())
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
                        processed_at = CURRENT_TIMESTAMP
                    WHERE id = ?3"#,
                    (
                        analysis.political_bias,
                        analysis.sachlichkeit,
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
                    processed_at = CURRENT_TIMESTAMP
                WHERE id = ?3"#,
                (
                    analysis.political_bias,
                    analysis.sachlichkeit,
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

    // Step 1: Load article content and bias weights
    let (client, title, content, article_date, bias_weights): (OllamaClient, String, String, Option<String>, BiasWeights) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let client = create_ollama_client(&db);
        let (title, content, article_date) = db.conn()
            .query_row(
                "SELECT title, COALESCE(content_full, ''), DATE(COALESCE(published_at, fetched_at)) FROM fnords WHERE id = ?1",
                [fnord_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| e.to_string())?;
        let bias = BiasWeights::load_from_db(db.conn()).unwrap_or_default();
        (client, title, content, article_date, bias)
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

    // Step 2: Run statistical pre-analysis with bias weights
    let text_for_analysis = format!("{} {}", title, content);

    // TF-IDF keyword extraction with bias
    let extractor = TfIdfExtractor::new().with_max_keywords(15);
    let stat_keywords: Vec<String> = extractor
        .extract_simple(&text_for_analysis)
        .into_iter()
        .map(|kc| {
            let _adjusted_score = bias_weights.apply_to_keyword(&kc.term, kc.score);
            kc.term
        })
        .collect();

    // Category matching with bias
    let matcher = CategoryMatcher::new().with_max_categories(5);
    let stat_categories: Vec<(String, f64)> = matcher
        .score_categories(&text_for_analysis, Some(&bias_weights))
        .into_iter()
        .map(|cs| (cs.name.clone(), cs.confidence))
        .collect();

    // Fallback local extraction (existing method)
    let local_keywords = extract_keywords(&title, &content, 10);
    let local_categories = classify_by_keywords(&local_keywords);

    // Step 3: Run LLM analysis with statistical context
    match client.discordian_analysis_with_stats(
        &model,
        &title,
        &content,
        &locale,
        &stat_keywords,
        &stat_categories,
    ).await {
        Ok(analysis_with_rejections) => {
            // Step 4: Learn from LLM rejections (update bias weights)
            {
                let db = state.db.lock().map_err(|e| e.to_string())?;

                // Record rejected keywords for bias learning
                for rejected_kw in &analysis_with_rejections.rejected_keywords {
                    let _ = record_correction(db.conn(), &CorrectionRecord {
                        fnord_id,
                        correction_type: CorrectionType::KeywordRemoved,
                        old_value: Some(rejected_kw.clone()),
                        new_value: None,
                        matching_terms: vec![],
                        category_id: None,
                    });
                }

                // Record rejected categories for bias learning
                for rejected_cat in &analysis_with_rejections.rejected_categories {
                    // Find category ID by name from database
                    let cat_id: Option<i64> = db.conn()
                        .query_row(
                            "SELECT id FROM sephiroth WHERE LOWER(name) = LOWER(?1)",
                            [rejected_cat],
                            |row| row.get(0),
                        )
                        .ok();

                    if let Some(cat_id) = cat_id {
                        // Use top stat_keywords as matching terms for this category
                        let matching_terms: Vec<String> = stat_keywords.iter().take(5).cloned().collect();

                        let _ = record_correction(db.conn(), &CorrectionRecord {
                            fnord_id,
                            correction_type: CorrectionType::CategoryRemoved,
                            old_value: Some(rejected_cat.clone()),
                            new_value: None,
                            matching_terms,
                            category_id: Some(cat_id),
                        });
                    }
                }
            }

            // Convert to regular analysis for storage
            let analysis: DiscordianAnalysis = analysis_with_rejections.clone().into();

            // Save to database
            {
                let db = state.db.lock().map_err(|e| e.to_string())?;

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
                let db = state.db.lock().map_err(|e| e.to_string())?;

                // Merge and determine category sources
                let merged_categories = validate_and_merge_categories(&analysis.categories, local_categories);
                let categories_with_source = determine_category_sources(&merged_categories, &stat_categories);
                let categories_saved = save_article_categories_with_source(db.conn(), fnord_id, &categories_with_source);

                // Merge and determine keyword sources
                let merged_keywords = merge_keywords(&analysis.keywords, local_keywords.clone(), 15);
                let keywords_with_source = determine_keyword_sources(&merged_keywords, &stat_keywords);
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

            // Generate article embedding (async, after DB lock is released)
            if let Err(e) = generate_and_save_article_embedding(
                &client,
                &state.db,
                fnord_id,
                &title,
                &content,
            ).await {
                warn!("Failed to generate embedding for article {}: {}", fnord_id, e);
                // Don't fail the whole operation - embedding is optional
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
            // Fallback: Use statistical + local extraction without LLM
            let db = state.db.lock().map_err(|e| e.to_string())?;

            // Merge statistical keywords with local extraction
            let combined_keywords: Vec<String> = stat_keywords.into_iter()
                .chain(local_keywords.into_iter())
                .take(15)
                .collect();

            let categories_saved = save_article_categories(db.conn(), fnord_id, &local_categories);
            let (tags_saved, tag_ids) = save_article_keywords_and_network(
                db.conn(),
                fnord_id,
                &combined_keywords,
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
                error: Some(format!("LLM failed, used statistical + local extraction: {}", e)),
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

    // Require minimum 100 chars to match batch processing filter
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
    let title = article.title.clone();
    let content = article.content.clone();

    if content.is_empty() {
        return (false, Some("No content".to_string()));
    }

    let local_keywords = extract_keywords(&title, &content, 10);
    let local_categories = classify_by_keywords(&local_keywords);

    let analysis_result = client
        .discordian_analysis_with_retry(model, &title, &content, locale, article.previous_error.as_deref())
        .await;

    match analysis_result {
        Ok(analysis) => {
            // Save analysis to DB (hold lock briefly)
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
            }

            // Generate article embedding (async, after DB lock is released)
            if let Err(e) = generate_and_save_article_embedding(
                client,
                &state.db,
                fnord_id,
                &title,
                &content,
            ).await {
                // Log warning but don't fail - embedding is optional
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

            // Mark as hopeless after 3 failed attempts (allows 1x, 1.5x, 2x context retries)
            let (error_msg, is_hopeless) = match &e {
                OllamaError::JsonParseError { message, .. } => {
                    if new_attempts >= 3 {
                        (format!("JSON parse error after {} attempts: {}", new_attempts, message), true)
                    } else {
                        (message.clone(), false)
                    }
                }
                other => {
                    if new_attempts >= 3 {
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
                format!("Attempt {}/3 failed, will retry: {}", new_attempts, error_msg)
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

        // Require minimum 100 chars to filter out failed retrievals (e.g. "<!DOCTYPE html>")
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

            // Calculate num_ctx multiplier based on retry attempts
            // 0 attempts (first try): 1.0x, 1 attempt (second try): 1.5x, 2+ attempts (third try): 2.0x
            let (ctx_multiplier, adjusted_num_ctx) = match article.attempts {
                0 => (1.0, num_ctx),
                1 => (1.5, ((num_ctx as f64) * 1.5) as u32),
                _ => (2.0, num_ctx * 2),
            };

            if article.attempts > 0 {
                info!(
                    "Retry {}/3 for article {}: using {}x context (num_ctx={})",
                    article.attempts + 1, fnord_id, ctx_multiplier, adjusted_num_ctx
                );
            }

            let client = OllamaClient::with_context(None, adjusted_num_ctx);

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

/// Reset articles for reprocessing.
/// This clears processed_at, analysis_hopeless, analysis_attempts, and analysis_error
/// so that ALL articles can be re-analyzed, including previously failed ones.
#[tauri::command]
pub fn reset_articles_for_reprocessing(
    state: State<AppState>,
    only_with_content: Option<bool>,
) -> Result<ResetForReprocessingResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let only_with_content = only_with_content.unwrap_or(true);

    // Reset all analysis-related fields so articles can be fully re-analyzed
    let sql = if only_with_content {
        r#"UPDATE fnords SET
           processed_at = NULL,
           analysis_hopeless = FALSE,
           analysis_attempts = 0,
           analysis_error = NULL
           WHERE content_full IS NOT NULL AND content_full != ''"#
    } else {
        r#"UPDATE fnords SET
           processed_at = NULL,
           analysis_hopeless = FALSE,
           analysis_attempts = 0,
           analysis_error = NULL"#
    };

    let reset_count = db.conn().execute(sql, []).map_err(|e| e.to_string())? as i64;
    info!("Reset {} articles for reprocessing (hopeless flags cleared)", reset_count);

    Ok(ResetForReprocessingResult { reset_count })
}

// ============================================================
// SIMILAR ARTICLES (Phase 3)
// ============================================================

#[derive(serde::Serialize, Clone)]
pub struct SimilarArticleTag {
    pub id: i64,
    pub name: String,
}

#[derive(serde::Serialize, Clone)]
pub struct SimilarArticleCategory {
    pub id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct SimilarArticle {
    pub fnord_id: i64,
    pub title: String,
    pub pentacle_title: Option<String>,
    pub published_at: Option<String>,
    pub similarity: f64,
    pub tags: Vec<SimilarArticleTag>,
    pub categories: Vec<SimilarArticleCategory>,
}

#[derive(serde::Serialize)]
pub struct SimilarArticlesResponse {
    pub fnord_id: i64,
    pub similar: Vec<SimilarArticle>,
}

/// Helper function to get tags for an article
fn get_article_tags(conn: &rusqlite::Connection, fnord_id: i64) -> Vec<SimilarArticleTag> {
    let mut stmt = match conn.prepare(
        r#"SELECT i.id, i.name
           FROM immanentize i
           JOIN fnord_immanentize fi ON fi.immanentize_id = i.id
           WHERE fi.fnord_id = ?
           ORDER BY i.article_count DESC
           LIMIT 5"#,
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    stmt.query_map([fnord_id], |row| {
        Ok(SimilarArticleTag {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })
    .map(|iter| iter.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

/// Helper function to get main categories for an article
fn get_article_main_categories(conn: &rusqlite::Connection, fnord_id: i64) -> Vec<SimilarArticleCategory> {
    let mut stmt = match conn.prepare(
        r#"SELECT DISTINCT m.id, m.name, m.icon, m.color
           FROM sephiroth m
           JOIN sephiroth s ON (s.parent_id = m.id OR s.id = m.id)
           JOIN fnord_sephiroth fs ON fs.sephiroth_id = s.id
           WHERE fs.fnord_id = ? AND m.level = 0
           ORDER BY m.name"#,
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    stmt.query_map([fnord_id], |row| {
        Ok(SimilarArticleCategory {
            id: row.get(0)?,
            name: row.get(1)?,
            icon: row.get(2)?,
            color: row.get(3)?,
        })
    })
    .map(|iter| iter.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

/// Find similar articles based on embedding similarity using sqlite-vec
#[tauri::command]
pub fn find_similar_articles(
    state: State<AppState>,
    fnord_id: i64,
    limit: Option<i64>,
) -> Result<SimilarArticlesResponse, String> {
    let limit = limit.unwrap_or(5);
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get the embedding for the source article
    let embedding: Option<Vec<u8>> = db
        .conn()
        .query_row(
            "SELECT embedding FROM fnords WHERE id = ?",
            [fnord_id],
            |row| row.get(0),
        )
        .ok();

    let embedding = match embedding {
        Some(e) if !e.is_empty() => e,
        _ => {
            return Ok(SimilarArticlesResponse {
                fnord_id,
                similar: vec![],
            });
        }
    };

    // Use sqlite-vec to find similar articles
    // vec_fnords uses cosine distance, so lower distance = more similar
    // Distance of 0 = identical, distance of 2 = opposite
    // Convert distance to similarity: similarity = 1 - (distance / 2)
    let mut stmt = db
        .conn()
        .prepare(
            r#"SELECT
                v.fnord_id,
                v.distance,
                f.title,
                p.title as pentacle_title,
                f.published_at
            FROM vec_fnords v
            JOIN fnords f ON f.id = v.fnord_id
            LEFT JOIN pentacles p ON p.id = f.pentacle_id
            WHERE v.embedding MATCH ?1
            AND k = ?2
            AND v.fnord_id != ?3
            ORDER BY v.distance ASC"#,
        )
        .map_err(|e| e.to_string())?;

    // First collect basic article info
    let basic_articles: Vec<(i64, String, Option<String>, Option<String>, f64)> = stmt
        .query_map(
            rusqlite::params![embedding, limit + 1, fnord_id],
            |row| {
                let distance: f64 = row.get(1)?;
                // Convert cosine distance to similarity score
                let similarity = 1.0 - (distance / 2.0);
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    similarity,
                ))
            },
        )
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        // Filter out low similarity results (< 0.5)
        .filter(|(_, _, _, _, similarity)| *similarity >= 0.5)
        .take(limit as usize)
        .collect();

    // Now fetch tags and categories for each article
    let similar: Vec<SimilarArticle> = basic_articles
        .into_iter()
        .map(|(article_id, title, pentacle_title, published_at, similarity)| {
            let tags = get_article_tags(db.conn(), article_id);
            let categories = get_article_main_categories(db.conn(), article_id);
            SimilarArticle {
                fnord_id: article_id,
                title,
                pentacle_title,
                published_at,
                similarity,
                tags,
                categories,
            }
        })
        .collect();

    Ok(SimilarArticlesResponse { fnord_id, similar })
}

// ============================================================
// SEMANTIC SEARCH (Phase 3)
// ============================================================

#[derive(serde::Serialize, Clone)]
pub struct SearchResult {
    pub fnord_id: i64,
    pub title: String,
    pub pentacle_title: Option<String>,
    pub published_at: Option<String>,
    pub summary: Option<String>,
    pub similarity: f64,
}

#[derive(serde::Serialize)]
pub struct SemanticSearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
}

/// Perform semantic search by embedding the query and finding similar articles
#[tauri::command]
pub async fn semantic_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<i64>,
) -> Result<SemanticSearchResponse, String> {
    let limit = limit.unwrap_or(20);

    if query.trim().is_empty() {
        return Ok(SemanticSearchResponse {
            query,
            results: vec![],
        });
    }

    // Get embedding model from settings and generate embedding for query
    let (embedding_model, client) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let model = get_embedding_model_from_db(db.conn());
        (model, OllamaClient::new(None))
    };

    let query_embedding = client
        .generate_embedding(&embedding_model, &query)
        .await
        .map_err(|e| format!("Failed to generate query embedding: {}", e))?;

    let query_blob = embedding_to_blob(&query_embedding);

    // Search using sqlite-vec
    let results: Vec<SearchResult> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        let mut stmt = db
            .conn()
            .prepare(
                r#"SELECT
                    v.fnord_id,
                    v.distance,
                    f.title,
                    p.title as pentacle_title,
                    f.published_at,
                    f.summary
                FROM vec_fnords v
                JOIN fnords f ON f.id = v.fnord_id
                LEFT JOIN pentacles p ON p.id = f.pentacle_id
                WHERE v.embedding MATCH ?1
                AND k = ?2
                ORDER BY v.distance ASC"#,
            )
            .map_err(|e| e.to_string())?;

        let result: Vec<SearchResult> = stmt
            .query_map(
                rusqlite::params![query_blob, limit],
                |row| {
                    let distance: f64 = row.get(1)?;
                    // Convert cosine distance to similarity score
                    let similarity = 1.0 - (distance / 2.0);
                    Ok(SearchResult {
                        fnord_id: row.get(0)?,
                        title: row.get(2)?,
                        pentacle_title: row.get(3)?,
                        published_at: row.get(4)?,
                        summary: row.get(5)?,
                        similarity,
                    })
                },
            )
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            // Filter out low similarity results (< 0.3 for search, lower threshold than similar articles)
            .filter(|r| r.similarity >= 0.3)
            .collect();
        result
    };

    info!("Semantic search for '{}' found {} results", query, results.len());

    Ok(SemanticSearchResponse { query, results })
}

// ============================================================
// ARTICLE EMBEDDING BATCH GENERATION (Phase 3)
// ============================================================

#[derive(serde::Serialize)]
pub struct ArticleEmbeddingCount {
    pub total_articles: i64,
    pub with_embedding: i64,
    pub without_embedding: i64,
    pub processable: i64,  // Articles with content_full that could get embeddings
}

/// Get count of articles with and without embeddings
#[tauri::command]
pub fn get_article_embedding_stats(state: State<AppState>) -> Result<ArticleEmbeddingCount, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let total_articles: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let with_embedding: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnords WHERE embedding IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let processable: i64 = db
        .conn()
        .query_row(
            r#"SELECT COUNT(*) FROM fnords
               WHERE embedding IS NULL
               AND processed_at IS NOT NULL
               AND content_full IS NOT NULL
               AND LENGTH(content_full) >= 100"#,
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(ArticleEmbeddingCount {
        total_articles,
        with_embedding,
        without_embedding: total_articles - with_embedding,
        processable,
    })
}

#[derive(serde::Serialize, Clone)]
pub struct ArticleEmbeddingProgress {
    pub current: i64,
    pub total: i64,
    pub fnord_id: i64,
    pub title: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(serde::Serialize)]
pub struct ArticleEmbeddingBatchResult {
    pub processed: i64,
    pub succeeded: i64,
    pub failed: i64,
}

/// Generate embeddings for all processed articles that don't have one yet
#[tauri::command]
pub async fn generate_article_embeddings_batch(
    window: Window,
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<ArticleEmbeddingBatchResult, String> {
    let limit = limit.unwrap_or(1000);

    // Get articles that need embeddings
    let articles: Vec<(i64, String, String)> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .conn()
            .prepare(
                r#"SELECT id, title, content_full
                   FROM fnords
                   WHERE embedding IS NULL
                   AND processed_at IS NOT NULL
                   AND content_full IS NOT NULL
                   AND LENGTH(content_full) >= 100
                   ORDER BY processed_at DESC
                   LIMIT ?"#,
            )
            .map_err(|e| e.to_string())?;

        let result: Vec<(i64, String, String)> = stmt
            .query_map([limit], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        result
    };

    let total = articles.len() as i64;
    if total == 0 {
        return Ok(ArticleEmbeddingBatchResult {
            processed: 0,
            succeeded: 0,
            failed: 0,
        });
    }

    let _ = window.emit(
        "article-embedding-progress",
        ArticleEmbeddingProgress {
            current: 0,
            total,
            fnord_id: 0,
            title: "Starting...".to_string(),
            success: true,
            error: None,
        },
    );

    let client = OllamaClient::new(None);
    let mut succeeded = 0i64;
    let mut failed = 0i64;

    for (idx, (fnord_id, title, content)) in articles.into_iter().enumerate() {
        let result = generate_and_save_article_embedding(
            &client,
            &state.db,
            fnord_id,
            &title,
            &content,
        )
        .await;

        let (success, error) = match result {
            Ok(()) => {
                succeeded += 1;
                (true, None)
            }
            Err(e) => {
                failed += 1;
                (false, Some(e))
            }
        };

        let _ = window.emit(
            "article-embedding-progress",
            ArticleEmbeddingProgress {
                current: (idx + 1) as i64,
                total,
                fnord_id,
                title: title.clone(),
                success,
                error,
            },
        );
    }

    info!(
        "Article embedding batch complete: {} succeeded, {} failed",
        succeeded, failed
    );

    Ok(ArticleEmbeddingBatchResult {
        processed: total,
        succeeded,
        failed,
    })
}
