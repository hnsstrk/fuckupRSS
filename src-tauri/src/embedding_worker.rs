//! Background worker for automatic embedding generation
//!
//! This module provides a background worker that continuously processes
//! the embedding queue, generating embeddings for new keywords.

use crate::db::Database;
use crate::embeddings::{cosine_similarity_from_blobs, embedding_to_blob};
use crate::ollama::{OllamaClient, RECOMMENDED_EMBEDDING_MODEL};
use futures::future::join_all;
use log::{debug, error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Worker state for tracking embedding queue processing
pub struct EmbeddingWorker {
    is_running: AtomicBool,
    is_processing: AtomicBool,
}

impl EmbeddingWorker {
    pub fn new() -> Self {
        Self {
            is_running: AtomicBool::new(false),
            is_processing: AtomicBool::new(false),
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn is_processing(&self) -> bool {
        self.is_processing.load(Ordering::SeqCst)
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }
}

impl Default for EmbeddingWorker {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress event for frontend updates
#[derive(Clone, serde::Serialize)]
pub struct EmbeddingProgress {
    pub queue_size: i64,
    pub processed: i64,
    pub failed: i64,
    pub is_processing: bool,
}

/// Get the current queue size
fn get_queue_size(db: &Arc<Mutex<Database>>) -> Result<i64, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    db.conn()
        .query_row("SELECT COUNT(*) FROM embedding_queue", [], |row| row.get(0))
        .map_err(|e| e.to_string())
}

/// Get keywords from the queue
fn get_queued_keywords(db: &Arc<Mutex<Database>>, limit: i64) -> Result<Vec<(i64, i64, String)>, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.conn()
        .prepare(
            r#"SELECT eq.id, eq.immanentize_id, i.name
               FROM embedding_queue eq
               JOIN immanentize i ON i.id = eq.immanentize_id
               WHERE eq.attempts < 3
               ORDER BY eq.priority DESC, eq.queued_at ASC
               LIMIT ?"#,
        )
        .map_err(|e| e.to_string())?;

    let result: Vec<(i64, i64, String)> = stmt
        .query_map([limit], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(result)
}

/// Save embedding and remove from queue
fn save_embedding_and_dequeue(
    db: &Arc<Mutex<Database>>,
    queue_id: i64,
    keyword_id: i64,
    embedding: &[f32],
) -> Result<(), String> {
    let blob = embedding_to_blob(embedding);
    let db = db.lock().map_err(|e| e.to_string())?;

    // Update the keyword with embedding
    db.conn()
        .execute(
            "UPDATE immanentize SET embedding = ?1, embedding_at = datetime('now') WHERE id = ?2",
            rusqlite::params![blob, keyword_id],
        )
        .map_err(|e| e.to_string())?;

    // Remove from queue
    db.conn()
        .execute("DELETE FROM embedding_queue WHERE id = ?", [queue_id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Record a failed attempt
fn record_failure(db: &Arc<Mutex<Database>>, queue_id: i64, error: &str) -> Result<(), String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    db.conn()
        .execute(
            "UPDATE embedding_queue SET attempts = attempts + 1, last_error = ?1 WHERE id = ?2",
            rusqlite::params![error, queue_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove entries that have exceeded max attempts
fn cleanup_failed_entries(db: &Arc<Mutex<Database>>) -> Result<i64, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    let count = db.conn()
        .execute("DELETE FROM embedding_queue WHERE attempts >= 3", [])
        .map_err(|e| e.to_string())?;
    Ok(count as i64)
}

/// Process a batch of keywords from the queue with parallel embedding generation
pub async fn process_embedding_queue(
    db: Arc<Mutex<Database>>,
    app_handle: Option<&AppHandle>,
    batch_size: i64,
) -> Result<(i64, i64), String> {
    let keywords = get_queued_keywords(&db, batch_size)?;

    if keywords.is_empty() {
        return Ok((0, 0));
    }

    let client = Arc::new(OllamaClient::new(None));

    // Check if Ollama is available
    if !client.is_available().await {
        debug!("Ollama not available, skipping embedding generation");
        return Ok((0, 0));
    }

    let model = RECOMMENDED_EMBEDDING_MODEL;
    let concurrency = 10; // Process 10 embeddings in parallel

    let mut processed = 0i64;
    let mut failed = 0i64;

    // Process in chunks for parallel execution
    for chunk in keywords.chunks(concurrency) {
        let futures: Vec<_> = chunk
            .iter()
            .map(|(queue_id, keyword_id, name)| {
                let client = Arc::clone(&client);
                let name = name.clone();
                let queue_id = *queue_id;
                let keyword_id = *keyword_id;

                async move {
                    // Add unique suffix to work around Ollama embedding cache issue
                    let embedding_text = format!("{}_{}", name, keyword_id);
                    let result = client.generate_embedding(model, &embedding_text).await;
                    (queue_id, keyword_id, name, result)
                }
            })
            .collect();

        // Execute all embeddings in parallel
        let results = join_all(futures).await;

        // Process results
        for (queue_id, keyword_id, name, result) in results {
            match result {
                Ok(embedding) => {
                    if let Err(e) = save_embedding_and_dequeue(&db, queue_id, keyword_id, &embedding) {
                        error!("Failed to save embedding for '{}': {}", name, e);
                        failed += 1;
                    } else {
                        debug!("Generated embedding for keyword: {}", name);
                        processed += 1;
                    }
                }
                Err(e) => {
                    warn!("Failed to generate embedding for '{}': {}", name, e);
                    let _ = record_failure(&db, queue_id, &e.to_string());
                    failed += 1;
                }
            }
        }

        // Emit progress event after each chunk
        if let Some(handle) = app_handle {
            let queue_size = get_queue_size(&db).unwrap_or(0);
            let _ = handle.emit("embedding-progress", EmbeddingProgress {
                queue_size,
                processed,
                failed,
                is_processing: true,
            });
        }
    }

    // Cleanup failed entries
    let cleaned = cleanup_failed_entries(&db).unwrap_or(0);
    if cleaned > 0 {
        info!("Cleaned up {} failed embedding queue entries", cleaned);
    }

    Ok((processed, failed))
}

/// Calculate quality scores for keywords that have embeddings but no score yet
pub fn calculate_pending_quality_scores(db: &Arc<Mutex<Database>>, limit: i64) -> Result<i64, String> {
    let db_guard = db.lock().map_err(|e| e.to_string())?;
    let conn = db_guard.conn();

    // Get keywords that have embeddings but no quality score
    let keywords: Vec<i64> = conn
        .prepare(
            r#"SELECT id FROM immanentize
               WHERE embedding IS NOT NULL
               AND quality_score IS NULL
               LIMIT ?"#,
        )
        .map_err(|e| e.to_string())?
        .query_map([limit], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    if keywords.is_empty() {
        return Ok(0);
    }

    let mut updated = 0i64;

    for keyword_id in keywords {
        // Calculate quality score based on:
        // - article_count (how often used)
        // - neighbor_count (how connected)
        // - category_count (breadth of usage)
        let result: Result<(i64, i64, i64), _> = conn.query_row(
            r#"SELECT
                COALESCE(i.article_count, 0),
                (SELECT COUNT(*) FROM immanentize_neighbors
                 WHERE immanentize_id_a = i.id OR immanentize_id_b = i.id),
                (SELECT COUNT(*) FROM immanentize_sephiroth WHERE immanentize_id = i.id)
               FROM immanentize i WHERE i.id = ?"#,
            [keyword_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        );

        if let Ok((article_count, neighbor_count, category_count)) = result {
            // Quality formula:
            // - Base score from article usage (log scale to prevent dominance)
            // - Bonus for connections
            // - Bonus for category diversity
            let usage_score = (article_count as f64 + 1.0).ln() / 5.0; // 0-1 range for typical values
            let connection_score = (neighbor_count as f64).min(20.0) / 20.0; // 0-1, cap at 20
            let category_score = (category_count as f64).min(5.0) / 5.0; // 0-1, cap at 5

            // Weighted combination (usage most important)
            let quality = (usage_score * 0.5 + connection_score * 0.3 + category_score * 0.2)
                .min(1.0)
                .max(0.0);

            conn.execute(
                "UPDATE immanentize SET quality_score = ?1, quality_calculated_at = datetime('now') WHERE id = ?2",
                rusqlite::params![quality, keyword_id],
            )
            .ok();

            updated += 1;
        }
    }

    Ok(updated)
}

/// Start the background worker
pub fn start_background_worker(
    db: Arc<Mutex<Database>>,
    worker: Arc<EmbeddingWorker>,
    app_handle: AppHandle,
    batch_running: Arc<AtomicBool>,
) {
    if worker.is_running.swap(true, Ordering::SeqCst) {
        info!("Embedding worker already running");
        return;
    }

    info!("Starting embedding background worker");

    let db_clone = db.clone();
    let worker_clone = worker.clone();
    let handle_clone = app_handle.clone();

    tauri::async_runtime::spawn(async move {
        let idle_interval = Duration::from_secs(10);  // When queue is empty
        let batch_pause = Duration::from_secs(5);     // When batch analysis is running
        let batch_size = 50i64;  // Process more at once

        while worker_clone.is_running.load(Ordering::SeqCst) {
            // Check if batch analysis is running - yield to it
            if batch_running.load(Ordering::SeqCst) {
                debug!("Batch analysis running, embedding worker pausing...");
                tokio::time::sleep(batch_pause).await;
                continue;
            }

            // Check queue size
            let queue_size = get_queue_size(&db_clone).unwrap_or(0);

            if queue_size > 0 {
                worker_clone.is_processing.store(true, Ordering::SeqCst);
                debug!("Embedding queue has {} items, processing...", queue_size);

                // Process batch
                match process_embedding_queue(db_clone.clone(), Some(&handle_clone), batch_size).await {
                    Ok((processed, failed)) => {
                        if processed > 0 || failed > 0 {
                            info!("Embedding worker: processed={}, failed={}", processed, failed);
                        }
                        // Quality scores are calculated when queue is empty (below)
                    }
                    Err(e) => {
                        error!("Embedding worker error: {}", e);
                    }
                }

                worker_clone.is_processing.store(false, Ordering::SeqCst);

                // Emit final progress
                let remaining = get_queue_size(&db_clone).unwrap_or(0);
                let _ = handle_clone.emit("embedding-progress", EmbeddingProgress {
                    queue_size: remaining,
                    processed: 0,
                    failed: 0,
                    is_processing: false,
                });

                // No pause between batches - keep going until queue is empty!
                // (unless batch analysis starts, checked at loop start)
            } else {
                // Queue is empty - calculate quality scores for all pending keywords
                if let Ok(scored) = calculate_pending_quality_scores(&db_clone, 1000) {
                    if scored > 0 {
                        info!("Calculated quality scores for {} keywords", scored);
                        // Keep going without sleep if there's more to do
                        continue;
                    }
                }

                // Then calculate neighbor similarities
                if let Ok(calculated) = calculate_neighbor_similarities(&db_clone, 500) {
                    if calculated > 0 {
                        info!("Calculated {} neighbor similarities", calculated);
                        // Keep going without sleep if there's more to do
                        continue;
                    }
                }

                // Longer wait when truly idle
                tokio::time::sleep(idle_interval).await;
            }
        }

        info!("Embedding background worker stopped");
    });
}

/// Calculate embedding similarity for all neighbor pairs that don't have it yet
pub fn calculate_neighbor_similarities(db: &Arc<Mutex<Database>>, limit: i64) -> Result<i64, String> {
    let db_guard = db.lock().map_err(|e| e.to_string())?;
    let conn = db_guard.conn();

    // Get neighbor pairs without similarity that have embeddings
    let pairs: Vec<(i64, i64)> = conn
        .prepare(
            r#"SELECT n.immanentize_id_a, n.immanentize_id_b
               FROM immanentize_neighbors n
               JOIN immanentize a ON a.id = n.immanentize_id_a
               JOIN immanentize b ON b.id = n.immanentize_id_b
               WHERE n.embedding_similarity IS NULL
               AND a.embedding IS NOT NULL
               AND b.embedding IS NOT NULL
               LIMIT ?"#,
        )
        .map_err(|e| e.to_string())?
        .query_map([limit], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    if pairs.is_empty() {
        return Ok(0);
    }

    let mut updated = 0i64;

    for (id_a, id_b) in pairs {
        // Get embeddings for both keywords
        let embeddings: Result<(Vec<u8>, Vec<u8>), _> = conn.query_row(
            r#"SELECT
                (SELECT embedding FROM immanentize WHERE id = ?1),
                (SELECT embedding FROM immanentize WHERE id = ?2)"#,
            [id_a, id_b],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        if let Ok((blob_a, blob_b)) = embeddings {
            if let (Some(sim), Some(combined)) = (
                cosine_similarity_from_blobs(&blob_a, &blob_b),
                calculate_combined_weight_for_pair(conn, id_a, id_b),
            ) {
                conn.execute(
                    r#"UPDATE immanentize_neighbors
                       SET embedding_similarity = ?1, combined_weight = ?2
                       WHERE immanentize_id_a = ?3 AND immanentize_id_b = ?4"#,
                    rusqlite::params![sim, combined, id_a, id_b],
                )
                .ok();
                updated += 1;
            }
        }
    }

    Ok(updated)
}

/// Calculate combined weight for a neighbor pair
fn calculate_combined_weight_for_pair(
    conn: &rusqlite::Connection,
    id_a: i64,
    id_b: i64,
) -> Option<f64> {
    // Get cooccurrence and embedding similarity
    let result: Result<(i64, Option<f64>), _> = conn.query_row(
        r#"SELECT cooccurrence, embedding_similarity
           FROM immanentize_neighbors
           WHERE immanentize_id_a = ?1 AND immanentize_id_b = ?2"#,
        [id_a, id_b],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    if let Ok((cooccurrence, sim)) = result {
        // Combined weight: 70% cooccurrence (log scale), 30% similarity
        let cooc_score = (cooccurrence as f64 + 1.0).ln() / 5.0;
        let sim_score = sim.unwrap_or(0.0);
        Some(cooc_score * 0.7 + sim_score * 0.3)
    } else {
        None
    }
}

/// Queue all keywords without embeddings (for initial setup or recovery)
pub fn queue_keywords_without_embeddings(db: &Arc<Mutex<Database>>) -> Result<i64, String> {
    let db_guard = db.lock().map_err(|e| e.to_string())?;
    let count = db_guard.conn()
        .execute(
            r#"INSERT OR IGNORE INTO embedding_queue (immanentize_id, priority, queued_at)
               SELECT id,
                      CASE WHEN article_count > 5 THEN 10
                           WHEN article_count > 1 THEN 5
                           ELSE 0 END,
                      datetime('now')
               FROM immanentize
               WHERE embedding IS NULL
               AND article_count > 0"#,
            [],
        )
        .map_err(|e| e.to_string())?;

    Ok(count as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_to_blob() {
        let embedding = vec![1.0f32, 2.0, 3.0];
        let blob = embedding_to_blob(&embedding);
        assert_eq!(blob.len(), 12); // 3 floats * 4 bytes
    }
}
