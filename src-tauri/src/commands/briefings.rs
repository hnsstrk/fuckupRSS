//! Briefings - AI-generated news summaries (daily/weekly)
//!
//! Generates structured briefings from recent articles using the configured
//! AI text provider, stores them in the database, and provides retrieval.

use crate::commands::ai::helpers::{create_text_provider, log_generation_cost, TokenUsage};
use crate::AppState;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tauri::State;

// ============================================================
// TYPES
// ============================================================

/// A stored briefing
#[derive(Debug, Serialize, Deserialize)]
pub struct Briefing {
    pub id: i64,
    pub period_type: String,
    pub period_start: String,
    pub period_end: String,
    pub content: String,
    pub top_keywords: Option<String>,
    pub article_count: i64,
    pub model_used: Option<String>,
    pub created_at: String,
}

/// Article data used to build briefing prompts
struct BriefingArticle {
    title: String,
    source: String,
    summary: String,
}

// ============================================================
// COMMANDS
// ============================================================

/// Generate a new AI briefing for the given period type ("daily" or "weekly")
#[tauri::command]
pub async fn generate_briefing(
    state: State<'_, AppState>,
    period_type: String,
) -> Result<Briefing, String> {
    if period_type != "daily" && period_type != "weekly" {
        return Err("period_type must be 'daily' or 'weekly'".to_string());
    }

    info!("Generating {} briefing...", period_type);

    // Step 1: Calculate time range and load articles + keywords (short lock)
    let (articles, trending_keywords, period_start, period_end) = {
        let db = state.db_conn()?;
        let conn = db.conn();

        let hours = if period_type == "daily" { 24 } else { 168 };
        let period_label = if period_type == "daily" {
            "24 Stunden"
        } else {
            "7 Tage"
        };

        // Calculate period boundaries
        let (p_start, p_end): (String, String) = conn
            .query_row(
                &format!(
                    "SELECT datetime('now', '-{} hours'), datetime('now')",
                    hours
                ),
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        // Load top articles with summaries from the period
        let mut stmt = conn
            .prepare(
                r#"SELECT f.title, COALESCE(p.title, p.url) AS source, f.summary
                   FROM fnords f
                   JOIN pentacles p ON f.pentacle_id = p.id
                   WHERE f.processed_at IS NOT NULL
                     AND f.summary IS NOT NULL
                     AND f.summary != ''
                     AND f.processed_at >= ?1
                   ORDER BY f.processed_at DESC
                   LIMIT 15"#,
            )
            .map_err(|e| e.to_string())?;

        let articles: Vec<BriefingArticle> = stmt
            .query_map([&p_start], |row| {
                Ok(BriefingArticle {
                    title: row.get(0)?,
                    source: row.get(1)?,
                    summary: row.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        if articles.is_empty() {
            return Err(format!(
                "Keine Artikel mit Zusammenfassung in den letzten {} gefunden",
                period_label
            ));
        }

        // Load trending keywords from the period
        let mut kw_stmt = conn
            .prepare(
                r#"SELECT i.keyword, SUM(d.count) AS total
                   FROM immanentize_daily d
                   JOIN immanentize i ON i.id = d.immanentize_id
                   WHERE d.date >= date(?1)
                   GROUP BY i.keyword
                   ORDER BY total DESC
                   LIMIT 20"#,
            )
            .map_err(|e| e.to_string())?;

        let keywords: Vec<String> = kw_stmt
            .query_map([&p_start], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        (articles, keywords, p_start, p_end)
    };
    // DB lock released

    let article_count = articles.len() as i64;
    let period_label = if period_type == "daily" {
        "24 Stunden"
    } else {
        "7 Tage"
    };

    // Step 2: Build prompt
    let mut article_list = String::new();
    for (i, article) in articles.iter().enumerate() {
        article_list.push_str(&format!(
            "{}. [{}] ({}) — {}\n",
            i + 1,
            article.title,
            article.source,
            article.summary,
        ));
    }

    let keywords_str = if trending_keywords.is_empty() {
        "Keine Trending-Keywords verfuegbar".to_string()
    } else {
        trending_keywords.join(", ")
    };

    let prompt = format!(
        "System: Du bist ein Nachrichten-Redakteur. \
         Erstelle ein kompaktes Briefing der wichtigsten Themen.\n\n\
         Hier sind die {} wichtigsten Artikel der letzten {}:\n\n\
         {}\n\
         Trending-Keywords: {}\n\n\
         Erstelle ein strukturiertes Briefing mit:\n\
         - Ueberblick (2-3 Saetze)\n\
         - Die 5 wichtigsten Themen mit jeweils 1-2 Saetzen\n\
         - Bemerkenswerte Trends oder Muster\n\n\
         Antworte auf Deutsch.",
        article_count, period_label, article_list, keywords_str,
    );

    // Step 3: Create provider and generate text (NO lock held)
    let (provider, model) = {
        let db = state.db_conn()?;
        create_text_provider(&db, Some(&state.proxy_manager))
    };

    let result = provider
        .generate_text(&model, &prompt, None)
        .await
        .map_err(|e| format!("AI-Fehler: {}", e))?;

    let usage = TokenUsage::from(&result);
    let content = result.text;

    info!(
        "Briefing generated: {} chars, {} articles",
        content.len(),
        article_count
    );

    // Step 4: Store briefing in database (short lock)
    let briefing = {
        let db = state.db_conn()?;
        let conn = db.conn();

        // Log cost
        log_generation_cost(conn, provider.provider_name(), &model, &usage);

        // Insert or replace briefing
        conn.execute(
            r#"INSERT OR REPLACE INTO briefings
               (period_type, period_start, period_end, content,
                top_keywords, article_count, model_used)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            rusqlite::params![
                &period_type,
                &period_start,
                &period_end,
                &content,
                &keywords_str,
                article_count,
                &model,
            ],
        )
        .map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();

        // Read back the inserted briefing
        conn.query_row(
            r#"SELECT id, period_type, period_start, period_end, content,
                      top_keywords, article_count, model_used, created_at
               FROM briefings WHERE id = ?1"#,
            [id],
            |row| {
                Ok(Briefing {
                    id: row.get(0)?,
                    period_type: row.get(1)?,
                    period_start: row.get(2)?,
                    period_end: row.get(3)?,
                    content: row.get(4)?,
                    top_keywords: row.get(5)?,
                    article_count: row.get(6)?,
                    model_used: row.get(7)?,
                    created_at: row.get(8)?,
                })
            },
        )
        .map_err(|e| e.to_string())?
    };

    Ok(briefing)
}

/// Get the most recent briefings
#[tauri::command]
pub fn get_briefings(state: State<AppState>, limit: Option<i32>) -> Result<Vec<Briefing>, String> {
    let db = state.db_conn()?;
    let conn = db.conn();
    let limit = limit.unwrap_or(10).min(50);

    let mut stmt = conn
        .prepare(
            r#"SELECT id, period_type, period_start, period_end, content,
                      top_keywords, article_count, model_used, created_at
               FROM briefings
               ORDER BY created_at DESC
               LIMIT ?1"#,
        )
        .map_err(|e| e.to_string())?;

    let briefings = stmt
        .query_map([limit], |row| {
            Ok(Briefing {
                id: row.get(0)?,
                period_type: row.get(1)?,
                period_start: row.get(2)?,
                period_end: row.get(3)?,
                content: row.get(4)?,
                top_keywords: row.get(5)?,
                article_count: row.get(6)?,
                model_used: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(briefings)
}

/// Get the latest briefing of a specific type
#[tauri::command]
pub fn get_latest_briefing(
    state: State<AppState>,
    period_type: String,
) -> Result<Option<Briefing>, String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    let result = conn
        .query_row(
            r#"SELECT id, period_type, period_start, period_end, content,
                      top_keywords, article_count, model_used, created_at
               FROM briefings
               WHERE period_type = ?1
               ORDER BY created_at DESC
               LIMIT 1"#,
            [&period_type],
            |row| {
                Ok(Briefing {
                    id: row.get(0)?,
                    period_type: row.get(1)?,
                    period_start: row.get(2)?,
                    period_end: row.get(3)?,
                    content: row.get(4)?,
                    top_keywords: row.get(5)?,
                    article_count: row.get(6)?,
                    model_used: row.get(7)?,
                    created_at: row.get(8)?,
                })
            },
        )
        .ok();

    Ok(result)
}

/// Delete a briefing by ID
#[tauri::command]
pub fn delete_briefing(state: State<AppState>, id: i64) -> Result<bool, String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    let deleted = conn
        .execute("DELETE FROM briefings WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;

    if deleted > 0 {
        info!("Deleted briefing {}", id);
    } else {
        warn!("Briefing {} not found for deletion", id);
    }

    Ok(deleted > 0)
}
