//! Recommendation Engine for Operation Mindfuck
//!
//! Generates personalized article recommendations based on:
//! - Embedding similarity to read articles
//! - Keyword overlap
//! - Category preferences
//! - Freshness and source diversity

use crate::AppState;
use log::{debug, info, warn};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tauri::State;
use uuid::Uuid;

// ============================================================
// Data Structures
// ============================================================

/// A recommended article with explanation
#[derive(Debug, Serialize, Deserialize)]
pub struct Recommendation {
    pub fnord_id: i64,
    pub title: String,
    pub summary: Option<String>,
    pub url: String,
    pub image_url: Option<String>,

    pub pentacle_id: i64,
    pub pentacle_title: Option<String>,
    pub pentacle_icon: Option<String>,

    pub published_at: Option<String>,

    pub relevance_score: f64,
    pub freshness_score: f64,

    pub political_bias: Option<i32>,
    pub sachlichkeit: Option<i32>,

    pub categories: Vec<CategoryInfo>,
    pub matching_keywords: Vec<String>,
    pub explanation: String,

    pub is_saved: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoryInfo {
    pub sephiroth_id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
}

/// Saved article info
#[derive(Debug, Serialize, Deserialize)]
pub struct SavedArticle {
    pub fnord_id: i64,
    pub title: String,
    pub pentacle_title: Option<String>,
    pub saved_at: String,
    pub published_at: Option<String>,
}

/// Statistics about the recommendation system
/// Must match frontend type in src/lib/types.ts:RecommendationStats
#[derive(Debug, Serialize, Deserialize)]
pub struct RecommendationStats {
    pub total_saved: i64,
    pub total_hidden: i64,
    pub total_clicks: i64,
    pub articles_read: i64,
    pub articles_with_embedding: i64,
    pub profile_strength: String,
    // Extended stats for diagnostics
    pub top_keywords: Vec<KeywordWeight>,
    pub top_categories: Vec<CategoryWeight>,
    pub candidate_pool_size: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeywordWeight {
    pub name: String,
    pub weight: f64,
    pub article_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryWeight {
    pub id: i64,
    pub name: String,
    pub weight: f64,
}

// ============================================================
// Internal Structures
// ============================================================

struct UserProfile {
    keyword_weights: HashMap<i64, f64>,
    category_weights: HashMap<i64, f64>,
    read_article_ids: HashSet<i64>,
    hidden_article_ids: HashSet<i64>,
    saved_article_ids: HashSet<i64>,
    total_read: i64,
}

#[derive(Clone)]
struct Candidate {
    fnord_id: i64,
    pentacle_id: i64,
    embedding_score: Option<f64>,
    keyword_score: Option<f64>,
    freshness_score: f64,
    final_score: f64,
    matching_keywords: Vec<String>,
    category_ids: HashSet<i64>,
}

// ============================================================
// Commands
// ============================================================

/// Get personalized recommendations
#[tauri::command]
pub fn get_recommendations(
    state: State<AppState>,
    limit: Option<i32>,
) -> Result<Vec<Recommendation>, String> {
    let request_id = Uuid::new_v4().to_string()[..8].to_string();
    let start_time = Instant::now();

    info!("[{}] Starting recommendation generation", request_id);

    let limit = limit.unwrap_or(10).min(50) as usize;
    let db = state.db.lock().map_err(|e| {
        warn!("[{}] Failed to acquire DB lock: {}", request_id, e);
        e.to_string()
    })?;

    let lock_time = start_time.elapsed();
    debug!("[{}] DB lock acquired in {:?}", request_id, lock_time);

    // Build user profile
    let profile_start = Instant::now();
    let profile = build_user_profile(db.conn())?;
    let profile_time = profile_start.elapsed();
    debug!("[{}] Profile built in {:?} (read: {}, saved: {})",
           request_id, profile_time, profile.total_read, profile.saved_article_ids.len());

    // Cold start check
    if profile.total_read == 0 {
        info!("[{}] Cold start: returning popular articles", request_id);
        let result = get_cold_start_recommendations(db.conn(), limit);
        info!("[{}] Cold start completed in {:?}", request_id, start_time.elapsed());
        return result;
    }

    // Generate candidates
    let candidates_start = Instant::now();
    let mut candidates = generate_candidates(db.conn(), &profile, 100)?;
    let candidates_time = candidates_start.elapsed();
    debug!("[{}] Generated {} candidates in {:?}",
           request_id, candidates.len(), candidates_time);

    // Score candidates
    let scoring_start = Instant::now();
    for c in &mut candidates {
        score_candidate(c, &profile);
    }
    let scoring_time = scoring_start.elapsed();
    debug!("[{}] Scored candidates in {:?}", request_id, scoring_time);

    // Sort by score (NaN-safe: treat NaN as equal to avoid panic)
    candidates.sort_by(|a, b| {
        b.final_score
            .partial_cmp(&a.final_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Rerank for diversity
    let rerank_start = Instant::now();
    let reranked = rerank_for_diversity(candidates, limit);
    let rerank_time = rerank_start.elapsed();
    debug!("[{}] Reranked to {} results in {:?}", request_id, reranked.len(), rerank_time);

    // Convert to recommendations
    let build_start = Instant::now();
    let mut recommendations = Vec::new();
    for candidate in reranked {
        let rec = build_recommendation(db.conn(), candidate, &profile)?;
        recommendations.push(rec);
    }
    let build_time = build_start.elapsed();
    debug!("[{}] Built {} recommendations in {:?}", request_id, recommendations.len(), build_time);

    let total_time = start_time.elapsed();
    info!(
        "[{}] Completed: {} recommendations in {:?} (profile: {:?}, candidates: {:?}, scoring: {:?}, build: {:?})",
        request_id,
        recommendations.len(),
        total_time,
        profile_time,
        candidates_time,
        scoring_time,
        build_time
    );

    Ok(recommendations)
}

/// Save an article (positive feedback)
#[tauri::command]
pub fn save_article(state: State<AppState>, fnord_id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            r#"INSERT INTO recommendation_feedback (fnord_id, action)
               VALUES (?1, 'save')
               ON CONFLICT (fnord_id, action) DO NOTHING"#,
            params![fnord_id],
        )
        .map_err(|e| e.to_string())?;

    debug!("User saved article {}", fnord_id);
    Ok(())
}

/// Unsave an article
#[tauri::command]
pub fn unsave_article(state: State<AppState>, fnord_id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "DELETE FROM recommendation_feedback WHERE fnord_id = ?1 AND action = 'save'",
            params![fnord_id],
        )
        .map_err(|e| e.to_string())?;

    debug!("User unsaved article {}", fnord_id);
    Ok(())
}

/// Hide a recommendation (negative feedback)
#[tauri::command]
pub fn hide_recommendation(state: State<AppState>, fnord_id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            r#"INSERT INTO recommendation_feedback (fnord_id, action)
               VALUES (?1, 'hide')
               ON CONFLICT (fnord_id, action) DO NOTHING"#,
            params![fnord_id],
        )
        .map_err(|e| e.to_string())?;

    debug!("User hid article {}", fnord_id);
    Ok(())
}

/// Get all saved articles
#[tauri::command]
pub fn get_saved_articles(
    state: State<AppState>,
    limit: Option<i32>,
) -> Result<Vec<SavedArticle>, String> {
    let limit = limit.unwrap_or(50).min(100);
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"SELECT
                f.id as fnord_id,
                f.title,
                p.title as pentacle_title,
                rf.created_at as saved_at,
                f.published_at
            FROM recommendation_feedback rf
            JOIN fnords f ON f.id = rf.fnord_id
            LEFT JOIN pentacles p ON p.id = f.pentacle_id
            WHERE rf.action = 'save'
            ORDER BY rf.created_at DESC
            LIMIT ?"#,
        )
        .map_err(|e| e.to_string())?;

    let articles: Vec<SavedArticle> = stmt
        .query_map([limit], |row| {
            Ok(SavedArticle {
                fnord_id: row.get(0)?,
                title: row.get(1)?,
                pentacle_title: row.get(2)?,
                saved_at: row.get(3)?,
                published_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(articles)
}

/// Get recommendation statistics
#[tauri::command]
pub fn get_recommendation_stats(state: State<AppState>) -> Result<RecommendationStats, String> {
    let request_id = Uuid::new_v4().to_string()[..8].to_string();
    let start_time = Instant::now();
    debug!("[{}] Loading recommendation stats", request_id);

    let db = state.db.lock().map_err(|e| {
        warn!("[{}] Failed to acquire DB lock: {}", request_id, e);
        e.to_string()
    })?;

    let articles_read: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnords WHERE read_at IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_saved: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM recommendation_feedback WHERE action = 'save'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_hidden: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM recommendation_feedback WHERE action = 'hide'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_clicks: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM recommendation_feedback WHERE action = 'click'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_articles: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
        .unwrap_or(0);

    let articles_with_embedding: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnords WHERE embedding IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Calculate profile strength based on read articles
    let profile_strength = if articles_read >= 50 {
        "Hot".to_string()
    } else if articles_read >= 10 {
        "Warm".to_string()
    } else {
        "Cold".to_string()
    };

    let candidate_pool_size = total_articles - articles_read - total_hidden;

    // Top keywords from read articles
    let top_keywords = get_top_user_keywords(db.conn(), 10)?;

    // Top categories
    let top_categories = get_top_user_categories(db.conn(), 5)?;

    let stats = RecommendationStats {
        total_saved,
        total_hidden,
        total_clicks,
        articles_read,
        articles_with_embedding,
        profile_strength: profile_strength.clone(),
        top_keywords,
        top_categories,
        candidate_pool_size,
    };

    debug!(
        "[{}] Stats loaded in {:?} (read: {}, pool: {}, profile: {})",
        request_id,
        start_time.elapsed(),
        articles_read,
        candidate_pool_size,
        profile_strength
    );

    Ok(stats)
}

// ============================================================
// Internal Functions
// ============================================================

fn build_user_profile(conn: &rusqlite::Connection) -> Result<UserProfile, String> {
    // Read articles
    let read_article_ids: HashSet<i64> = {
        let mut stmt = conn
            .prepare("SELECT id FROM fnords WHERE read_at IS NOT NULL")
            .map_err(|e| e.to_string())?;

        let rows: Vec<i64> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        rows.into_iter().collect()
    };

    let total_read = read_article_ids.len() as i64;

    // Saved articles (stronger signal)
    let saved_article_ids: HashSet<i64> = {
        let mut stmt = conn
            .prepare("SELECT fnord_id FROM recommendation_feedback WHERE action = 'save'")
            .map_err(|e| e.to_string())?;

        let rows: Vec<i64> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        rows.into_iter().collect()
    };

    // Hidden articles
    let hidden_article_ids: HashSet<i64> = {
        let mut stmt = conn
            .prepare("SELECT fnord_id FROM recommendation_feedback WHERE action = 'hide'")
            .map_err(|e| e.to_string())?;

        let rows: Vec<i64> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        rows.into_iter().collect()
    };

    // Aggregate keywords from read articles
    let keyword_weights = aggregate_keywords(conn, &read_article_ids, &saved_article_ids)?;

    // Aggregate categories
    let category_weights = aggregate_categories(conn, &read_article_ids)?;

    Ok(UserProfile {
        keyword_weights,
        category_weights,
        read_article_ids,
        hidden_article_ids,
        saved_article_ids,
        total_read,
    })
}

fn aggregate_keywords(
    conn: &rusqlite::Connection,
    _read_ids: &HashSet<i64>,
    saved_ids: &HashSet<i64>,
) -> Result<HashMap<i64, f64>, String> {
    let mut weights: HashMap<i64, f64> = HashMap::new();

    // Get keywords from read articles
    let mut stmt = conn
        .prepare(
            r#"SELECT fi.immanentize_id, COUNT(*) as count, COALESCE(i.quality_score, 0.5) as quality
               FROM fnord_immanentize fi
               JOIN immanentize i ON i.id = fi.immanentize_id
               JOIN fnords f ON f.id = fi.fnord_id
               WHERE f.read_at IS NOT NULL
               GROUP BY fi.immanentize_id
               ORDER BY count DESC
               LIMIT 50"#,
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<(i64, f64, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    for (keyword_id, count, quality) in rows {
        // TF-IDF-like weighting
        let weight = (1.0 + (count as f64).ln()) * quality;
        weights.insert(keyword_id, weight);
    }

    // Boost keywords from saved articles (3x)
    if !saved_ids.is_empty() {
        let saved_list: Vec<String> = saved_ids.iter().map(|id| id.to_string()).collect();
        let placeholders = saved_list.iter().map(|_| "?").collect::<Vec<_>>().join(",");

        let query = format!(
            r#"SELECT DISTINCT fi.immanentize_id
               FROM fnord_immanentize fi
               WHERE fi.fnord_id IN ({})"#,
            placeholders
        );

        let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;

        let saved_keywords: Vec<i64> = stmt
            .query_map(
                rusqlite::params_from_iter(saved_ids.iter()),
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        for kw_id in saved_keywords {
            if let Some(w) = weights.get_mut(&kw_id) {
                *w *= 3.0;
            }
        }
    }

    // Normalize weights
    let max_weight = weights.values().cloned().fold(0.0_f64, f64::max);
    if max_weight > 0.0 {
        for w in weights.values_mut() {
            *w /= max_weight;
        }
    }

    Ok(weights)
}

fn aggregate_categories(
    conn: &rusqlite::Connection,
    _read_ids: &HashSet<i64>,
) -> Result<HashMap<i64, f64>, String> {
    let mut weights: HashMap<i64, f64> = HashMap::new();

    let mut stmt = conn
        .prepare(
            r#"SELECT fs.sephiroth_id, COUNT(*) as count
               FROM fnord_sephiroth fs
               JOIN fnords f ON f.id = fs.fnord_id
               WHERE f.read_at IS NOT NULL
               GROUP BY fs.sephiroth_id
               ORDER BY count DESC"#,
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<(i64, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get::<_, i64>(1)? as f64)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let total: f64 = rows.iter().map(|(_, c)| c).sum();

    for (cat_id, count) in rows {
        weights.insert(cat_id, count / total.max(1.0));
    }

    Ok(weights)
}

fn generate_candidates(
    conn: &rusqlite::Connection,
    profile: &UserProfile,
    limit: usize,
) -> Result<Vec<Candidate>, String> {
    let mut candidates: HashMap<i64, Candidate> = HashMap::new();

    // Source 1: Embedding similarity (if user has read articles with embeddings)
    let embedding_candidates = get_embedding_candidates(conn, profile, 50)?;
    for c in embedding_candidates {
        candidates.insert(c.fnord_id, c);
    }

    // Source 2: Keyword overlap
    let keyword_candidates = get_keyword_candidates(conn, profile, 50)?;
    for c in keyword_candidates {
        candidates
            .entry(c.fnord_id)
            .and_modify(|existing| {
                existing.keyword_score = c.keyword_score;
                existing.matching_keywords = c.matching_keywords.clone();
            })
            .or_insert(c);
    }

    // Source 3: Recent popular (fallback)
    let popular_candidates = get_popular_candidates(conn, 30)?;
    for c in popular_candidates {
        candidates.entry(c.fnord_id).or_insert(c);
    }

    // Filter out read and hidden
    let filtered: Vec<Candidate> = candidates
        .into_values()
        .filter(|c| !profile.read_article_ids.contains(&c.fnord_id))
        .filter(|c| !profile.hidden_article_ids.contains(&c.fnord_id))
        .take(limit)
        .collect();

    Ok(filtered)
}

fn get_embedding_candidates(
    conn: &rusqlite::Connection,
    profile: &UserProfile,
    limit: usize,
) -> Result<Vec<Candidate>, String> {
    // Get centroid of read articles with embeddings
    let read_with_embeddings: Vec<i64> = {
        let read_list: Vec<String> = profile
            .read_article_ids
            .iter()
            .map(|id| id.to_string())
            .collect();

        if read_list.is_empty() {
            return Ok(vec![]);
        }

        let placeholders = read_list.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT id FROM fnords WHERE id IN ({}) AND embedding IS NOT NULL LIMIT 10",
            placeholders
        );

        let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
        let rows: Vec<i64> = stmt
            .query_map(
                rusqlite::params_from_iter(profile.read_article_ids.iter()),
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        rows
    };

    if read_with_embeddings.is_empty() {
        return Ok(vec![]);
    }

    // Use first read article as seed for similarity search
    let seed_id = read_with_embeddings[0];

    let seed_embedding: Option<Vec<u8>> = conn
        .query_row(
            "SELECT embedding FROM fnords WHERE id = ?",
            [seed_id],
            |row| row.get(0),
        )
        .ok();

    let embedding = match seed_embedding {
        Some(e) if !e.is_empty() => e,
        _ => return Ok(vec![]),
    };

    // Find similar articles
    let mut stmt = conn
        .prepare(
            r#"SELECT
                v.fnord_id,
                v.distance,
                f.pentacle_id,
                f.published_at
            FROM vec_fnords v
            JOIN fnords f ON f.id = v.fnord_id
            WHERE v.embedding MATCH ?1
            AND k = ?2
            AND f.read_at IS NULL
            ORDER BY v.distance ASC"#,
        )
        .map_err(|e| e.to_string())?;

    let mut candidates = Vec::new();

    let rows: Vec<(i64, f64, i64, Option<String>)> = stmt
        .query_map(params![embedding, limit as i64 * 2], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    for (fnord_id, distance, pentacle_id, published_at) in rows {
        let similarity = 1.0 - (distance / 2.0);
        if similarity < 0.4 {
            continue;
        }

        let freshness = calculate_freshness(&published_at);

        candidates.push(Candidate {
            fnord_id,
            pentacle_id,
            embedding_score: Some(similarity),
            keyword_score: None,
            freshness_score: freshness,
            final_score: 0.0,
            matching_keywords: vec![],
            category_ids: HashSet::new(),
        });
    }

    candidates.truncate(limit);
    Ok(candidates)
}

fn get_keyword_candidates(
    conn: &rusqlite::Connection,
    profile: &UserProfile,
    limit: usize,
) -> Result<Vec<Candidate>, String> {
    // Get top user keywords
    let user_keywords: Vec<i64> = profile
        .keyword_weights
        .iter()
        .filter(|(_, w)| **w > 0.3)
        .map(|(id, _)| *id)
        .take(20)
        .collect();

    if user_keywords.is_empty() {
        return Ok(vec![]);
    }

    let placeholders = user_keywords
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");

    let query = format!(
        r#"SELECT
            f.id as fnord_id,
            f.pentacle_id,
            f.published_at,
            COUNT(DISTINCT fi.immanentize_id) as overlap_count,
            GROUP_CONCAT(i.name) as matching_keywords
        FROM fnords f
        JOIN fnord_immanentize fi ON f.id = fi.fnord_id
        JOIN immanentize i ON i.id = fi.immanentize_id
        WHERE fi.immanentize_id IN ({})
          AND f.read_at IS NULL
          AND f.embedding IS NOT NULL
        GROUP BY f.id
        HAVING overlap_count >= 2
        ORDER BY overlap_count DESC
        LIMIT ?"#,
        placeholders
    );

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;

    let mut params: Vec<Box<dyn rusqlite::ToSql>> = user_keywords
        .iter()
        .map(|id| Box::new(*id) as Box<dyn rusqlite::ToSql>)
        .collect();
    params.push(Box::new(limit as i64 * 2));

    let mut candidates = Vec::new();

    let rows: Vec<(i64, i64, Option<String>, i64, String)> = stmt
        .query_map(rusqlite::params_from_iter(params.iter().map(|b| &**b)), |row| {
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

    for (fnord_id, pentacle_id, published_at, overlap, matching) in rows {
        let user_kw_count = user_keywords.len() as f64;
        let overlap_score = overlap as f64 / (user_kw_count + 5.0);
        let freshness = calculate_freshness(&published_at);

        let matching_keywords: Vec<String> = matching
            .split(',')
            .take(5)
            .map(|s| s.trim().to_string())
            .collect();

        candidates.push(Candidate {
            fnord_id,
            pentacle_id,
            embedding_score: None,
            keyword_score: Some(overlap_score),
            freshness_score: freshness,
            final_score: 0.0,
            matching_keywords,
            category_ids: HashSet::new(),
        });
    }

    candidates.truncate(limit);
    Ok(candidates)
}

fn get_popular_candidates(conn: &rusqlite::Connection, limit: usize) -> Result<Vec<Candidate>, String> {
    let mut stmt = conn
        .prepare(
            r#"SELECT
                f.id,
                f.pentacle_id,
                f.published_at
            FROM fnords f
            JOIN pentacles p ON p.id = f.pentacle_id
            WHERE f.read_at IS NULL
              AND f.embedding IS NOT NULL
              AND f.published_at > datetime('now', '-48 hours')
            ORDER BY p.article_count DESC, f.published_at DESC
            LIMIT ?"#,
        )
        .map_err(|e| e.to_string())?;

    let mut candidates = Vec::new();

    let rows: Vec<(i64, i64, Option<String>)> = stmt
        .query_map([limit as i64], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    for (fnord_id, pentacle_id, published_at) in rows {
        let freshness = calculate_freshness(&published_at);

        candidates.push(Candidate {
            fnord_id,
            pentacle_id,
            embedding_score: None,
            keyword_score: None,
            freshness_score: freshness,
            final_score: 0.0,
            matching_keywords: vec![],
            category_ids: HashSet::new(),
        });
    }

    Ok(candidates)
}

fn score_candidate(candidate: &mut Candidate, profile: &UserProfile) {
    let embedding_score = candidate.embedding_score.unwrap_or(0.3);
    let keyword_score = candidate.keyword_score.unwrap_or(0.0);
    let freshness_score = candidate.freshness_score;

    // Category boost
    let category_boost = if candidate.category_ids.iter().any(|id| profile.category_weights.contains_key(id)) {
        1.1
    } else {
        1.0
    };

    // Weighted combination
    candidate.final_score = (0.40 * embedding_score
        + 0.30 * keyword_score
        + 0.25 * freshness_score
        + 0.05)
        * category_boost;
}

fn rerank_for_diversity(mut candidates: Vec<Candidate>, limit: usize) -> Vec<Candidate> {
    let mut selected: Vec<Candidate> = Vec::new();
    let mut seen_sources: HashSet<i64> = HashSet::new();

    while selected.len() < limit && !candidates.is_empty() {
        // Sort remaining by score (NaN-safe)
        candidates.sort_by(|a, b| {
            b.final_score
                .partial_cmp(&a.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Find best candidate with source diversity
        let mut best_idx = 0;
        for (i, c) in candidates.iter().enumerate() {
            if !seen_sources.contains(&c.pentacle_id) {
                best_idx = i;
                break;
            }
            // Allow same source but with penalty after first 5
            if i == 0 && selected.len() >= 5 {
                break;
            }
        }

        let chosen = candidates.remove(best_idx);
        seen_sources.insert(chosen.pentacle_id);
        selected.push(chosen);
    }

    selected
}

fn build_recommendation(
    conn: &rusqlite::Connection,
    candidate: Candidate,
    profile: &UserProfile,
) -> Result<Recommendation, String> {
    // Get article details
    let (title, summary, url, image_url, pentacle_id, pentacle_title, pentacle_icon, published_at, political_bias, sachlichkeit): (
        String,
        Option<String>,
        String,
        Option<String>,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i32>,
        Option<i32>,
    ) = conn
        .query_row(
            r#"SELECT
                f.title, f.summary, f.url, f.image_url,
                f.pentacle_id, p.title, p.icon_url,
                f.published_at, f.political_bias, f.sachlichkeit
            FROM fnords f
            LEFT JOIN pentacles p ON p.id = f.pentacle_id
            WHERE f.id = ?"#,
            [candidate.fnord_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?;

    // Get categories
    let categories = get_article_categories(conn, candidate.fnord_id)?;

    // Check if saved
    let is_saved = profile.saved_article_ids.contains(&candidate.fnord_id);

    // Generate explanation
    let explanation = generate_explanation(&candidate, &categories, profile);

    Ok(Recommendation {
        fnord_id: candidate.fnord_id,
        title,
        summary,
        url,
        image_url,
        pentacle_id,
        pentacle_title,
        pentacle_icon,
        published_at,
        relevance_score: candidate.final_score,
        freshness_score: candidate.freshness_score,
        political_bias,
        sachlichkeit,
        categories,
        matching_keywords: candidate.matching_keywords,
        explanation,
        is_saved,
    })
}

fn get_article_categories(
    conn: &rusqlite::Connection,
    fnord_id: i64,
) -> Result<Vec<CategoryInfo>, String> {
    let mut stmt = conn
        .prepare(
            r#"SELECT DISTINCT m.id, m.name, m.icon, m.color
               FROM sephiroth m
               JOIN sephiroth s ON (s.parent_id = m.id OR s.id = m.id)
               JOIN fnord_sephiroth fs ON fs.sephiroth_id = s.id
               WHERE fs.fnord_id = ? AND m.level = 0
               ORDER BY m.name"#,
        )
        .map_err(|e| e.to_string())?;

    let categories: Vec<CategoryInfo> = stmt
        .query_map([fnord_id], |row| {
            Ok(CategoryInfo {
                sephiroth_id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                color: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(categories)
}

fn generate_explanation(
    candidate: &Candidate,
    categories: &[CategoryInfo],
    _profile: &UserProfile,
) -> String {
    // Priority 1: Keywords
    if candidate.matching_keywords.len() >= 2 {
        let kw_list = candidate.matching_keywords.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
        return format!("Basierend auf: {}", kw_list);
    }

    // Priority 2: Category
    if let Some(cat) = categories.first() {
        return format!("Aus dem Bereich: {}", cat.name);
    }

    // Priority 3: Embedding similarity
    if candidate.embedding_score.map(|s| s > 0.6).unwrap_or(false) {
        return "Thematisch ähnlich zu deinen Artikeln".to_string();
    }

    // Fallback
    if candidate.freshness_score > 0.8 {
        "Aktuell und relevant".to_string()
    } else {
        "Könnte dich interessieren".to_string()
    }
}

fn calculate_freshness(published_at: &Option<String>) -> f64 {
    match published_at {
        Some(dt) => {
            // Parse datetime and calculate age
            if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(dt) {
                let age_hours = (chrono::Utc::now() - parsed.with_timezone(&chrono::Utc))
                    .num_hours() as f64;
                // Half-life: 48 hours
                let decay = (-age_hours / 69.3).exp();
                decay.clamp(0.0, 1.0)
            } else {
                0.5
            }
        }
        None => 0.3,
    }
}

fn get_cold_start_recommendations(
    conn: &rusqlite::Connection,
    limit: usize,
) -> Result<Vec<Recommendation>, String> {
    let mut stmt = conn
        .prepare(
            r#"SELECT
                f.id, f.title, f.summary, f.url, f.image_url,
                f.pentacle_id, p.title, p.icon_url,
                f.published_at, f.political_bias, f.sachlichkeit
            FROM fnords f
            JOIN pentacles p ON p.id = f.pentacle_id
            WHERE f.summary IS NOT NULL
              AND f.published_at > datetime('now', '-48 hours')
            ORDER BY f.published_at DESC
            LIMIT ?"#,
        )
        .map_err(|e| e.to_string())?;

    let mut recommendations = Vec::new();

    let rows: Vec<_> = stmt
        .query_map([limit as i64], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<i32>>(9)?,
                row.get::<_, Option<i32>>(10)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    for (fnord_id, title, summary, url, image_url, pentacle_id, pentacle_title, pentacle_icon, published_at, political_bias, sachlichkeit) in rows {
        let categories = get_article_categories(conn, fnord_id)?;
        let freshness = calculate_freshness(&published_at);

        recommendations.push(Recommendation {
            fnord_id,
            title,
            summary,
            url,
            image_url,
            pentacle_id,
            pentacle_title,
            pentacle_icon,
            published_at,
            relevance_score: 0.5,
            freshness_score: freshness,
            political_bias,
            sachlichkeit,
            categories,
            matching_keywords: vec![],
            explanation: "Aktuelle Nachrichten".to_string(),
            is_saved: false,
        });
    }

    Ok(recommendations)
}

fn get_top_user_keywords(conn: &rusqlite::Connection, limit: i64) -> Result<Vec<KeywordWeight>, String> {
    let mut stmt = conn
        .prepare(
            r#"SELECT i.name, COUNT(*) as count, i.article_count
               FROM fnord_immanentize fi
               JOIN immanentize i ON i.id = fi.immanentize_id
               JOIN fnords f ON f.id = fi.fnord_id
               WHERE f.read_at IS NOT NULL
               GROUP BY fi.immanentize_id
               ORDER BY count DESC
               LIMIT ?"#,
        )
        .map_err(|e| e.to_string())?;

    let keywords: Vec<KeywordWeight> = stmt
        .query_map([limit], |row| {
            let count: f64 = row.get::<_, i64>(1)? as f64;
            Ok(KeywordWeight {
                name: row.get(0)?,
                weight: count,
                article_count: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(keywords)
}

fn get_top_user_categories(conn: &rusqlite::Connection, limit: i64) -> Result<Vec<CategoryWeight>, String> {
    let mut stmt = conn
        .prepare(
            r#"SELECT m.id, m.name, COUNT(*) as count
               FROM fnord_sephiroth fs
               JOIN sephiroth s ON s.id = fs.sephiroth_id
               JOIN sephiroth m ON (m.id = s.parent_id OR m.id = s.id) AND m.level = 0
               JOIN fnords f ON f.id = fs.fnord_id
               WHERE f.read_at IS NOT NULL
               GROUP BY m.id
               ORDER BY count DESC
               LIMIT ?"#,
        )
        .map_err(|e| e.to_string())?;

    let categories: Vec<CategoryWeight> = stmt
        .query_map([limit], |row| {
            Ok(CategoryWeight {
                id: row.get(0)?,
                name: row.get(1)?,
                weight: row.get::<_, i64>(2)? as f64,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(categories)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freshness_calculation() {
        // Very recent
        let now = chrono::Utc::now().to_rfc3339();
        let score = calculate_freshness(&Some(now));
        assert!(score > 0.95);

        // Unknown date
        let score = calculate_freshness(&None);
        assert!((score - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_explanation_keywords() {
        let candidate = Candidate {
            fnord_id: 1,
            pentacle_id: 1,
            embedding_score: Some(0.8),
            keyword_score: Some(0.5),
            freshness_score: 0.9,
            final_score: 0.7,
            matching_keywords: vec!["Trump".to_string(), "NATO".to_string(), "USA".to_string()],
            category_ids: HashSet::new(),
        };

        let profile = UserProfile {
            keyword_weights: HashMap::new(),
            category_weights: HashMap::new(),
            read_article_ids: HashSet::new(),
            hidden_article_ids: HashSet::new(),
            saved_article_ids: HashSet::new(),
            total_read: 10,
        };

        let explanation = generate_explanation(&candidate, &[], &profile);
        assert!(explanation.starts_with("Basierend auf:"));
    }
}
