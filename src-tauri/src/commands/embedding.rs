//! Commands for embedding queue management

use crate::embedding_worker::{self, EmbeddingProgress};
use crate::AppState;
use serde::Serialize;
use tauri::{State, AppHandle, Emitter};

#[derive(Serialize)]
pub struct EmbeddingQueueStatus {
    pub queue_size: i64,
    pub worker_running: bool,
    pub worker_processing: bool,
}

/// Get the current status of the embedding queue
#[tauri::command]
pub fn get_embedding_queue_status(state: State<AppState>) -> Result<EmbeddingQueueStatus, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let queue_size: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM embedding_queue", [], |row| row.get(0))
        .unwrap_or(0);

    Ok(EmbeddingQueueStatus {
        queue_size,
        worker_running: state.embedding_worker.is_running(),
        worker_processing: state.embedding_worker.is_processing(),
    })
}

#[derive(Serialize)]
pub struct ProcessQueueResult {
    pub processed: i64,
    pub failed: i64,
    pub remaining: i64,
}

/// Process the embedding queue immediately (Option B - after batch analysis)
#[tauri::command]
pub async fn process_embedding_queue_now(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    limit: Option<i64>,
) -> Result<ProcessQueueResult, String> {
    let limit = limit.unwrap_or(100);
    let db = state.db.clone();

    // Emit start event
    let _ = app_handle.emit("embedding-progress", EmbeddingProgress {
        queue_size: 0,
        processed: 0,
        failed: 0,
        is_processing: true,
    });

    // Process the queue
    let (processed, failed) = embedding_worker::process_embedding_queue(
        db.clone(),
        Some(&app_handle),
        limit,
    ).await?;

    // Calculate quality scores for processed keywords
    if processed > 0 {
        let _ = embedding_worker::calculate_pending_quality_scores(&db, processed);
    }

    // Get remaining count
    let remaining = {
        let db = db.lock().map_err(|e| e.to_string())?;
        db.conn()
            .query_row("SELECT COUNT(*) FROM embedding_queue", [], |row| row.get(0))
            .unwrap_or(0)
    };

    // Emit completion event
    let _ = app_handle.emit("embedding-progress", EmbeddingProgress {
        queue_size: remaining,
        processed,
        failed,
        is_processing: false,
    });

    Ok(ProcessQueueResult {
        processed,
        failed,
        remaining,
    })
}

/// Queue all keywords without embeddings for processing
#[tauri::command]
pub fn queue_missing_embeddings(state: State<AppState>) -> Result<i64, String> {
    embedding_worker::queue_keywords_without_embeddings(&state.db)
}

/// Get detailed queue entries (for debugging)
#[tauri::command]
pub fn get_embedding_queue_details(
    state: State<AppState>,
    limit: Option<i64>,
) -> Result<Vec<QueueEntry>, String> {
    let limit = limit.unwrap_or(50);
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db.conn()
        .prepare(
            r#"SELECT eq.id, eq.immanentize_id, i.name, eq.priority, eq.attempts, eq.last_error, eq.queued_at
               FROM embedding_queue eq
               JOIN immanentize i ON i.id = eq.immanentize_id
               ORDER BY eq.priority DESC, eq.queued_at ASC
               LIMIT ?"#,
        )
        .map_err(|e| e.to_string())?;

    let entries: Vec<QueueEntry> = stmt
        .query_map([limit], |row| {
            Ok(QueueEntry {
                id: row.get(0)?,
                keyword_id: row.get(1)?,
                keyword_name: row.get(2)?,
                priority: row.get(3)?,
                attempts: row.get(4)?,
                last_error: row.get(5)?,
                queued_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

#[derive(Serialize)]
pub struct QueueEntry {
    pub id: i64,
    pub keyword_id: i64,
    pub keyword_name: String,
    pub priority: i64,
    pub attempts: i64,
    pub last_error: Option<String>,
    pub queued_at: String,
}
