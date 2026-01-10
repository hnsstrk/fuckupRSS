use crate::embeddings::{blob_to_embedding, cosine_similarity};
use crate::{find_canonical_keyword, normalize_keyword, AppState};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

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
    pub recent_count: i64, // Last 7 days
    pub growth_rate: f64,  // recent / (total - recent)
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
    })
}

const KEYWORD_SELECT_COLUMNS: &str =
    "id, name, count, article_count, cluster_id, is_canonical, canonical_id, first_seen, last_used, quality_score";

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
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(100);
    let offset = offset.unwrap_or(0);

    let sql = format!(
        "SELECT {} FROM immanentize WHERE is_canonical = TRUE OR is_canonical IS NULL ORDER BY article_count DESC, count DESC LIMIT ? OFFSET ?",
        KEYWORD_SELECT_COLUMNS
    );

    let mut stmt = db.conn().prepare(&sql).map_err(|e| e.to_string())?;

    let keywords = stmt
        .query_map(rusqlite::params![limit, offset], keyword_from_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(keywords)
}

#[tauri::command]
pub fn get_keyword(state: State<AppState>, id: i64) -> Result<Option<Keyword>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

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
    let db = state.db.lock().map_err(|e| e.to_string())?;
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

    let neighbors = stmt
        .query_map(rusqlite::params![id, limit], |row| {
            Ok(KeywordNeighbor {
                id: row.get(0)?,
                name: row.get(1)?,
                cooccurrence: row.get(2)?,
                embedding_similarity: row.get(3)?,
                combined_weight: row.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(neighbors)
}

/// Get categories associated with a keyword
#[tauri::command]
pub fn get_keyword_categories(
    state: State<AppState>,
    id: i64,
) -> Result<Vec<KeywordCategory>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT s.id, s.name, s.icon, s.color, ims.weight, ims.article_count
            FROM immanentize_sephiroth ims
            JOIN sephiroth s ON s.id = ims.sephiroth_id
            WHERE ims.immanentize_id = ?
            ORDER BY ims.weight DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let categories = stmt
        .query_map([id], |row| {
            Ok(KeywordCategory {
                sephiroth_id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                color: row.get(3)?,
                weight: row.get(4)?,
                article_count: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(categories)
}

#[tauri::command]
pub fn get_category_keywords(
    state: State<AppState>,
    sephiroth_id: i64,
    limit: Option<i64>,
) -> Result<Vec<Keyword>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(50);

    let mut stmt = db
        .conn()
        .prepare(
            "SELECT i.id, i.name, i.count, i.article_count, i.cluster_id, i.is_canonical, i.canonical_id, i.first_seen, i.last_used, i.quality_score \
             FROM immanentize_sephiroth ims \
             JOIN immanentize i ON i.id = ims.immanentize_id \
             WHERE ims.sephiroth_id = ? \
             ORDER BY ims.weight DESC, ims.article_count DESC \
             LIMIT ?",
        )
        .map_err(|e| e.to_string())?;

    let keywords = stmt
        .query_map(rusqlite::params![sephiroth_id, limit], keyword_from_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(keywords)
}

/// Get trending keywords (most growth in last N days)
#[tauri::command]
pub fn get_trending_keywords(
    state: State<AppState>,
    days: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<TrendingKeyword>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let days = days.unwrap_or(7);
    let limit = limit.unwrap_or(20);

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                i.id,
                i.name,
                i.article_count as total_count,
                (SELECT COUNT(DISTINCT fi.fnord_id)
                 FROM fnord_immanentize fi
                 JOIN fnords f ON f.id = fi.fnord_id
                 WHERE fi.immanentize_id = i.id
                 AND f.published_at > datetime('now', '-' || ?1 || ' days')) as recent_count
            FROM immanentize i
            WHERE i.article_count > 2
            ORDER BY recent_count DESC
            LIMIT ?2
            "#,
        )
        .map_err(|e| e.to_string())?;

    let keywords = stmt
        .query_map(rusqlite::params![days, limit], |row| {
            let total: i64 = row.get::<_, Option<i64>>(2)?.unwrap_or(0);
            let recent: i64 = row.get::<_, Option<i64>>(3)?.unwrap_or(0);
            let older = total - recent;
            let growth = if older > 0 {
                recent as f64 / older as f64
            } else if recent > 0 {
                recent as f64
            } else {
                0.0
            };

            Ok(TrendingKeyword {
                id: row.get(0)?,
                name: row.get(1)?,
                total_count: total,
                recent_count: recent,
                growth_rate: growth,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(keywords)
}

/// Get network statistics
#[tauri::command]
pub fn get_network_stats(state: State<AppState>) -> Result<NetworkStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

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
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(20);
    let search_pattern = format!("%{}%", query.to_lowercase());

    let sql = format!(
        "SELECT {} FROM immanentize WHERE LOWER(name) LIKE ?1 ORDER BY article_count DESC, count DESC LIMIT ?2",
        KEYWORD_SELECT_COLUMNS
    );

    let mut stmt = db.conn().prepare(&sql).map_err(|e| e.to_string())?;

    let keywords = stmt
        .query_map(rusqlite::params![search_pattern, limit], keyword_from_row)
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
    let db = state.db.lock().map_err(|e| e.to_string())?;
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
) -> Result<NetworkGraph, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(100);
    let min_weight = min_weight.unwrap_or(0.1);

    // Get top keywords as nodes
    let mut node_stmt = db
        .conn()
        .prepare(
            r#"
            SELECT id, name, count, article_count, cluster_id
            FROM immanentize
            WHERE (is_canonical = TRUE OR is_canonical IS NULL)
            AND article_count > 0
            ORDER BY article_count DESC
            LIMIT ?
            "#,
        )
        .map_err(|e| e.to_string())?;

    let nodes: Vec<GraphNode> = node_stmt
        .query_map([limit], |row| {
            Ok(GraphNode {
                id: row.get(0)?,
                name: row.get(1)?,
                count: row.get(2)?,
                article_count: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                cluster_id: row.get(4)?,
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
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let days = days.unwrap_or(30);

    if ids.is_empty() {
        return Ok(TrendComparison {
            keywords: vec![],
            dates: vec![],
        });
    }

    // Generate all dates in the range
    let mut date_stmt = db
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
        .map_err(|e| e.to_string())?;

    let dates: Vec<String> = date_stmt
        .query_map([days], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Get trend data for each keyword
    let mut keywords: Vec<KeywordTrendData> = Vec::new();

    for id in ids {
        // Get keyword name
        let name: String = db
            .conn()
            .query_row("SELECT name FROM immanentize WHERE id = ?", [id], |row| {
                row.get(0)
            })
            .unwrap_or_else(|_| format!("Keyword {}", id));

        // Get daily counts
        let mut daily_counts: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();

        let mut stmt = db
            .conn()
            .prepare(
                r#"
                SELECT date, count
                FROM immanentize_daily
                WHERE immanentize_id = ?1
                AND date >= DATE('now', '-' || ?2 || ' days')
                "#,
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(rusqlite::params![id, days], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|e| e.to_string())?;

        for row in rows {
            if let Ok((date, count)) = row {
                daily_counts.insert(date, count);
            }
        }

        // Build counts array aligned with dates
        let counts: Vec<i64> = dates
            .iter()
            .map(|d| *daily_counts.get(d).unwrap_or(&0))
            .collect();

        keywords.push(KeywordTrendData { id, name, counts });
    }

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
    let db = state.db.lock().map_err(|e| e.to_string())?;
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
    let db = state.db.lock().map_err(|e| e.to_string())?;
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

    let removed_orphan_relations: i64 = conn
        .query_row(
            r#"SELECT COUNT(*) FROM immanentize_neighbors 
               WHERE immanentize_id_a NOT IN (SELECT id FROM immanentize)
               OR immanentize_id_b NOT IN (SELECT id FROM immanentize)"#,
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    conn.execute(
        r#"DELETE FROM immanentize_neighbors 
           WHERE immanentize_id_a NOT IN (SELECT id FROM immanentize)
           OR immanentize_id_b NOT IN (SELECT id FROM immanentize)"#,
        [],
    )
    .ok();

    conn.execute(
        r#"DELETE FROM immanentize_sephiroth 
           WHERE immanentize_id NOT IN (SELECT id FROM immanentize)"#,
        [],
    )
    .ok();

    conn.execute(
        r#"DELETE FROM immanentize_daily 
           WHERE immanentize_id NOT IN (SELECT id FROM immanentize)"#,
        [],
    )
    .ok();

    Ok(PruneResult {
        removed_keywords,
        removed_orphan_relations,
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
    let db = state.db.lock().map_err(|e| e.to_string())?;
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
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.conn();

    let keywords: Vec<(i64, String)> = conn
        .prepare("SELECT id, name FROM immanentize")
        .map_err(|e| e.to_string())?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut merged_count = 0i64;
    let mut affected_articles = 0i64;

    for (id, name) in &keywords {
        if let Some(canonical) = find_canonical_keyword(name) {
            if canonical.to_lowercase() != name.to_lowercase() {
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

                        conn.execute(
                            r#"UPDATE OR IGNORE fnord_immanentize 
                               SET immanentize_id = ?1 
                               WHERE immanentize_id = ?2"#,
                            rusqlite::params![can_id, id],
                        )
                        .ok();

                        conn.execute(
                            "DELETE FROM fnord_immanentize WHERE immanentize_id = ?",
                            [id],
                        )
                        .ok();

                        conn.execute(
                            r#"UPDATE immanentize SET 
                               count = count + (SELECT COALESCE(count, 0) FROM immanentize WHERE id = ?2),
                               article_count = (SELECT COUNT(DISTINCT fnord_id) FROM fnord_immanentize WHERE immanentize_id = ?1)
                               WHERE id = ?1"#,
                            rusqlite::params![can_id, id],
                        )
                        .ok();

                        conn.execute("DELETE FROM immanentize WHERE id = ?", [id])
                            .ok();
                        // Also remove from vec_immanentize (sqlite-vec)
                        conn.execute("DELETE FROM vec_immanentize WHERE immanentize_id = ?", [id])
                            .ok();

                        merged_count += 1;
                        affected_articles += moved;
                    }
                }
            }
        }
    }

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
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.conn();

    let keywords: Vec<(i64, String)> = conn
        .prepare("SELECT id, name FROM immanentize")
        .map_err(|e| e.to_string())?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut garbage_ids: Vec<i64> = Vec::new();

    for (id, name) in keywords {
        if normalize_keyword(&name).is_none() {
            garbage_ids.push(id);
        }
    }

    let removed_garbage = garbage_ids.len() as i64;

    for id in &garbage_ids {
        conn.execute(
            "DELETE FROM fnord_immanentize WHERE immanentize_id = ?",
            [id],
        )
        .ok();
        conn.execute(
            "DELETE FROM immanentize_neighbors WHERE immanentize_id_a = ? OR immanentize_id_b = ?",
            rusqlite::params![id, id],
        )
        .ok();
        conn.execute(
            "DELETE FROM immanentize_sephiroth WHERE immanentize_id = ?",
            [id],
        )
        .ok();
        conn.execute(
            "DELETE FROM immanentize_daily WHERE immanentize_id = ?",
            [id],
        )
        .ok();
        conn.execute("DELETE FROM immanentize WHERE id = ?", [id])
            .ok();
        // Also remove from vec_immanentize (sqlite-vec)
        conn.execute("DELETE FROM vec_immanentize WHERE immanentize_id = ?", [id])
            .ok();
    }

    let removed_relations = conn
        .query_row("SELECT changes()", [], |row| row.get::<_, i64>(0))
        .unwrap_or(0);

    Ok(CleanupResult {
        removed_garbage,
        removed_relations,
    })
}

// ============================================================
// QUALITY SCORE SYSTEM
// ============================================================

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
    let normalized = (raw_score / 1.5).min(1.0).max(0.0);

    Ok(normalized)
}

#[tauri::command]
pub fn calculate_keyword_quality_scores(
    state: State<AppState>,
    limit: Option<i64>,
) -> Result<QualityScoreResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.conn();
    let limit = limit.unwrap_or(1000);

    let keyword_ids: Vec<i64> = conn
        .prepare(
            r#"SELECT id FROM immanentize 
               WHERE quality_calculated_at IS NULL 
               OR quality_calculated_at < datetime('now', '-1 day')
               ORDER BY article_count DESC
               LIMIT ?"#,
        )
        .map_err(|e| e.to_string())?
        .query_map([limit], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut updated_count = 0i64;
    let mut total_score = 0.0;
    let mut low_quality_count = 0i64;
    let mut high_quality_count = 0i64;

    for id in &keyword_ids {
        if let Ok(score) = calculate_single_keyword_quality(conn, *id) {
            conn.execute(
                r#"UPDATE immanentize 
                   SET quality_score = ?1, quality_calculated_at = datetime('now')
                   WHERE id = ?2"#,
                params![score, id],
            )
            .ok();

            updated_count += 1;
            total_score += score;

            if score < 0.3 {
                low_quality_count += 1;
            } else if score >= 0.7 {
                high_quality_count += 1;
            }
        }
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
    let db = state.db.lock().map_err(|e| e.to_string())?;
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
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.conn();
    let threshold = quality_threshold.unwrap_or(0.2);
    let min_age = min_age_days.unwrap_or(7);
    let dry_run = dry_run.unwrap_or(true);

    let candidates: Vec<(i64, String)> = conn
        .prepare(
            r#"SELECT id, name FROM immanentize 
               WHERE quality_score IS NOT NULL 
               AND quality_score < ?1
               AND first_seen < datetime('now', '-' || ?2 || ' days')
               AND article_count <= 1"#,
        )
        .map_err(|e| e.to_string())?
        .query_map(params![threshold, min_age], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let pruned_keywords: Vec<String> = candidates.iter().map(|(_, name)| name.clone()).collect();
    let pruned_count = candidates.len() as i64;

    if !dry_run {
        for (id, _) in &candidates {
            conn.execute(
                "DELETE FROM fnord_immanentize WHERE immanentize_id = ?",
                [id],
            )
            .ok();
            conn.execute(
                "DELETE FROM immanentize_neighbors WHERE immanentize_id_a = ? OR immanentize_id_b = ?",
                params![id, id],
            )
            .ok();
            conn.execute(
                "DELETE FROM immanentize_sephiroth WHERE immanentize_id = ?",
                [id],
            )
            .ok();
            conn.execute(
                "DELETE FROM immanentize_daily WHERE immanentize_id = ?",
                [id],
            )
            .ok();
            conn.execute("DELETE FROM immanentize WHERE id = ?", [id])
                .ok();
            // Also remove from vec_immanentize (sqlite-vec)
            conn.execute("DELETE FROM vec_immanentize WHERE immanentize_id = ?", [id])
                .ok();
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
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.conn();
    let threshold = threshold.unwrap_or(0.7);
    let limit = limit.unwrap_or(20);

    let target_embedding: Option<Vec<u8>> = conn
        .query_row(
            "SELECT embedding FROM immanentize WHERE id = ?",
            [keyword_id],
            |row| row.get(0),
        )
        .ok()
        .flatten();

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

    let mut similar: Vec<SimilarKeyword> = all_keywords
        .into_iter()
        .filter_map(|(id, name, blob, article_count)| {
            let embedding = blob_to_embedding(&blob);
            let similarity = cosine_similarity(&target_embedding, &embedding);
            if similarity >= threshold {
                Some(SimilarKeyword {
                    id,
                    name,
                    similarity,
                    article_count,
                })
            } else {
                None
            }
        })
        .collect();

    similar.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
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
    let db = state.db.lock().map_err(|e| e.to_string())?;
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

            // Get neighbor name
            let neighbor_name: String = conn
                .query_row(
                    "SELECT name FROM immanentize WHERE id = ?",
                    [neighbor_id],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| format!("Keyword {}", neighbor_id));

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
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.conn();

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
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "DELETE FROM fnord_immanentize WHERE immanentize_id = ?",
        [remove_id],
    )
    .ok();

    conn.execute(
        r#"UPDATE immanentize SET 
           article_count = (SELECT COUNT(DISTINCT fnord_id) FROM fnord_immanentize WHERE immanentize_id = ?1)
           WHERE id = ?1"#,
        [keep_id],
    )
    .ok();

    conn.execute(
        "DELETE FROM immanentize_neighbors WHERE immanentize_id_a = ? OR immanentize_id_b = ?",
        params![remove_id, remove_id],
    )
    .ok();

    conn.execute(
        "DELETE FROM immanentize_sephiroth WHERE immanentize_id = ?",
        [remove_id],
    )
    .ok();

    conn.execute(
        "DELETE FROM immanentize_daily WHERE immanentize_id = ?",
        [remove_id],
    )
    .ok();

    conn.execute("DELETE FROM immanentize WHERE id = ?", [remove_id])
        .ok();

    // Also remove from vec_immanentize (sqlite-vec)
    conn.execute("DELETE FROM vec_immanentize WHERE immanentize_id = ?", [remove_id])
        .ok();

    Ok(MergeSynonymsResult {
        merged_pairs: 1,
        affected_articles: affected,
    })
}

#[tauri::command]
pub fn dismiss_synonym_pair(
    state: State<AppState>,
    keyword_a_id: i64,
    keyword_b_id: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
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
    let db = state.db.lock().map_err(|e| e.to_string())?;
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
