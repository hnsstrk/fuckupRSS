use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

// ============================================================
// IMMANENTIZE NETWORK API
// ============================================================

/// Keyword with full network data
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
    pub recent_count: i64,  // Last 7 days
    pub growth_rate: f64,   // recent / (total - recent)
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

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT id, name, count, article_count, cluster_id,
                   is_canonical, canonical_id, first_seen, last_used
            FROM immanentize
            WHERE is_canonical = TRUE OR is_canonical IS NULL
            ORDER BY article_count DESC, count DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .map_err(|e| e.to_string())?;

    let keywords = stmt
        .query_map(rusqlite::params![limit, offset], |row| {
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
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(keywords)
}

/// Get a single keyword by ID
#[tauri::command]
pub fn get_keyword(state: State<AppState>, id: i64) -> Result<Option<Keyword>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let keyword = db
        .conn()
        .query_row(
            r#"
            SELECT id, name, count, article_count, cluster_id,
                   is_canonical, canonical_id, first_seen, last_used
            FROM immanentize
            WHERE id = ?
            "#,
            [id],
            |row| {
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
                })
            },
        )
        .ok();

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

/// Get top keywords for a category
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
            r#"
            SELECT i.id, i.name, i.count, i.article_count, i.cluster_id,
                   i.is_canonical, i.canonical_id, i.first_seen, i.last_used
            FROM immanentize_sephiroth ims
            JOIN immanentize i ON i.id = ims.immanentize_id
            WHERE ims.sephiroth_id = ?
            ORDER BY ims.weight DESC, ims.article_count DESC
            LIMIT ?
            "#,
        )
        .map_err(|e| e.to_string())?;

    let keywords = stmt
        .query_map(rusqlite::params![sephiroth_id, limit], |row| {
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
            })
        })
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

/// Search keywords by name (case-insensitive)
#[tauri::command]
pub fn search_keywords(
    state: State<AppState>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<Keyword>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(20);
    let search_pattern = format!("%{}%", query.to_lowercase());

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT id, name, count, article_count, cluster_id,
                   is_canonical, canonical_id, first_seen, last_used
            FROM immanentize
            WHERE LOWER(name) LIKE ?1
            ORDER BY article_count DESC, count DESC
            LIMIT ?2
            "#,
        )
        .map_err(|e| e.to_string())?;

    let keywords = stmt
        .query_map(rusqlite::params![search_pattern, limit], |row| {
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
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(keywords)
}
