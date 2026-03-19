use crate::embeddings::{blob_to_embedding, cosine_similarity};
use crate::keywords::{get_compound_components, should_split_compound};
use crate::similarity::string::{
    calculate_abbreviation_score, calculate_exact_token_match_score, calculate_string_similarity,
};
use crate::{find_canonical_keyword_with_db, normalize_keyword, AppState};
use log::{info, trace, warn};
use regex::Regex;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::{Emitter, State};

// ============================================================
// IMMANENTIZE NETWORK API
// ============================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Keyword {
    pub id: i64,
    pub name: String,
    pub count: i64,
    pub article_count: i64,
    pub cluster_id: Option<i64>,
    pub is_canonical: bool,
    pub canonical_id: Option<i64>,
    pub first_seen: Option<String>,
    pub last_used: Option<String>,
    pub quality_score: Option<f64>,
    pub keyword_type: String,
}

/// Keyword neighbor with relationship strength
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeywordNeighbor {
    pub id: i64,
    pub name: String,
    pub cooccurrence: i64,
    pub embedding_similarity: Option<f64>,
    pub combined_weight: f64,
}

/// Category association for a keyword
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeywordCategory {
    pub sephiroth_id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub weight: f64,
    pub article_count: i64,
    pub parent_id: Option<i64>,
    pub parent_name: Option<String>,
    pub parent_icon: Option<String>,
}

/// Keyword cluster (for future clustering feature)
#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
pub struct KeywordCluster {
    pub id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub keyword_count: i64,
    pub auto_generated: bool,
}

/// Trending keyword with recent activity
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrendingKeyword {
    pub id: i64,
    pub name: String,
    pub total_count: i64,
    pub recent_count: i64,   // Last N days
    pub growth_rate: f64,    // (recent - previous) / previous
    pub trending_score: f64, // ln(recent_count + 1) * (1.0 + growth_rate)
    pub is_new: bool,        // true when previous_count == 0 and recent_count > 0
}

/// Network statistics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkStats {
    pub total_keywords: i64,
    pub total_connections: i64,
    pub total_clusters: i64,
    pub avg_neighbors_per_keyword: f64,
}

// ============================================================
// HELPER FUNCTIONS
// ============================================================

fn keyword_from_row(row: &rusqlite::Row) -> Result<Keyword, rusqlite::Error> {
    Ok(Keyword {
        id: row.get(0)?,
        name: row.get(1)?,
        count: row.get(2)?,
        article_count: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
        cluster_id: row.get(4)?,
        is_canonical: row.get::<_, Option<bool>>(5)?.unwrap_or(true),
        canonical_id: row.get(6)?,
        first_seen: row.get(7)?,
        last_used: row.get(8)?,
        quality_score: row.get(9)?,
        keyword_type: row
            .get::<_, Option<String>>(10)?
            .unwrap_or_else(|| "concept".to_string()),
    })
}

const KEYWORD_SELECT_COLUMNS: &str =
    "id, name, count, article_count, cluster_id, is_canonical, canonical_id, first_seen, last_used, quality_score, keyword_type";

// ============================================================
// QUERIES
// ============================================================

/// Get all keywords (paginated)
#[tauri::command]
pub fn get_keywords(
    state: State<AppState>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Keyword>, String> {
    let db = state.db_conn()?;
    let limit = limit.unwrap_or(100);
    let offset = offset.unwrap_or(0);

    let sql = format!(
        "SELECT {} FROM immanentize WHERE is_canonical = TRUE OR is_canonical IS NULL ORDER BY article_count DESC, count DESC LIMIT ? OFFSET ?",
        KEYWORD_SELECT_COLUMNS
    );

    let mut stmt = to_cmd_err!(db.conn().prepare(&sql));

    let keywords = to_cmd_err!(to_cmd_err!(
        stmt.query_map(rusqlite::params![limit, offset], keyword_from_row)
    )
    .collect::<Result<Vec<_>, _>>());

    Ok(keywords)
}

#[tauri::command]
pub fn get_keyword(state: State<AppState>, id: i64) -> Result<Option<Keyword>, String> {
    let db = state.db_conn()?;

    let sql = format!(
        "SELECT {} FROM immanentize WHERE id = ?",
        KEYWORD_SELECT_COLUMNS
    );

    let keyword = db.conn().query_row(&sql, [id], keyword_from_row).ok();

    Ok(keyword)
}

/// Get top N neighbors of a keyword
#[tauri::command]
pub fn get_keyword_neighbors(
    state: State<AppState>,
    id: i64,
    limit: Option<i64>,
) -> Result<Vec<KeywordNeighbor>, String> {
    let db = state.db_conn()?;
    let limit = limit.unwrap_or(20);

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                i.id,
                i.name,
                n.cooccurrence,
                n.embedding_similarity,
                n.combined_weight
            FROM immanentize_neighbors n
            JOIN immanentize i ON i.id = CASE
                WHEN n.immanentize_id_a = ?1 THEN n.immanentize_id_b
                ELSE n.immanentize_id_a
            END
            WHERE n.immanentize_id_a = ?1 OR n.immanentize_id_b = ?1
            ORDER BY n.combined_weight DESC, n.cooccurrence DESC
            LIMIT ?2
            "#,
        )
        .map_err(|e| e.to_string())?;

    let neighbors = to_cmd_err!(
        to_cmd_err!(stmt.query_map(rusqlite::params![id, limit], |row| {
            Ok(KeywordNeighbor {
                id: row.get(0)?,
                name: row.get(1)?,
                cooccurrence: row.get(2)?,
                embedding_similarity: row.get(3)?,
                combined_weight: row.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
            })
        }))
        .collect::<Result<Vec<_>, _>>()
    );

    Ok(neighbors)
}

/// Get categories associated with a keyword (subcategories with parent color)
#[tauri::command]
pub fn get_keyword_categories(
    state: State<AppState>,
    id: i64,
) -> Result<Vec<KeywordCategory>, String> {
    let db = state.db_conn()?;

    let mut stmt = to_cmd_err!(db.conn().prepare(
        r#"
            SELECT s.id, s.name, s.icon, COALESCE(m.color, s.color), ims.weight, ims.article_count,
                   s.parent_id, m.name as parent_name, m.icon as parent_icon
            FROM immanentize_sephiroth ims
            JOIN sephiroth s ON s.id = ims.sephiroth_id
            LEFT JOIN sephiroth m ON m.id = s.parent_id
            WHERE ims.immanentize_id = ?
            ORDER BY m.name, ims.weight DESC
            "#,
    ));

    let categories = to_cmd_err!(to_cmd_err!(stmt.query_map([id], |row| {
        Ok(KeywordCategory {
            sephiroth_id: row.get(0)?,
            name: row.get(1)?,
            icon: row.get(2)?,
            color: row.get(3)?,
            weight: row.get(4)?,
            article_count: row.get(5)?,
            parent_id: row.get(6)?,
            parent_name: row.get(7)?,
            parent_icon: row.get(8)?,
        })
    }))
    .collect::<Result<Vec<_>, _>>());

    Ok(categories)
}

#[tauri::command]
pub fn get_category_keywords(
    state: State<AppState>,
    sephiroth_id: i64,
    limit: Option<i64>,
) -> Result<Vec<Keyword>, String> {
    let db = state.db_conn()?;
    let limit = limit.unwrap_or(50);

    let mut stmt = to_cmd_err!(db.conn().prepare(
        "SELECT i.id, i.name, i.count, i.article_count, i.cluster_id, i.is_canonical, i.canonical_id, i.first_seen, i.last_used, i.quality_score, i.keyword_type \
         FROM immanentize_sephiroth ims \
         JOIN immanentize i ON i.id = ims.immanentize_id \
         WHERE ims.sephiroth_id = ? \
         ORDER BY ims.weight DESC, ims.article_count DESC \
         LIMIT ?",
    ));

    let keywords = to_cmd_err!(to_cmd_err!(
        stmt.query_map(rusqlite::params![sephiroth_id, limit], keyword_from_row)
    )
    .collect::<Result<Vec<_>, _>>());

    Ok(keywords)
}

/// Get trending keywords (most growth in last N days)
///
/// Uses immanentize_daily for efficient period comparison:
/// - Recent period: last N days
/// - Previous period: N to 2N days ago
/// - trending_score = ln(recent_count + 1) * (1.0 + growth_rate)
///
/// sort_by options:
/// - "score" (default): trending_score DESC
/// - "growth": growth_rate DESC, then recent_count DESC
/// - "count": recent_count DESC, then growth_rate DESC
/// - "new": is_new DESC, then trending_score DESC
#[tauri::command]
pub fn get_trending_keywords(
    state: State<AppState>,
    days: Option<i64>,
    limit: Option<i64>,
    sort_by: Option<String>,
) -> Result<Vec<TrendingKeyword>, String> {
    let db = state.db_conn()?;
    let days = days.unwrap_or(7);
    let limit = limit.unwrap_or(20);
    let sort_by = sort_by.unwrap_or_else(|| "score".to_string());

    let mut stmt = to_cmd_err!(db.conn().prepare(
        r#"
            SELECT
                i.id,
                i.name,
                i.article_count as total_count,
                COALESCE(recent.cnt, 0) as recent_count,
                COALESCE(prev.cnt, 0) as previous_count
            FROM immanentize i
            LEFT JOIN (
                SELECT immanentize_id, SUM(count) as cnt
                FROM immanentize_daily
                WHERE date >= DATE('now', '-' || ?1 || ' days')
                GROUP BY immanentize_id
            ) recent ON recent.immanentize_id = i.id
            LEFT JOIN (
                SELECT immanentize_id, SUM(count) as cnt
                FROM immanentize_daily
                WHERE date >= DATE('now', '-' || (?1 * 2) || ' days')
                  AND date < DATE('now', '-' || ?1 || ' days')
                GROUP BY immanentize_id
            ) prev ON prev.immanentize_id = i.id
            WHERE (i.is_canonical = TRUE OR i.is_canonical IS NULL)
              AND i.article_count >= 3
              AND COALESCE(recent.cnt, 0) > 0
            "#,
    ));

    let mut keywords = stmt
        .query_map(rusqlite::params![days], |row| {
            let total: i64 = row.get::<_, Option<i64>>(2)?.unwrap_or(0);
            let recent: i64 = row.get::<_, Option<i64>>(3)?.unwrap_or(0);
            let previous: i64 = row.get::<_, Option<i64>>(4)?.unwrap_or(0);

            let growth_rate = if previous > 0 {
                (recent as f64 - previous as f64) / previous as f64
            } else if recent > 0 {
                1.0
            } else {
                0.0
            };

            let trending_score = (recent as f64 + 1.0).ln() * (1.0 + growth_rate);
            let is_new = previous == 0 && recent > 0;

            Ok(TrendingKeyword {
                id: row.get(0)?,
                name: row.get(1)?,
                total_count: total,
                recent_count: recent,
                growth_rate,
                trending_score,
                is_new,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Sort in Rust (ln() not available in SQLite)
    match sort_by.as_str() {
        "growth" => keywords.sort_by(|a, b| {
            b.growth_rate
                .partial_cmp(&a.growth_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.recent_count.cmp(&a.recent_count))
        }),
        "count" => keywords.sort_by(|a, b| {
            b.recent_count.cmp(&a.recent_count).then_with(|| {
                b.growth_rate
                    .partial_cmp(&a.growth_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        }),
        "new" => keywords.sort_by(|a, b| {
            b.is_new.cmp(&a.is_new).then_with(|| {
                b.trending_score
                    .partial_cmp(&a.trending_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        }),
        _ => keywords.sort_by(|a, b| {
            b.trending_score
                .partial_cmp(&a.trending_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }

    keywords.truncate(limit as usize);

    Ok(keywords)
}

/// Get network statistics
#[tauri::command]
pub fn get_network_stats(state: State<AppState>) -> Result<NetworkStats, String> {
    let db = state.db_conn()?;

    let total_keywords: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM immanentize", [], |row| row.get(0))
        .unwrap_or(0);

    let total_connections: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM immanentize_neighbors", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    let total_clusters: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM immanentize_clusters", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    let avg_neighbors = if total_keywords > 0 {
        (total_connections as f64 * 2.0) / total_keywords as f64
    } else {
        0.0
    };

    Ok(NetworkStats {
        total_keywords,
        total_connections,
        total_clusters,
        avg_neighbors_per_keyword: avg_neighbors,
    })
}

#[tauri::command]
pub fn search_keywords(
    state: State<AppState>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<Keyword>, String> {
    let db = state.db_conn()?;
    let limit = limit.unwrap_or(20);
    let query_lower = query.to_lowercase();

    // Build fuzzy pattern: "abc" -> "%a%b%c%"
    let fuzzy_pattern: String = query_lower
        .chars()
        .map(|c| format!("%{}", c))
        .collect::<String>()
        + "%";

    // Also keep exact contains pattern for prioritization
    let contains_pattern = format!("%{}%", query_lower);
    let starts_pattern = format!("{}%", query_lower);

    // Search with priority: exact start > contains > fuzzy
    // Use CASE to create a sort priority
    let sql = format!(
        "SELECT {} FROM immanentize
         WHERE LOWER(name) LIKE ?1
         ORDER BY
           CASE
             WHEN LOWER(name) LIKE ?2 THEN 0
             WHEN LOWER(name) LIKE ?3 THEN 1
             ELSE 2
           END,
           article_count DESC,
           count DESC
         LIMIT ?4",
        KEYWORD_SELECT_COLUMNS
    );

    let mut stmt = db.conn().prepare(&sql).map_err(|e| e.to_string())?;

    let keywords = stmt
        .query_map(
            rusqlite::params![fuzzy_pattern, starts_pattern, contains_pattern, limit],
            keyword_from_row,
        )
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(keywords)
}

// ============================================================
// GRAPH & TREND VISUALIZATION API
// ============================================================

/// Daily count for trend visualization
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DailyCount {
    pub date: String, // "2024-01-15"
    pub count: i64,
}

/// Graph node for Cytoscape visualization
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphNode {
    pub id: i64,
    pub name: String,
    pub count: i64,
    pub article_count: i64,
    pub cluster_id: Option<i64>,
    pub primary_category_id: Option<i64>,
    pub primary_category_name: Option<String>,
}

/// Graph edge for Cytoscape visualization
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphEdge {
    pub source: i64,
    pub target: i64,
    pub weight: f64,
    pub cooccurrence: i64,
}

/// Network graph data for visualization
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// Trend comparison data for multiple keywords
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrendComparison {
    pub keywords: Vec<KeywordTrendData>,
    pub dates: Vec<String>,
}

/// Trend data for a single keyword
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeywordTrendData {
    pub id: i64,
    pub name: String,
    pub counts: Vec<i64>, // Counts aligned with dates
}

/// Get daily trend data for a keyword
#[tauri::command]
pub fn get_keyword_trend(
    state: State<AppState>,
    id: i64,
    days: Option<i64>,
) -> Result<Vec<DailyCount>, String> {
    let db = state.db_conn()?;
    let days = days.unwrap_or(30);

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT date, count
            FROM immanentize_daily
            WHERE immanentize_id = ?1
            AND date >= DATE('now', '-' || ?2 || ' days')
            ORDER BY date ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let trend = stmt
        .query_map(rusqlite::params![id, days], |row| {
            Ok(DailyCount {
                date: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(trend)
}

/// Get network graph data for visualization
#[tauri::command]
pub fn get_network_graph(
    state: State<AppState>,
    limit: Option<i64>,
    min_weight: Option<f64>,
    min_article_count: Option<i64>,
) -> Result<NetworkGraph, String> {
    let db = state.db_conn()?;
    let limit = limit.unwrap_or(100);
    let min_weight = min_weight.unwrap_or(0.1);
    let min_article_count = min_article_count.unwrap_or(3);

    // Get top keywords as nodes (filtered by min_article_count) with primary category
    let mut node_stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                i.id,
                i.name,
                i.count,
                i.article_count,
                i.cluster_id,
                pc.sephiroth_id as primary_category_id,
                s.name as primary_category_name
            FROM immanentize i
            LEFT JOIN (
                SELECT immanentize_id, sephiroth_id, weight,
                       ROW_NUMBER() OVER (PARTITION BY immanentize_id ORDER BY weight DESC) as rn
                FROM immanentize_sephiroth
            ) pc ON pc.immanentize_id = i.id AND pc.rn = 1
            LEFT JOIN sephiroth s ON s.id = pc.sephiroth_id
            WHERE (i.is_canonical = TRUE OR i.is_canonical IS NULL)
            AND i.article_count >= ?
            ORDER BY i.article_count DESC
            LIMIT ?
            "#,
        )
        .map_err(|e| e.to_string())?;

    let nodes: Vec<GraphNode> = node_stmt
        .query_map([min_article_count, limit], |row| {
            Ok(GraphNode {
                id: row.get(0)?,
                name: row.get(1)?,
                count: row.get(2)?,
                article_count: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                cluster_id: row.get(4)?,
                primary_category_id: row.get(5)?,
                primary_category_name: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Get node IDs for filtering edges
    let node_ids: Vec<i64> = nodes.iter().map(|n| n.id).collect();
    if node_ids.is_empty() {
        return Ok(NetworkGraph {
            nodes: vec![],
            edges: vec![],
        });
    }

    // Build a parameterized query for edges
    let placeholders: Vec<String> = node_ids.iter().map(|_| "?".to_string()).collect();
    let placeholders_str = placeholders.join(",");

    let query = format!(
        r#"
        SELECT immanentize_id_a, immanentize_id_b, combined_weight, cooccurrence
        FROM immanentize_neighbors
        WHERE immanentize_id_a IN ({0})
        AND immanentize_id_b IN ({0})
        AND combined_weight >= ?
        ORDER BY combined_weight DESC
        "#,
        placeholders_str
    );

    let mut edge_stmt = db.conn().prepare(&query).map_err(|e| e.to_string())?;

    // Build parameters: node_ids twice (for a and b) + min_weight
    let mut params: Vec<rusqlite::types::Value> = node_ids
        .iter()
        .map(|&id| rusqlite::types::Value::Integer(id))
        .collect();
    params.extend(
        node_ids
            .iter()
            .map(|&id| rusqlite::types::Value::Integer(id)),
    );
    params.push(rusqlite::types::Value::Real(min_weight));

    let edges: Vec<GraphEdge> = edge_stmt
        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(GraphEdge {
                source: row.get(0)?,
                target: row.get(1)?,
                weight: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                cooccurrence: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(NetworkGraph { nodes, edges })
}

/// Get trend comparison for multiple keywords
#[tauri::command]
pub fn get_trending_comparison(
    state: State<AppState>,
    ids: Vec<i64>,
    days: Option<i64>,
) -> Result<TrendComparison, String> {
    let db = state.db_conn()?;
    let days = days.unwrap_or(30);

    if ids.is_empty() {
        return Ok(TrendComparison {
            keywords: vec![],
            dates: vec![],
        });
    }

    // Generate all dates in the range (single query)
    let dates: Vec<String> = db
        .conn()
        .prepare(
            r#"
            WITH RECURSIVE dates(date) AS (
                SELECT DATE('now', '-' || ?1 || ' days')
                UNION ALL
                SELECT DATE(date, '+1 day')
                FROM dates
                WHERE date < DATE('now')
            )
            SELECT date FROM dates ORDER BY date
            "#,
        )
        .map_err(|e| e.to_string())?
        .query_map([days], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Build placeholder string for IN clause
    let placeholders: String = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");

    // Get all keyword names in a single query
    let names_query = format!(
        "SELECT id, name FROM immanentize WHERE id IN ({})",
        placeholders
    );
    let mut names_stmt = db.conn().prepare(&names_query).map_err(|e| e.to_string())?;
    let names_map: std::collections::HashMap<i64, String> = names_stmt
        .query_map(rusqlite::params_from_iter(ids.iter()), |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Get all daily counts in a single query
    let counts_query = format!(
        r#"
        SELECT immanentize_id, date, count
        FROM immanentize_daily
        WHERE immanentize_id IN ({})
        AND date >= DATE('now', '-' || ? || ' days')
        "#,
        placeholders
    );
    let mut counts_stmt = db
        .conn()
        .prepare(&counts_query)
        .map_err(|e| e.to_string())?;

    // Build params: all ids first, then days
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = ids
        .iter()
        .map(|id| Box::new(*id) as Box<dyn rusqlite::ToSql>)
        .collect();
    params.push(Box::new(days));

    let mut daily_counts: std::collections::HashMap<i64, std::collections::HashMap<String, i64>> =
        std::collections::HashMap::new();

    let rows = counts_stmt
        .query_map(
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?;

    for (id, date, count) in rows.flatten() {
        daily_counts.entry(id).or_default().insert(date, count);
    }

    // Build result preserving original order, skip entries without name (orphaned)
    let keywords: Vec<KeywordTrendData> = ids
        .iter()
        .filter_map(|&id| {
            let name = names_map.get(&id).cloned()?;
            let id_counts = daily_counts.get(&id);
            let counts: Vec<i64> = dates
                .iter()
                .map(|d| id_counts.and_then(|c| c.get(d)).copied().unwrap_or(0))
                .collect();
            Some(KeywordTrendData { id, name, counts })
        })
        .collect();

    Ok(TrendComparison { keywords, dates })
}

// ============================================================
// KEYWORD ARTICLES API
// ============================================================

/// Article linked to a keyword (minimal info for list display)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeywordArticle {
    pub id: i64,
    pub title: String,
    pub pentacle_title: Option<String>,
    pub published_at: Option<String>,
    pub status: String,
}

/// Get articles that have a specific keyword
#[tauri::command]
pub fn get_keyword_articles(
    state: State<AppState>,
    id: i64,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<KeywordArticle>, String> {
    let db = state.db_conn()?;
    let limit = limit.unwrap_or(20);
    let offset = offset.unwrap_or(0);

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT f.id, f.title, p.title as pentacle_title, f.published_at, f.status
            FROM fnords f
            JOIN fnord_immanentize fi ON fi.fnord_id = f.id
            LEFT JOIN pentacles p ON p.id = f.pentacle_id
            WHERE fi.immanentize_id = ?1
            ORDER BY f.published_at DESC
            LIMIT ?2 OFFSET ?3
            "#,
        )
        .map_err(|e| e.to_string())?;

    let articles = stmt
        .query_map(rusqlite::params![id, limit, offset], |row| {
            Ok(KeywordArticle {
                id: row.get(0)?,
                title: row.get(1)?,
                pentacle_title: row.get(2)?,
                published_at: row.get(3)?,
                status: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(articles)
}

#[derive(Debug, Serialize)]
pub struct PruneResult {
    pub removed_keywords: i64,
    pub removed_orphan_relations: i64,
}

#[tauri::command]
pub fn prune_keywords(
    state: State<AppState>,
    min_article_count: Option<i64>,
    older_than_days: Option<i64>,
) -> Result<PruneResult, String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    let min_articles = min_article_count.unwrap_or(1);
    let days = older_than_days.unwrap_or(30);

    let removed_keywords: i64 = conn
        .query_row(
            r#"SELECT COUNT(*) FROM immanentize 
               WHERE article_count <= ?1 
               AND last_used < datetime('now', '-' || ?2 || ' days')"#,
            rusqlite::params![min_articles, days],
            |row| row.get(0),
        )
        .unwrap_or(0);

    conn.execute(
        r#"DELETE FROM immanentize
           WHERE article_count <= ?1
           AND last_used < datetime('now', '-' || ?2 || ' days')"#,
        rusqlite::params![min_articles, days],
    )
    .map_err(|e| e.to_string())?;

    // Note: CASCADE on immanentize FKs automatically cleans up
    // immanentize_neighbors, immanentize_sephiroth, immanentize_daily,
    // embedding_queue, dismissed_synonyms, preserved_compounds, compound_decisions.
    // Trigger immanentize_delete_vec handles vec_immanentize.

    Ok(PruneResult {
        removed_keywords,
        removed_orphan_relations: 0,
    })
}

#[derive(Debug, Serialize)]
pub struct KeywordHealthStats {
    pub total_keywords: i64,
    pub single_use_keywords: i64,
    pub active_keywords: i64,
    pub orphan_keywords: i64,
    pub duplicate_candidates: i64,
    pub oldest_unused_days: i64,
}

#[tauri::command]
pub fn get_keyword_health(state: State<AppState>) -> Result<KeywordHealthStats, String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    let total_keywords: i64 = conn
        .query_row("SELECT COUNT(*) FROM immanentize", [], |row| row.get(0))
        .unwrap_or(0);

    let single_use_keywords: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM immanentize WHERE article_count <= 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let active_keywords: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM immanentize WHERE last_used > datetime('now', '-7 days')",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let orphan_keywords: i64 = conn
        .query_row(
            r#"SELECT COUNT(*) FROM immanentize 
               WHERE id NOT IN (SELECT DISTINCT immanentize_id FROM fnord_immanentize)"#,
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let duplicate_candidates: i64 = conn
        .query_row(
            r#"SELECT COUNT(*) FROM (
                SELECT LOWER(name) as ln, COUNT(*) as cnt 
                FROM immanentize 
                GROUP BY LOWER(name) 
                HAVING cnt > 1
            )"#,
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let oldest_unused_days: i64 = conn
        .query_row(
            r#"SELECT COALESCE(MAX(CAST((julianday('now') - julianday(last_used)) AS INTEGER)), 0) 
               FROM immanentize"#,
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(KeywordHealthStats {
        total_keywords,
        single_use_keywords,
        active_keywords,
        orphan_keywords,
        duplicate_candidates,
        oldest_unused_days,
    })
}

#[derive(Debug, Serialize)]
pub struct MergeResult {
    pub merged_count: i64,
    pub affected_articles: i64,
}

#[tauri::command]
pub fn merge_synonym_keywords(state: State<AppState>) -> Result<MergeResult, String> {
    // Load keywords with short lock
    let keywords: Vec<(i64, String)> = {
        let db = state.db_conn()?;
        let conn = db.conn();
        let mut stmt = conn
            .prepare("SELECT id, name FROM immanentize")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // Identify merge candidates (canonical lookups don't need DB)
    let mut merge_candidates: Vec<(i64, String, i64)> = Vec::new();
    for (id, name) in &keywords {
        if let Some(canonical) = find_canonical_keyword_with_db(name) {
            if canonical.to_lowercase() != name.to_lowercase() {
                // We'll look up the canonical_id in the transaction
                merge_candidates.push((*id, canonical.clone(), 0));
            }
        }
    }

    // Perform all merges in a single transaction
    let db = state.db_conn()?;
    let conn = db.conn();

    conn.execute("BEGIN TRANSACTION", [])
        .map_err(|e| e.to_string())?;

    let mut merged_count = 0i64;
    let mut affected_articles = 0i64;

    for (id, canonical, _) in &merge_candidates {
        let canonical_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM immanentize WHERE LOWER(name) = LOWER(?)",
                [canonical],
                |row| row.get(0),
            )
            .ok();

        if let Some(can_id) = canonical_id {
            if can_id != *id {
                let moved: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM fnord_immanentize WHERE immanentize_id = ?",
                        [id],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);

                // Merge operation: move references, update counts, delete old keyword
                if let Err(e) = conn.execute(
                    r#"UPDATE OR IGNORE fnord_immanentize
                       SET immanentize_id = ?1
                       WHERE immanentize_id = ?2"#,
                    rusqlite::params![can_id, id],
                ) {
                    warn!(
                        "Failed to move keyword references {} -> {}: {}",
                        id, can_id, e
                    );
                    continue;
                }

                if let Err(e) = conn.execute(
                    "DELETE FROM fnord_immanentize WHERE immanentize_id = ?",
                    [id],
                ) {
                    trace!(
                        "Failed to clean duplicate references for keyword {}: {}",
                        id,
                        e
                    );
                }

                if let Err(e) = conn.execute(
                    r#"UPDATE immanentize SET
                       count = count + (SELECT COALESCE(count, 0) FROM immanentize WHERE id = ?2),
                       article_count = (SELECT COUNT(DISTINCT fnord_id) FROM fnord_immanentize WHERE immanentize_id = ?1)
                       WHERE id = ?1"#,
                    rusqlite::params![can_id, id],
                ) {
                    warn!("Failed to update merged keyword counts: {}", e);
                }

                // Transfer category associations (sephiroth) from source to target
                if let Err(e) = conn.execute(
                    r#"INSERT INTO immanentize_sephiroth (immanentize_id, sephiroth_id, weight, article_count, first_seen, updated_at)
                       SELECT ?1, sephiroth_id, weight, article_count, first_seen, updated_at
                       FROM immanentize_sephiroth WHERE immanentize_id = ?2
                       ON CONFLICT(immanentize_id, sephiroth_id) DO UPDATE SET
                           article_count = article_count + excluded.article_count,
                           updated_at = CURRENT_TIMESTAMP"#,
                    rusqlite::params![can_id, id],
                ) {
                    warn!("Failed to transfer sephiroth for keyword {}: {}", id, e);
                }

                // Transfer neighbor edges (re-wire, respecting CHECK constraint)
                // Case 1: source is id_a
                if let Err(e) = conn.execute(
                    r#"INSERT INTO immanentize_neighbors (immanentize_id_a, immanentize_id_b, cooccurrence, embedding_similarity, combined_weight, first_seen, last_seen)
                       SELECT
                           CASE WHEN ?1 < immanentize_id_b THEN ?1 ELSE immanentize_id_b END,
                           CASE WHEN ?1 < immanentize_id_b THEN immanentize_id_b ELSE ?1 END,
                           cooccurrence, embedding_similarity, combined_weight, first_seen, last_seen
                       FROM immanentize_neighbors
                       WHERE immanentize_id_a = ?2 AND immanentize_id_b != ?1
                       ON CONFLICT(immanentize_id_a, immanentize_id_b) DO UPDATE SET
                           cooccurrence = cooccurrence + excluded.cooccurrence,
                           last_seen = MAX(last_seen, excluded.last_seen)"#,
                    rusqlite::params![can_id, id],
                ) {
                    warn!("Failed to transfer neighbor edges (a) for keyword {}: {}", id, e);
                }
                // Case 2: source is id_b
                if let Err(e) = conn.execute(
                    r#"INSERT INTO immanentize_neighbors (immanentize_id_a, immanentize_id_b, cooccurrence, embedding_similarity, combined_weight, first_seen, last_seen)
                       SELECT
                           CASE WHEN immanentize_id_a < ?1 THEN immanentize_id_a ELSE ?1 END,
                           CASE WHEN immanentize_id_a < ?1 THEN ?1 ELSE immanentize_id_a END,
                           cooccurrence, embedding_similarity, combined_weight, first_seen, last_seen
                       FROM immanentize_neighbors
                       WHERE immanentize_id_b = ?2 AND immanentize_id_a != ?1
                       ON CONFLICT(immanentize_id_a, immanentize_id_b) DO UPDATE SET
                           cooccurrence = cooccurrence + excluded.cooccurrence,
                           last_seen = MAX(last_seen, excluded.last_seen)"#,
                    rusqlite::params![can_id, id],
                ) {
                    warn!("Failed to transfer neighbor edges (b) for keyword {}: {}", id, e);
                }

                // Merge daily trend data before deleting
                if let Err(e) = conn.execute(
                    r#"INSERT INTO immanentize_daily (immanentize_id, date, count)
                       SELECT ?1, date, count FROM immanentize_daily WHERE immanentize_id = ?2
                       ON CONFLICT(immanentize_id, date) DO UPDATE SET count = count + excluded.count"#,
                    rusqlite::params![can_id, id],
                ) {
                    warn!("Failed to merge daily stats for keyword {}: {}", id, e);
                }

                // Delete neighbor relationships BEFORE deleting the keyword to prevent orphaned relationships
                if let Err(e) = conn.execute(
                    "DELETE FROM immanentize_neighbors WHERE immanentize_id_a = ? OR immanentize_id_b = ?",
                    rusqlite::params![id, id],
                ) {
                    warn!("Failed to delete neighbor relationships for keyword {}: {}", id, e);
                }

                if let Err(e) = conn.execute("DELETE FROM immanentize WHERE id = ?", [id]) {
                    warn!("Failed to delete merged keyword {}: {}", id, e);
                }
                // Also remove from vec_immanentize (sqlite-vec)
                if let Err(e) =
                    conn.execute("DELETE FROM vec_immanentize WHERE immanentize_id = ?", [id])
                {
                    trace!("Failed to delete keyword {} from vec table: {}", id, e);
                }

                merged_count += 1;
                affected_articles += moved;
            }
        }
    }

    conn.execute("COMMIT", []).map_err(|e| e.to_string())?;

    Ok(MergeResult {
        merged_count,
        affected_articles,
    })
}

#[derive(Debug, Serialize)]
pub struct CleanupResult {
    pub removed_garbage: i64,
    pub removed_relations: i64,
}

#[tauri::command]
pub fn cleanup_garbage_keywords(state: State<AppState>) -> Result<CleanupResult, String> {
    use crate::db::transaction::with_transaction_result;

    // Load keywords with short lock and identify garbage outside of transaction
    let garbage_ids: Vec<i64> = {
        let db = state.db_conn()?;
        let conn = db.conn();

        let mut stmt = conn
            .prepare("SELECT id, name FROM immanentize")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?;
        let keywords: Vec<(i64, String)> = rows.filter_map(|r| r.ok()).collect();

        keywords
            .into_iter()
            .filter(|(_, name)| normalize_keyword(name).is_none())
            .map(|(id, _)| id)
            .collect()
    };

    let removed_garbage = garbage_ids.len() as i64;

    if garbage_ids.is_empty() {
        return Ok(CleanupResult {
            removed_garbage: 0,
            removed_relations: 0,
        });
    }

    // Perform all deletions in a single transaction
    let db = state.db_conn()?;

    with_transaction_result(db.conn(), |conn| {
        for id in &garbage_ids {
            // Delete garbage keyword and all its relations
            conn.execute(
                "DELETE FROM fnord_immanentize WHERE immanentize_id = ?",
                [id],
            )?;
            conn.execute(
                "DELETE FROM immanentize_neighbors WHERE immanentize_id_a = ? OR immanentize_id_b = ?",
                rusqlite::params![id, id],
            )?;
            conn.execute(
                "DELETE FROM immanentize_sephiroth WHERE immanentize_id = ?",
                [id],
            )?;
            conn.execute(
                "DELETE FROM immanentize_daily WHERE immanentize_id = ?",
                [id],
            )?;
            conn.execute("DELETE FROM immanentize WHERE id = ?", [id])?;
            // Also remove from vec_immanentize (sqlite-vec) - ignore errors
            let _ = conn.execute("DELETE FROM vec_immanentize WHERE immanentize_id = ?", [id]);
        }

        // Approximate removed relations based on garbage count
        let removed_relations = removed_garbage * 5; // Estimated avg relations per keyword

        Ok(CleanupResult {
            removed_garbage,
            removed_relations,
        })
    })
}

// ============================================================
// QUALITY SCORE SYSTEM
// ============================================================

#[derive(Debug, Serialize, Clone)]
pub struct QualityScoreProgress {
    pub current: i64,
    pub total: i64,
    pub keyword_name: String,
    pub score: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct QualityScoreResult {
    pub updated_count: i64,
    pub avg_score: f64,
    pub low_quality_count: i64,
    pub high_quality_count: i64,
}

fn calculate_single_keyword_quality(
    conn: &rusqlite::Connection,
    keyword_id: i64,
) -> Result<f64, rusqlite::Error> {
    let article_count: i64 = conn
        .query_row(
            "SELECT COALESCE(article_count, 0) FROM immanentize WHERE id = ?",
            [keyword_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let neighbor_count: i64 = conn
        .query_row(
            r#"SELECT COUNT(*) FROM immanentize_neighbors 
               WHERE immanentize_id_a = ?1 OR immanentize_id_b = ?1"#,
            [keyword_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let category_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM immanentize_sephiroth WHERE immanentize_id = ?",
            [keyword_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let days_since_last_use: i64 = conn
        .query_row(
            r#"SELECT COALESCE(CAST(julianday('now') - julianday(last_used) AS INTEGER), 365) 
               FROM immanentize WHERE id = ?"#,
            [keyword_id],
            |row| row.get(0),
        )
        .unwrap_or(365);

    let article_score = (article_count as f64).ln_1p() / 5.0;
    let neighbor_score = (neighbor_count as f64).ln_1p() / 4.0;
    let category_score = if category_count > 0 && category_count <= 3 {
        0.2
    } else if category_count > 3 {
        0.1
    } else {
        0.0
    };
    let recency_score = if days_since_last_use <= 7 {
        0.2
    } else if days_since_last_use <= 30 {
        0.1
    } else {
        0.0
    };

    let raw_score = article_score + neighbor_score + category_score + recency_score;
    let normalized = (raw_score / 1.5).clamp(0.0, 1.0);

    Ok(normalized)
}

#[tauri::command]
pub async fn calculate_keyword_quality_scores(
    window: tauri::Window,
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<QualityScoreResult, String> {
    // Load keyword IDs and names with short lock
    let keywords: Vec<(i64, String)> = {
        let db = state.db_conn()?;
        let conn = db.conn();

        // Build query - None means no limit (all keywords)
        let base_query = r#"SELECT id, name FROM immanentize
                   WHERE quality_calculated_at IS NULL
                   OR quality_calculated_at < datetime('now', '-1 day')
                   ORDER BY article_count DESC"#;

        if let Some(limit_val) = limit {
            conn.prepare(&format!("{} LIMIT ?", base_query))
                .map_err(|e| e.to_string())?
                .query_map([limit_val], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect()
        } else {
            conn.prepare(base_query)
                .map_err(|e| e.to_string())?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect()
        }
    }; // Lock released here

    let total = keywords.len() as i64;
    let mut updated_count = 0i64;
    let mut total_score = 0.0;
    let mut low_quality_count = 0i64;
    let mut high_quality_count = 0i64;

    // Emit initial progress
    let _ = window.emit(
        "quality-score-progress",
        QualityScoreProgress {
            current: 0,
            total,
            keyword_name: "Starting...".to_string(),
            score: None,
        },
    );

    // Process each keyword with separate lock acquisition
    for (idx, (id, name)) in keywords.into_iter().enumerate() {
        let result = {
            let db = state.db_conn()?;
            let conn = db.conn();

            match calculate_single_keyword_quality(conn, id) {
                Ok(score) => {
                    match conn.execute(
                        r#"UPDATE immanentize
                           SET quality_score = ?1, quality_calculated_at = datetime('now')
                           WHERE id = ?2"#,
                        params![score, id],
                    ) {
                        Ok(_) => Some(score),
                        Err(e) => {
                            trace!("Failed to update quality score for keyword {}: {}", id, e);
                            None
                        }
                    }
                }
                Err(_) => None,
            }
        }; // Lock released here

        // Update statistics outside of lock
        if let Some(score) = result {
            updated_count += 1;
            total_score += score;

            if score < 0.3 {
                low_quality_count += 1;
            } else if score >= 0.7 {
                high_quality_count += 1;
            }
        }

        // Emit progress every 50 keywords or on last item
        if idx % 50 == 0 || idx == (total as usize - 1) {
            let _ = window.emit(
                "quality-score-progress",
                QualityScoreProgress {
                    current: (idx + 1) as i64,
                    total,
                    keyword_name: name,
                    score: result,
                },
            );
        }

        // Yield to allow other tasks to run
        tokio::task::yield_now().await;
    }

    let avg_score = if updated_count > 0 {
        total_score / updated_count as f64
    } else {
        0.0
    };

    Ok(QualityScoreResult {
        updated_count,
        avg_score,
        low_quality_count,
        high_quality_count,
    })
}

#[derive(Debug, Serialize)]
pub struct LowQualityKeyword {
    pub id: i64,
    pub name: String,
    pub quality_score: f64,
    pub article_count: i64,
    pub days_old: i64,
}

#[tauri::command]
pub fn get_low_quality_keywords(
    state: State<AppState>,
    threshold: Option<f64>,
    limit: Option<i64>,
) -> Result<Vec<LowQualityKeyword>, String> {
    let db = state.db_conn()?;
    let conn = db.conn();
    let threshold = threshold.unwrap_or(0.3);
    let limit = limit.unwrap_or(100);

    let keywords: Vec<LowQualityKeyword> = conn
        .prepare(
            r#"SELECT id, name, COALESCE(quality_score, 0.0), 
                      COALESCE(article_count, 0),
                      COALESCE(CAST(julianday('now') - julianday(first_seen) AS INTEGER), 0)
               FROM immanentize 
               WHERE quality_score IS NOT NULL AND quality_score < ?1
               ORDER BY quality_score ASC
               LIMIT ?2"#,
        )
        .map_err(|e| e.to_string())?
        .query_map(params![threshold, limit], |row| {
            Ok(LowQualityKeyword {
                id: row.get(0)?,
                name: row.get(1)?,
                quality_score: row.get(2)?,
                article_count: row.get(3)?,
                days_old: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(keywords)
}

#[derive(Debug, Serialize)]
pub struct AutoPruneResult {
    pub pruned_count: i64,
    pub pruned_keywords: Vec<String>,
}

#[tauri::command]
pub fn auto_prune_low_quality(
    state: State<AppState>,
    quality_threshold: Option<f64>,
    min_age_days: Option<i64>,
    dry_run: Option<bool>,
) -> Result<AutoPruneResult, String> {
    let threshold = quality_threshold.unwrap_or(0.2);
    let min_age = min_age_days.unwrap_or(7);
    let dry_run = dry_run.unwrap_or(true);

    // Load candidates with short lock
    let candidates: Vec<(i64, String)> = {
        let db = state.db_conn()?;
        let conn = db.conn();

        let mut stmt = conn
            .prepare(
                r#"SELECT id, name FROM immanentize
               WHERE quality_score IS NOT NULL
               AND quality_score < ?1
               AND first_seen < datetime('now', '-' || ?2 || ' days')
               AND article_count <= 1"#,
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![threshold, min_age], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let pruned_keywords: Vec<String> = candidates.iter().map(|(_, name)| name.clone()).collect();
    let pruned_count = candidates.len() as i64;

    if !dry_run && !candidates.is_empty() {
        // Perform all deletions in a single transaction with proper error handling
        let db = state.db_conn()?;
        let conn = db.conn();

        conn.execute("BEGIN TRANSACTION", [])
            .map_err(|e| e.to_string())?;

        let delete_result: Result<(), String> = (|| {
            for (id, name) in &candidates {
                // Delete low-quality keyword and all its relations
                conn.execute(
                    "DELETE FROM fnord_immanentize WHERE immanentize_id = ?",
                    [id],
                )
                .map_err(|e| format!("Failed to delete article refs for {}: {}", name, e))?;

                conn.execute(
                    "DELETE FROM immanentize_neighbors WHERE immanentize_id_a = ? OR immanentize_id_b = ?",
                    params![id, id],
                ).map_err(|e| format!("Failed to delete neighbors for {}: {}", name, e))?;

                conn.execute(
                    "DELETE FROM immanentize_sephiroth WHERE immanentize_id = ?",
                    [id],
                )
                .map_err(|e| format!("Failed to delete category refs for {}: {}", name, e))?;

                conn.execute(
                    "DELETE FROM immanentize_daily WHERE immanentize_id = ?",
                    [id],
                )
                .map_err(|e| format!("Failed to delete daily stats for {}: {}", name, e))?;

                conn.execute("DELETE FROM immanentize WHERE id = ?", [id])
                    .map_err(|e| format!("Failed to delete keyword {}: {}", name, e))?;

                // Also remove from vec_immanentize (sqlite-vec)
                conn.execute("DELETE FROM vec_immanentize WHERE immanentize_id = ?", [id])
                    .map_err(|e| format!("Failed to delete vec for {}: {}", name, e))?;
            }
            Ok(())
        })();

        match delete_result {
            Ok(()) => {
                conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                return Err(e);
            }
        }
    }

    Ok(AutoPruneResult {
        pruned_count,
        pruned_keywords,
    })
}

// ============================================================
// EMBEDDING-BASED SYNONYM DETECTION
// ============================================================

#[derive(Debug, Serialize)]
pub struct SimilarKeyword {
    pub id: i64,
    pub name: String,
    pub similarity: f64,
    pub article_count: i64,
}

#[tauri::command]
pub fn find_similar_keywords(
    state: State<AppState>,
    keyword_id: i64,
    threshold: Option<f64>,
    limit: Option<i64>,
) -> Result<Vec<SimilarKeyword>, String> {
    let db = state.db_conn()?;
    let conn = db.conn();
    let threshold = threshold.unwrap_or(0.7);
    let limit = limit.unwrap_or(20);

    // Get target keyword name and embedding
    let (target_name, target_embedding): (String, Option<Vec<u8>>) = conn
        .query_row(
            "SELECT name, embedding FROM immanentize WHERE id = ?",
            [keyword_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    let target_embedding = match target_embedding {
        Some(blob) => blob_to_embedding(&blob),
        None => return Ok(vec![]),
    };

    let all_keywords: Vec<(i64, String, Vec<u8>, i64)> = conn
        .prepare(
            r#"SELECT id, name, embedding, COALESCE(article_count, 0)
               FROM immanentize
               WHERE embedding IS NOT NULL AND id != ?"#,
        )
        .map_err(|e| e.to_string())?
        .query_map([keyword_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let target_name_lower = target_name.to_lowercase();

    let mut similar: Vec<SimilarKeyword> = all_keywords
        .into_iter()
        .filter_map(|(id, name, blob, article_count)| {
            let embedding = blob_to_embedding(&blob);
            let embedding_similarity = cosine_similarity(&target_embedding, &embedding);

            // Check if this is a name variant (e.g., "Trump" <-> "Donald Trump")
            let name_variant_score =
                calculate_exact_token_match_score(&target_name_lower, &name.to_lowercase());

            // Boost similarity for name variants - they should always appear
            let effective_similarity = if name_variant_score > 0.7 {
                // Name variant: use high similarity to ensure it appears at top
                0.95_f64.max(embedding_similarity)
            } else {
                embedding_similarity
            };

            if effective_similarity >= threshold || name_variant_score > 0.7 {
                Some(SimilarKeyword {
                    id,
                    name,
                    similarity: effective_similarity,
                    article_count,
                })
            } else {
                None
            }
        })
        .collect();

    similar.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    similar.truncate(limit as usize);

    Ok(similar)
}

#[derive(Debug, Serialize)]
pub struct SynonymCandidate {
    pub keyword_a_id: i64,
    pub keyword_a_name: String,
    pub keyword_b_id: i64,
    pub keyword_b_name: String,
    pub similarity: f64,
}

/// Find synonym candidates using sqlite-vec for O(log n) approximate nearest neighbor search
#[tauri::command]
pub fn find_synonym_candidates(
    state: State<AppState>,
    threshold: Option<f64>,
    limit: Option<i64>,
) -> Result<Vec<SynonymCandidate>, String> {
    let db = state.db_conn()?;
    let conn = db.conn();
    let threshold = threshold.unwrap_or(0.85);
    let limit = limit.unwrap_or(50);

    // Cosine distance = 1 - cosine similarity
    // For similarity >= 0.85, we need distance <= 0.15
    let max_distance = 1.0 - threshold;

    // Load dismissed pairs
    let dismissed: std::collections::HashSet<(i64, i64)> = conn
        .prepare("SELECT keyword_a_id, keyword_b_id FROM dismissed_synonyms")
        .map_err(|e| e.to_string())?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Get keywords with embeddings (sorted by importance for prioritized search)
    let keywords: Vec<(i64, String, Vec<u8>)> = conn
        .prepare(
            r#"SELECT id, name, embedding FROM immanentize
               WHERE embedding IS NOT NULL
               ORDER BY article_count DESC
               LIMIT 500"#,
        )
        .map_err(|e| e.to_string())?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut candidates: Vec<SynonymCandidate> = Vec::new();
    let mut seen_pairs: std::collections::HashSet<(i64, i64)> = std::collections::HashSet::new();

    // Use sqlite-vec for O(log n) KNN search per keyword
    // Each keyword searches for its nearest neighbors in the vector index
    for (keyword_id, keyword_name, embedding_blob) in &keywords {
        // Query vec_immanentize for nearest neighbors using KNN search
        // The WHERE clause with MATCH performs approximate nearest neighbor search
        let neighbors: Vec<(i64, f64)> = conn
            .prepare(
                r#"SELECT immanentize_id, distance
                   FROM vec_immanentize
                   WHERE embedding MATCH ?1 AND k = 10
                   ORDER BY distance"#,
            )
            .map_err(|e| e.to_string())?
            .query_map([embedding_blob], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        for (neighbor_id, distance) in neighbors {
            // Skip self-matches
            if neighbor_id == *keyword_id {
                continue;
            }

            // Check distance threshold
            if distance > max_distance {
                continue;
            }

            // Normalize pair ordering for deduplication
            let (min_id, max_id) = if *keyword_id < neighbor_id {
                (*keyword_id, neighbor_id)
            } else {
                (neighbor_id, *keyword_id)
            };

            // Skip if already seen or dismissed
            if seen_pairs.contains(&(min_id, max_id)) || dismissed.contains(&(min_id, max_id)) {
                continue;
            }
            seen_pairs.insert((min_id, max_id));

            // Get neighbor name (skip orphaned vec_immanentize entries)
            let neighbor_name: String = match conn.query_row(
                "SELECT name FROM immanentize WHERE id = ?",
                [neighbor_id],
                |row| row.get(0),
            ) {
                Ok(name) => name,
                Err(_) => continue, // Orphaned entry in vec_immanentize, skip
            };

            // Convert distance back to similarity
            let similarity = 1.0 - distance;

            candidates.push(SynonymCandidate {
                keyword_a_id: *keyword_id,
                keyword_a_name: keyword_name.clone(),
                keyword_b_id: neighbor_id,
                keyword_b_name: neighbor_name,
                similarity,
            });

            if candidates.len() >= limit as usize * 2 {
                break;
            }
        }

        if candidates.len() >= limit as usize * 2 {
            break;
        }
    }

    // Sort by similarity descending and limit results
    candidates.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.truncate(limit as usize);

    Ok(candidates)
}

// String similarity functions (calculate_string_similarity, calculate_abbreviation_score,
// calculate_exact_token_match_score) are provided by the crate::similarity::string module.

/// True synonym candidate with both string similarity and embedding similarity
#[derive(Debug, Serialize)]
pub struct TrueSynonymCandidate {
    pub keyword_a_id: i64,
    pub keyword_a_name: String,
    pub keyword_b_id: i64,
    pub keyword_b_name: String,
    pub string_similarity: f64,
    pub embedding_similarity: f64,
    pub combined_score: f64,
    pub is_abbreviation: bool,
    /// True if one keyword is a single token that appears in the other multi-token keyword
    /// Examples: "Trump" <-> "Donald Trump", "Biden" <-> "Joe Biden"
    pub is_name_variant: bool,
}

/// Result of LLM synonym verification
#[derive(Debug, Serialize, Deserialize)]
pub struct SynonymVerificationResult {
    pub keyword_a: String,
    pub keyword_b: String,
    pub is_synonym: bool,
    pub confidence: f64,
    pub explanation: Option<String>,
}

/// Find true synonym candidates using both string similarity and embedding similarity.
/// This hybrid approach helps distinguish between:
/// - True synonyms (lexical variants): "EU" / "European Union", "AI" / "Artificial Intelligence"
/// - Semantically related but not synonyms: "Climate" / "Weather", "Politics" / "Government"
#[tauri::command]
pub fn find_true_synonyms(
    state: State<AppState>,
    string_threshold: Option<f64>,
    embedding_threshold: Option<f64>,
    limit: Option<i64>,
) -> Result<Vec<TrueSynonymCandidate>, String> {
    let db = state.db_conn()?;
    let conn = db.conn();
    let string_threshold = string_threshold.unwrap_or(0.6);
    let embedding_threshold = embedding_threshold.unwrap_or(0.7);
    let limit = limit.unwrap_or(50);

    // Load dismissed pairs
    let dismissed: std::collections::HashSet<(i64, i64)> = conn
        .prepare("SELECT keyword_a_id, keyword_b_id FROM dismissed_synonyms")
        .map_err(|e| e.to_string())?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Get keywords with embeddings (sorted by importance)
    let keywords: Vec<(i64, String, Vec<u8>)> = conn
        .prepare(
            r#"SELECT id, name, embedding FROM immanentize
               WHERE embedding IS NOT NULL
               ORDER BY article_count DESC
               LIMIT 300"#,
        )
        .map_err(|e| e.to_string())?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut candidates: Vec<TrueSynonymCandidate> = Vec::new();
    let mut seen_pairs: std::collections::HashSet<(i64, i64)> = std::collections::HashSet::new();

    // Compare keywords pairwise for string similarity first (fast)
    // Then check embedding similarity for candidates (slower)
    for i in 0..keywords.len() {
        for j in (i + 1)..keywords.len() {
            let (id_a, name_a, emb_a) = &keywords[i];
            let (id_b, name_b, emb_b) = &keywords[j];

            // Normalize pair ordering
            let (min_id, max_id) = if *id_a < *id_b {
                (*id_a, *id_b)
            } else {
                (*id_b, *id_a)
            };

            // Skip if already seen or dismissed
            if seen_pairs.contains(&(min_id, max_id)) || dismissed.contains(&(min_id, max_id)) {
                continue;
            }

            // Calculate string similarity
            let string_sim = calculate_string_similarity(name_a, name_b);

            // Only check embedding similarity if string similarity is promising
            if string_sim < string_threshold * 0.5 {
                continue;
            }

            // Calculate embedding similarity
            let emb_a_vec = blob_to_embedding(emb_a);
            let emb_b_vec = blob_to_embedding(emb_b);
            let embedding_sim = if !emb_a_vec.is_empty() && !emb_b_vec.is_empty() {
                cosine_similarity(&emb_a_vec, &emb_b_vec)
            } else {
                0.0
            };

            // Check if this is likely an abbreviation
            let is_abbrev =
                calculate_abbreviation_score(&name_a.to_lowercase(), &name_b.to_lowercase()) > 0.7;

            // Check if this is a name variant (single token matches multi-token)
            let is_name_variant =
                calculate_exact_token_match_score(&name_a.to_lowercase(), &name_b.to_lowercase())
                    > 0.7;

            // Combined score: weight string similarity more for true synonyms
            let combined = if is_abbrev {
                // Abbreviations: trust string similarity more
                string_sim * 0.7 + embedding_sim * 0.3
            } else if is_name_variant {
                // Name variants like "Trump" / "Donald Trump" - trust string similarity
                string_sim * 0.7 + embedding_sim * 0.3
            } else if string_sim > 0.8 {
                // High string similarity: likely true synonym
                string_sim * 0.6 + embedding_sim * 0.4
            } else {
                // Mixed: balance both signals
                string_sim * 0.4 + embedding_sim * 0.6
            };

            // Filter by thresholds
            if string_sim >= string_threshold
                || (embedding_sim >= embedding_threshold && string_sim >= string_threshold * 0.5)
            {
                seen_pairs.insert((min_id, max_id));
                candidates.push(TrueSynonymCandidate {
                    keyword_a_id: *id_a,
                    keyword_a_name: name_a.clone(),
                    keyword_b_id: *id_b,
                    keyword_b_name: name_b.clone(),
                    string_similarity: string_sim,
                    embedding_similarity: embedding_sim,
                    combined_score: combined,
                    is_abbreviation: is_abbrev,
                    is_name_variant,
                });
            }
        }
    }

    // Sort by combined score (prioritize high-confidence matches)
    candidates.sort_by(|a, b| {
        // First by high-confidence matches (abbreviations and name variants)
        let a_high_confidence = a.is_abbreviation || a.is_name_variant;
        let b_high_confidence = b.is_abbreviation || b.is_name_variant;

        match (a_high_confidence, b_high_confidence) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                // Then by combined score
                b.combined_score
                    .partial_cmp(&a.combined_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
        }
    });

    candidates.truncate(limit as usize);

    Ok(candidates)
}

/// Verify if two keywords are true synonyms using LLM.
/// Uses ministral-3 (or configured model) to determine if keywords refer to the same entity.
///
/// Examples where LLM should return true:
/// - "EU" and "Europäische Union"
/// - "AI" and "Artificial Intelligence"
/// - "USA" and "United States"
///
/// Examples where LLM should return false:
/// - "Climate" and "Weather" (related but not synonymous)
/// - "Politics" and "Government" (related but distinct concepts)
#[tauri::command]
pub async fn verify_synonym_pair(
    state: State<'_, AppState>,
    keyword_a: String,
    keyword_b: String,
) -> Result<SynonymVerificationResult, String> {
    use crate::ai_provider::TaskType;
    use crate::commands::ai::helpers::create_text_provider;

    // Get the configured provider and model from settings
    let (provider, model) = {
        let db = state.db_conn()?;
        create_text_provider(&db, Some(&state.proxy_manager), TaskType::Fast)
    };

    // Prompt designed for YES/NO response with optional explanation
    // No /no_think prefix - provider handles that automatically for Ollama
    let prompt = format!(
        r#"Are "{}" and "{}" synonyms, alternate names, or abbreviations referring to the SAME entity or concept?

Rules:
- YES if they refer to the exact same thing (e.g., "EU" = "European Union", "AI" = "Artificial Intelligence")
- YES if one is an abbreviation/acronym of the other
- YES if they are different spellings/translations of the same thing
- NO if they are merely related but distinct concepts (e.g., "Climate" vs "Weather")
- NO if they are hypernym/hyponym relationships (e.g., "Animal" vs "Dog")

Answer with ONLY a JSON object:
{{"is_synonym": true/false, "confidence": 0.0-1.0, "explanation": "brief reason"}}

Keywords: "{}" and "{}""#,
        keyword_a, keyword_b, keyword_a, keyword_b
    );

    // Generate response via provider (JSON schema mode)
    let schema = crate::ollama::synonym_schema();
    let result = provider
        .generate_text(&model, &prompt, Some(schema))
        .await
        .map_err(|e| format!("LLM error: {}", e))?;
    let response = result.text;

    // Parse JSON response
    #[derive(Deserialize)]
    struct LlmResponse {
        is_synonym: bool,
        #[serde(default)]
        confidence: f64,
        #[serde(default)]
        explanation: Option<String>,
    }

    // Try to parse JSON from response
    let parsed: LlmResponse = serde_json::from_str(&response)
        .map_err(|e| {
            // Fallback: check for YES/NO in response
            let response_lower = response.to_lowercase();
            if response_lower.contains("yes") && !response_lower.contains("no") {
                return format!("Parsed as YES (confidence: low). Original error: {}", e);
            }
            if response_lower.contains("no") {
                return format!("Parsed as NO (confidence: low). Original error: {}", e);
            }
            format!(
                "Failed to parse LLM response: {}. Response: {}",
                e, response
            )
        })
        .unwrap_or_else(|_err| {
            // Fallback parsing from YES/NO
            let response_lower = response.to_lowercase();
            LlmResponse {
                is_synonym: response_lower.contains("yes") && !response_lower.contains("no"),
                confidence: 0.5, // Low confidence for fallback parsing
                explanation: Some(format!(
                    "Parsed from raw response: {}",
                    response.chars().take(100).collect::<String>()
                )),
            }
        });

    Ok(SynonymVerificationResult {
        keyword_a,
        keyword_b,
        is_synonym: parsed.is_synonym,
        confidence: if parsed.confidence > 0.0 {
            parsed.confidence
        } else {
            0.8
        },
        explanation: parsed.explanation,
    })
}

/// Batch verify multiple synonym pairs using LLM.
/// More efficient than calling verify_synonym_pair for each pair individually.
#[tauri::command]
pub async fn verify_synonym_pairs_batch(
    state: State<'_, AppState>,
    pairs: Vec<(String, String)>,
) -> Result<Vec<SynonymVerificationResult>, String> {
    let mut results = Vec::with_capacity(pairs.len());

    for (keyword_a, keyword_b) in pairs {
        match verify_synonym_pair(state.clone(), keyword_a.clone(), keyword_b.clone()).await {
            Ok(result) => results.push(result),
            Err(e) => {
                // On error, add a result with is_synonym=false
                results.push(SynonymVerificationResult {
                    keyword_a,
                    keyword_b,
                    is_synonym: false,
                    confidence: 0.0,
                    explanation: Some(format!("Verification failed: {}", e)),
                });
            }
        }
    }

    Ok(results)
}

#[derive(Debug, Serialize)]
pub struct MergeSynonymsResult {
    pub merged_pairs: i64,
    pub affected_articles: i64,
}

#[tauri::command]
pub fn merge_keyword_pair(
    state: State<AppState>,
    keep_id: i64,
    remove_id: i64,
) -> Result<MergeSynonymsResult, String> {
    use crate::db::transaction::with_transaction_result;

    log::info!(
        "merge_keyword_pair called: keep_id={}, remove_id={}",
        keep_id,
        remove_id
    );

    let db = state.db_conn()?;

    // Use transaction wrapper for atomic multi-write merge operation
    let result = with_transaction_result(db.conn(), |conn| {
        let affected: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM fnord_immanentize WHERE immanentize_id = ?",
                [remove_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        conn.execute(
            "UPDATE OR IGNORE fnord_immanentize SET immanentize_id = ?1 WHERE immanentize_id = ?2",
            params![keep_id, remove_id],
        )?;

        // Clean up merged keyword's remaining references
        conn.execute(
            "DELETE FROM fnord_immanentize WHERE immanentize_id = ?",
            [remove_id],
        )?;

        conn.execute(
            r#"UPDATE immanentize SET
               article_count = (SELECT COUNT(DISTINCT fnord_id) FROM fnord_immanentize WHERE immanentize_id = ?1)
               WHERE id = ?1"#,
            [keep_id],
        )?;

        // Transfer category associations before deleting
        conn.execute(
            r#"INSERT INTO immanentize_sephiroth (immanentize_id, sephiroth_id, weight, article_count, first_seen, updated_at)
               SELECT ?1, sephiroth_id, weight, article_count, first_seen, updated_at
               FROM immanentize_sephiroth WHERE immanentize_id = ?2
               ON CONFLICT(immanentize_id, sephiroth_id) DO UPDATE SET
                   article_count = article_count + excluded.article_count,
                   updated_at = CURRENT_TIMESTAMP"#,
            params![keep_id, remove_id],
        )?;

        // Transfer neighbor edges (re-wire to keep_id, respecting CHECK constraint)
        // Case 1: remove_id is id_a
        conn.execute(
            r#"INSERT INTO immanentize_neighbors (immanentize_id_a, immanentize_id_b, cooccurrence, embedding_similarity, combined_weight, first_seen, last_seen)
               SELECT
                   CASE WHEN ?1 < immanentize_id_b THEN ?1 ELSE immanentize_id_b END,
                   CASE WHEN ?1 < immanentize_id_b THEN immanentize_id_b ELSE ?1 END,
                   cooccurrence, embedding_similarity, combined_weight, first_seen, last_seen
               FROM immanentize_neighbors
               WHERE immanentize_id_a = ?2 AND immanentize_id_b != ?1
               ON CONFLICT(immanentize_id_a, immanentize_id_b) DO UPDATE SET
                   cooccurrence = cooccurrence + excluded.cooccurrence,
                   last_seen = MAX(last_seen, excluded.last_seen)"#,
            params![keep_id, remove_id],
        )?;
        // Case 2: remove_id is id_b
        conn.execute(
            r#"INSERT INTO immanentize_neighbors (immanentize_id_a, immanentize_id_b, cooccurrence, embedding_similarity, combined_weight, first_seen, last_seen)
               SELECT
                   CASE WHEN immanentize_id_a < ?1 THEN immanentize_id_a ELSE ?1 END,
                   CASE WHEN immanentize_id_a < ?1 THEN ?1 ELSE immanentize_id_a END,
                   cooccurrence, embedding_similarity, combined_weight, first_seen, last_seen
               FROM immanentize_neighbors
               WHERE immanentize_id_b = ?2 AND immanentize_id_a != ?1
               ON CONFLICT(immanentize_id_a, immanentize_id_b) DO UPDATE SET
                   cooccurrence = cooccurrence + excluded.cooccurrence,
                   last_seen = MAX(last_seen, excluded.last_seen)"#,
            params![keep_id, remove_id],
        )?;

        conn.execute(
            "DELETE FROM immanentize_neighbors WHERE immanentize_id_a = ? OR immanentize_id_b = ?",
            params![remove_id, remove_id],
        )?;

        conn.execute(
            "DELETE FROM immanentize_sephiroth WHERE immanentize_id = ?",
            [remove_id],
        )?;

        // Merge daily trend data into keep_id before deleting remove_id's records
        conn.execute(
            r#"INSERT INTO immanentize_daily (immanentize_id, date, count)
               SELECT ?1, date, count FROM immanentize_daily WHERE immanentize_id = ?2
               ON CONFLICT(immanentize_id, date) DO UPDATE SET count = count + excluded.count"#,
            params![keep_id, remove_id],
        )?;

        conn.execute(
            "DELETE FROM immanentize_daily WHERE immanentize_id = ?",
            [remove_id],
        )?;

        conn.execute("DELETE FROM immanentize WHERE id = ?", [remove_id])?;

        // Also remove from vec_immanentize (sqlite-vec)
        // Ignore error if table doesn't exist
        let _ = conn.execute(
            "DELETE FROM vec_immanentize WHERE immanentize_id = ?",
            [remove_id],
        );

        Ok(MergeSynonymsResult {
            merged_pairs: 1,
            affected_articles: affected,
        })
    });

    match &result {
        Ok(r) => log::info!(
            "merge_keyword_pair success: {} merged, {} articles affected",
            r.merged_pairs,
            r.affected_articles
        ),
        Err(e) => log::error!("merge_keyword_pair failed: {}", e),
    }

    result
}

#[tauri::command]
pub fn dismiss_synonym_pair(
    state: State<AppState>,
    keyword_a_id: i64,
    keyword_b_id: i64,
) -> Result<(), String> {
    let db = state.db_conn()?;
    let (min_id, max_id) = if keyword_a_id < keyword_b_id {
        (keyword_a_id, keyword_b_id)
    } else {
        (keyword_b_id, keyword_a_id)
    };

    db.conn()
        .execute(
            "INSERT OR IGNORE INTO dismissed_synonyms (keyword_a_id, keyword_b_id) VALUES (?1, ?2)",
            params![min_id, max_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(serde::Serialize)]
pub struct CooccurringKeyword {
    pub id: i64,
    pub name: String,
    pub cooccurrence_count: i64,
}

/// Get keywords that co-occur with the given keyword in articles within a time period
#[tauri::command]
pub fn get_cooccurring_keywords(
    state: State<AppState>,
    keyword_id: i64,
    days: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<CooccurringKeyword>, String> {
    let db = state.db_conn()?;
    let days = days.unwrap_or(30);
    let limit = limit.unwrap_or(20);

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                i.id,
                i.name,
                COUNT(DISTINCT fi2.fnord_id) as cooccurrence_count
            FROM fnord_immanentize fi1
            JOIN fnords f ON f.id = fi1.fnord_id
            JOIN fnord_immanentize fi2 ON fi2.fnord_id = fi1.fnord_id AND fi2.immanentize_id != ?1
            JOIN immanentize i ON i.id = fi2.immanentize_id
            WHERE fi1.immanentize_id = ?1
            AND f.published_at > datetime('now', '-' || ?2 || ' days')
            GROUP BY i.id
            ORDER BY cooccurrence_count DESC
            LIMIT ?3
            "#,
        )
        .map_err(|e| e.to_string())?;

    let keywords = stmt
        .query_map(rusqlite::params![keyword_id, days, limit], |row| {
            Ok(CooccurringKeyword {
                id: row.get(0)?,
                name: row.get(1)?,
                cooccurrence_count: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(keywords)
}

// ============================================================
// KEYWORD MANAGEMENT (Manual Creation, Deletion, Linking)
// ============================================================

/// Result of creating a keyword
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateKeywordResult {
    pub id: i64,
    pub name: String,
    pub created: bool, // true if newly created, false if already existed
}

/// Create a new keyword manually
/// Returns the keyword ID (either new or existing)
#[tauri::command]
pub fn create_keyword(state: State<AppState>, name: String) -> Result<CreateKeywordResult, String> {
    use crate::normalize_keyword;

    let db = state.db_conn()?;
    let conn = db.conn();

    // Normalize the keyword name
    let normalized = normalize_keyword(&name).ok_or_else(|| {
        format!(
            "Ungültiges Keyword: '{}'. Keywords müssen 3-50 Zeichen haben und sinnvolle Wörter enthalten.",
            name
        )
    })?;

    // Check if keyword already exists (case-insensitive)
    let existing: Option<(i64, String)> = conn
        .query_row(
            "SELECT id, name FROM immanentize WHERE LOWER(name) = LOWER(?)",
            [&normalized],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    if let Some((id, existing_name)) = existing {
        return Ok(CreateKeywordResult {
            id,
            name: existing_name,
            created: false,
        });
    }

    // Insert new keyword
    conn.execute(
        r#"INSERT INTO immanentize (name, count, article_count, is_canonical, first_seen)
           VALUES (?1, 0, 0, TRUE, CURRENT_TIMESTAMP)"#,
        [&normalized],
    )
    .map_err(|e| format!("Fehler beim Erstellen des Keywords: {}", e))?;

    let id = conn.last_insert_rowid();

    Ok(CreateKeywordResult {
        id,
        name: normalized,
        created: true,
    })
}

/// Delete a keyword and all its associations
#[tauri::command]
pub fn delete_keyword(state: State<AppState>, id: i64) -> Result<(), String> {
    use crate::db::transaction::with_transaction_result;

    let db = state.db_conn()?;

    with_transaction_result(db.conn(), |conn| {
        // Verify keyword exists
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM immanentize WHERE id = ?",
                [id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !exists {
            return Err(crate::db::transaction::TransactionError::Failed(
                format!("Keyword mit ID {} nicht gefunden.", id),
            ));
        }

        // Single DELETE — CASCADE handles fnord_immanentize, immanentize_neighbors,
        // immanentize_sephiroth, immanentize_daily, embedding_queue, dismissed_synonyms,
        // preserved_compounds, compound_decisions.
        // Trigger immanentize_delete_vec handles vec_immanentize.
        conn.execute("DELETE FROM immanentize WHERE id = ?", [id])?;

        Ok(())
    })
}

/// Rename a keyword
#[tauri::command]
pub fn rename_keyword(state: State<AppState>, id: i64, new_name: String) -> Result<String, String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    // Trim and validate new name
    let new_name = new_name.trim().to_string();
    if new_name.len() < 2 {
        return Err("Der Name muss mindestens 2 Zeichen haben.".to_string());
    }
    if new_name.len() > 100 {
        return Err("Der Name darf maximal 100 Zeichen haben.".to_string());
    }

    // Check if another keyword with this name already exists (case-insensitive)
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM immanentize WHERE LOWER(name) = LOWER(?) AND id != ?",
            params![&new_name, id],
            |row| row.get(0),
        )
        .ok();

    if existing.is_some() {
        return Err(format!(
            "Ein Keyword mit dem Namen '{}' existiert bereits.",
            new_name
        ));
    }

    // Update the name
    conn.execute(
        "UPDATE immanentize SET name = ? WHERE id = ?",
        params![&new_name, id],
    )
    .map_err(|e| format!("Fehler beim Umbenennen: {}", e))?;

    // Invalidate old embedding and re-queue for regeneration
    // The embedding was generated from the old name, so it's no longer accurate
    let _ = conn.execute(
        "UPDATE immanentize SET embedding = NULL WHERE id = ?",
        [id],
    );

    // Remove from vec search index
    let _ = conn.execute("DELETE FROM vec_immanentize WHERE immanentize_id = ?", [id]);

    // Queue for re-embedding with high priority
    let _ = conn.execute(
        r#"INSERT INTO embedding_queue (immanentize_id, priority, queued_at)
           VALUES (?1, 10, CURRENT_TIMESTAMP)
           ON CONFLICT(immanentize_id) DO UPDATE SET
               priority = 10,
               queued_at = CURRENT_TIMESTAMP,
               attempts = 0,
               last_error = NULL"#,
        [id],
    );

    log::info!("Keyword {} renamed to '{}'", id, new_name);

    Ok(new_name)
}

// ============================================================
// LEARNING SYSTEM: Auto-merge similar keywords
// ============================================================

#[derive(Debug, Serialize)]
pub struct AutoMergeResult {
    pub checked: i64,
    pub merged: i64,
    pub merge_details: Vec<AutoMergeDetail>,
}

#[derive(Debug, Serialize)]
pub struct AutoMergeDetail {
    pub source_name: String,
    pub target_name: String,
    pub similarity: f64,
    pub articles_moved: i64,
}

/// Automatically merge very similar keywords using embedding similarity.
/// This is the "learning" part of the system - it consolidates the keyword space
/// by merging new/less-used keywords into established similar ones.
///
/// Strategy:
/// 1. Find keywords with embeddings, ordered by article_count ASC (merge smaller into larger)
/// 2. For each keyword, find the most similar existing keyword using vec_immanentize
/// 3. If similarity >= threshold AND the target has more articles, merge source into target
/// 4. Skip already dismissed pairs
#[tauri::command]
pub fn auto_merge_similar_keywords(
    state: State<AppState>,
    threshold: Option<f64>,
    limit: Option<i64>,
    dry_run: Option<bool>,
) -> Result<AutoMergeResult, String> {
    let db = state.db_conn()?;
    let conn = db.conn();
    let threshold = threshold.unwrap_or(0.92);
    let limit = limit.unwrap_or(100);
    let dry_run = dry_run.unwrap_or(false);

    // Cosine distance = 1 - cosine similarity
    let max_distance = 1.0 - threshold;

    // Load dismissed pairs to skip
    let dismissed: std::collections::HashSet<(i64, i64)> = conn
        .prepare("SELECT keyword_a_id, keyword_b_id FROM dismissed_synonyms")
        .map_err(|e| e.to_string())?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Get keywords ordered by article_count ASC (process smaller ones first for merging)
    // This ensures we merge less-used keywords into more-used ones
    let keywords: Vec<(i64, String, Vec<u8>, i64)> = conn
        .prepare(
            r#"SELECT id, name, embedding, COALESCE(article_count, 0)
               FROM immanentize
               WHERE embedding IS NOT NULL
               ORDER BY article_count ASC
               LIMIT ?"#,
        )
        .map_err(|e| e.to_string())?
        .query_map([limit * 2], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut checked = 0i64;
    let mut merged = 0i64;
    let mut merge_details: Vec<AutoMergeDetail> = Vec::new();
    let mut merged_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();

    for (keyword_id, keyword_name, embedding_blob, article_count) in keywords {
        if merged >= limit {
            break;
        }

        // Skip if already merged
        if merged_ids.contains(&keyword_id) {
            continue;
        }

        checked += 1;

        // Use vec_immanentize KNN search to find nearest neighbor
        let neighbors: Vec<(i64, String, f64, i64)> = conn
            .prepare(
                r#"SELECT v.immanentize_id, i.name, v.distance, COALESCE(i.article_count, 0)
                   FROM vec_immanentize v
                   JOIN immanentize i ON i.id = v.immanentize_id
                   WHERE v.embedding MATCH ?1 AND k = 5
                   ORDER BY v.distance"#,
            )
            .and_then(|mut stmt| {
                let rows = stmt.query_map([&embedding_blob], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, f64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                })?;
                Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
            })
            .unwrap_or_default();

        // Find the best match that satisfies our criteria
        let neighbor = neighbors.into_iter().find(|(id, _, dist, target_count)| {
            *id != keyword_id
                && !merged_ids.contains(id)
                && *dist <= max_distance
                && *target_count >= article_count // Target has same or more articles
        });

        if let Some((target_id, target_name, distance, _)) = neighbor {
            let similarity = 1.0 - distance;

            // Check if this pair was dismissed
            let (min_id, max_id) = if keyword_id < target_id {
                (keyword_id, target_id)
            } else {
                (target_id, keyword_id)
            };

            if dismissed.contains(&(min_id, max_id)) {
                continue;
            }

            // Skip if names are too different in length (probably not synonyms)
            let len_ratio = keyword_name.len().min(target_name.len()) as f64
                / keyword_name.len().max(target_name.len()) as f64;
            if len_ratio < 0.3 {
                continue;
            }

            if dry_run {
                log::info!(
                    "Would merge '{}' ({} articles) into '{}' (similarity: {:.3})",
                    keyword_name,
                    article_count,
                    target_name,
                    similarity
                );
                merge_details.push(AutoMergeDetail {
                    source_name: keyword_name.clone(),
                    target_name: target_name.clone(),
                    similarity,
                    articles_moved: article_count,
                });
                merged += 1;
            } else {
                // Actually perform the merge
                match perform_merge(conn, target_id, keyword_id) {
                    Ok(articles_moved) => {
                        log::info!(
                            "Merged '{}' into '{}' (similarity: {:.3}, {} articles moved)",
                            keyword_name,
                            target_name,
                            similarity,
                            articles_moved
                        );
                        merge_details.push(AutoMergeDetail {
                            source_name: keyword_name.clone(),
                            target_name: target_name.clone(),
                            similarity,
                            articles_moved,
                        });
                        merged_ids.insert(keyword_id);
                        merged += 1;
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to merge '{}' into '{}': {}",
                            keyword_name,
                            target_name,
                            e
                        );
                    }
                }
            }
        }
    }

    Ok(AutoMergeResult {
        checked,
        merged,
        merge_details,
    })
}

/// Internal helper to perform the actual merge operation
fn perform_merge(conn: &rusqlite::Connection, keep_id: i64, remove_id: i64) -> Result<i64, String> {
    // Use SAVEPOINT for nested transaction support (caller may already have a transaction)
    conn.execute("SAVEPOINT merge_op", [])
        .map_err(|e| e.to_string())?;

    let result = perform_merge_inner(conn, keep_id, remove_id);

    match &result {
        Ok(_) => {
            conn.execute("RELEASE merge_op", [])
                .map_err(|e| e.to_string())?;
        }
        Err(_) => {
            let _ = conn.execute("ROLLBACK TO merge_op", []);
            let _ = conn.execute("RELEASE merge_op", []);
        }
    }

    result
}

fn perform_merge_inner(
    conn: &rusqlite::Connection,
    keep_id: i64,
    remove_id: i64,
) -> Result<i64, String> {
    // Count articles that will be affected
    let affected: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT fnord_id) FROM fnord_immanentize WHERE immanentize_id = ?",
            [remove_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // --- MERGE fnord_immanentize ---
    // Delete duplicates first (articles that reference both keywords)
    conn.execute(
        r#"DELETE FROM fnord_immanentize
           WHERE immanentize_id = ?1
           AND fnord_id IN (SELECT fnord_id FROM fnord_immanentize WHERE immanentize_id = ?2)"#,
        params![remove_id, keep_id],
    )
    .map_err(|e| e.to_string())?;

    // Move remaining to target
    conn.execute(
        "UPDATE fnord_immanentize SET immanentize_id = ?1 WHERE immanentize_id = ?2",
        params![keep_id, remove_id],
    )
    .map_err(|e| e.to_string())?;

    // Update article_count
    conn.execute(
        r#"UPDATE immanentize SET
           article_count = (SELECT COUNT(DISTINCT fnord_id) FROM fnord_immanentize WHERE immanentize_id = ?1),
           last_used = CURRENT_TIMESTAMP
           WHERE id = ?1"#,
        [keep_id],
    )
    .map_err(|e| e.to_string())?;

    // --- MERGE immanentize_sephiroth (transfer category associations) ---
    conn.execute(
        r#"INSERT INTO immanentize_sephiroth (immanentize_id, sephiroth_id, weight, article_count, first_seen, updated_at)
           SELECT ?1, sephiroth_id, weight, article_count, first_seen, updated_at
           FROM immanentize_sephiroth WHERE immanentize_id = ?2
           ON CONFLICT(immanentize_id, sephiroth_id) DO UPDATE SET
               article_count = article_count + excluded.article_count,
               updated_at = CURRENT_TIMESTAMP"#,
        params![keep_id, remove_id],
    )
    .map_err(|e| e.to_string())?;

    // --- MERGE immanentize_neighbors edges (re-wire, respecting CHECK id_a < id_b) ---
    // Case 1: remove_id is id_a — re-wire to keep_id
    conn.execute(
        r#"INSERT INTO immanentize_neighbors (immanentize_id_a, immanentize_id_b, cooccurrence, embedding_similarity, combined_weight, first_seen, last_seen)
           SELECT
               CASE WHEN ?1 < immanentize_id_b THEN ?1 ELSE immanentize_id_b END,
               CASE WHEN ?1 < immanentize_id_b THEN immanentize_id_b ELSE ?1 END,
               cooccurrence, embedding_similarity, combined_weight, first_seen, last_seen
           FROM immanentize_neighbors
           WHERE immanentize_id_a = ?2 AND immanentize_id_b != ?1
           ON CONFLICT(immanentize_id_a, immanentize_id_b) DO UPDATE SET
               cooccurrence = cooccurrence + excluded.cooccurrence,
               last_seen = MAX(last_seen, excluded.last_seen)"#,
        params![keep_id, remove_id],
    )
    .map_err(|e| e.to_string())?;

    // Case 2: remove_id is id_b — re-wire to keep_id
    conn.execute(
        r#"INSERT INTO immanentize_neighbors (immanentize_id_a, immanentize_id_b, cooccurrence, embedding_similarity, combined_weight, first_seen, last_seen)
           SELECT
               CASE WHEN immanentize_id_a < ?1 THEN immanentize_id_a ELSE ?1 END,
               CASE WHEN immanentize_id_a < ?1 THEN ?1 ELSE immanentize_id_a END,
               cooccurrence, embedding_similarity, combined_weight, first_seen, last_seen
           FROM immanentize_neighbors
           WHERE immanentize_id_b = ?2 AND immanentize_id_a != ?1
           ON CONFLICT(immanentize_id_a, immanentize_id_b) DO UPDATE SET
               cooccurrence = cooccurrence + excluded.cooccurrence,
               last_seen = MAX(last_seen, excluded.last_seen)"#,
        params![keep_id, remove_id],
    )
    .map_err(|e| e.to_string())?;

    // --- MERGE immanentize_daily (transfer trending data) ---
    conn.execute(
        r#"INSERT INTO immanentize_daily (immanentize_id, date, count)
           SELECT ?1, date, count FROM immanentize_daily WHERE immanentize_id = ?2
           ON CONFLICT(immanentize_id, date) DO UPDATE SET count = count + excluded.count"#,
        params![keep_id, remove_id],
    )
    .map_err(|e| e.to_string())?;

    // --- DELETE source keyword (CASCADE handles remaining dependent rows) ---
    conn.execute("DELETE FROM immanentize WHERE id = ?", [remove_id])
        .map_err(|e| e.to_string())?;

    Ok(affected)
}

// ============================================================
// KEYWORD TYPE BATCH UPDATE
// ============================================================

/// Result of batch updating keyword types
#[derive(Debug, Serialize)]
pub struct KeywordTypeUpdateResult {
    pub total_updated: i64,
    pub concept_count: i64,
    pub person_count: i64,
    pub organization_count: i64,
    pub location_count: i64,
    pub acronym_count: i64,
}

/// Batch update all keyword types using heuristic detection
/// Re-analyzes all keywords and updates their type based on pattern matching
#[tauri::command]
pub fn update_keyword_types(state: State<AppState>) -> Result<KeywordTypeUpdateResult, String> {
    use super::ai::helpers::detect_keyword_type;

    let db = state.db_conn()?;
    let conn = db.conn();

    // Get all keywords
    let keywords: Vec<(i64, String)> = conn
        .prepare("SELECT id, name FROM immanentize")
        .map_err(|e| e.to_string())?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut total_updated = 0i64;
    let mut concept_count = 0i64;
    let mut person_count = 0i64;
    let mut organization_count = 0i64;
    let mut location_count = 0i64;
    let mut acronym_count = 0i64;

    for (id, name) in &keywords {
        let detected_type = detect_keyword_type(name);

        // Update the keyword type in the database
        if let Err(e) = conn.execute(
            "UPDATE immanentize SET keyword_type = ?1 WHERE id = ?2",
            params![&detected_type, id],
        ) {
            warn!("Failed to update keyword type for {}: {}", id, e);
            continue;
        }

        total_updated += 1;

        // Count by type
        match detected_type.as_str() {
            "concept" => concept_count += 1,
            "person" => person_count += 1,
            "organization" => organization_count += 1,
            "location" => location_count += 1,
            "acronym" => acronym_count += 1,
            _ => concept_count += 1,
        }
    }

    log::info!(
        "Keyword types updated: {} total ({} concept, {} person, {} org, {} location, {} acronym)",
        total_updated,
        concept_count,
        person_count,
        organization_count,
        location_count,
        acronym_count
    );

    Ok(KeywordTypeUpdateResult {
        total_updated,
        concept_count,
        person_count,
        organization_count,
        location_count,
        acronym_count,
    })
}

/// Result of keyword cleanup operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordCleanupResult {
    pub stopwords_refreshed: i64,
    pub stopword_keywords_deleted: i64,
    pub seeds_inserted: i64,
    pub types_updated_from_seeds: i64,
    pub types_updated_from_heuristics: i64,
}

/// Comprehensive keyword cleanup: refresh stopwords, delete stopword-only keywords,
/// seed known entities, and update keyword types
#[tauri::command]
pub fn cleanup_keywords(state: State<AppState>) -> Result<KeywordCleanupResult, String> {
    use super::ai::helpers::detect_keyword_type;
    use crate::text_analysis::{seed_known_keywords, update_types_from_seeds, STOPWORDS};

    let db = state.db_conn()?;
    let conn = db.conn();

    // Step 1: Refresh system stopwords in database
    // First, get current count
    let old_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stopwords WHERE source = 'system'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Delete old system stopwords and re-insert
    conn.execute("DELETE FROM stopwords WHERE source = 'system'", [])
        .map_err(|e| e.to_string())?;

    let mut stopwords_inserted = 0i64;
    for word in STOPWORDS.iter() {
        if conn.execute(
            "INSERT OR IGNORE INTO stopwords (word, source, language) VALUES (?1, 'system', NULL)",
            params![word.to_lowercase()],
        ).is_ok() {
            stopwords_inserted += 1;
        }
    }

    log::info!(
        "Stopwords refreshed: {} old -> {} new",
        old_count,
        stopwords_inserted
    );

    // Step 2: Delete keywords that are pure stopwords
    // Get all stopwords from DB for comparison
    let db_stopwords: HashSet<String> = conn
        .prepare("SELECT word FROM stopwords")
        .map_err(|e| e.to_string())?
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Find keywords that are stopwords (case-insensitive)
    let keywords_to_check: Vec<(i64, String)> = conn
        .prepare("SELECT id, name FROM immanentize")
        .map_err(|e| e.to_string())?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut stopword_keywords_deleted = 0i64;
    for (id, name) in &keywords_to_check {
        let name_lower = name.to_lowercase();
        // Check if the keyword is a single stopword
        if db_stopwords.contains(&name_lower) {
            // Delete the keyword and its associations
            conn.execute(
                "DELETE FROM fnord_immanentize WHERE immanentize_id = ?1",
                params![id],
            )
            .ok();
            conn.execute(
                "DELETE FROM immanentize_sephiroth WHERE immanentize_id = ?1",
                params![id],
            )
            .ok();
            conn.execute("DELETE FROM immanentize_neighbors WHERE immanentize_id_a = ?1 OR immanentize_id_b = ?1", params![id])
                .ok();
            conn.execute(
                "DELETE FROM vec_immanentize WHERE immanentize_id = ?1",
                params![id],
            )
            .ok();
            if conn
                .execute("DELETE FROM immanentize WHERE id = ?1", params![id])
                .is_ok()
            {
                stopword_keywords_deleted += 1;
            }
        }
    }

    log::info!(
        "Deleted {} stopword-only keywords",
        stopword_keywords_deleted
    );

    // Step 3: Seed known keywords
    let seeds_inserted = seed_known_keywords(conn).unwrap_or(0) as i64;
    log::info!("Seeded {} known keywords", seeds_inserted);

    // Step 4: Update types from seed data (for existing keywords)
    let types_updated_from_seeds = update_types_from_seeds(conn).unwrap_or(0) as i64;
    log::info!(
        "Updated {} keyword types from seeds",
        types_updated_from_seeds
    );

    // Step 5: Run heuristic type detection on remaining keywords
    let keywords_needing_update: Vec<(i64, String)> = conn
        .prepare("SELECT id, name FROM immanentize WHERE keyword_type IS NULL OR keyword_type = 'concept'")
        .map_err(|e| e.to_string())?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut types_updated_from_heuristics = 0i64;
    for (id, name) in &keywords_needing_update {
        let detected_type = detect_keyword_type(name);
        // Only update if not concept (we want to keep specific types)
        if detected_type != "concept"
            && conn
                .execute(
                    "UPDATE immanentize SET keyword_type = ?1 WHERE id = ?2",
                    params![&detected_type, id],
                )
                .is_ok()
        {
            types_updated_from_heuristics += 1;
        }
    }

    log::info!(
        "Updated {} keyword types from heuristics",
        types_updated_from_heuristics
    );

    Ok(KeywordCleanupResult {
        stopwords_refreshed: stopwords_inserted,
        stopword_keywords_deleted,
        seeds_inserted,
        types_updated_from_seeds,
        types_updated_from_heuristics,
    })
}

// ============================================================
// COMPOUND KEYWORD SPLITTING
// ============================================================

/// Result of splitting compound keywords
#[derive(Debug, Serialize, Clone)]
pub struct CompoundSplitResult {
    pub compounds_found: i64,
    pub compounds_split: i64,
    pub components_created: i64,
    pub articles_transferred: i64,
    pub split_details: Vec<CompoundSplitDetail>,
}

/// Detail of a single compound split
#[derive(Debug, Serialize, Clone)]
pub struct CompoundSplitDetail {
    pub id: i64,
    pub original: String,
    pub components: Vec<String>,
    pub articles_affected: i64,
    pub is_preserved: bool,
}

/// Split all compound keywords in the database
/// - Finds keywords with hyphens that should be split
/// - Creates or finds component keywords
/// - Transfers article associations to components
/// - Deletes the original compound keyword
/// - Excludes keywords that are preserved (decision='preserve' or in legacy preserved_compounds)
/// - Also excludes keywords with type 'person' or 'location' and "Anti-*" keywords
#[tauri::command]
pub fn split_compound_keywords(
    state: State<AppState>,
    dry_run: Option<bool>,
) -> Result<CompoundSplitResult, String> {
    let dry_run = dry_run.unwrap_or(false);
    let db = state.db_conn()?;
    let conn = db.conn();

    // Find all hyphenated keywords that are NOT preserved and NOT auto-excluded
    // Excludes:
    // - Keywords with decision='preserve' in compound_decisions
    // - Keywords in legacy preserved_compounds table
    // - Keywords with type 'person' or 'location'
    // - Keywords starting with "Anti-"
    let compounds: Vec<(i64, String)> = conn
        .prepare(
            r#"SELECT id, name FROM immanentize
             WHERE name LIKE '%-%' AND LENGTH(name) > 5
             AND id NOT IN (SELECT immanentize_id FROM preserved_compounds)
             AND id NOT IN (SELECT immanentize_id FROM compound_decisions WHERE decision = 'preserve')
             AND keyword_type NOT IN ('person', 'location')
             AND LOWER(name) NOT LIKE 'anti-%'"#
        )
        .map_err(|e| e.to_string())?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let compounds_found = compounds.len() as i64;
    let mut compounds_split = 0i64;
    let mut components_created = 0i64;
    let mut articles_transferred = 0i64;
    let mut split_details: Vec<CompoundSplitDetail> = Vec::new();

    for (compound_id, compound_name) in compounds {
        // Check if this compound should be split
        if !should_split_compound(&compound_name) {
            continue;
        }

        let components = get_compound_components(&compound_name);
        if components.is_empty() {
            continue;
        }

        // Get article associations for this compound
        let article_ids: Vec<i64> = conn
            .prepare("SELECT fnord_id FROM fnord_immanentize WHERE immanentize_id = ?")
            .map_err(|e| e.to_string())?
            .query_map([compound_id], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        let articles_affected = article_ids.len() as i64;

        if dry_run {
            split_details.push(CompoundSplitDetail {
                id: compound_id,
                original: compound_name.clone(),
                components: components.clone(),
                articles_affected,
                is_preserved: false,
            });
            compounds_split += 1;
            articles_transferred += articles_affected;
            continue;
        }

        // Begin transaction for this compound
        conn.execute("BEGIN", []).map_err(|e| e.to_string())?;

        let mut component_ids: Vec<i64> = Vec::new();

        // Create or find component keywords
        for component in &components {
            // Check if component already exists
            let existing_id: Option<i64> = conn
                .query_row(
                    "SELECT id FROM immanentize WHERE LOWER(name) = LOWER(?)",
                    [component],
                    |row| row.get(0),
                )
                .ok();

            let component_id = match existing_id {
                Some(id) => id,
                None => {
                    // Create new keyword
                    conn.execute(
                        "INSERT INTO immanentize (name, count, article_count, is_canonical, keyword_type, first_seen, last_used, quality_score)
                         VALUES (?, 0, 0, 1, 'concept', datetime('now'), datetime('now'), 0.5)",
                        [component],
                    )
                    .map_err(|e| {
                        let _ = conn.execute("ROLLBACK", []);
                        e.to_string()
                    })?;
                    components_created += 1;
                    conn.last_insert_rowid()
                }
            };
            component_ids.push(component_id);
        }

        // Transfer article associations to all components
        for article_id in &article_ids {
            for component_id in &component_ids {
                // Check if association already exists
                let exists: bool = conn
                    .query_row(
                        "SELECT 1 FROM fnord_immanentize WHERE fnord_id = ? AND immanentize_id = ?",
                        params![article_id, component_id],
                        |_| Ok(true),
                    )
                    .unwrap_or(false);

                if !exists {
                    // Get confidence from original association
                    let confidence: f64 = conn
                        .query_row(
                            "SELECT confidence FROM fnord_immanentize WHERE fnord_id = ? AND immanentize_id = ?",
                            params![article_id, compound_id],
                            |row| row.get(0),
                        )
                        .unwrap_or(1.0);

                    conn.execute(
                        "INSERT INTO fnord_immanentize (fnord_id, immanentize_id, source, confidence) VALUES (?, ?, 'ai', ?)",
                        params![article_id, component_id, confidence],
                    )
                    .map_err(|e| {
                        let _ = conn.execute("ROLLBACK", []);
                        e.to_string()
                    })?;
                }
            }
        }

        // Update article counts for components
        for component_id in &component_ids {
            conn.execute(
                "UPDATE immanentize SET article_count = (
                    SELECT COUNT(DISTINCT fnord_id) FROM fnord_immanentize WHERE immanentize_id = ?
                ) WHERE id = ?",
                params![component_id, component_id],
            )
            .map_err(|e| {
                let _ = conn.execute("ROLLBACK", []);
                e.to_string()
            })?;
        }

        // Delete the compound keyword's associations and the keyword itself
        conn.execute(
            "DELETE FROM fnord_immanentize WHERE immanentize_id = ?",
            [compound_id],
        )
        .map_err(|e| {
            let _ = conn.execute("ROLLBACK", []);
            e.to_string()
        })?;

        conn.execute(
            "DELETE FROM immanentize_neighbors WHERE immanentize_id_a = ? OR immanentize_id_b = ?",
            params![compound_id, compound_id],
        )
        .map_err(|e| {
            let _ = conn.execute("ROLLBACK", []);
            e.to_string()
        })?;

        conn.execute(
            "DELETE FROM embedding_queue WHERE immanentize_id = ?",
            [compound_id],
        )
        .ok(); // Ignore if table doesn't exist

        conn.execute(
            "DELETE FROM dismissed_synonyms WHERE keyword_a_id = ? OR keyword_b_id = ?",
            params![compound_id, compound_id],
        )
        .ok();

        conn.execute(
            "DELETE FROM vec_immanentize WHERE immanentize_id = ?",
            [compound_id],
        )
        .ok();

        conn.execute("DELETE FROM immanentize WHERE id = ?", [compound_id])
            .map_err(|e| {
                let _ = conn.execute("ROLLBACK", []);
                e.to_string()
            })?;

        conn.execute("COMMIT", []).map_err(|e| e.to_string())?;

        split_details.push(CompoundSplitDetail {
            id: compound_id,
            original: compound_name,
            components,
            articles_affected,
            is_preserved: false,
        });
        compounds_split += 1;
        articles_transferred += articles_affected;
    }

    info!(
        "Compound splitting complete: {} found, {} split, {} components created, {} articles transferred",
        compounds_found, compounds_split, components_created, articles_transferred
    );

    Ok(CompoundSplitResult {
        compounds_found,
        compounds_split,
        components_created,
        articles_transferred,
        split_details,
    })
}

/// Preview which compound keywords would be split
/// Returns only splittable compounds that need a decision (no decision yet)
/// Filters out:
/// - Keywords with existing decision in compound_decisions table
/// - Keywords with keyword_type = 'person' or 'location'
/// - Keywords starting with "Anti-" (case-insensitive)
#[tauri::command]
pub fn preview_compound_splits(state: State<AppState>) -> Result<Vec<CompoundSplitDetail>, String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    // Load IDs with existing decisions (these should not appear in the review list)
    let decided_ids: std::collections::HashSet<i64> = conn
        .prepare("SELECT immanentize_id FROM compound_decisions")
        .map_err(|e| e.to_string())?
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Also load legacy preserved_compounds for backward compatibility
    let legacy_preserved: std::collections::HashSet<i64> = conn
        .prepare("SELECT immanentize_id FROM preserved_compounds")
        .and_then(|mut stmt| {
            let rows = stmt.query_map([], |row| row.get(0))?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    // Find all hyphenated keywords that:
    // - Have no decision in compound_decisions
    // - Are NOT person or location type
    // - Do NOT start with "Anti-"
    let compounds: Vec<(i64, String, String)> = conn
        .prepare(
            r#"SELECT id, name, COALESCE(keyword_type, 'concept') as kw_type
               FROM immanentize
               WHERE name LIKE '%-%' AND LENGTH(name) > 5
               AND keyword_type NOT IN ('person', 'location')
               AND LOWER(name) NOT LIKE 'anti-%'"#,
        )
        .map_err(|e| e.to_string())?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut split_details: Vec<CompoundSplitDetail> = Vec::new();

    for (compound_id, compound_name, _keyword_type) in compounds {
        // Skip if decision already exists
        if decided_ids.contains(&compound_id) || legacy_preserved.contains(&compound_id) {
            continue;
        }

        // Check if this compound should be split (validates against NO_SPLIT list)
        if !should_split_compound(&compound_name) {
            continue;
        }

        let components = get_compound_components(&compound_name);
        if components.is_empty() {
            continue;
        }

        // Count article associations
        let articles_affected: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM fnord_immanentize WHERE immanentize_id = ?",
                [compound_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        split_details.push(CompoundSplitDetail {
            id: compound_id,
            original: compound_name,
            components,
            articles_affected,
            is_preserved: false, // Not preserved, needs decision
        });
    }

    Ok(split_details)
}

/// Split a single compound keyword by ID
/// Returns the split detail if successful
#[tauri::command]
pub fn split_single_compound(
    state: State<AppState>,
    keyword_id: i64,
) -> Result<CompoundSplitDetail, String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    // Get the keyword
    let keyword_name: String = conn
        .query_row(
            "SELECT name FROM immanentize WHERE id = ?",
            [keyword_id],
            |row| row.get(0),
        )
        .map_err(|_| format!("Keyword mit ID {} nicht gefunden", keyword_id))?;

    // Check if this keyword is preserved
    let is_preserved: bool = conn
        .query_row(
            "SELECT 1 FROM preserved_compounds WHERE immanentize_id = ?",
            [keyword_id],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if is_preserved {
        return Err("Dieses Keyword ist geschützt und kann nicht gesplittet werden. Entfernen Sie zuerst den Schutz.".to_string());
    }

    // Check if this compound should be split
    if !should_split_compound(&keyword_name) {
        return Err(format!(
            "Das Keyword '{}' kann nicht gesplittet werden (enthält kein Trennzeichen oder sollte nicht gesplittet werden)",
            keyword_name
        ));
    }

    let components = get_compound_components(&keyword_name);
    if components.is_empty() {
        return Err(format!(
            "Das Keyword '{}' kann nicht in Komponenten zerlegt werden",
            keyword_name
        ));
    }

    // Get article associations for this compound
    let article_ids: Vec<i64> = conn
        .prepare("SELECT fnord_id FROM fnord_immanentize WHERE immanentize_id = ?")
        .map_err(|e| e.to_string())?
        .query_map([keyword_id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let articles_affected = article_ids.len() as i64;

    // Begin transaction
    conn.execute("BEGIN", []).map_err(|e| e.to_string())?;

    let mut component_ids: Vec<i64> = Vec::new();

    // Create or find component keywords
    for component in &components {
        // Check if component already exists
        let existing_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM immanentize WHERE LOWER(name) = LOWER(?)",
                [component],
                |row| row.get(0),
            )
            .ok();

        let component_id = match existing_id {
            Some(id) => id,
            None => {
                // Create new keyword
                conn.execute(
                    "INSERT INTO immanentize (name, count, article_count, is_canonical, keyword_type, first_seen, last_used, quality_score)
                     VALUES (?, 0, 0, 1, 'concept', datetime('now'), datetime('now'), 0.5)",
                    [component],
                )
                .map_err(|e| {
                    let _ = conn.execute("ROLLBACK", []);
                    e.to_string()
                })?;
                conn.last_insert_rowid()
            }
        };
        component_ids.push(component_id);
    }

    // Transfer article associations to all components
    for article_id in &article_ids {
        for component_id in &component_ids {
            // Check if association already exists
            let exists: bool = conn
                .query_row(
                    "SELECT 1 FROM fnord_immanentize WHERE fnord_id = ? AND immanentize_id = ?",
                    params![article_id, component_id],
                    |_| Ok(true),
                )
                .unwrap_or(false);

            if !exists {
                // Get confidence from original association
                let confidence: f64 = conn
                    .query_row(
                        "SELECT confidence FROM fnord_immanentize WHERE fnord_id = ? AND immanentize_id = ?",
                        params![article_id, keyword_id],
                        |row| row.get(0),
                    )
                    .unwrap_or(1.0);

                conn.execute(
                    "INSERT INTO fnord_immanentize (fnord_id, immanentize_id, source, confidence) VALUES (?, ?, 'ai', ?)",
                    params![article_id, component_id, confidence],
                )
                .map_err(|e| {
                    let _ = conn.execute("ROLLBACK", []);
                    e.to_string()
                })?;
            }
        }
    }

    // Update article counts for components
    for component_id in &component_ids {
        conn.execute(
            "UPDATE immanentize SET article_count = (
                SELECT COUNT(DISTINCT fnord_id) FROM fnord_immanentize WHERE immanentize_id = ?
            ) WHERE id = ?",
            params![component_id, component_id],
        )
        .map_err(|e| {
            let _ = conn.execute("ROLLBACK", []);
            e.to_string()
        })?;
    }

    // Delete the compound keyword's associations and the keyword itself
    conn.execute(
        "DELETE FROM fnord_immanentize WHERE immanentize_id = ?",
        [keyword_id],
    )
    .map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        e.to_string()
    })?;

    conn.execute(
        "DELETE FROM immanentize_neighbors WHERE immanentize_id_a = ? OR immanentize_id_b = ?",
        params![keyword_id, keyword_id],
    )
    .map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        e.to_string()
    })?;

    conn.execute(
        "DELETE FROM embedding_queue WHERE immanentize_id = ?",
        [keyword_id],
    )
    .ok();

    conn.execute(
        "DELETE FROM dismissed_synonyms WHERE keyword_a_id = ? OR keyword_b_id = ?",
        params![keyword_id, keyword_id],
    )
    .ok();

    conn.execute(
        "DELETE FROM vec_immanentize WHERE immanentize_id = ?",
        [keyword_id],
    )
    .ok();

    conn.execute("DELETE FROM immanentize WHERE id = ?", [keyword_id])
        .map_err(|e| {
            let _ = conn.execute("ROLLBACK", []);
            e.to_string()
        })?;

    // Also clean up from compound_decisions (if there was any)
    conn.execute(
        "DELETE FROM compound_decisions WHERE immanentize_id = ?",
        [keyword_id],
    )
    .ok();

    conn.execute("COMMIT", []).map_err(|e| e.to_string())?;

    info!(
        "Split compound '{}' (ID {}) into {} components, {} articles transferred",
        keyword_name,
        keyword_id,
        components.len(),
        articles_affected
    );

    Ok(CompoundSplitDetail {
        id: keyword_id,
        original: keyword_name,
        components,
        articles_affected,
        is_preserved: false,
    })
}

/// Mark a compound keyword as preserved (will not be split)
#[tauri::command]
pub fn preserve_compound_keyword(state: State<AppState>, keyword_id: i64) -> Result<(), String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    // Verify keyword exists
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM immanentize WHERE id = ?",
            [keyword_id],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !exists {
        return Err(format!("Keyword mit ID {} nicht gefunden", keyword_id));
    }

    // Insert into preserved_compounds (ignore if already exists)
    conn.execute(
        "INSERT OR IGNORE INTO preserved_compounds (immanentize_id) VALUES (?)",
        [keyword_id],
    )
    .map_err(|e| format!("Fehler beim Schützen des Keywords: {}", e))?;

    info!("Compound keyword {} marked as preserved", keyword_id);

    Ok(())
}

/// Remove preservation from a compound keyword (allow splitting again)
#[tauri::command]
pub fn unpreserve_compound_keyword(state: State<AppState>, keyword_id: i64) -> Result<(), String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    conn.execute(
        "DELETE FROM preserved_compounds WHERE immanentize_id = ?",
        [keyword_id],
    )
    .map_err(|e| format!("Fehler beim Entfernen des Schutzes: {}", e))?;

    info!("Compound keyword {} preservation removed", keyword_id);

    Ok(())
}

/// Get all preserved compound keywords
#[tauri::command]
pub fn get_preserved_compounds(state: State<AppState>) -> Result<Vec<Keyword>, String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    let sql = format!(
        "SELECT {} FROM immanentize i
         INNER JOIN preserved_compounds pc ON pc.immanentize_id = i.id
         ORDER BY i.article_count DESC, i.name",
        KEYWORD_SELECT_COLUMNS
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let keywords = stmt
        .query_map([], keyword_from_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(keywords)
}

// ============================================================
// COMPOUND KEYWORD DECISION SYSTEM (New in Phase 4)
// ============================================================

/// A compound keyword with its decision status
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompoundDecision {
    pub id: i64,
    pub name: String,
    pub decision: String, // 'preserve' or 'split'
    pub decided_at: String,
    pub article_count: i64,
    pub components: Vec<String>, // Potential split components
}

/// Set a decision for a compound keyword (preserve or split)
/// - 'preserve' = Keyword stays as-is, marked with shield
/// - 'split' = Keyword will be split into components
#[tauri::command]
pub fn set_compound_decision(
    state: State<AppState>,
    keyword_id: i64,
    decision: String,
) -> Result<(), String> {
    // Validate decision value
    if decision != "preserve" && decision != "split" {
        return Err(format!(
            "Ungueltige Entscheidung '{}'. Erlaubt sind: 'preserve', 'split'",
            decision
        ));
    }

    let db = state.db_conn()?;
    let conn = db.conn();

    // Verify keyword exists
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM immanentize WHERE id = ?",
            [keyword_id],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !exists {
        return Err(format!("Keyword mit ID {} nicht gefunden", keyword_id));
    }

    // Insert or update decision
    conn.execute(
        r#"INSERT INTO compound_decisions (immanentize_id, decision, decided_at)
           VALUES (?1, ?2, datetime('now'))
           ON CONFLICT(immanentize_id) DO UPDATE SET
             decision = excluded.decision,
             decided_at = excluded.decided_at"#,
        params![keyword_id, &decision],
    )
    .map_err(|e| format!("Fehler beim Speichern der Entscheidung: {}", e))?;

    info!(
        "Compound decision set for keyword {}: {}",
        keyword_id, decision
    );

    Ok(())
}

/// Get all compound keyword decisions
/// Returns keywords with their decision (preserve/split), sorted by decision type then name
#[tauri::command]
pub fn get_compound_decisions(state: State<AppState>) -> Result<Vec<CompoundDecision>, String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    let decisions: Vec<(i64, String, String, String, i64)> = conn
        .prepare(
            r#"SELECT i.id, i.name, cd.decision, cd.decided_at, COALESCE(i.article_count, 0)
               FROM compound_decisions cd
               JOIN immanentize i ON i.id = cd.immanentize_id
               ORDER BY cd.decision, i.name"#,
        )
        .map_err(|e| e.to_string())?
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let result: Vec<CompoundDecision> = decisions
        .into_iter()
        .map(|(id, name, decision, decided_at, article_count)| {
            // Get potential components for display
            let components = get_compound_components(&name);
            CompoundDecision {
                id,
                name,
                decision,
                decided_at,
                article_count,
                components,
            }
        })
        .collect();

    Ok(result)
}

/// Clear a compound keyword decision (move back to review list)
#[tauri::command]
pub fn clear_compound_decision(state: State<AppState>, keyword_id: i64) -> Result<(), String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    let deleted = conn
        .execute(
            "DELETE FROM compound_decisions WHERE immanentize_id = ?",
            [keyword_id],
        )
        .map_err(|e| format!("Fehler beim Löschen der Entscheidung: {}", e))?;

    if deleted == 0 {
        return Err(format!(
            "Keine Entscheidung für Keyword {} gefunden",
            keyword_id
        ));
    }

    info!("Compound decision cleared for keyword {}", keyword_id);

    Ok(())
}

/// Batch set decisions for multiple keywords
#[tauri::command]
pub fn batch_set_compound_decisions(
    state: State<AppState>,
    keyword_ids: Vec<i64>,
    decision: String,
) -> Result<i64, String> {
    // Validate decision value
    if decision != "preserve" && decision != "split" {
        return Err(format!(
            "Ungueltige Entscheidung '{}'. Erlaubt sind: 'preserve', 'split'",
            decision
        ));
    }

    let db = state.db_conn()?;
    let conn = db.conn();

    conn.execute("BEGIN", []).map_err(|e| e.to_string())?;

    let mut count = 0i64;
    for keyword_id in &keyword_ids {
        // Insert or update decision
        let result = conn.execute(
            r#"INSERT INTO compound_decisions (immanentize_id, decision, decided_at)
               SELECT ?1, ?2, datetime('now')
               WHERE EXISTS (SELECT 1 FROM immanentize WHERE id = ?1)
               ON CONFLICT(immanentize_id) DO UPDATE SET
                 decision = excluded.decision,
                 decided_at = excluded.decided_at"#,
            params![keyword_id, &decision],
        );

        if result.is_ok() {
            count += 1;
        }
    }

    conn.execute("COMMIT", []).map_err(|e| e.to_string())?;

    info!(
        "Batch compound decision set for {} keywords: {}",
        count, decision
    );

    Ok(count)
}

/// Get statistics about compound keyword decisions
#[derive(Debug, Serialize, Deserialize)]
pub struct CompoundDecisionStats {
    pub total_compounds: i64,     // All hyphenated keywords
    pub needs_decision: i64,      // Without decision (in review list)
    pub preserved_count: i64,     // Decision = preserve
    pub split_count: i64,         // Decision = split
    pub auto_excluded_count: i64, // Excluded by rules (person, location, Anti-)
}

#[tauri::command]
pub fn get_compound_decision_stats(
    state: State<AppState>,
) -> Result<CompoundDecisionStats, String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    // Total hyphenated keywords
    let total_compounds: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM immanentize WHERE name LIKE '%-%' AND LENGTH(name) > 5",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Count by decision type
    let preserved_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM compound_decisions WHERE decision = 'preserve'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let split_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM compound_decisions WHERE decision = 'split'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Auto-excluded (person, location, Anti-*)
    let auto_excluded_count: i64 = conn
        .query_row(
            r#"SELECT COUNT(*) FROM immanentize
               WHERE name LIKE '%-%' AND LENGTH(name) > 5
               AND (keyword_type IN ('person', 'location') OR LOWER(name) LIKE 'anti-%')"#,
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Needs decision = total - decided - auto_excluded
    let decided_total = preserved_count + split_count;
    let needs_decision = total_compounds - decided_total - auto_excluded_count;
    let needs_decision = if needs_decision < 0 {
        0
    } else {
        needs_decision
    };

    Ok(CompoundDecisionStats {
        total_compounds,
        needs_decision,
        preserved_count,
        split_count,
        auto_excluded_count,
    })
}

/// Update the type of a single keyword
/// Valid types: 'concept', 'person', 'organization', 'location', 'acronym'
#[tauri::command]
pub fn update_keyword_type(
    state: State<AppState>,
    keyword_id: i64,
    keyword_type: String,
) -> Result<(), String> {
    // Validate keyword type
    let valid_types = ["concept", "person", "organization", "location", "acronym"];
    if !valid_types.contains(&keyword_type.as_str()) {
        return Err(format!(
            "Invalid keyword type '{}'. Must be one of: {}",
            keyword_type,
            valid_types.join(", ")
        ));
    }

    let db = state.db_conn()?;
    let conn = db.conn();

    // Check if keyword exists
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM immanentize WHERE id = ?)",
            [keyword_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if !exists {
        return Err(format!("Keyword with id {} not found", keyword_id));
    }

    // Update the keyword type
    conn.execute(
        "UPDATE immanentize SET keyword_type = ? WHERE id = ?",
        rusqlite::params![keyword_type, keyword_id],
    )
    .map_err(|e| e.to_string())?;

    log::info!("Updated keyword {} type to '{}'", keyword_id, keyword_type);

    Ok(())
}

// ============================================================
// KEYWORD CONTEXT (for Tooltips)
// ============================================================

/// Context information for a keyword (for tooltips)
#[derive(Debug, Serialize, Deserialize)]
pub struct KeywordContext {
    pub sentence: Option<String>,      // Sentence containing the keyword
    pub article_title: Option<String>, // Title of the most recent article
    pub article_date: Option<String>,  // Publication date
}

/// Get the context of a keyword (sentence from most recent article)
#[tauri::command]
pub fn get_keyword_context(
    state: State<AppState>,
    keyword_id: i64,
) -> Result<KeywordContext, String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    // Get keyword name
    let keyword_name: String = conn
        .query_row(
            "SELECT name FROM immanentize WHERE id = ?",
            [keyword_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Keyword not found: {}", e))?;

    // Find the most recent article with this keyword
    let article_data: Option<(String, String, Option<String>, Option<String>)> = conn
        .query_row(
            r#"SELECT f.title, COALESCE(f.published_at, f.fetched_at),
                      f.content_full, f.content_raw
               FROM fnords f
               JOIN fnord_immanentize fi ON f.id = fi.fnord_id
               WHERE fi.immanentize_id = ?
               ORDER BY f.published_at DESC, f.fetched_at DESC
               LIMIT 1"#,
            [keyword_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .ok();

    if let Some((title, date, content_full, content_raw)) = article_data {
        // Use content_full if available, otherwise content_raw
        let content = content_full.or(content_raw);

        // Extract sentence containing the keyword
        let sentence = content.and_then(|c| extract_sentence_with_keyword(&c, &keyword_name));

        Ok(KeywordContext {
            sentence,
            article_title: Some(title),
            article_date: Some(date),
        })
    } else {
        // No article found with this keyword
        Ok(KeywordContext {
            sentence: None,
            article_title: None,
            article_date: None,
        })
    }
}

/// Extract a sentence containing the keyword from text
pub(crate) fn extract_sentence_with_keyword(text: &str, keyword: &str) -> Option<String> {
    // Clean HTML tags from content
    let clean_text = strip_html_tags(text);

    // Case-insensitive search for keyword (Linear search first for speed)
    let lower_text = clean_text.to_lowercase();
    let lower_keyword = keyword.to_lowercase();

    // 1. Try exact match first
    if let Some(pos) = lower_text.find(&lower_keyword) {
        return extract_sentence_at_pos(&clean_text, pos, keyword.len());
    }

    // 2. Try fuzzy match (handling declensions/suffixes)
    // "Vereinigte Staaten" should match "Vereinigten Staaten"
    // We split by non-word chars to get tokens
    let tokens: Vec<&str> = keyword
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();

    if !tokens.is_empty() {
        // Build a regex pattern: token + [suffixes] + spaces + token ...
        // matching the keyword parts but allowing for German declensions (en, es, er, etc.)
        // e.g., "Vereinigte" -> "Vereinigte[a-zäöüß]*"
        let pattern_str = tokens
            .iter()
            .map(|t| format!("{}[a-zäöüß]*", regex::escape(t)))
            .collect::<Vec<_>>()
            .join(r"[\s\-]+");

        // Word boundary start, case insensitive
        let pattern = format!(r"(?i)\b{}", pattern_str);

        if let Ok(re) = Regex::new(&pattern) {
            if let Some(mat) = re.find(&clean_text) {
                return extract_sentence_at_pos(&clean_text, mat.start(), mat.end() - mat.start());
            }
        }
    }

    None
}

fn extract_sentence_at_pos(text: &str, pos: usize, match_len: usize) -> Option<String> {
    // Find sentence boundaries
    let start = find_sentence_start(text, pos);
    let end = find_sentence_end(text, pos + match_len);

    let sentence = text[start..end].trim().to_string();

    // Limit to ~200 characters, preserving word boundaries
    if sentence.len() > 200 {
        let truncated = truncate_at_word_boundary(&sentence, 200);
        Some(format!("{}...", truncated))
    } else {
        Some(sentence)
    }
}

/// Strip HTML tags from text
pub(crate) fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for c in html.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }

    // Normalize whitespace
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Find the start of the sentence containing the position
pub(crate) fn find_sentence_start(text: &str, pos: usize) -> usize {
    let sentence_enders = ['.', '!', '?', '\n'];
    let bytes = text.as_bytes();

    for i in (0..pos).rev() {
        if sentence_enders.contains(&(bytes[i] as char)) {
            // Skip whitespace after sentence ender
            let next = i + 1;
            if next < text.len() {
                return text[next..]
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .count()
                    + next;
            }
            return next;
        }
    }
    0
}

/// Find the end of the sentence containing the position
pub(crate) fn find_sentence_end(text: &str, pos: usize) -> usize {
    let sentence_enders = ['.', '!', '?', '\n'];

    for (i, c) in text[pos..].char_indices() {
        if sentence_enders.contains(&c) {
            return pos + i + 1;
        }
    }
    text.len()
}

/// Truncate text at word boundary
pub(crate) fn truncate_at_word_boundary(text: &str, max_len: usize) -> &str {
    if text.len() <= max_len {
        return text;
    }

    // Find last space before max_len
    if let Some(last_space) = text[..max_len].rfind(' ') {
        &text[..last_space]
    } else {
        &text[..max_len]
    }
}

// ============================================================
// SYNONYM ASSIGNMENT
// ============================================================

/// Assign a keyword as a synonym of another (canonical) keyword
/// This sets canonical_id and is_canonical = false on the synonym
#[tauri::command]
pub fn assign_synonym(
    state: State<AppState>,
    synonym_id: i64,   // The keyword that becomes a synonym
    canonical_id: i64, // The main/canonical keyword
) -> Result<(), String> {
    if synonym_id == canonical_id {
        return Err("Synonym und Canonical duerfen nicht gleich sein".to_string());
    }

    let db = state.db_conn()?;
    let conn = db.conn();

    // Verify both keywords exist
    let synonym_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM immanentize WHERE id = ?)",
            [synonym_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let canonical_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM immanentize WHERE id = ?)",
            [canonical_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if !synonym_exists {
        return Err(format!(
            "Synonym-Keyword mit ID {} nicht gefunden",
            synonym_id
        ));
    }
    if !canonical_exists {
        return Err(format!(
            "Canonical-Keyword mit ID {} nicht gefunden",
            canonical_id
        ));
    }

    // Check if canonical is itself a synonym (prevent chains)
    let canonical_has_parent: Option<i64> = conn
        .query_row(
            "SELECT canonical_id FROM immanentize WHERE id = ? AND canonical_id IS NOT NULL",
            [canonical_id],
            |row| row.get(0),
        )
        .ok();

    if canonical_has_parent.is_some() {
        return Err(
            "Canonical-Keyword ist selbst ein Synonym. Synonym-Ketten sind nicht erlaubt."
                .to_string(),
        );
    }

    // Update the synonym keyword
    conn.execute(
        "UPDATE immanentize SET canonical_id = ?, is_canonical = FALSE WHERE id = ?",
        params![canonical_id, synonym_id],
    )
    .map_err(|e| e.to_string())?;

    // Get names for logging
    let synonym_name: String = conn
        .query_row(
            "SELECT name FROM immanentize WHERE id = ?",
            [synonym_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| format!("ID:{}", synonym_id));
    let canonical_name: String = conn
        .query_row(
            "SELECT name FROM immanentize WHERE id = ?",
            [canonical_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| format!("ID:{}", canonical_id));

    log::info!(
        "Assigned '{}' (ID:{}) as synonym of '{}' (ID:{})",
        synonym_name,
        synonym_id,
        canonical_name,
        canonical_id
    );

    Ok(())
}

/// Remove synonym assignment (make keyword independent again)
#[tauri::command]
pub fn unassign_synonym(state: State<AppState>, keyword_id: i64) -> Result<(), String> {
    let db = state.db_conn()?;
    let conn = db.conn();

    // Verify keyword exists
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM immanentize WHERE id = ?)",
            [keyword_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if !exists {
        return Err(format!("Keyword mit ID {} nicht gefunden", keyword_id));
    }

    // Remove synonym assignment
    conn.execute(
        "UPDATE immanentize SET canonical_id = NULL, is_canonical = TRUE WHERE id = ?",
        [keyword_id],
    )
    .map_err(|e| e.to_string())?;

    log::info!("Removed synonym assignment for keyword ID:{}", keyword_id);

    Ok(())
}

#[test]
fn test_exact_match() {
    let text = "Hier sind die Vereinigte Staaten von Amerika.";
    let kw = "Vereinigte Staaten";
    let sent = extract_sentence_with_keyword(text, kw);
    assert_eq!(
        sent,
        Some("Hier sind die Vereinigte Staaten von Amerika.".to_string())
    );
}

#[test]
fn test_fuzzy_declension_match() {
    let text = "Wir reisen in die Vereinigten Staaten bald.";
    let kw = "Vereinigte Staaten";
    // Should match "Vereinigten Staaten" because "Vereinigte" matches "Vereinigten" via regex "Vereinigte[...]*"
    let sent = extract_sentence_with_keyword(text, kw);
    assert_eq!(
        sent,
        Some("Wir reisen in die Vereinigten Staaten bald.".to_string())
    );
}

#[test]
fn test_fuzzy_genitive_match() {
    let text = "Der Präsident der Vereinigten Staaten sprach.";
    let kw = "Vereinigte Staaten";
    let sent = extract_sentence_with_keyword(text, kw);
    assert_eq!(
        sent,
        Some("Der Präsident der Vereinigten Staaten sprach.".to_string())
    );
}

#[test]
fn test_no_match() {
    let text = "Hier ist nichts zu sehen.";
    let kw = "Vereinigte Staaten";
    let sent = extract_sentence_with_keyword(text, kw);
    assert_eq!(sent, None);
}

#[test]
fn test_hyphen_variation() {
    // Keyword: "Trump-Zölle"
    // Text: "Die neuen Trump Zölle sind da."
    // Tokens: Trump, Zölle -> Trump...[\s\-]+Zölle...
    let text = "Die neuen Trump Zölle sind hoch.";
    let kw = "Trump-Zölle";
    let sent = extract_sentence_with_keyword(text, kw);
    assert_eq!(sent, Some("Die neuen Trump Zölle sind hoch.".to_string()));
}
