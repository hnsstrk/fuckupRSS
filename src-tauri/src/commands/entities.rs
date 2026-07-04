//! Named Entity Recognition (NER) commands
//!
//! Extracts persons, organizations, locations, and events from articles
//! using the configured AI provider. Entities are normalized and deduplicated.

use crate::ai_provider::{AiTextProvider, TaskType};
use crate::error::CmdResult;
use crate::AppState;
use log::{debug, info, warn};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Emitter, State};

use serde_json::json;

use super::ai::helpers::{
    create_text_provider, get_locale_from_db, log_generation_cost, TokenUsage,
};

// ============================================================
// TYPES
// ============================================================

/// Entity as returned to the frontend
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EntityInfo {
    pub id: i64,
    pub name: String,
    pub entity_type: String,
    pub normalized_name: String,
    pub article_count: i64,
    pub mention_count: Option<i32>,
    pub confidence: Option<f64>,
}

/// Raw NER entity from LLM response
#[derive(Deserialize, Debug)]
struct RawNerEntity {
    name: String,
    #[serde(rename = "type")]
    entity_type: String,
    #[serde(default = "default_mentions")]
    mentions: i32,
}

fn ner_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "entities": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "type": { "type": "string", "enum": ["person", "organization", "location", "event"] },
                        "mentions": { "type": "integer" }
                    },
                    "required": ["name", "type", "mentions"]
                }
            }
        },
        "required": ["entities"]
    })
}

fn default_mentions() -> i32 {
    1
}

/// Raw NER response from LLM
#[derive(Deserialize, Debug)]
struct RawNerResponse {
    #[serde(default)]
    entities: Vec<RawNerEntity>,
}

/// Result of entity extraction
#[derive(Serialize, Debug)]
pub struct ExtractionResult {
    pub fnord_id: i64,
    pub entities_found: usize,
    pub entities_new: usize,
    pub success: bool,
    pub error: Option<String>,
}

/// Result of batch entity extraction
#[derive(Serialize, Debug)]
pub struct BatchExtractionResult {
    pub processed: usize,
    pub total_entities: usize,
    pub errors: usize,
}

/// Progress payload emitted during `extract_entities_backfill_all`.
/// Event name: `"ner-backfill-progress"`.
#[derive(Serialize, Debug, Clone)]
pub struct NerBackfillProgress {
    pub processed: usize,
    pub total: usize,
    pub entities_found: usize,
    pub errors: usize,
    /// Article id currently being processed (if any).
    pub current_fnord_id: Option<i64>,
}

// ============================================================
// NER PROMPT
// ============================================================

const NER_PROMPT: &str = r#"You are a Named Entity Recognition specialist. Extract all named entities from the following text.

Categorize each entity as exactly one of: person, organization, location, event.

Rules:
- Only extract clearly identifiable named entities (proper nouns)
- Do NOT extract generic terms like "government", "company", "city" without a specific name
- Merge slight variations (e.g., "Angela Merkel" and "Merkel" -> use the most complete form)
- Count how many times each entity is mentioned
- Return ONLY valid JSON, no explanation

Return this JSON format:
{
  "entities": [
    {"name": "Angela Merkel", "type": "person", "mentions": 3},
    {"name": "Berlin", "type": "location", "mentions": 2},
    {"name": "Bundestag", "type": "organization", "mentions": 1},
    {"name": "Klimagipfel 2025", "type": "event", "mentions": 1}
  ]
}

Title: {title}
Content: {content}"#;

// ============================================================
// NORMALIZATION
// ============================================================

/// Normalize an entity name for deduplication:
/// - lowercase
/// - trim whitespace
/// - remove common titles (Dr., Prof., etc.)
/// - collapse multiple spaces
fn normalize_entity_name(name: &str) -> String {
    let mut normalized = name.trim().to_lowercase();

    // Remove common titles/prefixes
    let prefixes = [
        "dr. ",
        "prof. ",
        "prof.dr. ",
        "sir ",
        "lord ",
        "frau ",
        "herr ",
        "mr. ",
        "mrs. ",
        "ms. ",
        "president ",
        "chancellor ",
        "minister ",
    ];
    for prefix in &prefixes {
        if normalized.starts_with(prefix) {
            normalized = normalized[prefix.len()..].to_string();
        }
    }

    // Collapse multiple spaces
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    parts.join(" ")
}

/// Validate entity type
fn is_valid_entity_type(entity_type: &str) -> bool {
    matches!(
        entity_type,
        "person" | "organization" | "location" | "event"
    )
}

// ============================================================
// DATABASE OPERATIONS
// ============================================================

/// Upsert an entity and link it to an article.
/// Returns the entity_id.
///
/// `entities.article_count` is incremented **only** when a new
/// `(fnord_id, entity_id)` link is created. Re-running NER for an article
/// that already has a given entity only refreshes `last_seen` and the
/// mention count, preventing article_count from inflating on retry/backfill.
fn upsert_entity_for_article(
    conn: &Connection,
    fnord_id: i64,
    name: &str,
    entity_type: &str,
    mention_count: i32,
) -> Result<i64, rusqlite::Error> {
    let normalized = normalize_entity_name(name);

    // Try to find existing entity
    let existing_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM entities WHERE normalized_name = ?1 AND entity_type = ?2",
            params![&normalized, entity_type],
            |row| row.get(0),
        )
        .ok();

    let entity_id = if let Some(id) = existing_id {
        // Entity exists — touch last_seen unconditionally; only bump
        // article_count if this fnord is NOT yet linked (decided below).
        conn.execute(
            r#"UPDATE entities SET last_seen = CURRENT_TIMESTAMP WHERE id = ?1"#,
            params![id],
        )?;
        id
    } else {
        // Brand-new entity — article_count starts at 0 and is incremented
        // below together with the new link, keeping the invariant consistent.
        conn.execute(
            r#"INSERT INTO entities (name, entity_type, normalized_name, article_count)
               VALUES (?1, ?2, ?3, 0)"#,
            params![name, entity_type, &normalized],
        )?;
        conn.last_insert_rowid()
    };

    // Check whether the link already exists for THIS article; we only bump
    // article_count when the link is new, so re-runs don't inflate counts.
    let link_exists: bool = conn
        .query_row(
            "SELECT 1 FROM fnord_entities WHERE fnord_id = ?1 AND entity_id = ?2",
            params![fnord_id, entity_id],
            |_| Ok(true),
        )
        .unwrap_or(false);

    // Upsert the link (refreshes mention_count on re-run, keeps PK stable)
    conn.execute(
        r#"INSERT OR REPLACE INTO fnord_entities (fnord_id, entity_id, mention_count, confidence)
           VALUES (?1, ?2, ?3, 0.8)"#,
        params![fnord_id, entity_id, mention_count],
    )?;

    if !link_exists {
        conn.execute(
            r#"UPDATE entities SET article_count = article_count + 1 WHERE id = ?1"#,
            params![entity_id],
        )?;
    }

    Ok(entity_id)
}

// ============================================================
// NER EXTRACTION
// ============================================================

/// Extract entities from article content using the AI provider
async fn extract_entities_for_article(
    provider: &dyn AiTextProvider,
    model: &str,
    title: &str,
    content: &str,
) -> Result<(Vec<RawNerEntity>, TokenUsage), String> {
    let truncated_content: String = content.chars().take(3000).collect();

    let prompt = NER_PROMPT
        .replace("{title}", title)
        .replace("{content}", &truncated_content);

    let result = provider
        .generate_text(model, &prompt, Some(ner_schema()))
        .await
        .map_err(|e| format!("NER generation failed: {}", e))?;

    let usage = TokenUsage {
        input_tokens: result.input_tokens,
        output_tokens: result.output_tokens,
    };

    let ner_response: RawNerResponse = serde_json::from_str(&result.text).map_err(|e| {
        warn!(
            "NER JSON parse error: {}. Response: {}",
            e,
            &result.text[..result.text.len().min(300)]
        );
        format!("NER JSON parse error: {}", e)
    })?;

    // Filter out invalid entity types
    let valid_entities: Vec<RawNerEntity> = ner_response
        .entities
        .into_iter()
        .filter(|e| {
            let valid = is_valid_entity_type(&e.entity_type) && !e.name.trim().is_empty();
            if !valid {
                debug!(
                    "Filtered out invalid entity: {:?} (type: {})",
                    e.name, e.entity_type
                );
            }
            valid
        })
        .collect();

    Ok((valid_entities, usage))
}

// ============================================================
// PIPELINE HELPER (called from batch_processor)
// ============================================================

/// Run NER for a single article as part of the analysis pipeline.
///
/// This is the entry point called by `process_single_article` after the main
/// Discordian analysis + DB update has succeeded. It:
/// 1. Calls the LLM with the NER prompt/schema (short-lived, reuses the same
///    Fast-task provider already set up by the batch).
/// 2. Persists extracted entities to `entities` + `fnord_entities` inside a
///    transaction.
/// 3. Updates `fnords.ner_status` to `'completed'` on success or `'failed'`
///    (plus `ner_error`) on any failure.
///
/// Never panics. Never returns Err. Failures are logged and reflected in
/// `ner_status` so the backfill command can retry them. An empty entity list
/// counts as success (some articles legitimately contain no named entities).
pub(crate) async fn run_ner_for_article_in_pipeline(
    state: &AppState,
    provider: &dyn AiTextProvider,
    model: &str,
    fnord_id: i64,
    title: &str,
    content: &str,
) {
    // Skip explicitly empty content — mark as completed with no entities so
    // the backfill doesn't keep retrying.
    if content.trim().is_empty() {
        if let Ok(db) = state.db_conn() {
            if let Err(e) = db.conn().execute(
                "UPDATE fnords SET ner_status = 'completed', ner_error = NULL WHERE id = ?1",
                params![fnord_id],
            ) {
                warn!(
                    "[NER Pipeline] Could not mark empty-content article {} as completed: {}",
                    fnord_id, e
                );
            }
        } else {
            warn!(
                "[NER Pipeline] db_conn unavailable — cannot mark empty-content article {} as completed",
                fnord_id
            );
        }
        return;
    }

    // Atomic claim: set ner_status='running' BEFORE the LLM call so a
    // parallel backfill (which filters `ner_status IS NULL OR 'failed'`) will
    // skip this article. The claim is only applied if the article is still
    // eligible, which means concurrent Batch+Backfill attempts race cleanly:
    // the loser's UPDATE affects 0 rows and the winner proceeds.
    let claimed: bool = match state.db_conn() {
        Ok(db) => {
            let affected = db
                .conn()
                .execute(
                    r#"UPDATE fnords
                       SET ner_status = 'running', ner_error = NULL
                       WHERE id = ?1
                         AND (ner_status IS NULL OR ner_status = 'failed')"#,
                    params![fnord_id],
                )
                .unwrap_or(0);
            affected > 0
        }
        Err(e) => {
            warn!(
                "[NER Pipeline] db_conn failed while claiming article {}: {}",
                fnord_id, e
            );
            false
        }
    };

    if !claimed {
        debug!(
            "[NER Pipeline] Article {} skipped — already claimed, completed, or running",
            fnord_id
        );
        return;
    }

    let extraction = extract_entities_for_article(provider, model, title, content).await;

    match extraction {
        Ok((entities, usage)) => {
            let db = match state.db_conn() {
                Ok(db) => db,
                Err(e) => {
                    warn!(
                        "[NER Pipeline] db_conn failed for article {}: {}",
                        fnord_id, e
                    );
                    return;
                }
            };

            log_generation_cost(db.conn(), provider.provider_name(), model, &usage);

            let entities_len = entities.len();

            // Persist entities in a transaction. If the transaction fails we
            // mark the article as 'failed' so the backfill can retry.
            if let Err(e) = db.conn().execute("BEGIN", []) {
                warn!(
                    "[NER Pipeline] BEGIN failed for article {}: {}",
                    fnord_id, e
                );
                mark_ner_failed(db.conn(), fnord_id, &format!("BEGIN failed: {}", e));
                return;
            }

            let save_result = (|| -> Result<usize, rusqlite::Error> {
                db.conn().execute(
                    "DELETE FROM fnord_entities WHERE fnord_id = ?1",
                    params![fnord_id],
                )?;
                let mut count = 0;
                for entity in &entities {
                    if is_valid_entity_type(&entity.entity_type) {
                        upsert_entity_for_article(
                            db.conn(),
                            fnord_id,
                            &entity.name,
                            &entity.entity_type,
                            entity.mentions,
                        )?;
                        count += 1;
                    }
                }
                Ok(count)
            })();

            match save_result {
                Ok(count) => {
                    if let Err(e) = db.conn().execute("COMMIT", []) {
                        warn!(
                            "[NER Pipeline] COMMIT failed for article {}: {}",
                            fnord_id, e
                        );
                        let _ = db.conn().execute("ROLLBACK", []);
                        mark_ner_failed(db.conn(), fnord_id, &format!("COMMIT failed: {}", e));
                        return;
                    }
                    let _ = db.conn().execute(
                        "UPDATE fnords SET ner_status = 'completed', ner_error = NULL WHERE id = ?1",
                        params![fnord_id],
                    );
                    debug!(
                        "[NER Pipeline] Article {}: {} entities saved ({} found)",
                        fnord_id, count, entities_len
                    );
                }
                Err(e) => {
                    let _ = db.conn().execute("ROLLBACK", []);
                    warn!("[NER Pipeline] DB error for article {}: {}", fnord_id, e);
                    mark_ner_failed(db.conn(), fnord_id, &format!("DB error: {}", e));
                }
            }
        }
        Err(e) => {
            warn!(
                "[NER Pipeline] Extraction failed for article {}: {}",
                fnord_id, e
            );
            if let Ok(db) = state.db_conn() {
                mark_ner_failed(db.conn(), fnord_id, &e);
            }
        }
    }
}

/// Mark an article's NER status as 'failed' with an error message.
/// Logs the status-update error itself, rather than silently dropping it,
/// so that observability isn't broken when the ner_status column can't be
/// written (e.g. DB lock poisoning, rare corner cases).
fn mark_ner_failed(conn: &Connection, fnord_id: i64, error: &str) {
    let truncated: String = error.chars().take(500).collect();
    if let Err(e) = conn.execute(
        "UPDATE fnords SET ner_status = 'failed', ner_error = ?1 WHERE id = ?2",
        params![&truncated, fnord_id],
    ) {
        warn!(
            "[NER Pipeline] FAILED to persist failure status for article {}: {} (original error: {})",
            fnord_id, e, truncated
        );
    }
}

// ============================================================
// TAURI COMMANDS
// ============================================================

/// Extract entities for a single article
#[tauri::command]
pub async fn extract_entities(
    state: State<'_, AppState>,
    fnord_id: i64,
) -> CmdResult<ExtractionResult> {
    let (provider, effective_model, title, content): (
        Arc<dyn AiTextProvider>,
        String,
        String,
        String,
    ) = {
        let db = state.db_conn()?;
        let (provider, provider_model) =
            create_text_provider(&db, Some(&state.proxy_manager), TaskType::Fast);
        let (title, content): (String, String) = db
            .conn()
            .query_row(
                "SELECT title, COALESCE(content_full, '') FROM fnords WHERE id = ?1",
                params![fnord_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| crate::error::FuckupError::Generic(format!("Article not found: {}", e)))?;
        let effective_model = crate::ai_provider::resolve_effective_model(
            provider.provider_name(),
            "",
            &provider_model,
        );
        (provider, effective_model, title, content)
    };

    if content.is_empty() {
        return Ok(ExtractionResult {
            fnord_id,
            entities_found: 0,
            entities_new: 0,
            success: false,
            error: Some("No content available".to_string()),
        });
    }

    match extract_entities_for_article(provider.as_ref(), &effective_model, &title, &content).await
    {
        Ok((entities, usage)) => {
            let db = state.db_conn()?;
            log_generation_cost(
                db.conn(),
                provider.provider_name(),
                &effective_model,
                &usage,
            );

            let mut new_count = 0;
            let found_count = entities.len();

            // Save entities in a transaction
            db.conn().execute("BEGIN", [])?;
            match (|| -> Result<(), rusqlite::Error> {
                // Remove old entity links for this article
                db.conn().execute(
                    "DELETE FROM fnord_entities WHERE fnord_id = ?1",
                    params![fnord_id],
                )?;

                for entity in &entities {
                    let normalized = normalize_entity_name(&entity.name);
                    let existed: bool = db
                        .conn()
                        .query_row(
                            "SELECT COUNT(*) FROM entities WHERE normalized_name = ?1 AND entity_type = ?2",
                            params![&normalized, &entity.entity_type],
                            |row| row.get::<_, i64>(0),
                        )
                        .map(|c| c > 0)
                        .unwrap_or(false);

                    upsert_entity_for_article(
                        db.conn(),
                        fnord_id,
                        &entity.name,
                        &entity.entity_type,
                        entity.mentions,
                    )?;

                    if !existed {
                        new_count += 1;
                    }
                }
                Ok(())
            })() {
                Ok(()) => {
                    db.conn().execute("COMMIT", [])?;
                }
                Err(e) => {
                    let _ = db.conn().execute("ROLLBACK", []);
                    return Ok(ExtractionResult {
                        fnord_id,
                        entities_found: 0,
                        entities_new: 0,
                        success: false,
                        error: Some(format!("DB error: {}", e)),
                    });
                }
            }

            info!(
                "[NER] Extracted {} entities ({} new) for article {}",
                found_count, new_count, fnord_id
            );

            // Mark ner_status = completed (the single-article command is also
            // used to retry failed articles — this clears the failed state).
            let _ = db.conn().execute(
                "UPDATE fnords SET ner_status = 'completed', ner_error = NULL WHERE id = ?1",
                params![fnord_id],
            );

            Ok(ExtractionResult {
                fnord_id,
                entities_found: found_count,
                entities_new: new_count,
                success: true,
                error: None,
            })
        }
        Err(e) => {
            // Record failure so the backfill can retry and the UI can surface it.
            if let Ok(db) = state.db_conn() {
                mark_ner_failed(db.conn(), fnord_id, &e);
            }
            Ok(ExtractionResult {
                fnord_id,
                entities_found: 0,
                entities_new: 0,
                success: false,
                error: Some(e),
            })
        }
    }
}

/// Batch extract entities for articles that don't have them yet
#[tauri::command]
pub async fn extract_entities_batch(
    state: State<'_, AppState>,
    limit: Option<i32>,
) -> CmdResult<BatchExtractionResult> {
    let limit = limit.unwrap_or(50);
    let _locale = get_locale_from_db(&state);

    // Load articles that haven't had NER run yet OR failed on the last attempt.
    // Using `ner_status` instead of `NOT IN fnord_entities` means articles that
    // legitimately produced zero entities ('completed') are not retried, while
    // transient failures ('failed') are retried with priority.
    let articles: Vec<(i64, String, String)> = {
        let db = state.db_conn()?;
        let mut stmt = db.conn().prepare(
            r#"SELECT f.id, f.title, COALESCE(f.content_full, '')
               FROM fnords f
               WHERE f.content_full IS NOT NULL
                 AND f.content_full != ''
                 AND f.processed_at IS NOT NULL
                 AND (f.ner_status IS NULL OR f.ner_status = 'failed')
               ORDER BY CASE f.ner_status WHEN 'failed' THEN 0 ELSE 1 END,
                        f.published_at DESC
               LIMIT ?1"#,
        )?;
        let rows = stmt
            .query_map(params![limit], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .filter_map(|r| r.ok())
            .collect();
        rows
    };

    if articles.is_empty() {
        return Ok(BatchExtractionResult {
            processed: 0,
            total_entities: 0,
            errors: 0,
        });
    }

    let total_articles = articles.len();
    info!(
        "[NER Batch] Starting extraction for {} articles",
        total_articles
    );

    let mut processed = 0;
    let mut total_entities = 0;
    let mut errors = 0;

    for (fnord_id, title, content) in articles {
        // Create provider per article (short-lived lock)
        let (provider, effective_model) = {
            let db = state.db_conn()?;
            let (provider, provider_model) =
                create_text_provider(&db, Some(&state.proxy_manager), TaskType::Fast);
            let effective_model = crate::ai_provider::resolve_effective_model(
                provider.provider_name(),
                "",
                &provider_model,
            );
            (provider, effective_model)
        };

        match extract_entities_for_article(provider.as_ref(), &effective_model, &title, &content)
            .await
        {
            Ok((entities, usage)) => {
                let db = state.db_conn()?;
                log_generation_cost(
                    db.conn(),
                    provider.provider_name(),
                    &effective_model,
                    &usage,
                );

                // Save in transaction
                db.conn().execute("BEGIN", [])?;
                let save_result = (|| -> Result<usize, rusqlite::Error> {
                    let mut count = 0;
                    for entity in &entities {
                        if is_valid_entity_type(&entity.entity_type) {
                            upsert_entity_for_article(
                                db.conn(),
                                fnord_id,
                                &entity.name,
                                &entity.entity_type,
                                entity.mentions,
                            )?;
                            count += 1;
                        }
                    }
                    Ok(count)
                })();

                match save_result {
                    Ok(count) => {
                        db.conn().execute("COMMIT", [])?;
                        let _ = db.conn().execute(
                            "UPDATE fnords SET ner_status = 'completed', ner_error = NULL WHERE id = ?1",
                            params![fnord_id],
                        );
                        total_entities += count;
                        processed += 1;
                        debug!(
                            "[NER Batch] Article {} '{}': {} entities",
                            fnord_id,
                            &title[..title.len().min(50)],
                            count
                        );
                    }
                    Err(e) => {
                        let _ = db.conn().execute("ROLLBACK", []);
                        warn!("[NER Batch] DB error for article {}: {}", fnord_id, e);
                        mark_ner_failed(db.conn(), fnord_id, &format!("DB error: {}", e));
                        errors += 1;
                    }
                }
            }
            Err(e) => {
                warn!(
                    "[NER Batch] Extraction failed for article {}: {}",
                    fnord_id, e
                );
                if let Ok(db) = state.db_conn() {
                    mark_ner_failed(db.conn(), fnord_id, &e);
                }
                errors += 1;
            }
        }

        // Yield between articles for better concurrency
        tokio::task::yield_now().await;
    }

    info!(
        "[NER Batch] Complete: {}/{} processed, {} entities, {} errors",
        processed, total_articles, total_entities, errors
    );

    Ok(BatchExtractionResult {
        processed,
        total_entities,
        errors,
    })
}

/// Get entities for a specific article
#[tauri::command]
pub async fn get_article_entities(
    state: State<'_, AppState>,
    fnord_id: i64,
) -> CmdResult<Vec<EntityInfo>> {
    let db = state.db_conn()?;
    let mut stmt = db.conn().prepare(
        r#"SELECT e.id, e.name, e.entity_type, e.normalized_name,
                  e.article_count, fe.mention_count, fe.confidence
           FROM entities e
           JOIN fnord_entities fe ON fe.entity_id = e.id
           WHERE fe.fnord_id = ?1
           ORDER BY fe.mention_count DESC, e.name ASC"#,
    )?;

    let entities = stmt
        .query_map(params![fnord_id], |row| {
            Ok(EntityInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: row.get(2)?,
                normalized_name: row.get(3)?,
                article_count: row.get(4)?,
                mention_count: Some(row.get(5)?),
                confidence: Some(row.get(6)?),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entities)
}

/// Search entities by name and optional type filter
#[tauri::command]
pub async fn search_entities(
    state: State<'_, AppState>,
    query: String,
    entity_type: Option<String>,
) -> CmdResult<Vec<EntityInfo>> {
    let db = state.db_conn()?;
    let search_pattern = format!("%{}%", query.to_lowercase());

    let entities: Vec<EntityInfo> = if let Some(ref etype) = entity_type {
        let mut stmt = db.conn().prepare(
            r#"SELECT id, name, entity_type, normalized_name,
                      article_count
               FROM entities
               WHERE normalized_name LIKE ?1
                 AND entity_type = ?2
               ORDER BY article_count DESC
               LIMIT 50"#,
        )?;
        let result: Vec<EntityInfo> = stmt
            .query_map(params![&search_pattern, etype], |row| {
                Ok(EntityInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    normalized_name: row.get(3)?,
                    article_count: row.get(4)?,
                    mention_count: None,
                    confidence: None,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        result
    } else {
        let mut stmt = db.conn().prepare(
            r#"SELECT id, name, entity_type, normalized_name,
                      article_count
               FROM entities
               WHERE normalized_name LIKE ?1
               ORDER BY article_count DESC
               LIMIT 50"#,
        )?;
        let result: Vec<EntityInfo> = stmt
            .query_map(params![&search_pattern], |row| {
                Ok(EntityInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    normalized_name: row.get(3)?,
                    article_count: row.get(4)?,
                    mention_count: None,
                    confidence: None,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        result
    };

    Ok(entities)
}

/// Get all articles that contain a specific entity
#[tauri::command]
pub async fn get_entity_articles(
    state: State<'_, AppState>,
    entity_id: i64,
) -> CmdResult<Vec<EntityArticleInfo>> {
    let db = state.db_conn()?;
    let mut stmt = db.conn().prepare(
        r#"SELECT f.id, f.title, f.url, f.published_at,
                  f.summary, fe.mention_count
           FROM fnords f
           JOIN fnord_entities fe ON fe.fnord_id = f.id
           WHERE fe.entity_id = ?1
           ORDER BY f.published_at DESC
           LIMIT 100"#,
    )?;

    let articles = stmt
        .query_map(params![entity_id], |row| {
            Ok(EntityArticleInfo {
                fnord_id: row.get(0)?,
                title: row.get(1)?,
                url: row.get(2)?,
                published_at: row.get(3)?,
                summary: row.get(4)?,
                mention_count: row.get(5)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(articles)
}

/// Article info for entity detail view
#[derive(Serialize, Debug)]
pub struct EntityArticleInfo {
    pub fnord_id: i64,
    pub title: String,
    pub url: String,
    pub published_at: Option<String>,
    pub summary: Option<String>,
    pub mention_count: i32,
}

/// Get top entities by article count
#[tauri::command]
pub async fn get_top_entities(
    state: State<'_, AppState>,
    entity_type: Option<String>,
    limit: Option<i32>,
) -> CmdResult<Vec<EntityInfo>> {
    let limit = limit.unwrap_or(20);
    let db = state.db_conn()?;

    let entities: Vec<EntityInfo> = if let Some(ref etype) = entity_type {
        let mut stmt = db.conn().prepare(
            r#"SELECT id, name, entity_type, normalized_name,
                      article_count
               FROM entities
               WHERE entity_type = ?1
               ORDER BY article_count DESC
               LIMIT ?2"#,
        )?;
        let result: Vec<EntityInfo> = stmt
            .query_map(params![etype, limit], |row| {
                Ok(EntityInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    normalized_name: row.get(3)?,
                    article_count: row.get(4)?,
                    mention_count: None,
                    confidence: None,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        result
    } else {
        let mut stmt = db.conn().prepare(
            r#"SELECT id, name, entity_type, normalized_name,
                      article_count
               FROM entities
               ORDER BY article_count DESC
               LIMIT ?1"#,
        )?;
        let result: Vec<EntityInfo> = stmt
            .query_map(params![limit], |row| {
                Ok(EntityInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    normalized_name: row.get(3)?,
                    article_count: row.get(4)?,
                    mention_count: None,
                    confidence: None,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        result
    };

    Ok(entities)
}

/// Backfill NER for ALL articles that need it (no limit).
///
/// Selects articles where `ner_status IS NULL` (never attempted) or `'failed'`
/// (transient error on previous run), prioritising failures first. Emits
/// `"ner-backfill-progress"` events after each article so the Settings UI can
/// show a progress bar. Articles that legitimately produced zero entities
/// ('completed') are never retried.
#[tauri::command]
pub async fn extract_entities_backfill_all(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> CmdResult<BatchExtractionResult> {
    // Load all articles needing NER (short lock).
    let articles: Vec<(i64, String, String)> = {
        let db = state.db_conn()?;
        let mut stmt = db.conn().prepare(
            r#"SELECT f.id, f.title, COALESCE(f.content_full, '')
               FROM fnords f
               WHERE f.content_full IS NOT NULL
                 AND f.content_full != ''
                 AND f.processed_at IS NOT NULL
                 AND (f.ner_status IS NULL OR f.ner_status = 'failed')
               ORDER BY CASE f.ner_status WHEN 'failed' THEN 0 ELSE 1 END,
                        f.published_at DESC"#,
        )?;
        let rows: Vec<(i64, String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .filter_map(|r| r.ok())
            .collect();
        rows
    };

    let total = articles.len();
    info!("[NER Backfill] Starting — {} articles queued", total);

    if total == 0 {
        let _ = app_handle.emit(
            "ner-backfill-progress",
            NerBackfillProgress {
                processed: 0,
                total: 0,
                entities_found: 0,
                errors: 0,
                current_fnord_id: None,
            },
        );
        return Ok(BatchExtractionResult {
            processed: 0,
            total_entities: 0,
            errors: 0,
        });
    }

    let mut processed = 0;
    let mut total_entities = 0;
    let mut errors = 0;

    for (fnord_id, title, content) in articles {
        // Progress before each article
        let _ = app_handle.emit(
            "ner-backfill-progress",
            NerBackfillProgress {
                processed,
                total,
                entities_found: total_entities,
                errors,
                current_fnord_id: Some(fnord_id),
            },
        );

        // Short-lived provider setup
        let (provider, effective_model): (Arc<dyn AiTextProvider>, String) = {
            let db = state.db_conn()?;
            let (provider, provider_model) =
                create_text_provider(&db, Some(&state.proxy_manager), TaskType::Fast);
            let effective_model = crate::ai_provider::resolve_effective_model(
                provider.provider_name(),
                "",
                &provider_model,
            );
            (provider, effective_model)
        };

        // Count entities before + after to compute delta
        let before_count: i64 = {
            let db = state.db_conn()?;
            db.conn()
                .query_row(
                    "SELECT COUNT(*) FROM fnord_entities WHERE fnord_id = ?1",
                    params![fnord_id],
                    |row| row.get(0),
                )
                .unwrap_or(0)
        };

        run_ner_for_article_in_pipeline(
            &state,
            provider.as_ref(),
            &effective_model,
            fnord_id,
            &title,
            &content,
        )
        .await;

        // Inspect final status to count success/error
        let (status, after_count): (Option<String>, i64) = {
            let db = state.db_conn()?;
            let status: Option<String> = db
                .conn()
                .query_row(
                    "SELECT ner_status FROM fnords WHERE id = ?1",
                    params![fnord_id],
                    |row| row.get(0),
                )
                .ok();
            let count: i64 = db
                .conn()
                .query_row(
                    "SELECT COUNT(*) FROM fnord_entities WHERE fnord_id = ?1",
                    params![fnord_id],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            (status, count)
        };

        match status.as_deref() {
            Some("completed") => {
                processed += 1;
                let delta = (after_count - before_count).max(0) as usize;
                total_entities += delta;
            }
            _ => {
                errors += 1;
            }
        }

        // Yield between articles for concurrency
        tokio::task::yield_now().await;
    }

    // Final progress event
    let _ = app_handle.emit(
        "ner-backfill-progress",
        NerBackfillProgress {
            processed,
            total,
            entities_found: total_entities,
            errors,
            current_fnord_id: None,
        },
    );

    info!(
        "[NER Backfill] Complete: {}/{} processed, {} entities, {} errors",
        processed, total, total_entities, errors
    );

    Ok(BatchExtractionResult {
        processed,
        total_entities,
        errors,
    })
}

/// Count of articles that still need NER (pending / failed).
#[derive(Serialize, Debug)]
pub struct NerPendingCount {
    pub pending: i64,
    pub failed: i64,
}

/// Returns how many articles are waiting for NER (never tried + failed).
/// Used by the Settings UI to show the backfill button only when needed.
///
/// The WHERE filters match `extract_entities_backfill_all` exactly so the
/// displayed counts always reflect what the backfill will actually process.
#[tauri::command]
pub async fn get_ner_pending_count(state: State<'_, AppState>) -> CmdResult<NerPendingCount> {
    let db = state.db_conn()?;
    let pending: i64 = db
        .conn()
        .query_row(
            r#"SELECT COUNT(*) FROM fnords
               WHERE content_full IS NOT NULL
                 AND content_full != ''
                 AND processed_at IS NOT NULL
                 AND ner_status IS NULL"#,
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let failed: i64 = db
        .conn()
        .query_row(
            r#"SELECT COUNT(*) FROM fnords
               WHERE content_full IS NOT NULL
                 AND content_full != ''
                 AND processed_at IS NOT NULL
                 AND ner_status = 'failed'"#,
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(NerPendingCount { pending, failed })
}
