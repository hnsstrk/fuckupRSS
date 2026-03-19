//! Story Clustering: Groups related articles by topic for perspective comparison

use crate::ai_provider::{AiTextProvider, TaskType};
use crate::commands::ai::helpers::create_text_provider;
use crate::error::CmdResult;
use crate::AppState;
use log::info;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tauri::State;

// ============================================================
// TYPES
// ============================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoryCluster {
    pub id: i64,
    pub title: String,
    pub summary: Option<String>,
    pub perspective_comparison: Option<String>,
    pub article_count: i64,
    pub source_names: Vec<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoryClusterArticle {
    pub fnord_id: i64,
    pub title: String,
    pub summary: Option<String>,
    pub political_bias: Option<i32>,
    pub sachlichkeit: Option<i32>,
    pub source_name: String,
    pub published_at: Option<String>,
    pub similarity_score: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoryClusterDetail {
    pub cluster: StoryCluster,
    pub articles: Vec<StoryClusterArticle>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscoverResult {
    pub clusters_found: usize,
    pub clusters: Vec<StoryCluster>,
}

// ============================================================
// HELPER: Build clusters from similarity graph via Union-Find
// ============================================================

/// Simple Union-Find for clustering
struct UnionFind {
    parent: HashMap<i64, i64>,
    rank: HashMap<i64, usize>,
}

impl UnionFind {
    fn new() -> Self {
        UnionFind {
            parent: HashMap::new(),
            rank: HashMap::new(),
        }
    }

    fn make_set(&mut self, x: i64) {
        self.parent.entry(x).or_insert(x);
        self.rank.entry(x).or_insert(0);
    }

    fn find(&mut self, x: i64) -> i64 {
        let p = *self.parent.get(&x).unwrap_or(&x);
        if p != x {
            let root = self.find(p);
            self.parent.insert(x, root);
            root
        } else {
            x
        }
    }

    fn union(&mut self, x: i64, y: i64) {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry {
            return;
        }
        let rank_x = *self.rank.get(&rx).unwrap_or(&0);
        let rank_y = *self.rank.get(&ry).unwrap_or(&0);
        if rank_x < rank_y {
            self.parent.insert(rx, ry);
        } else if rank_x > rank_y {
            self.parent.insert(ry, rx);
        } else {
            self.parent.insert(ry, rx);
            self.rank.insert(rx, rank_x + 1);
        }
    }
}

/// Similarity threshold for clustering (cosine distance based)
const SIMILARITY_THRESHOLD: f64 = 0.78;

/// Bias label for perspective comparison prompt
fn bias_label(bias: Option<i32>) -> &'static str {
    match bias {
        Some(-2) => "stark links",
        Some(-1) => "leicht links",
        Some(0) => "neutral",
        Some(1) => "leicht rechts",
        Some(2) => "stark rechts",
        _ => "unbekannt",
    }
}

// ============================================================
// COMMANDS
// ============================================================

/// Discover story clusters by finding groups of similar articles
#[tauri::command]
pub async fn discover_story_clusters(
    state: State<'_, AppState>,
    min_articles: Option<i32>,
    days: Option<i32>,
) -> CmdResult<DiscoverResult> {
    let min_articles = min_articles.unwrap_or(3) as usize;
    let days = days.unwrap_or(7);

    info!(
        "Discovering story clusters (min_articles={}, days={})",
        min_articles, days
    );

    // Step 1: Load articles with embeddings from last N days
    let articles: Vec<(i64, i64, Vec<u8>)> = {
        let db = state.db_conn()?;
        let mut stmt = db.conn().prepare(
            r#"SELECT f.id, f.pentacle_id, f.embedding
               FROM fnords f
               WHERE f.embedding IS NOT NULL
               AND f.published_at >= datetime('now', ?1)
               ORDER BY f.published_at DESC"#,
        )?;
        let days_param = format!("-{} days", days);
        let result: Vec<(i64, i64, Vec<u8>)> = stmt
            .query_map(params![days_param], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();
        result
    };

    if articles.is_empty() {
        return Ok(DiscoverResult {
            clusters_found: 0,
            clusters: vec![],
        });
    }

    info!("Found {} articles with embeddings", articles.len());

    // Step 2: For each article, find similar via vec_fnords
    // Build similarity edges using Union-Find
    let mut uf = UnionFind::new();
    let article_pentacles: HashMap<i64, i64> =
        articles.iter().map(|(id, pid, _)| (*id, *pid)).collect();
    let article_ids: HashSet<i64> = articles.iter().map(|(id, _, _)| *id).collect();

    // Store similarity scores for cluster articles
    let mut similarity_pairs: Vec<(i64, i64, f64)> = Vec::new();

    for (fnord_id, _, embedding) in &articles {
        uf.make_set(*fnord_id);

        let neighbors: Vec<(i64, f64)> = {
            let db = state.db_conn()?;
            let mut stmt = db.conn().prepare(
                r#"SELECT v.fnord_id, v.distance
                   FROM vec_fnords v
                   WHERE v.embedding MATCH ?1
                   AND k = 50
                   AND v.fnord_id != ?2
                   ORDER BY v.distance ASC"#,
            )?;
            let result: Vec<(i64, f64)> = stmt
                .query_map(params![embedding, fnord_id], |row| {
                    let distance: f64 = row.get(1)?;
                    let similarity = 1.0 - (distance / 2.0);
                    Ok((row.get::<_, i64>(0)?, similarity))
                })?
                .filter_map(|r| r.ok())
                .filter(|(nid, sim)| *sim >= SIMILARITY_THRESHOLD && article_ids.contains(nid))
                .collect();
            result
        };

        for (neighbor_id, sim) in &neighbors {
            uf.make_set(*neighbor_id);
            uf.union(*fnord_id, *neighbor_id);
            similarity_pairs.push((*fnord_id, *neighbor_id, *sim));
        }
    }

    // Step 3: Group by cluster root
    let mut cluster_groups: HashMap<i64, Vec<i64>> = HashMap::new();
    for &aid in &article_ids {
        let root = uf.find(aid);
        cluster_groups.entry(root).or_default().push(aid);
    }

    // Step 4: Filter clusters by min_articles and >= 2 sources
    let valid_clusters: Vec<Vec<i64>> = cluster_groups
        .into_values()
        .filter(|members| {
            if members.len() < min_articles {
                return false;
            }
            let source_count = members
                .iter()
                .filter_map(|id| article_pentacles.get(id))
                .collect::<HashSet<_>>()
                .len();
            source_count >= 2
        })
        .collect();

    info!(
        "Found {} valid clusters (>= {} articles, >= 2 sources)",
        valid_clusters.len(),
        min_articles
    );

    // Build similarity lookup
    let mut sim_lookup: HashMap<(i64, i64), f64> = HashMap::new();
    for (a, b, sim) in &similarity_pairs {
        let key = if a < b { (*a, *b) } else { (*b, *a) };
        sim_lookup
            .entry(key)
            .and_modify(|s| {
                if *sim > *s {
                    *s = *sim
                }
            })
            .or_insert(*sim);
    }

    // Step 5: Delete old clusters (older than 30 days)
    {
        let db = state.db_conn()?;
        db.conn().execute(
            "DELETE FROM story_clusters WHERE created_at < datetime('now', '-30 days')",
            [],
        )?;
    }

    // Step 6: Save clusters
    let mut result_clusters: Vec<StoryCluster> = Vec::new();

    for members in &valid_clusters {
        // Generate title from common keywords
        let title = {
            let db = state.db_conn()?;
            generate_cluster_title(db.conn(), members)?
        };

        // Insert cluster
        let cluster_id: i64 = {
            let db = state.db_conn()?;
            db.conn().execute(
                r#"INSERT INTO story_clusters (title, article_count, created_at, updated_at)
                   VALUES (?1, ?2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"#,
                params![title, members.len() as i64],
            )?;
            db.conn().last_insert_rowid()
        };

        // Insert cluster articles with similarity scores
        {
            let db = state.db_conn()?;
            db.conn().execute("BEGIN", [])?;
            for member_id in members {
                // Find best similarity score for this member to any other
                let best_sim = members
                    .iter()
                    .filter(|other| *other != member_id)
                    .map(|other| {
                        let key = if member_id < other {
                            (*member_id, *other)
                        } else {
                            (*other, *member_id)
                        };
                        sim_lookup.get(&key).copied().unwrap_or(0.0)
                    })
                    .fold(0.0_f64, f64::max);

                db.conn().execute(
                    r#"INSERT OR IGNORE INTO story_cluster_articles
                       (cluster_id, fnord_id, similarity_score)
                       VALUES (?1, ?2, ?3)"#,
                    params![cluster_id, member_id, best_sim],
                )?;
            }
            db.conn().execute("COMMIT", [])?;
        }

        // Load source names for result
        let source_names: Vec<String> = {
            let db = state.db_conn()?;
            let mut stmt = db.conn().prepare(
                r#"SELECT DISTINCT p.title
                   FROM story_cluster_articles sca
                   JOIN fnords f ON f.id = sca.fnord_id
                   JOIN pentacles p ON p.id = f.pentacle_id
                   WHERE sca.cluster_id = ?1
                   ORDER BY p.title"#,
            )?;
            let result: Vec<String> = stmt
                .query_map(params![cluster_id], |row| row.get::<_, String>(0))?
                .filter_map(|r| r.ok())
                .collect();
            result
        };

        result_clusters.push(StoryCluster {
            id: cluster_id,
            title,
            summary: None,
            perspective_comparison: None,
            article_count: members.len() as i64,
            source_names,
            created_at: None,
            updated_at: None,
        });
    }

    info!("Created {} story clusters", result_clusters.len());

    Ok(DiscoverResult {
        clusters_found: result_clusters.len(),
        clusters: result_clusters,
    })
}

/// Get all story clusters sorted by updated_at DESC
#[tauri::command]
pub fn get_story_clusters(
    state: State<AppState>,
    limit: Option<i32>,
) -> CmdResult<Vec<StoryCluster>> {
    let limit = limit.unwrap_or(50);
    let db = state.db_conn()?;

    let mut stmt = db.conn().prepare(
        r#"SELECT id, title, summary, perspective_comparison,
                  article_count, created_at, updated_at
           FROM story_clusters
           ORDER BY updated_at DESC
           LIMIT ?1"#,
    )?;

    let clusters: Vec<StoryCluster> = stmt
        .query_map(params![limit], |row| {
            Ok(StoryCluster {
                id: row.get(0)?,
                title: row.get(1)?,
                summary: row.get(2)?,
                perspective_comparison: row.get(3)?,
                article_count: row.get(4)?,
                source_names: vec![],
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Enrich with source names
    let mut enriched: Vec<StoryCluster> = Vec::with_capacity(clusters.len());
    for mut cluster in clusters {
        let mut stmt = db.conn().prepare(
            r#"SELECT DISTINCT p.title
               FROM story_cluster_articles sca
               JOIN fnords f ON f.id = sca.fnord_id
               JOIN pentacles p ON p.id = f.pentacle_id
               WHERE sca.cluster_id = ?1
               ORDER BY p.title"#,
        )?;
        cluster.source_names = stmt
            .query_map(params![cluster.id], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        enriched.push(cluster);
    }

    Ok(enriched)
}

/// Get detail of a specific story cluster with all articles
#[tauri::command]
pub fn get_story_cluster_detail(
    state: State<AppState>,
    cluster_id: i64,
) -> CmdResult<StoryClusterDetail> {
    let db = state.db_conn()?;

    // Load cluster info
    let cluster = db.conn().query_row(
        r#"SELECT id, title, summary, perspective_comparison,
                  article_count, created_at, updated_at
           FROM story_clusters WHERE id = ?1"#,
        params![cluster_id],
        |row| {
            Ok(StoryCluster {
                id: row.get(0)?,
                title: row.get(1)?,
                summary: row.get(2)?,
                perspective_comparison: row.get(3)?,
                article_count: row.get(4)?,
                source_names: vec![],
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        },
    )?;

    // Load source names
    let mut source_stmt = db.conn().prepare(
        r#"SELECT DISTINCT p.title
           FROM story_cluster_articles sca
           JOIN fnords f ON f.id = sca.fnord_id
           JOIN pentacles p ON p.id = f.pentacle_id
           WHERE sca.cluster_id = ?1
           ORDER BY p.title"#,
    )?;
    let source_names: Vec<String> = source_stmt
        .query_map(params![cluster_id], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();

    let cluster = StoryCluster {
        source_names,
        ..cluster
    };

    // Load articles
    let mut stmt = db.conn().prepare(
        r#"SELECT f.id, f.title, f.summary, f.political_bias,
                  f.sachlichkeit,
                  COALESCE(p.title, 'Unbekannt') as source_name,
                  f.published_at, sca.similarity_score
           FROM story_cluster_articles sca
           JOIN fnords f ON f.id = sca.fnord_id
           LEFT JOIN pentacles p ON p.id = f.pentacle_id
           WHERE sca.cluster_id = ?1
           ORDER BY f.published_at DESC"#,
    )?;

    let articles: Vec<StoryClusterArticle> = stmt
        .query_map(params![cluster_id], |row| {
            Ok(StoryClusterArticle {
                fnord_id: row.get(0)?,
                title: row.get(1)?,
                summary: row.get(2)?,
                political_bias: row.get(3)?,
                sachlichkeit: row.get(4)?,
                source_name: row.get(5)?,
                published_at: row.get(6)?,
                similarity_score: row.get(7)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(StoryClusterDetail { cluster, articles })
}

/// Compare perspectives of articles in a cluster using LLM
#[tauri::command]
pub async fn compare_perspectives(
    state: State<'_, AppState>,
    cluster_id: i64,
) -> CmdResult<String> {
    info!("Comparing perspectives for cluster {}", cluster_id);

    // Load cluster info and articles
    let (cluster_title, articles): (String, Vec<StoryClusterArticle>) = {
        let db = state.db_conn()?;
        let title: String = db.conn().query_row(
            "SELECT title FROM story_clusters WHERE id = ?1",
            params![cluster_id],
            |row| row.get(0),
        )?;

        let mut stmt = db.conn().prepare(
            r#"SELECT f.id, f.title, f.summary, f.political_bias,
                      f.sachlichkeit,
                      COALESCE(p.title, 'Unbekannt') as source_name,
                      f.published_at, sca.similarity_score
               FROM story_cluster_articles sca
               JOIN fnords f ON f.id = sca.fnord_id
               LEFT JOIN pentacles p ON p.id = f.pentacle_id
               WHERE sca.cluster_id = ?1
               ORDER BY f.published_at ASC"#,
        )?;

        let arts: Vec<StoryClusterArticle> = stmt
            .query_map(params![cluster_id], |row| {
                Ok(StoryClusterArticle {
                    fnord_id: row.get(0)?,
                    title: row.get(1)?,
                    summary: row.get(2)?,
                    political_bias: row.get(3)?,
                    sachlichkeit: row.get(4)?,
                    source_name: row.get(5)?,
                    published_at: row.get(6)?,
                    similarity_score: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        (title, arts)
    };

    if articles.is_empty() {
        return Err(crate::error::FuckupError::Validation(
            "Cluster has no articles".to_string(),
        ));
    }

    // Build prompt
    let mut source_parts = String::new();
    for (i, article) in articles.iter().enumerate() {
        let summary_text = article
            .summary
            .as_deref()
            .unwrap_or("Keine Zusammenfassung verfügbar");
        source_parts.push_str(&format!(
            "\nQuelle {} ({}, Bias: {}): {}\n",
            i + 1,
            article.source_name,
            bias_label(article.political_bias),
            summary_text,
        ));
    }

    let prompt = format!(
        r#"System: Du bist ein Medienanalyst. Vergleiche die Berichterstattung verschiedener Quellen über dasselbe Thema.

User: Thema: {}
{}
Analysiere:
1. Welche Fakten berichten alle Quellen übereinstimmend?
2. Welche Aspekte betont welche Quelle besonders?
3. Wo gibt es Widersprüche oder unterschiedliche Bewertungen?
4. Fazit: Wie unterschiedlich ist die Berichterstattung?

Antworte auf Deutsch, strukturiert mit Überschriften."#,
        cluster_title, source_parts
    );

    // Create text provider and generate
    let (provider, model): (Arc<dyn AiTextProvider>, String) = {
        let db = state.db_conn()?;
        create_text_provider(&db, Some(&state.proxy_manager), TaskType::Reasoning)
    };

    let result = provider
        .generate_text(&model, &prompt, None)
        .await
        .map_err(|e| crate::error::FuckupError::Generic(format!("LLM generation failed: {}", e)))?;

    let comparison_text = result.text;

    // Save to database
    {
        let db = state.db_conn()?;
        db.conn().execute(
            r#"UPDATE story_clusters
               SET perspective_comparison = ?1,
                   updated_at = CURRENT_TIMESTAMP
               WHERE id = ?2"#,
            params![comparison_text, cluster_id],
        )?;
    }

    info!(
        "Perspective comparison generated for cluster {}",
        cluster_id
    );

    Ok(comparison_text)
}

/// Delete a story cluster
#[tauri::command]
pub fn delete_story_cluster(state: State<AppState>, cluster_id: i64) -> CmdResult<()> {
    let db = state.db_conn()?;
    db.conn().execute(
        "DELETE FROM story_clusters WHERE id = ?1",
        params![cluster_id],
    )?;
    info!("Deleted story cluster {}", cluster_id);
    Ok(())
}

// ============================================================
// HELPER FUNCTIONS
// ============================================================

/// Generate a cluster title from the common keywords of its articles
fn generate_cluster_title(
    conn: &rusqlite::Connection,
    article_ids: &[i64],
) -> Result<String, rusqlite::Error> {
    if article_ids.is_empty() {
        return Ok("Unbekanntes Thema".to_string());
    }

    // Build placeholders for IN clause
    let placeholders: String = article_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");

    let query = format!(
        r#"SELECT i.name, COUNT(DISTINCT fi.fnord_id) as article_count
           FROM immanentize i
           JOIN fnord_immanentize fi ON fi.immanentize_id = i.id
           WHERE fi.fnord_id IN ({})
           GROUP BY i.id
           HAVING article_count >= ?
           ORDER BY article_count DESC, i.article_count DESC
           LIMIT 4"#,
        placeholders
    );

    let min_articles = (article_ids.len() as f64 * 0.4).ceil() as i64;

    let mut stmt = conn.prepare(&query)?;

    // Build params: article_ids + min_articles threshold
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = article_ids
        .iter()
        .map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>)
        .collect();
    param_values.push(Box::new(min_articles));

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();

    let keywords: Vec<String> = stmt
        .query_map(params_refs.as_slice(), |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();

    if keywords.is_empty() {
        // Fallback: use the title of the first article
        let first_title: String = conn
            .query_row(
                "SELECT title FROM fnords WHERE id = ?1",
                params![article_ids[0]],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "Unbekanntes Thema".to_string());
        // Truncate to reasonable length
        Ok(if first_title.len() > 80 {
            format!("{}...", &first_title[..77])
        } else {
            first_title
        })
    } else {
        Ok(keywords.join(", "))
    }
}
