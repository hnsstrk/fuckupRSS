//! Ollama model management commands (status, load, unload, pull)

use crate::ollama::{OllamaClient, RECOMMENDED_EMBEDDING_MODEL, RECOMMENDED_MAIN_MODEL};
use tauri::{Emitter, Window};

use super::types::{LoadedModel, LoadedModelsResponse, ModelPullResult, OllamaStatus};

/// Base URL for Ollama API - centralized for future configurability
const OLLAMA_BASE_URL: &str = "http://localhost:11434";

/// Check Ollama availability and list installed models
#[tauri::command]
pub async fn check_ollama() -> Result<OllamaStatus, String> {
    let client = OllamaClient::new(None);

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
pub async fn get_loaded_models() -> Result<LoadedModelsResponse, String> {
    let client = reqwest_new::Client::new();

    let response = client
        .get(&format!("{}/api/ps", OLLAMA_BASE_URL))
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
pub async fn load_model(model: String) -> Result<bool, String> {
    let client = reqwest_new::Client::new();

    // Try embedding endpoint first
    let embed_body = serde_json::json!({
        "model": model,
        "prompt": "test",
        "keep_alive": -1
    });

    let response = client
        .post(&format!("{}/api/embeddings", OLLAMA_BASE_URL))
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
        .post(&format!("{}/api/generate", OLLAMA_BASE_URL))
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
        .post(&format!("{}/api/generate", OLLAMA_BASE_URL))
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
    main_model: String,
    embedding_model: String,
) -> Result<LoadedModelsResponse, String> {
    let loaded = get_loaded_models().await?;
    let loaded_names: Vec<&str> = loaded.models.iter().map(|m| m.name.as_str()).collect();

    // Unload models that aren't needed
    for model in &loaded.models {
        if model.name != main_model && model.name != embedding_model {
            let _ = unload_model(model.name.clone()).await;
        }
    }

    // Load embedding model if needed
    if !loaded_names.contains(&embedding_model.as_str()) {
        load_model(embedding_model).await?;
    }

    // Load main model if needed
    if !loaded_names.contains(&main_model.as_str()) {
        load_model(main_model).await?;
    }

    get_loaded_models().await
}

/// Pull (download) a model from Ollama
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
