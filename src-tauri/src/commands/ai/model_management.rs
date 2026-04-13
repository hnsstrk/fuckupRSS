//! Ollama model management and AI provider commands (status, load, unload, pull, cost tracking)

use crate::ai_provider::{AiTextProvider, ProviderTestResult, ProviderType};
use crate::commands::ai::helpers::get_effective_ollama_url;
use crate::ollama::{OllamaClient, RECOMMENDED_EMBEDDING_MODEL, RECOMMENDED_MAIN_MODEL};
use crate::AppState;
use log::warn;
use serde::Serialize;
use std::time::Instant;
use tauri::{Emitter, State, Window};

use super::types::{LoadedModel, LoadedModelsResponse, ModelPullResult, OllamaStatus};

/// Check Ollama availability and list installed models
#[tauri::command]
pub async fn check_ollama(state: State<'_, AppState>) -> Result<OllamaStatus, String> {
    let url = {
        let db = state.db_conn()?;
        get_effective_ollama_url(&db, &state.proxy_manager)
    };
    let client = OllamaClient::new(Some(url));

    match client.list_models().await {
        Ok(models) => {
            let model_names: Vec<String> = models.into_iter().map(|m| m.name).collect();

            let has_recommended_main = model_names.iter().any(|m| {
                m == RECOMMENDED_MAIN_MODEL
                    || m.starts_with(&format!(
                        "{}:",
                        RECOMMENDED_MAIN_MODEL.split(':').next().unwrap_or("")
                    ))
            });
            let has_recommended_embedding = model_names.iter().any(|m| {
                m == RECOMMENDED_EMBEDDING_MODEL
                    || m.starts_with(&format!("{}:", RECOMMENDED_EMBEDDING_MODEL))
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
pub async fn get_loaded_models(state: State<'_, AppState>) -> Result<LoadedModelsResponse, String> {
    let ollama_url = {
        let db = state.db_conn()?;
        get_effective_ollama_url(&db, &state.proxy_manager)
    };
    let client = reqwest_new::Client::new();

    let response = client
        .get(format!("{}/api/ps", ollama_url))
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

    let models = ps_response
        .models
        .unwrap_or_default()
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
pub async fn load_model(state: State<'_, AppState>, model: String) -> Result<bool, String> {
    let ollama_url = {
        let db = state.db_conn()?;
        get_effective_ollama_url(&db, &state.proxy_manager)
    };
    let client = reqwest_new::Client::new();

    // Try embedding endpoint first
    let embed_body = serde_json::json!({
        "model": model,
        "prompt": "test",
        "keep_alive": -1
    });

    let response = client
        .post(format!("{}/api/embeddings", ollama_url))
        .json(&embed_body)
        .send()
        .await;

    if let Ok(resp) = response {
        if resp.status().is_success() {
            return Ok(true);
        }
    }

    // Fall back to generate endpoint
    let gen_body = serde_json::json!({
        "model": model,
        "prompt": "",
        "keep_alive": -1
    });

    let response = client
        .post(format!("{}/api/generate", ollama_url))
        .json(&gen_body)
        .send()
        .await
        .map_err(|e| format!("Failed to load model: {}", e))?;

    Ok(response.status().is_success())
}

/// Unload a model from Ollama VRAM
#[tauri::command]
pub async fn unload_model(state: State<'_, AppState>, model: String) -> Result<bool, String> {
    let ollama_url = {
        let db = state.db_conn()?;
        get_effective_ollama_url(&db, &state.proxy_manager)
    };
    let client = reqwest_new::Client::new();

    let body = format!(r#"{{"model":"{}","prompt":"","keep_alive":0}}"#, model);

    let response: reqwest_new::Response = client
        .post(format!("{}/api/generate", ollama_url))
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e: reqwest_new::Error| format!("Failed to unload model: {}", e))?;

    Ok(response.status().is_success())
}

/// Ensure only the specified models are loaded (main + embedding)
#[tauri::command]
pub async fn ensure_models_loaded(
    state: State<'_, AppState>,
    main_model: String,
    embedding_model: String,
) -> Result<LoadedModelsResponse, String> {
    let loaded = get_loaded_models(state.clone()).await?;
    let loaded_names: Vec<&str> = loaded.models.iter().map(|m| m.name.as_str()).collect();

    // Unload models that aren't needed
    for model in &loaded.models {
        if model.name != main_model && model.name != embedding_model {
            let _ = unload_model(state.clone(), model.name.clone()).await;
        }
    }

    // Load embedding model if needed
    if !loaded_names.contains(&embedding_model.as_str()) {
        load_model(state.clone(), embedding_model).await?;
    }

    // Load main model if needed
    if !loaded_names.contains(&main_model.as_str()) {
        load_model(state.clone(), main_model).await?;
    }

    get_loaded_models(state).await
}

/// Pull (download) a model from Ollama
#[tauri::command]
pub async fn pull_model(
    state: State<'_, AppState>,
    window: Window,
    model: String,
) -> Result<ModelPullResult, String> {
    let url = {
        let db = state.db_conn()?;
        get_effective_ollama_url(&db, &state.proxy_manager)
    };
    let client = OllamaClient::new(Some(url));

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
// AI PROVIDER MANAGEMENT
// ============================================================

/// Test an AI provider connection and return available models
#[tauri::command]
pub async fn test_ai_provider(
    _state: State<'_, AppState>,
    provider_type: String,
    base_url: String,
    api_key: Option<String>,
) -> Result<ProviderTestResult, String> {
    let start = Instant::now();

    let ptype = ProviderType::from_str_setting(&provider_type);

    match ptype {
        ProviderType::Ollama => {
            // Test Ollama connection by listing models
            let client = OllamaClient::new(Some(base_url));
            match client.list_models().await {
                Ok(models) => {
                    let latency = start.elapsed().as_millis() as u64;
                    Ok(ProviderTestResult {
                        success: true,
                        latency_ms: latency,
                        models: models.into_iter().map(|m| m.name).collect(),
                        error: None,
                    })
                }
                Err(e) => {
                    let latency = start.elapsed().as_millis() as u64;
                    Ok(ProviderTestResult {
                        success: false,
                        latency_ms: latency,
                        models: vec![],
                        error: Some(e.to_string()),
                    })
                }
            }
        }
        ProviderType::OpenAiCompatible => {
            // Test OpenAI-compatible API by listing models
            let key = api_key.unwrap_or_default();
            let client = reqwest_new::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| e.to_string())?;

            let url = format!("{}/v1/models", base_url.trim_end_matches('/'));
            match client
                .get(&url)
                .header("Authorization", format!("Bearer {}", key))
                .send()
                .await
            {
                Ok(resp) => {
                    let latency = start.elapsed().as_millis() as u64;
                    if resp.status().is_success() {
                        // Try to parse model list
                        #[derive(serde::Deserialize)]
                        struct ModelsResponse {
                            data: Option<Vec<ModelEntry>>,
                        }
                        #[derive(serde::Deserialize)]
                        struct ModelEntry {
                            id: String,
                        }

                        let models = match resp.json::<ModelsResponse>().await {
                            Ok(r) => r
                                .data
                                .unwrap_or_default()
                                .into_iter()
                                .map(|m| m.id)
                                .collect(),
                            Err(_) => vec![],
                        };

                        Ok(ProviderTestResult {
                            success: true,
                            latency_ms: latency,
                            models,
                            error: None,
                        })
                    } else if resp.status().as_u16() == 401 {
                        Ok(ProviderTestResult {
                            success: false,
                            latency_ms: latency,
                            models: vec![],
                            error: Some("Authentication failed - check API key".to_string()),
                        })
                    } else {
                        Ok(ProviderTestResult {
                            success: false,
                            latency_ms: latency,
                            models: vec![],
                            error: Some(format!("HTTP {}", resp.status())),
                        })
                    }
                }
                Err(e) => {
                    let latency = start.elapsed().as_millis() as u64;
                    Ok(ProviderTestResult {
                        success: false,
                        latency_ms: latency,
                        models: vec![],
                        error: Some(e.to_string()),
                    })
                }
            }
        }
    }
}

// ============================================================
// COST TRACKING
// ============================================================

/// Monthly cost summary
#[derive(Debug, Serialize)]
pub struct MonthlyCost {
    pub spent: f64,
    pub limit: f64,
    pub remaining: f64,
    pub percentage: f64,
}

/// Cost history entry
#[derive(Debug, Serialize)]
pub struct CostEntry {
    pub id: i64,
    pub provider: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub estimated_cost_usd: f64,
    pub created_at: String,
}

/// Get current monthly cost summary
#[tauri::command]
pub fn get_monthly_cost(state: State<AppState>) -> Result<MonthlyCost, String> {
    let db = state.db_conn()?;

    // Get monthly spend
    let spent: f64 = db
        .conn()
        .query_row(
            r#"SELECT COALESCE(SUM(estimated_cost_usd), 0.0)
               FROM ai_cost_log
               WHERE created_at >= datetime('now', 'start of month')"#,
            [],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // Get cost limit from settings
    let limit: f64 = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'cost_limit_monthly'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5.0);

    let remaining = (limit - spent).max(0.0);
    let percentage = if limit > 0.0 {
        (spent / limit * 100.0).min(100.0)
    } else {
        0.0
    };

    Ok(MonthlyCost {
        spent,
        limit,
        remaining,
        percentage,
    })
}

/// Get cost history entries
#[tauri::command]
pub fn get_cost_history(
    state: State<AppState>,
    limit: Option<i64>,
) -> Result<Vec<CostEntry>, String> {
    let limit = limit.unwrap_or(100);
    let db = state.db_conn()?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"SELECT id, provider, model, input_tokens, output_tokens,
                      estimated_cost_usd, created_at
               FROM ai_cost_log
               ORDER BY created_at DESC
               LIMIT ?"#,
        )
        .map_err(|e| e.to_string())?;

    let entries: Vec<CostEntry> = stmt
        .query_map([limit], |row| {
            Ok(CostEntry {
                id: row.get(0)?,
                provider: row.get(1)?,
                model: row.get(2)?,
                input_tokens: row.get(3)?,
                output_tokens: row.get(4)?,
                estimated_cost_usd: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

/// Log an AI API cost entry (called internally after API calls)
pub fn log_ai_cost(
    conn: &rusqlite::Connection,
    provider: &str,
    model: &str,
    input_tokens: u32,
    output_tokens: u32,
    estimated_cost_usd: f64,
) {
    if let Err(e) = conn.execute(
        r#"INSERT INTO ai_cost_log (provider, model, input_tokens, output_tokens, estimated_cost_usd)
           VALUES (?1, ?2, ?3, ?4, ?5)"#,
        rusqlite::params![provider, model, input_tokens, output_tokens, estimated_cost_usd],
    ) {
        warn!("Failed to log AI cost: {}", e);
    }
}

/// Check if the monthly cost limit has been exceeded
///
/// Currently not called but will be used for pre-batch cost limit enforcement
/// when OpenAI-compatible provider is active.
#[allow(dead_code)]
pub fn check_cost_limit(
    conn: &rusqlite::Connection,
) -> Result<(), crate::ai_provider::AiProviderError> {
    let spent: f64 = conn
        .query_row(
            r#"SELECT COALESCE(SUM(estimated_cost_usd), 0.0)
               FROM ai_cost_log
               WHERE created_at >= datetime('now', 'start of month')"#,
            [],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let limit: f64 = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'cost_limit_monthly'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5.0);

    if spent >= limit {
        Err(crate::ai_provider::AiProviderError::CostLimitReached { spent, limit })
    } else {
        Ok(())
    }
}
