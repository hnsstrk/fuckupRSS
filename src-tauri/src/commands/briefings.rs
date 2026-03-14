//! Briefings - AI-generated news summaries (daily/weekly)
//!
//! Generates structured briefings from recent articles using the configured
//! AI text provider, stores them in the database, and provides retrieval.

use crate::commands::ai::helpers::{log_generation_cost, TokenUsage};
use crate::ollama::BRIEFING_NUM_CTX;
use crate::AppState;
use log::{info, warn};
use rusqlite::Connection;
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
    pub article_refs: Option<String>,
}

/// Article data used to build briefing prompts
struct BriefingArticle {
    id: i64,
    title: String,
    source: String,
    summary: String,
}

// ============================================================
// BRIEFING ARTICLE SELECTION CONSTANTS
// ============================================================

/// Max articles for daily briefing (sent to LLM)
const DAILY_ARTICLE_LIMIT: usize = 20;
/// Max articles for weekly briefing (sent to LLM)
const WEEKLY_ARTICLE_LIMIT: usize = 35;
/// Max articles from same source (diversity)
const MAX_PER_SOURCE: usize = 3;
/// Min different categories required
const MIN_CATEGORIES: usize = 3;
/// Candidate pool multiplier (fetch more, then filter for diversity)
const CANDIDATE_MULTIPLIER: usize = 3;

/// Spike threshold: recent_count must be > avg * this factor to count as spike
const SPIKE_FACTOR: f64 = 2.0;
/// Score weight for trending keyword matches
const WEIGHT_TREND: f64 = 1.0;
/// Score weight for strong spike keywords
const WEIGHT_SPIKE: f64 = 3.0;
/// Score weight for story cluster membership
const WEIGHT_CLUSTER: f64 = 2.0;
/// Score weight for sachlichkeit (0-4 mapped to 0-1)
const WEIGHT_QUALITY: f64 = 0.5;

/// Scored article candidate before diversity filtering
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ScoredArticle {
    id: i64,
    title: String,
    source: String,
    summary: String,
    pentacle_id: i64,
    category_id: Option<i64>,
    score: f64,
}

// ============================================================
// ARTICLE SELECTION
// ============================================================

/// Select articles for briefing using hybrid scoring.
///
/// Scoring dimensions:
/// 1. Trending keywords with spike detection (immanentize_daily)
/// 2. Story cluster membership (story_cluster_articles)
/// 3. Article quality/sachlichkeit
/// 4. Post-processing: source diversity + category diversity
fn select_briefing_articles(
    conn: &Connection,
    period_start: &str,
    period_type: &str,
) -> Result<Vec<ScoredArticle>, String> {
    let article_limit = if period_type == "weekly" {
        WEEKLY_ARTICLE_LIMIT
    } else {
        DAILY_ARTICLE_LIMIT
    };
    let candidate_limit = article_limit * CANDIDATE_MULTIPLIER;

    // Lookback for spike baseline: 14 days for daily, 28 for weekly
    let baseline_days = if period_type == "weekly" { 28 } else { 14 };

    let query = format!(
        r#"
        WITH trending AS (
            SELECT
                i.id AS keyword_id,
                SUM(CASE WHEN d.date >= date(?1) THEN d.count ELSE 0 END) AS recent_count,
                AVG(d.count) AS avg_count
            FROM immanentize_daily d
            JOIN immanentize i ON i.id = d.immanentize_id
            WHERE d.date >= date(?1, '-{baseline_days} days')
            GROUP BY i.id
            HAVING recent_count > 0
        ),
        article_scores AS (
            SELECT
                f.id,
                f.title,
                COALESCE(p.title, p.url) AS source,
                f.summary,
                f.pentacle_id,
                (SELECT fs.sephiroth_id FROM fnord_sephiroth fs
                 WHERE fs.fnord_id = f.id
                 ORDER BY fs.confidence DESC LIMIT 1) AS category_id,
                -- Dimension 1: Trending keyword matches with spike weighting
                COALESCE(SUM(
                    CASE
                        WHEN t.recent_count > t.avg_count * {spike} THEN {w_spike}
                        WHEN t.recent_count > 0 THEN {w_trend}
                        ELSE 0.0
                    END
                ), 0.0) AS trend_score,
                -- Dimension 2: Story cluster membership
                CASE WHEN EXISTS(
                    SELECT 1 FROM story_cluster_articles sca
                    WHERE sca.fnord_id = f.id
                ) THEN {w_cluster} ELSE 0.0 END AS cluster_score,
                -- Dimension 3: Quality (sachlichkeit 0-4 -> 0-1)
                COALESCE(f.sachlichkeit, 2) * 0.25 * {w_quality} AS quality_score
            FROM fnords f
            JOIN pentacles p ON p.id = f.pentacle_id
            LEFT JOIN fnord_immanentize fi ON fi.fnord_id = f.id
            LEFT JOIN trending t ON t.keyword_id = fi.immanentize_id
            WHERE f.processed_at IS NOT NULL
              AND f.summary IS NOT NULL
              AND f.summary != ''
              AND f.processed_at >= ?1
            GROUP BY f.id
        )
        SELECT
            id, title, source, summary, pentacle_id, category_id,
            (trend_score + cluster_score + quality_score) AS total_score
        FROM article_scores
        ORDER BY total_score DESC, id DESC
        LIMIT ?2
        "#,
        spike = SPIKE_FACTOR,
        w_spike = WEIGHT_SPIKE,
        w_trend = WEIGHT_TREND,
        w_cluster = WEIGHT_CLUSTER,
        w_quality = WEIGHT_QUALITY,
        baseline_days = baseline_days,
    );

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let candidates: Vec<ScoredArticle> = stmt
        .query_map(
            rusqlite::params![period_start, candidate_limit as i64],
            |row| {
                Ok(ScoredArticle {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    source: row.get(2)?,
                    summary: row.get(3)?,
                    pentacle_id: row.get(4)?,
                    category_id: row.get(5)?,
                    score: row.get(6)?,
                })
            },
        )
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Diversity post-processing
    Ok(diversify_articles(candidates, article_limit))
}

/// Post-process scored candidates for source and category diversity.
///
/// Rules:
/// - Max MAX_PER_SOURCE articles from same feed
/// - At least MIN_CATEGORIES different categories
/// - Maintains score ordering within constraints
fn diversify_articles(candidates: Vec<ScoredArticle>, limit: usize) -> Vec<ScoredArticle> {
    use std::collections::{HashMap, HashSet};

    if candidates.is_empty() {
        return candidates;
    }

    let mut result: Vec<ScoredArticle> = Vec::with_capacity(limit);
    let mut source_count: HashMap<i64, usize> = HashMap::new();
    let mut categories_seen: HashSet<i64> = HashSet::new();

    // First pass: select articles respecting source limits
    for article in &candidates {
        if result.len() >= limit {
            break;
        }
        let count = source_count.entry(article.pentacle_id).or_insert(0);
        if *count >= MAX_PER_SOURCE {
            continue;
        }
        *count += 1;
        if let Some(cat) = article.category_id {
            categories_seen.insert(cat);
        }
        result.push(article.clone());
    }

    // Second pass: if category diversity is too low, add articles from missing categories
    if categories_seen.len() < MIN_CATEGORIES && result.len() >= MIN_CATEGORIES {
        // Find articles from categories not yet represented (skip those already in result)
        let result_ids: HashSet<i64> = result.iter().map(|a| a.id).collect();
        let missing_cat_articles: Vec<&ScoredArticle> = candidates
            .iter()
            .filter(|a| {
                !result_ids.contains(&a.id)
                    && a.category_id
                        .map(|c| !categories_seen.contains(&c))
                        .unwrap_or(false)
            })
            .collect();

        // Replace from the end of the first-pass results (lowest scored)
        // Track how many we've replaced so we don't replace diversity articles
        let original_len = result.len();
        let mut replace_idx = original_len;

        for diverse_article in missing_cat_articles {
            if categories_seen.len() >= MIN_CATEGORIES {
                break;
            }
            if let Some(cat) = diverse_article.category_id {
                categories_seen.insert(cat);
            }
            if result.len() < limit {
                // List has room -- just append
                result.push(diverse_article.clone());
            } else if replace_idx > 0 {
                // List full -- replace lowest-scored original article (from end)
                replace_idx -= 1;
                result[replace_idx] = diverse_article.clone();
            }
        }
    }

    result
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

    // Step 1: Load scored + diversified articles and trending keywords (short lock)
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

        // Use hybrid scoring for article selection
        let scored_articles = select_briefing_articles(conn, &p_start, &period_type)?;

        if scored_articles.is_empty() {
            return Err(format!(
                "Keine Artikel mit Zusammenfassung in den letzten {} gefunden",
                period_label
            ));
        }

        // Convert to BriefingArticle format
        let articles: Vec<BriefingArticle> = scored_articles
            .iter()
            .map(|sa| BriefingArticle {
                id: sa.id,
                title: sa.title.clone(),
                source: sa.source.clone(),
                summary: sa.summary.clone(),
            })
            .collect();

        // Load trending keywords from the period (for LLM context)
        let mut kw_stmt = conn
            .prepare(
                r#"SELECT i.name, SUM(d.count) AS total
                   FROM immanentize_daily d
                   JOIN immanentize i ON i.id = d.immanentize_id
                   WHERE d.date >= date(?1)
                   GROUP BY i.name
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

    // Build article_refs JSON for frontend navigation
    let article_refs_json = {
        let refs: Vec<serde_json::Value> = articles
            .iter()
            .enumerate()
            .map(|(i, a)| {
                serde_json::json!({
                    "index": i,
                    "fnord_id": a.id,
                    "title": a.title,
                    "source": a.source
                })
            })
            .collect();
        serde_json::to_string(&refs).unwrap_or_default()
    };

    // Step 2: Build prompt
    let mut article_list = String::new();
    for (i, article) in articles.iter().enumerate() {
        article_list.push_str(&format!(
            "{}: [{}] ({}) — {}\n",
            i,
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
        "Du bist ein Nachrichten-Redakteur. Erstelle ein kompaktes Briefing \
         der wichtigsten Themen als JSON.\n\n\
         Hier sind die {} relevantesten Artikel der letzten {} \
         (vorselektiert nach Trending-Relevanz, Themen-Clustering und Quellenvielfalt):\n\n\
         {}\n\
         Trending-Keywords: {}\n\n\
         Erstelle ein JSON mit:\n\
         - tldr.overview: Ueberblick in 2-3 Saetzen\n\
         - tldr.trends: Bemerkenswerte Trends als Markdown-Liste (z.B. '1. **Trend**: Beschreibung'), ein Trend pro Zeile\n\
         - tldr.conclusion: Fazit und Einordnung\n\
         - topics: Array mit den 5-7 wichtigsten Themen, je:\n\
           - title: Themenueberschrift\n\
           - body: 2-4 Saetze als Markdown-Text\n\
           - article_indices: Array der relevanten Artikel-Nummern (0-basiert)\n\
           - keywords: Array relevanter Keywords aus der Trending-Liste\n\n\
         Kein Redaktionshinweis. Antworte auf Deutsch.",
        article_count, period_label, article_list, keywords_str,
    );

    // Step 3: Create provider with BRIEFING context size (NO lock held)
    let (provider, model) = {
        let db = state.db_conn()?;
        let mut config =
            super::ai::helpers::get_provider_config(&db, Some(&state.proxy_manager));
        // Use larger context for briefings (more articles in prompt)
        if config.ollama_num_ctx < BRIEFING_NUM_CTX {
            config.ollama_num_ctx = BRIEFING_NUM_CTX;
        }
        let model = match config.provider_type {
            crate::ai_provider::ProviderType::Ollama => config.ollama_model.clone(),
            crate::ai_provider::ProviderType::OpenAiCompatible => config.openai_model.clone(),
        };
        (crate::ai_provider::create_provider(&config), model)
    };

    let schema = crate::ollama::briefing_schema();
    let result = provider
        .generate_text(&model, &prompt, Some(schema))
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
                top_keywords, article_count, model_used, article_refs)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
            rusqlite::params![
                &period_type,
                &period_start,
                &period_end,
                &content,
                &keywords_str,
                article_count,
                &model,
                &article_refs_json,
            ],
        )
        .map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();

        // Read back the inserted briefing
        conn.query_row(
            r#"SELECT id, period_type, period_start, period_end, content,
                      top_keywords, article_count, model_used, created_at, article_refs
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
                    article_refs: row.get(9)?,
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
                      top_keywords, article_count, model_used, created_at, article_refs
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
                article_refs: row.get(9)?,
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
                      top_keywords, article_count, model_used, created_at, article_refs
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
                    article_refs: row.get(9)?,
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

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_article(
        id: i64,
        pentacle_id: i64,
        category_id: Option<i64>,
        score: f64,
    ) -> ScoredArticle {
        ScoredArticle {
            id,
            title: format!("Article {}", id),
            source: format!("Source {}", pentacle_id),
            summary: format!("Summary for article {}", id),
            pentacle_id,
            category_id,
            score,
        }
    }

    #[test]
    fn test_diversify_empty() {
        let result = diversify_articles(vec![], 10);
        assert!(result.is_empty());
    }

    #[test]
    fn test_diversify_respects_limit() {
        let candidates: Vec<ScoredArticle> = (1..=10)
            .map(|i| make_article(i, i, Some(1), 10.0 - i as f64))
            .collect();
        let result = diversify_articles(candidates, 5);
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_diversify_source_limit() {
        // 6 articles from same source, limit should cap at MAX_PER_SOURCE
        let mut candidates = vec![];
        for i in 1..=6 {
            candidates.push(make_article(i, 1, Some(1), 10.0 - i as f64)); // all pentacle_id=1
        }
        // Add 2 from different source
        candidates.push(make_article(7, 2, Some(2), 1.0));
        candidates.push(make_article(8, 3, Some(3), 0.5));

        let result = diversify_articles(candidates, 5);

        // Count articles from pentacle_id=1
        let from_source_1 = result.iter().filter(|a| a.pentacle_id == 1).count();
        assert!(
            from_source_1 <= MAX_PER_SOURCE,
            "Expected max {} from same source, got {}",
            MAX_PER_SOURCE,
            from_source_1
        );
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_diversify_preserves_score_order() {
        let candidates = vec![
            make_article(1, 1, Some(1), 10.0),
            make_article(2, 2, Some(2), 8.0),
            make_article(3, 3, Some(3), 5.0),
        ];
        let result = diversify_articles(candidates, 3);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[1].id, 2);
        assert_eq!(result[2].id, 3);
    }

    #[test]
    fn test_diversify_fewer_than_limit() {
        let candidates = vec![
            make_article(1, 1, Some(1), 10.0),
            make_article(2, 2, Some(2), 8.0),
        ];
        let result = diversify_articles(candidates, 10);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_diversify_category_diversity() {
        // All articles from category 1 (high score), one from category 2 (low score)
        let candidates = vec![
            make_article(1, 1, Some(1), 10.0),
            make_article(2, 2, Some(1), 9.0),
            make_article(3, 3, Some(1), 8.0),
            make_article(4, 4, Some(1), 7.0),
            make_article(5, 5, Some(1), 6.0),
            // Low-scored but different categories
            make_article(6, 6, Some(2), 1.0),
            make_article(7, 7, Some(3), 0.5),
            make_article(8, 8, Some(4), 0.1),
        ];
        let result = diversify_articles(candidates, 5);

        // Should include articles from at least MIN_CATEGORIES categories
        let cat_count: std::collections::HashSet<i64> =
            result.iter().filter_map(|a| a.category_id).collect();
        assert!(
            cat_count.len() >= MIN_CATEGORIES,
            "Expected >= {} categories, got {} ({:?})",
            MIN_CATEGORIES,
            cat_count.len(),
            cat_count,
        );
    }

    #[test]
    fn test_constants_valid() {
        assert!(DAILY_ARTICLE_LIMIT > 0);
        assert!(WEEKLY_ARTICLE_LIMIT > DAILY_ARTICLE_LIMIT);
        assert!(MAX_PER_SOURCE > 0);
        assert!(MIN_CATEGORIES > 0);
        assert!(CANDIDATE_MULTIPLIER >= 2);
        assert!(SPIKE_FACTOR > 1.0);
    }
}
