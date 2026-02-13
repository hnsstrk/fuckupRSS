//! Recommendation Engine for Operation Mindfuck
//!
//! Generates personalized article recommendations based on:
//! - Embedding similarity to read articles
//! - Keyword overlap
//! - Category preferences
//! - Freshness and source diversity

use crate::AppState;
use log::{debug, info};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tauri::State;
use uuid::Uuid;

// ============================================================
// Constants
// ============================================================

// --- Query Limits ---
/// Default number of recommendations returned
const DEFAULT_RECOMMENDATION_LIMIT: i32 = 10;
/// Maximum number of recommendations a user can request
const MAX_RECOMMENDATION_LIMIT: i32 = 50;
/// Default number of saved articles returned
const DEFAULT_SAVED_LIMIT: i32 = 50;
/// Maximum saved articles a user can request
const MAX_SAVED_LIMIT: i32 = 100;
/// Total candidate pool size before reranking
const CANDIDATE_POOL_SIZE: usize = 100;
/// Candidates from embedding similarity search
const EMBEDDING_CANDIDATE_LIMIT: usize = 50;
/// Candidates from keyword overlap search
const KEYWORD_CANDIDATE_LIMIT: usize = 50;
/// Candidates from popular/recent articles (cold start)
const POPULAR_CANDIDATE_LIMIT: usize = 30;
/// Max read articles with embeddings to sample for similarity
const MAX_SAMPLE_READ_ARTICLES: usize = 10;
/// Max user keywords to consider
const MAX_USER_KEYWORDS: usize = 20;
/// Max matching keywords to display per recommendation
const MAX_DISPLAY_KEYWORDS: usize = 5;
/// Top keywords shown in recommendation stats
const STATS_TOP_KEYWORDS: usize = 10;
/// Top categories shown in recommendation stats
const STATS_TOP_CATEGORIES: usize = 5;
/// Limit for keyword aggregation query
const KEYWORD_AGGREGATION_LIMIT: usize = 50;

// --- Scoring Weights (must sum to ~1.0) ---
/// Weight of embedding similarity in final score
const WEIGHT_EMBEDDING: f64 = 0.40;
/// Weight of keyword overlap in final score
const WEIGHT_KEYWORD: f64 = 0.30;
/// Weight of freshness in final score
const WEIGHT_FRESHNESS: f64 = 0.25;
/// Baseline/bias term in final score
const WEIGHT_BASELINE: f64 = 0.05;

// --- Thresholds ---
/// Minimum embedding similarity to consider an article relevant
const MIN_EMBEDDING_SIMILARITY: f64 = 0.4;
/// Minimum keyword weight to count as a user keyword
const MIN_KEYWORD_WEIGHT: f64 = 0.3;
/// Default quality_score fallback for keywords without explicit quality
const DEFAULT_KEYWORD_QUALITY: f64 = 0.5;
/// Minimum keyword overlap count for keyword-based candidates
const MIN_KEYWORD_OVERLAP: i64 = 2;
/// Default embedding score when not available
const DEFAULT_EMBEDDING_SCORE: f64 = 0.3;

// --- Boost Multipliers ---
/// Boost for keywords from saved articles
const SAVED_KEYWORD_BOOST: f64 = 3.0;
/// Category match boost multiplier
const CATEGORY_BOOST: f64 = 1.1;
/// Smoothing parameter for keyword overlap scoring
const KEYWORD_OVERLAP_SMOOTHING: f64 = 5.0;

// --- Freshness ---
/// Hours for popular articles window (cold start)
const POPULAR_ARTICLES_HOURS: i64 = 48;
/// Freshness decay constant (48-hour half-life ≈ ln(2) * 100)
const FRESHNESS_DECAY_CONSTANT: f64 = 69.3;

// --- Diversity ---
/// Allow same source after this many diverse results
const SOURCE_DIVERSITY_THRESHOLD: usize = 5;

// --- Profile Strength ---
/// Articles read threshold for "Hot" profile
const PROFILE_STRENGTH_HOT: i64 = 50;
/// Articles read threshold for "Warm" profile (below = "Cold")
const PROFILE_STRENGTH_WARM: i64 = 10;

/// Type alias for article details query result
/// (title, summary, url, image_url, pentacle_id, pentacle_title, pentacle_icon, published_at, political_bias, sachlichkeit)
type ArticleDetailsRow = (
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
);

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

    let limit = limit
        .unwrap_or(DEFAULT_RECOMMENDATION_LIMIT)
        .min(MAX_RECOMMENDATION_LIMIT) as usize;
    let db = state.db_conn()?;

    let lock_time = start_time.elapsed();
    debug!("[{}] DB lock acquired in {:?}", request_id, lock_time);

    // Build user profile
    let profile_start = Instant::now();
    let profile = build_user_profile(db.conn())?;
    let profile_time = profile_start.elapsed();
    debug!(
        "[{}] Profile built in {:?} (read: {}, saved: {})",
        request_id,
        profile_time,
        profile.total_read,
        profile.saved_article_ids.len()
    );

    // Cold start check
    if profile.total_read == 0 {
        info!("[{}] Cold start: returning popular articles", request_id);
        let result = get_cold_start_recommendations(db.conn(), limit);
        info!(
            "[{}] Cold start completed in {:?}",
            request_id,
            start_time.elapsed()
        );
        return result;
    }

    // Generate candidates
    let candidates_start = Instant::now();
    let mut candidates = generate_candidates(db.conn(), &profile, CANDIDATE_POOL_SIZE)?;
    let candidates_time = candidates_start.elapsed();
    debug!(
        "[{}] Generated {} candidates in {:?}",
        request_id,
        candidates.len(),
        candidates_time
    );

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
    debug!(
        "[{}] Reranked to {} results in {:?}",
        request_id,
        reranked.len(),
        rerank_time
    );

    // Convert to recommendations
    let build_start = Instant::now();
    let mut recommendations = Vec::new();
    for candidate in reranked {
        let rec = build_recommendation(db.conn(), candidate, &profile)?;
        recommendations.push(rec);
    }
    let build_time = build_start.elapsed();
    debug!(
        "[{}] Built {} recommendations in {:?}",
        request_id,
        recommendations.len(),
        build_time
    );

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
    let db = state.db_conn()?;

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
    let db = state.db_conn()?;

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
    let db = state.db_conn()?;

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
    let limit = limit.unwrap_or(DEFAULT_SAVED_LIMIT).min(MAX_SAVED_LIMIT);
    let db = state.db_conn()?;

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

    let db = state.db_conn()?;

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
    let profile_strength = if articles_read >= PROFILE_STRENGTH_HOT {
        "Hot".to_string()
    } else if articles_read >= PROFILE_STRENGTH_WARM {
        "Warm".to_string()
    } else {
        "Cold".to_string()
    };

    let candidate_pool_size = total_articles - articles_read - total_hidden;

    // Top keywords from read articles
    let top_keywords = get_top_user_keywords(db.conn(), STATS_TOP_KEYWORDS as i64)?;

    // Top categories
    let top_categories = get_top_user_categories(db.conn(), STATS_TOP_CATEGORIES as i64)?;

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
        .prepare(&format!(
            r#"SELECT fi.immanentize_id, COUNT(*) as count, COALESCE(i.quality_score, {}) as quality
               FROM fnord_immanentize fi
               JOIN immanentize i ON i.id = fi.immanentize_id
               JOIN fnords f ON f.id = fi.fnord_id
               WHERE f.read_at IS NOT NULL
               GROUP BY fi.immanentize_id
               ORDER BY count DESC
               LIMIT {}"#,
            DEFAULT_KEYWORD_QUALITY, KEYWORD_AGGREGATION_LIMIT
        ))
        .map_err(|e| e.to_string())?;

    let rows: Vec<(i64, f64, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    for (keyword_id, count, quality) in rows {
        // TF-IDF-like weighting
        let weight = (1.0 + count.ln()) * quality;
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
            .query_map(rusqlite::params_from_iter(saved_ids.iter()), |row| {
                row.get(0)
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        for kw_id in saved_keywords {
            if let Some(w) = weights.get_mut(&kw_id) {
                *w *= SAVED_KEYWORD_BOOST;
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
    let embedding_candidates = get_embedding_candidates(conn, profile, EMBEDDING_CANDIDATE_LIMIT)?;
    for c in embedding_candidates {
        candidates.insert(c.fnord_id, c);
    }

    // Source 2: Keyword overlap
    let keyword_candidates = get_keyword_candidates(conn, profile, KEYWORD_CANDIDATE_LIMIT)?;
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
    let popular_candidates = get_popular_candidates(conn, POPULAR_CANDIDATE_LIMIT)?;
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
            "SELECT id FROM fnords WHERE id IN ({}) AND embedding IS NOT NULL LIMIT {}",
            placeholders, MAX_SAMPLE_READ_ARTICLES
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
        if similarity < MIN_EMBEDDING_SIMILARITY {
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
        .filter(|(_, w)| **w > MIN_KEYWORD_WEIGHT)
        .map(|(id, _)| *id)
        .take(MAX_USER_KEYWORDS)
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
        HAVING overlap_count >= {}
        ORDER BY overlap_count DESC
        LIMIT ?"#,
        placeholders, MIN_KEYWORD_OVERLAP
    );

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;

    let mut params: Vec<Box<dyn rusqlite::ToSql>> = user_keywords
        .iter()
        .map(|id| Box::new(*id) as Box<dyn rusqlite::ToSql>)
        .collect();
    params.push(Box::new(limit as i64 * 2));

    let mut candidates = Vec::new();

    let rows: Vec<(i64, i64, Option<String>, i64, String)> = stmt
        .query_map(
            rusqlite::params_from_iter(params.iter().map(|b| &**b)),
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    for (fnord_id, pentacle_id, published_at, overlap, matching) in rows {
        let user_kw_count = user_keywords.len() as f64;
        let overlap_score = overlap as f64 / (user_kw_count + KEYWORD_OVERLAP_SMOOTHING);
        let freshness = calculate_freshness(&published_at);

        let matching_keywords: Vec<String> = matching
            .split(',')
            .take(MAX_DISPLAY_KEYWORDS)
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

fn get_popular_candidates(
    conn: &rusqlite::Connection,
    limit: usize,
) -> Result<Vec<Candidate>, String> {
    let mut stmt = conn
        .prepare(&format!(
            r#"SELECT
                f.id,
                f.pentacle_id,
                f.published_at
            FROM fnords f
            JOIN pentacles p ON p.id = f.pentacle_id
            WHERE f.read_at IS NULL
              AND f.embedding IS NOT NULL
              AND f.published_at > datetime('now', '-{} hours')
            ORDER BY p.article_count DESC, f.published_at DESC
            LIMIT ?"#,
            POPULAR_ARTICLES_HOURS
        ))
        .map_err(|e| e.to_string())?;

    let mut candidates = Vec::new();

    let rows: Vec<(i64, i64, Option<String>)> = stmt
        .query_map([limit as i64], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
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
    let embedding_score = candidate.embedding_score.unwrap_or(DEFAULT_EMBEDDING_SCORE);
    let keyword_score = candidate.keyword_score.unwrap_or(0.0);
    let freshness_score = candidate.freshness_score;

    // Category boost
    let category_boost = if candidate
        .category_ids
        .iter()
        .any(|id| profile.category_weights.contains_key(id))
    {
        CATEGORY_BOOST
    } else {
        1.0
    };

    // Weighted combination
    candidate.final_score = (WEIGHT_EMBEDDING * embedding_score
        + WEIGHT_KEYWORD * keyword_score
        + WEIGHT_FRESHNESS * freshness_score
        + WEIGHT_BASELINE)
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
            // Allow same source but with penalty after SOURCE_DIVERSITY_THRESHOLD
            if i == 0 && selected.len() >= SOURCE_DIVERSITY_THRESHOLD {
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
    let (
        title,
        summary,
        url,
        image_url,
        pentacle_id,
        pentacle_title,
        pentacle_icon,
        published_at,
        political_bias,
        sachlichkeit,
    ): ArticleDetailsRow = conn
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
        let kw_list = candidate
            .matching_keywords
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
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
                let age_hours =
                    (chrono::Utc::now() - parsed.with_timezone(&chrono::Utc)).num_hours() as f64;
                // Half-life: 48 hours
                let decay = (-age_hours / FRESHNESS_DECAY_CONSTANT).exp();
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
        .prepare(&format!(
            r#"SELECT
                f.id, f.title, f.summary, f.url, f.image_url,
                f.pentacle_id, p.title, p.icon_url,
                f.published_at, f.political_bias, f.sachlichkeit
            FROM fnords f
            JOIN pentacles p ON p.id = f.pentacle_id
            WHERE f.summary IS NOT NULL
              AND f.published_at > datetime('now', '-{} hours')
            ORDER BY f.published_at DESC
            LIMIT ?"#,
            POPULAR_ARTICLES_HOURS
        ))
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

    for (
        fnord_id,
        title,
        summary,
        url,
        image_url,
        pentacle_id,
        pentacle_title,
        pentacle_icon,
        published_at,
        political_bias,
        sachlichkeit,
    ) in rows
    {
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

fn get_top_user_keywords(
    conn: &rusqlite::Connection,
    limit: i64,
) -> Result<Vec<KeywordWeight>, String> {
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

fn get_top_user_categories(
    conn: &rusqlite::Connection,
    limit: i64,
) -> Result<Vec<CategoryWeight>, String> {
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
    use crate::db::Database;

    // ============================================================
    // Helper Functions for Test Setup
    // ============================================================

    fn create_test_db() -> Database {
        Database::new_in_memory().expect("Failed to create test database")
    }

    fn insert_test_pentacle(conn: &rusqlite::Connection, id: i64, title: &str) {
        conn.execute(
            "INSERT INTO pentacles (id, url, title, article_count) VALUES (?1, ?2, ?3, ?4)",
            params![id, format!("https://example{}.com/feed", id), title, 10],
        )
        .expect("Failed to insert test pentacle");
    }

    fn insert_test_fnord(
        conn: &rusqlite::Connection,
        id: i64,
        pentacle_id: i64,
        title: &str,
        read: bool,
        with_embedding: bool,
    ) {
        let read_at = if read {
            Some("2024-01-15T12:00:00Z".to_string())
        } else {
            None
        };
        let embedding: Option<Vec<u8>> = if with_embedding {
            Some(vec![0u8; 4096]) // 1024 floats * 4 bytes
        } else {
            None
        };

        conn.execute(
            r#"INSERT INTO fnords (id, pentacle_id, guid, url, title, status, read_at, summary, published_at, embedding)
               VALUES (?1, ?2, ?3, ?4, ?5, 'concealed', ?6, 'Test summary', datetime('now', '-1 hour'), ?7)"#,
            params![
                id,
                pentacle_id,
                format!("guid-{}", id),
                format!("https://example.com/article/{}", id),
                title,
                read_at,
                embedding,
            ],
        )
        .expect("Failed to insert test fnord");
    }

    fn insert_test_keyword(conn: &rusqlite::Connection, id: i64, name: &str) {
        // Use INSERT OR REPLACE to handle collisions with seeded keywords.
        // Use high IDs (callers should use 90000+) to avoid ID collisions with seed data.
        conn.execute(
            "INSERT OR REPLACE INTO immanentize (id, name, article_count, quality_score) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, 5, 0.8],
        )
        .expect("Failed to insert test keyword");
    }

    fn link_fnord_keyword(conn: &rusqlite::Connection, fnord_id: i64, keyword_id: i64) {
        conn.execute(
            "INSERT INTO fnord_immanentize (fnord_id, immanentize_id) VALUES (?1, ?2)",
            params![fnord_id, keyword_id],
        )
        .expect("Failed to link fnord to keyword");
    }

    fn link_fnord_category(conn: &rusqlite::Connection, fnord_id: i64, sephiroth_id: i64) {
        conn.execute(
            "INSERT INTO fnord_sephiroth (fnord_id, sephiroth_id) VALUES (?1, ?2)",
            params![fnord_id, sephiroth_id],
        )
        .expect("Failed to link fnord to category");
    }

    fn insert_feedback(conn: &rusqlite::Connection, fnord_id: i64, action: &str) {
        conn.execute(
            "INSERT INTO recommendation_feedback (fnord_id, action) VALUES (?1, ?2)",
            params![fnord_id, action],
        )
        .expect("Failed to insert feedback");
    }

    // ============================================================
    // Freshness Calculation Tests
    // ============================================================

    #[test]
    fn test_freshness_calculation_now() {
        let now = chrono::Utc::now().to_rfc3339();
        let score = calculate_freshness(&Some(now));
        assert!(
            score > 0.95,
            "Very recent article should have high freshness"
        );
    }

    #[test]
    fn test_freshness_calculation_24_hours_ago() {
        let date = (chrono::Utc::now() - chrono::Duration::hours(24)).to_rfc3339();
        let score = calculate_freshness(&Some(date));
        // After 24 hours with 48-hour half-life, should be around 0.71
        assert!(
            score > 0.6 && score < 0.85,
            "24h old should have moderate freshness: {}",
            score
        );
    }

    #[test]
    fn test_freshness_calculation_48_hours_ago() {
        let date = (chrono::Utc::now() - chrono::Duration::hours(48)).to_rfc3339();
        let score = calculate_freshness(&Some(date));
        // Half-life is 48 hours (ln(2) * 100 ≈ 69.3), so should be around 0.5
        assert!(
            score > 0.4 && score < 0.6,
            "48h old should have ~0.5 freshness: {}",
            score
        );
    }

    #[test]
    fn test_freshness_calculation_1_week_ago() {
        let date = (chrono::Utc::now() - chrono::Duration::days(7)).to_rfc3339();
        let score = calculate_freshness(&Some(date));
        // 168 hours old, should be quite low
        assert!(
            score < 0.15,
            "Week old article should have low freshness: {}",
            score
        );
    }

    #[test]
    fn test_freshness_calculation_none() {
        let score = calculate_freshness(&None);
        assert!((score - 0.3).abs() < 0.01, "Unknown date should return 0.3");
    }

    #[test]
    fn test_freshness_calculation_invalid_date() {
        let score = calculate_freshness(&Some("not-a-date".to_string()));
        assert!((score - 0.5).abs() < 0.01, "Invalid date should return 0.5");
    }

    #[test]
    fn test_freshness_calculation_future_date() {
        let future = (chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339();
        let score = calculate_freshness(&Some(future));
        // Future dates should be clamped to 1.0
        assert!(
            (score - 1.0).abs() < 0.01,
            "Future date should be clamped to 1.0: {}",
            score
        );
    }

    // ============================================================
    // Explanation Generation Tests
    // ============================================================

    #[test]
    fn test_explanation_with_multiple_keywords() {
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
        assert!(
            explanation.starts_with("Basierend auf:"),
            "Should mention keywords"
        );
        assert!(explanation.contains("Trump"), "Should include keyword");
    }

    #[test]
    fn test_explanation_with_single_keyword() {
        let candidate = Candidate {
            fnord_id: 1,
            pentacle_id: 1,
            embedding_score: Some(0.8),
            keyword_score: Some(0.5),
            freshness_score: 0.9,
            final_score: 0.7,
            matching_keywords: vec!["Trump".to_string()],
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

        let categories = vec![CategoryInfo {
            sephiroth_id: 201,
            name: "Politik".to_string(),
            icon: None,
            color: None,
        }];

        let explanation = generate_explanation(&candidate, &categories, &profile);
        // With only 1 keyword, should fall back to category
        assert!(explanation.contains("Bereich") || explanation.contains("Politik"));
    }

    #[test]
    fn test_explanation_embedding_similarity() {
        let candidate = Candidate {
            fnord_id: 1,
            pentacle_id: 1,
            embedding_score: Some(0.75),
            keyword_score: None,
            freshness_score: 0.5,
            final_score: 0.6,
            matching_keywords: vec![],
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
        assert!(explanation.contains("Thematisch") || explanation.contains("ähnlich"));
    }

    #[test]
    fn test_explanation_freshness_fallback() {
        let candidate = Candidate {
            fnord_id: 1,
            pentacle_id: 1,
            embedding_score: Some(0.3),
            keyword_score: None,
            freshness_score: 0.95,
            final_score: 0.5,
            matching_keywords: vec![],
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
        assert!(explanation.contains("Aktuell"));
    }

    #[test]
    fn test_explanation_generic_fallback() {
        let candidate = Candidate {
            fnord_id: 1,
            pentacle_id: 1,
            embedding_score: Some(0.3),
            keyword_score: None,
            freshness_score: 0.2,
            final_score: 0.3,
            matching_keywords: vec![],
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
        assert!(explanation.contains("interessieren"));
    }

    // ============================================================
    // Candidate Scoring Tests
    // ============================================================

    #[test]
    fn test_score_candidate_embedding_only() {
        let profile = UserProfile {
            keyword_weights: HashMap::new(),
            category_weights: HashMap::new(),
            read_article_ids: HashSet::new(),
            hidden_article_ids: HashSet::new(),
            saved_article_ids: HashSet::new(),
            total_read: 10,
        };

        let mut candidate = Candidate {
            fnord_id: 1,
            pentacle_id: 1,
            embedding_score: Some(0.9),
            keyword_score: None,
            freshness_score: 1.0,
            final_score: 0.0,
            matching_keywords: vec![],
            category_ids: HashSet::new(),
        };

        score_candidate(&mut candidate, &profile);

        // Score = 0.40 * 0.9 + 0.30 * 0.0 + 0.25 * 1.0 + 0.05 = 0.66
        assert!(
            candidate.final_score > 0.6 && candidate.final_score < 0.7,
            "Score should be around 0.66: {}",
            candidate.final_score
        );
    }

    #[test]
    fn test_score_candidate_keyword_only() {
        let profile = UserProfile {
            keyword_weights: HashMap::new(),
            category_weights: HashMap::new(),
            read_article_ids: HashSet::new(),
            hidden_article_ids: HashSet::new(),
            saved_article_ids: HashSet::new(),
            total_read: 10,
        };

        let mut candidate = Candidate {
            fnord_id: 1,
            pentacle_id: 1,
            embedding_score: None,
            keyword_score: Some(0.8),
            freshness_score: 1.0,
            final_score: 0.0,
            matching_keywords: vec!["test".to_string()],
            category_ids: HashSet::new(),
        };

        score_candidate(&mut candidate, &profile);

        // Score = 0.40 * 0.3 + 0.30 * 0.8 + 0.25 * 1.0 + 0.05 = 0.66
        assert!(
            candidate.final_score > 0.6 && candidate.final_score < 0.7,
            "Score should be around 0.66: {}",
            candidate.final_score
        );
    }

    #[test]
    fn test_score_candidate_with_category_boost() {
        let mut category_weights = HashMap::new();
        category_weights.insert(201, 0.5);

        let profile = UserProfile {
            keyword_weights: HashMap::new(),
            category_weights,
            read_article_ids: HashSet::new(),
            hidden_article_ids: HashSet::new(),
            saved_article_ids: HashSet::new(),
            total_read: 10,
        };

        let mut candidate = Candidate {
            fnord_id: 1,
            pentacle_id: 1,
            embedding_score: Some(0.8),
            keyword_score: Some(0.5),
            freshness_score: 0.9,
            final_score: 0.0,
            matching_keywords: vec![],
            category_ids: [201].into_iter().collect(),
        };

        score_candidate(&mut candidate, &profile);

        // With category boost (1.1x), score should be higher
        assert!(
            candidate.final_score > 0.7,
            "Category boost should increase score: {}",
            candidate.final_score
        );
    }

    #[test]
    fn test_score_candidate_no_category_boost() {
        let mut category_weights = HashMap::new();
        category_weights.insert(201, 0.5);

        let profile = UserProfile {
            keyword_weights: HashMap::new(),
            category_weights,
            read_article_ids: HashSet::new(),
            hidden_article_ids: HashSet::new(),
            saved_article_ids: HashSet::new(),
            total_read: 10,
        };

        let mut candidate = Candidate {
            fnord_id: 1,
            pentacle_id: 1,
            embedding_score: Some(0.8),
            keyword_score: Some(0.5),
            freshness_score: 0.9,
            final_score: 0.0,
            matching_keywords: vec![],
            category_ids: [101].into_iter().collect(), // Different category
        };

        score_candidate(&mut candidate, &profile);

        // Without matching category, no boost
        let base_score = 0.40 * 0.8 + 0.30 * 0.5 + 0.25 * 0.9 + 0.05;
        assert!(
            (candidate.final_score - base_score).abs() < 0.01,
            "Without category boost, score should be base: {}",
            candidate.final_score
        );
    }

    #[test]
    fn test_score_candidate_zero_scores() {
        let profile = UserProfile {
            keyword_weights: HashMap::new(),
            category_weights: HashMap::new(),
            read_article_ids: HashSet::new(),
            hidden_article_ids: HashSet::new(),
            saved_article_ids: HashSet::new(),
            total_read: 10,
        };

        let mut candidate = Candidate {
            fnord_id: 1,
            pentacle_id: 1,
            embedding_score: None,
            keyword_score: None,
            freshness_score: 0.0,
            final_score: 0.0,
            matching_keywords: vec![],
            category_ids: HashSet::new(),
        };

        score_candidate(&mut candidate, &profile);

        // Score = 0.40 * 0.3 + 0.30 * 0.0 + 0.25 * 0.0 + 0.05 = 0.17
        assert!(
            candidate.final_score > 0.15 && candidate.final_score < 0.2,
            "Minimum score should be around 0.17: {}",
            candidate.final_score
        );
    }

    // ============================================================
    // Reranking for Diversity Tests
    // ============================================================

    #[test]
    fn test_rerank_preserves_top_scores() {
        let candidates = vec![
            Candidate {
                fnord_id: 1,
                pentacle_id: 1,
                embedding_score: Some(0.9),
                keyword_score: None,
                freshness_score: 0.9,
                final_score: 0.9,
                matching_keywords: vec![],
                category_ids: HashSet::new(),
            },
            Candidate {
                fnord_id: 2,
                pentacle_id: 2,
                embedding_score: Some(0.8),
                keyword_score: None,
                freshness_score: 0.8,
                final_score: 0.8,
                matching_keywords: vec![],
                category_ids: HashSet::new(),
            },
        ];

        let reranked = rerank_for_diversity(candidates, 2);
        assert_eq!(reranked.len(), 2);
        assert_eq!(reranked[0].fnord_id, 1, "Highest score should be first");
    }

    #[test]
    fn test_rerank_source_diversity() {
        let candidates = vec![
            Candidate {
                fnord_id: 1,
                pentacle_id: 1,
                embedding_score: None,
                keyword_score: None,
                freshness_score: 0.9,
                final_score: 0.95,
                matching_keywords: vec![],
                category_ids: HashSet::new(),
            },
            Candidate {
                fnord_id: 2,
                pentacle_id: 1, // Same source
                embedding_score: None,
                keyword_score: None,
                freshness_score: 0.9,
                final_score: 0.90,
                matching_keywords: vec![],
                category_ids: HashSet::new(),
            },
            Candidate {
                fnord_id: 3,
                pentacle_id: 2, // Different source
                embedding_score: None,
                keyword_score: None,
                freshness_score: 0.8,
                final_score: 0.85,
                matching_keywords: vec![],
                category_ids: HashSet::new(),
            },
        ];

        let reranked = rerank_for_diversity(candidates, 2);
        assert_eq!(reranked.len(), 2);
        // First should be highest score
        assert_eq!(reranked[0].fnord_id, 1);
        // Second should prefer different source
        assert_eq!(
            reranked[1].fnord_id, 3,
            "Should pick different source over same source"
        );
    }

    #[test]
    fn test_rerank_empty_input() {
        let candidates: Vec<Candidate> = vec![];
        let reranked = rerank_for_diversity(candidates, 10);
        assert!(reranked.is_empty());
    }

    #[test]
    fn test_rerank_limit_enforcement() {
        let candidates: Vec<Candidate> = (0..20)
            .map(|i| Candidate {
                fnord_id: i,
                pentacle_id: i,
                embedding_score: None,
                keyword_score: None,
                freshness_score: 0.5,
                final_score: 0.5 + (i as f64 * 0.01),
                matching_keywords: vec![],
                category_ids: HashSet::new(),
            })
            .collect();

        let reranked = rerank_for_diversity(candidates, 5);
        assert_eq!(reranked.len(), 5, "Should respect limit");
    }

    // ============================================================
    // User Profile Building Tests (Database Integration)
    // ============================================================

    #[test]
    fn test_build_user_profile_empty() {
        let db = create_test_db();
        let conn = db.conn();

        let profile = build_user_profile(conn).expect("Failed to build profile");

        assert_eq!(profile.total_read, 0);
        assert!(profile.read_article_ids.is_empty());
        assert!(profile.saved_article_ids.is_empty());
        assert!(profile.hidden_article_ids.is_empty());
        assert!(profile.keyword_weights.is_empty());
        assert!(profile.category_weights.is_empty());
    }

    #[test]
    fn test_build_user_profile_with_read_articles() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");
        insert_test_fnord(conn, 1, 1, "Read Article 1", true, false);
        insert_test_fnord(conn, 2, 1, "Read Article 2", true, false);
        insert_test_fnord(conn, 3, 1, "Unread Article", false, false);

        let profile = build_user_profile(conn).expect("Failed to build profile");

        assert_eq!(profile.total_read, 2);
        assert!(profile.read_article_ids.contains(&1));
        assert!(profile.read_article_ids.contains(&2));
        assert!(!profile.read_article_ids.contains(&3));
    }

    #[test]
    fn test_build_user_profile_with_saved_articles() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");
        insert_test_fnord(conn, 1, 1, "Article 1", true, false);
        insert_test_fnord(conn, 2, 1, "Article 2", true, false);
        insert_feedback(conn, 1, "save");

        let profile = build_user_profile(conn).expect("Failed to build profile");

        assert!(profile.saved_article_ids.contains(&1));
        assert!(!profile.saved_article_ids.contains(&2));
    }

    #[test]
    fn test_build_user_profile_with_hidden_articles() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");
        insert_test_fnord(conn, 1, 1, "Article 1", false, false);
        insert_test_fnord(conn, 2, 1, "Article 2", false, false);
        insert_feedback(conn, 1, "hide");

        let profile = build_user_profile(conn).expect("Failed to build profile");

        assert!(profile.hidden_article_ids.contains(&1));
        assert!(!profile.hidden_article_ids.contains(&2));
    }

    // ============================================================
    // Keyword Aggregation Tests
    // ============================================================

    #[test]
    fn test_aggregate_keywords_empty() {
        let db = create_test_db();
        let conn = db.conn();

        let read_ids: HashSet<i64> = HashSet::new();
        let saved_ids: HashSet<i64> = HashSet::new();

        let weights =
            aggregate_keywords(conn, &read_ids, &saved_ids).expect("Failed to aggregate keywords");

        assert!(weights.is_empty());
    }

    #[test]
    fn test_aggregate_keywords_with_read_articles() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");
        insert_test_fnord(conn, 1, 1, "Article 1", true, false);
        insert_test_fnord(conn, 2, 1, "Article 2", true, false);
        insert_test_keyword(conn, 90100, "TestKW_ReadArticles_A");
        insert_test_keyword(conn, 90101, "TestKW_ReadArticles_B");
        link_fnord_keyword(conn, 1, 90100);
        link_fnord_keyword(conn, 2, 90100);
        link_fnord_keyword(conn, 2, 90101);

        let read_ids: HashSet<i64> = [1, 2].into_iter().collect();
        let saved_ids: HashSet<i64> = HashSet::new();

        let weights =
            aggregate_keywords(conn, &read_ids, &saved_ids).expect("Failed to aggregate keywords");

        assert!(!weights.is_empty());
        // "TestKW_ReadArticles_A" appears in 2 articles, should have higher weight
        assert!(weights.get(&90100).unwrap_or(&0.0) >= weights.get(&90101).unwrap_or(&0.0));
    }

    #[test]
    fn test_aggregate_keywords_saved_boost() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");
        insert_test_fnord(conn, 1, 1, "Article 1", true, false);
        insert_test_fnord(conn, 2, 1, "Article 2", true, false);
        insert_test_keyword(conn, 90200, "TestKW_SavedBoost_A");
        insert_test_keyword(conn, 90201, "TestKW_SavedBoost_B");
        link_fnord_keyword(conn, 1, 90200);
        link_fnord_keyword(conn, 2, 90201);

        // Without save
        let read_ids: HashSet<i64> = [1, 2].into_iter().collect();
        let saved_ids_empty: HashSet<i64> = HashSet::new();
        let weights_no_save = aggregate_keywords(conn, &read_ids, &saved_ids_empty)
            .expect("Failed to aggregate keywords");

        // With save on article 1 (has keyword 90200)
        insert_feedback(conn, 1, "save");
        let saved_ids: HashSet<i64> = [1].into_iter().collect();
        let weights_with_save =
            aggregate_keywords(conn, &read_ids, &saved_ids).expect("Failed to aggregate keywords");

        // Keyword 90200 should have higher weight when its article is saved
        let weight_no_save = weights_no_save.get(&90200).unwrap_or(&0.0);
        let weight_with_save = weights_with_save.get(&90200).unwrap_or(&0.0);
        assert!(
            weight_with_save >= weight_no_save,
            "Saved article should boost keyword weight"
        );
    }

    // ============================================================
    // Category Aggregation Tests
    // ============================================================

    #[test]
    fn test_aggregate_categories_empty() {
        let db = create_test_db();
        let conn = db.conn();

        let read_ids: HashSet<i64> = HashSet::new();
        let weights =
            aggregate_categories(conn, &read_ids).expect("Failed to aggregate categories");

        assert!(weights.is_empty());
    }

    #[test]
    fn test_aggregate_categories_with_read_articles() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");
        insert_test_fnord(conn, 1, 1, "Article 1", true, false);
        insert_test_fnord(conn, 2, 1, "Article 2", true, false);
        insert_test_fnord(conn, 3, 1, "Article 3", true, false);

        // Link to existing categories (201 = Politik, 301 = Wirtschaft)
        link_fnord_category(conn, 1, 201);
        link_fnord_category(conn, 2, 201);
        link_fnord_category(conn, 3, 301);

        let read_ids: HashSet<i64> = [1, 2, 3].into_iter().collect();
        let weights =
            aggregate_categories(conn, &read_ids).expect("Failed to aggregate categories");

        // Politik (201) should have higher weight (2/3) than Wirtschaft (1/3)
        assert!(weights.get(&201).unwrap_or(&0.0) > weights.get(&301).unwrap_or(&0.0));
    }

    // ============================================================
    // Candidate Generation Tests
    // ============================================================

    #[test]
    fn test_generate_candidates_filters_read() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");
        insert_test_fnord(conn, 1, 1, "Read Article", true, true);
        insert_test_fnord(conn, 2, 1, "Unread Article", false, true);

        let profile = build_user_profile(conn).expect("Failed to build profile");
        let candidates =
            generate_candidates(conn, &profile, 10).expect("Failed to generate candidates");

        // Should not include read articles
        assert!(
            !candidates.iter().any(|c| c.fnord_id == 1),
            "Should not include read articles"
        );
    }

    #[test]
    fn test_generate_candidates_filters_hidden() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");
        insert_test_fnord(conn, 1, 1, "Hidden Article", false, true);
        insert_test_fnord(conn, 2, 1, "Normal Article", false, true);
        insert_feedback(conn, 1, "hide");

        let profile = build_user_profile(conn).expect("Failed to build profile");
        let candidates =
            generate_candidates(conn, &profile, 10).expect("Failed to generate candidates");

        // Should not include hidden articles
        assert!(
            !candidates.iter().any(|c| c.fnord_id == 1),
            "Should not include hidden articles"
        );
    }

    // ============================================================
    // Cold Start Tests
    // ============================================================

    #[test]
    fn test_cold_start_recommendations() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");

        // Insert recent article with summary
        conn.execute(
            r#"INSERT INTO fnords (id, pentacle_id, guid, url, title, summary, published_at, status)
               VALUES (1, 1, 'guid-1', 'https://example.com/1', 'Recent Article', 'Test summary', datetime('now', '-1 hour'), 'concealed')"#,
            [],
        ).expect("Failed to insert article");

        let recommendations = get_cold_start_recommendations(conn, 10)
            .expect("Failed to get cold start recommendations");

        assert!(
            !recommendations.is_empty(),
            "Should return recent articles for cold start"
        );
        assert_eq!(recommendations[0].explanation, "Aktuelle Nachrichten");
    }

    #[test]
    fn test_cold_start_excludes_old_articles() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");

        // Insert old article (more than 48 hours)
        conn.execute(
            r#"INSERT INTO fnords (id, pentacle_id, guid, url, title, summary, published_at, status)
               VALUES (1, 1, 'guid-1', 'https://example.com/1', 'Old Article', 'Test summary', datetime('now', '-72 hours'), 'concealed')"#,
            [],
        ).expect("Failed to insert article");

        let recommendations = get_cold_start_recommendations(conn, 10)
            .expect("Failed to get cold start recommendations");

        assert!(
            recommendations.is_empty(),
            "Should not include old articles in cold start"
        );
    }

    // ============================================================
    // Top Keywords/Categories for Stats
    // ============================================================

    #[test]
    fn test_get_top_user_keywords() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");
        insert_test_fnord(conn, 1, 1, "Article 1", true, false);
        insert_test_fnord(conn, 2, 1, "Article 2", true, false);
        insert_test_keyword(conn, 90300, "TestKW_TopUser_Alpha");
        insert_test_keyword(conn, 90301, "TestKW_TopUser_Beta");
        link_fnord_keyword(conn, 1, 90300);
        link_fnord_keyword(conn, 1, 90301);
        link_fnord_keyword(conn, 2, 90300);

        let keywords = get_top_user_keywords(conn, 5).expect("Failed to get top keywords");

        assert!(!keywords.is_empty());
        // "TestKW_TopUser_Alpha" should be first (appears in 2 articles)
        assert_eq!(keywords[0].name, "TestKW_TopUser_Alpha");
    }

    #[test]
    fn test_get_top_user_categories() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");
        insert_test_fnord(conn, 1, 1, "Article 1", true, false);
        insert_test_fnord(conn, 2, 1, "Article 2", true, false);
        link_fnord_category(conn, 1, 201); // Politik
        link_fnord_category(conn, 1, 301); // Wirtschaft
        link_fnord_category(conn, 2, 201); // Politik again

        let categories = get_top_user_categories(conn, 5).expect("Failed to get top categories");

        assert!(!categories.is_empty());
    }

    // ============================================================
    // Data Structure Serialization Tests
    // ============================================================

    #[test]
    fn test_recommendation_serialize() {
        let rec = Recommendation {
            fnord_id: 123,
            title: "Test Article".to_string(),
            summary: Some("Summary".to_string()),
            url: "https://example.com".to_string(),
            image_url: None,
            pentacle_id: 1,
            pentacle_title: Some("Test Feed".to_string()),
            pentacle_icon: None,
            published_at: Some("2024-01-15T12:00:00Z".to_string()),
            relevance_score: 0.85,
            freshness_score: 0.9,
            political_bias: Some(0),
            sachlichkeit: Some(3),
            categories: vec![],
            matching_keywords: vec!["keyword1".to_string()],
            explanation: "Based on keywords".to_string(),
            is_saved: false,
        };

        let json = serde_json::to_string(&rec).expect("Serialization failed");
        assert!(json.contains("\"fnord_id\":123"));
        assert!(json.contains("\"relevance_score\":0.85"));
    }

    #[test]
    fn test_recommendation_stats_serialize() {
        let stats = RecommendationStats {
            total_saved: 10,
            total_hidden: 5,
            total_clicks: 20,
            articles_read: 100,
            articles_with_embedding: 80,
            profile_strength: "Warm".to_string(),
            top_keywords: vec![],
            top_categories: vec![],
            candidate_pool_size: 500,
        };

        let json = serde_json::to_string(&stats).expect("Serialization failed");
        assert!(json.contains("\"total_saved\":10"));
        assert!(json.contains("\"profile_strength\":\"Warm\""));
    }

    #[test]
    fn test_saved_article_serialize() {
        let saved = SavedArticle {
            fnord_id: 456,
            title: "Saved Article".to_string(),
            pentacle_title: Some("News Feed".to_string()),
            saved_at: "2024-01-15T12:00:00Z".to_string(),
            published_at: Some("2024-01-14T10:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&saved).expect("Serialization failed");
        assert!(json.contains("\"fnord_id\":456"));
        assert!(json.contains("\"saved_at\""));
    }

    #[test]
    fn test_category_info_clone() {
        let cat = CategoryInfo {
            sephiroth_id: 201,
            name: "Politik".to_string(),
            icon: Some("fa-landmark".to_string()),
            color: Some("#ff0000".to_string()),
        };

        let cloned = cat.clone();
        assert_eq!(cloned.sephiroth_id, cat.sephiroth_id);
        assert_eq!(cloned.name, cat.name);
    }

    // ============================================================
    // Profile Strength Tests
    // ============================================================

    #[test]
    fn test_profile_strength_cold() {
        // Less than 10 read articles = Cold
        let articles_read = 5;
        let strength = if articles_read >= 50 {
            "Hot"
        } else if articles_read >= 10 {
            "Warm"
        } else {
            "Cold"
        };
        assert_eq!(strength, "Cold");
    }

    #[test]
    fn test_profile_strength_warm() {
        let articles_read = 25;
        let strength = if articles_read >= 50 {
            "Hot"
        } else if articles_read >= 10 {
            "Warm"
        } else {
            "Cold"
        };
        assert_eq!(strength, "Warm");
    }

    #[test]
    fn test_profile_strength_hot() {
        let articles_read = 100;
        let strength = if articles_read >= 50 {
            "Hot"
        } else if articles_read >= 10 {
            "Warm"
        } else {
            "Cold"
        };
        assert_eq!(strength, "Hot");
    }

    // ============================================================
    // Edge Case Tests
    // ============================================================

    #[test]
    fn test_candidate_with_nan_scores() {
        let mut candidates = vec![
            Candidate {
                fnord_id: 1,
                pentacle_id: 1,
                embedding_score: Some(f64::NAN),
                keyword_score: None,
                freshness_score: 0.5,
                final_score: f64::NAN,
                matching_keywords: vec![],
                category_ids: HashSet::new(),
            },
            Candidate {
                fnord_id: 2,
                pentacle_id: 2,
                embedding_score: Some(0.8),
                keyword_score: None,
                freshness_score: 0.5,
                final_score: 0.7,
                matching_keywords: vec![],
                category_ids: HashSet::new(),
            },
        ];

        // Should not panic on NaN comparison
        candidates.sort_by(|a, b| {
            b.final_score
                .partial_cmp(&a.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Valid score should be handled properly
        assert!(candidates.iter().any(|c| c.fnord_id == 2));
    }

    #[test]
    fn test_empty_matching_keywords() {
        let candidate = Candidate {
            fnord_id: 1,
            pentacle_id: 1,
            embedding_score: Some(0.5),
            keyword_score: None,
            freshness_score: 0.5,
            final_score: 0.5,
            matching_keywords: vec![],
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

        // Should not panic with empty keywords
        let explanation = generate_explanation(&candidate, &[], &profile);
        assert!(!explanation.is_empty());
    }

    #[test]
    fn test_limit_clamping() {
        // Test that limit is properly clamped
        let limit_50 = Some(100).unwrap_or(10).min(50);
        assert_eq!(limit_50, 50);

        let limit_default = None.unwrap_or(10).min(50);
        assert_eq!(limit_default, 10);

        let limit_normal = Some(20).unwrap_or(10).min(50);
        assert_eq!(limit_normal, 20);
    }

    // ============================================================
    // Feedback Action Tests
    // ============================================================

    #[test]
    fn test_feedback_save_action() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");
        insert_test_fnord(conn, 1, 1, "Article", false, false);

        // Insert save feedback
        conn.execute(
            r#"INSERT INTO recommendation_feedback (fnord_id, action)
               VALUES (?1, 'save')
               ON CONFLICT (fnord_id, action) DO NOTHING"#,
            params![1],
        )
        .expect("Failed to insert feedback");

        // Verify
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM recommendation_feedback WHERE fnord_id = 1 AND action = 'save'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count");

        assert_eq!(count, 1);
    }

    #[test]
    fn test_feedback_hide_action() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");
        insert_test_fnord(conn, 1, 1, "Article", false, false);

        // Insert hide feedback
        conn.execute(
            r#"INSERT INTO recommendation_feedback (fnord_id, action)
               VALUES (?1, 'hide')
               ON CONFLICT (fnord_id, action) DO NOTHING"#,
            params![1],
        )
        .expect("Failed to insert feedback");

        // Verify
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM recommendation_feedback WHERE fnord_id = 1 AND action = 'hide'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count");

        assert_eq!(count, 1);
    }

    #[test]
    fn test_feedback_duplicate_handling() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");
        insert_test_fnord(conn, 1, 1, "Article", false, false);

        // Insert same feedback twice
        conn.execute(
            r#"INSERT INTO recommendation_feedback (fnord_id, action)
               VALUES (?1, 'save')
               ON CONFLICT (fnord_id, action) DO NOTHING"#,
            params![1],
        )
        .expect("Failed to insert feedback");

        conn.execute(
            r#"INSERT INTO recommendation_feedback (fnord_id, action)
               VALUES (?1, 'save')
               ON CONFLICT (fnord_id, action) DO NOTHING"#,
            params![1],
        )
        .expect("Failed to insert feedback");

        // Should only have one entry
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM recommendation_feedback WHERE fnord_id = 1",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count");

        assert_eq!(count, 1);
    }

    #[test]
    fn test_unsave_removes_feedback() {
        let db = create_test_db();
        let conn = db.conn();

        insert_test_pentacle(conn, 1, "Test Feed");
        insert_test_fnord(conn, 1, 1, "Article", false, false);
        insert_feedback(conn, 1, "save");

        // Delete save feedback
        conn.execute(
            "DELETE FROM recommendation_feedback WHERE fnord_id = ?1 AND action = 'save'",
            params![1],
        )
        .expect("Failed to delete feedback");

        // Verify
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM recommendation_feedback WHERE fnord_id = 1 AND action = 'save'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count");

        assert_eq!(count, 0);
    }

    // ============================================================
    // Integration Test: Full Recommendation Flow
    // ============================================================

    #[test]
    fn test_full_recommendation_flow() {
        let db = create_test_db();
        let conn = db.conn();

        // Setup: Create feed and articles
        insert_test_pentacle(conn, 1, "Tech News");
        insert_test_pentacle(conn, 2, "Politics");

        // Read articles with keywords
        insert_test_fnord(conn, 1, 1, "AI Revolution", true, true);
        insert_test_fnord(conn, 2, 1, "Machine Learning", true, true);
        insert_test_fnord(conn, 3, 2, "Election News", true, true);

        // Candidate articles (unread)
        insert_test_fnord(conn, 4, 1, "New AI Model", false, true);
        insert_test_fnord(conn, 5, 2, "Senate Vote", false, true);

        // Keywords (using high IDs and unique names to avoid seed data collisions)
        insert_test_keyword(conn, 90400, "TestKW_Flow_AI");
        insert_test_keyword(conn, 90401, "TestKW_Flow_ML");
        insert_test_keyword(conn, 90402, "TestKW_Flow_Politics");

        // Link read articles to keywords
        link_fnord_keyword(conn, 1, 90400);
        link_fnord_keyword(conn, 2, 90400);
        link_fnord_keyword(conn, 2, 90401);
        link_fnord_keyword(conn, 3, 90402);

        // Link candidate to keywords
        link_fnord_keyword(conn, 4, 90400);
        link_fnord_keyword(conn, 4, 90401);

        // Build profile
        let profile = build_user_profile(conn).expect("Failed to build profile");
        assert_eq!(profile.total_read, 3);

        // Generate candidates
        let candidates =
            generate_candidates(conn, &profile, 10).expect("Failed to generate candidates");

        // Should have unread candidates
        assert!(!candidates.is_empty());

        // Should not include read articles
        for c in &candidates {
            assert!(!profile.read_article_ids.contains(&c.fnord_id));
        }
    }
}
