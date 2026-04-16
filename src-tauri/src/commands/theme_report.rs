//! Theme Reports: Multi-signal topic detection with LLM-powered analysis
//!
//! Replaces the old story_clusters module. Uses a 3-phase pipeline:
//! Phase 1: Statistical multi-signal clustering (no LLM)
//! Phase 2: Batch validation via Fast LLM
//! Phase 3: Per-theme report generation via Reasoning LLM

use crate::ai_provider::TaskType;
use crate::commands::ai::helpers::{create_embedding_provider_from_db, create_text_provider};
use crate::embeddings::blob_to_embedding;
use crate::error::{CmdResult, FuckupError};
use crate::theme_clustering::{
    agglomerative_cluster, decay_hours_for_days, topic_score, ArticlePair, ArticleSignals,
    ClusterCandidate, ANN_PREFILTER_THRESHOLD, MIN_ARTICLES_FOR_REPORT, MIN_SOURCE_COUNT,
    ORPHAN_ATTACH_THRESHOLD,
};
use crate::AppState;
use log::{error, info, warn};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tauri::{Emitter, State};

// ============================================================
// TYPES (returned to Frontend)
// ============================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeReportSummary {
    pub id: i64,
    pub period_start: String,
    pub period_end: String,
    pub search_query: Option<String>,
    pub theme_count: i32,
    pub model_used: Option<String>,
    pub locale: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeReportDetail {
    pub report: ThemeReportSummary,
    pub themes: Vec<ThemeReportTheme>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeReportTheme {
    pub id: i64,
    pub label: String,
    pub headline: Option<String>,
    pub report_json: Option<String>,
    pub report_status: String,
    pub cluster_score: f64,
    pub article_count: i32,
    pub source_count: i32,
    pub articles: Vec<ThemeArticle>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeArticle {
    pub fnord_id: i64,
    pub title: String,
    pub summary: Option<String>,
    pub source_name: String,
    pub political_bias: Option<i32>,
    pub sachlichkeit: Option<i32>,
    pub published_at: String,
    pub topic_score: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeProgress {
    pub report_id: i64,
    pub themes_complete: usize,
    pub themes_total: usize,
    pub current_theme: String,
}

/// Bias label for report context
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
// DATABASE HELPERS
// ============================================================

/// Load articles with signals for clustering
fn load_articles_with_signals(
    conn: &rusqlite::Connection,
    period_start: &str,
    period_end: &str,
    search_fnord_ids: Option<&HashSet<i64>>,
) -> Result<Vec<ArticleSignals>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT f.id, f.pentacle_id, f.title, f.summary, f.published_at,
                f.political_bias, f.sachlichkeit, COALESCE(p.title, p.url) as source_name
         FROM fnords f
         JOIN pentacles p ON p.id = f.pentacle_id
         WHERE f.embedding IS NOT NULL
           AND f.processed_at IS NOT NULL
           AND datetime(f.published_at) >= datetime(?1)
           AND datetime(f.published_at) <= datetime(?2)
         ORDER BY f.published_at ASC",
    )?;

    let mut articles: Vec<ArticleSignals> = stmt
        .query_map(params![period_start, period_end], |row| {
            Ok(ArticleSignals {
                fnord_id: row.get(0)?,
                pentacle_id: row.get(1)?,
                title: row.get(2)?,
                summary: row.get(3)?,
                published_at: row.get::<_, String>(4).unwrap_or_default(),
                political_bias: row.get(5)?,
                sachlichkeit: row.get(6)?,
                source_name: row.get(7)?,
                category_ids: vec![],
                keyword_ids: vec![],
                entity_ids: vec![],
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Filter by search results if provided
    if let Some(search_ids) = search_fnord_ids {
        articles.retain(|a| search_ids.contains(&a.fnord_id));
    }

    // Load keywords per article
    {
        let mut kw_stmt =
            conn.prepare("SELECT immanentize_id FROM fnord_immanentize WHERE fnord_id = ?1")?;
        for article in &mut articles {
            article.keyword_ids = kw_stmt
                .query_map(params![article.fnord_id], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
        }
    }

    // Load categories per article
    {
        let mut cat_stmt =
            conn.prepare("SELECT sephiroth_id FROM fnord_sephiroth WHERE fnord_id = ?1")?;
        for article in &mut articles {
            article.category_ids = cat_stmt
                .query_map(params![article.fnord_id], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
        }
    }

    // Load NER entities per article
    {
        let mut ent_stmt = conn.prepare(
            "SELECT e.id, e.entity_type FROM fnord_entities fe
             JOIN entities e ON e.id = fe.entity_id
             WHERE fe.fnord_id = ?1",
        )?;
        for article in &mut articles {
            article.entity_ids = ent_stmt
                .query_map(params![article.fnord_id], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })?
                .filter_map(|r| r.ok())
                .collect();
        }
    }

    Ok(articles)
}

/// (keyword_id_to_name, entity_id_to_name) lookup pair.
type NameMaps = (HashMap<i64, String>, HashMap<i64, String>);

/// Load keyword (immanentize) and entity names for all keyword/entity IDs
/// referenced by the given articles. Returns (keyword_map, entity_map).
///
/// Loaded upfront so Phase 2 can verify that an LLM-generated label actually
/// mentions at least one keyword or entity from the cluster — guarding against
/// the model hallucinating labels by mixing up cluster_ids within the batch.
fn load_keyword_entity_names(
    conn: &rusqlite::Connection,
    articles: &[ArticleSignals],
) -> Result<NameMaps, rusqlite::Error> {
    let mut keyword_ids: HashSet<i64> = HashSet::new();
    let mut entity_ids: HashSet<i64> = HashSet::new();
    for a in articles {
        keyword_ids.extend(a.keyword_ids.iter().copied());
        entity_ids.extend(a.entity_ids.iter().map(|(id, _)| *id));
    }

    let mut keyword_names: HashMap<i64, String> = HashMap::new();
    if !keyword_ids.is_empty() {
        let mut stmt = conn.prepare("SELECT id, name FROM immanentize WHERE id = ?1")?;
        for id in &keyword_ids {
            if let Ok((i, name)) =
                stmt.query_row(params![id], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })
            {
                keyword_names.insert(i, name);
            }
        }
    }

    let mut entity_names: HashMap<i64, String> = HashMap::new();
    if !entity_ids.is_empty() {
        let mut stmt = conn.prepare("SELECT id, name FROM entities WHERE id = ?1")?;
        for id in &entity_ids {
            if let Ok((i, name)) =
                stmt.query_row(params![id], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })
            {
                entity_names.insert(i, name);
            }
        }
    }

    Ok((keyword_names, entity_names))
}

/// Compute cosine similarity between two embedding vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        let x = *x as f64;
        let y = *y as f64;
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

/// Get article pairs by computing pairwise cosine similarity for in-period articles.
/// Uses fnords.embedding (regular table) instead of vec_fnords (virtual table)
/// to ensure reliable blob access in the expected f32-LE format.
fn get_ann_pairs(
    conn: &rusqlite::Connection,
    article_ids: &[i64],
) -> Result<Vec<ArticlePair>, rusqlite::Error> {
    // Load embeddings from fnords table (regular table, reliable blob format)
    let mut embeddings: HashMap<i64, Vec<f32>> = HashMap::new();
    let mut stmt =
        conn.prepare("SELECT embedding FROM fnords WHERE id = ?1 AND embedding IS NOT NULL")?;

    for &fnord_id in article_ids {
        let blob: Option<Vec<u8>> = stmt.query_row(params![fnord_id], |row| row.get(0)).ok();
        if let Some(blob) = blob {
            let emb = blob_to_embedding(&blob);
            if emb.len() == 1024 {
                embeddings.insert(fnord_id, emb);
            }
        }
    }
    info!(
        "Pairwise: loaded {} embeddings for {} articles",
        embeddings.len(),
        article_ids.len()
    );

    // Compute pairwise cosine similarity for all unique pairs
    let ids: Vec<i64> = embeddings.keys().copied().collect();
    let mut pairs = Vec::new();

    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            let (a_id, b_id) = if ids[i] < ids[j] {
                (ids[i], ids[j])
            } else {
                (ids[j], ids[i])
            };
            let sim = cosine_similarity(&embeddings[&ids[i]], &embeddings[&ids[j]]);
            if sim >= ANN_PREFILTER_THRESHOLD {
                pairs.push(ArticlePair {
                    fnord_id_a: a_id,
                    fnord_id_b: b_id,
                    embedding_similarity: sim,
                });
            }
        }
    }

    info!(
        "Pairwise: {} pairs above threshold {} from {} possible",
        pairs.len(),
        ANN_PREFILTER_THRESHOLD,
        ids.len() * (ids.len().saturating_sub(1)) / 2
    );

    Ok(pairs)
}

/// Search for articles by embedding similarity to a query
fn semantic_search_filter(
    conn: &rusqlite::Connection,
    query_embedding: &[f32],
    threshold: f64,
) -> Result<HashSet<i64>, rusqlite::Error> {
    let blob: Vec<u8> = query_embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect();

    let mut stmt = conn.prepare(
        "SELECT v.fnord_id, v.distance
         FROM vec_fnords v
         WHERE v.embedding MATCH ?1 AND k = 200
         ORDER BY v.distance ASC",
    )?;

    let results: HashSet<i64> = stmt
        .query_map(params![blob], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        })?
        .filter_map(|r| r.ok())
        .filter(|(_, distance)| {
            let sim = 1.0 - (distance / 2.0);
            sim >= threshold
        })
        .map(|(id, _)| id)
        .collect();

    Ok(results)
}

/// Search for articles by keyword name match (case-insensitive, supports partial match)
fn keyword_search_filter(
    conn: &rusqlite::Connection,
    query: &str,
) -> Result<HashSet<i64>, rusqlite::Error> {
    let pattern = format!("%{}%", query.trim().to_lowercase());
    let mut stmt = conn.prepare(
        "SELECT DISTINCT fi.fnord_id
         FROM fnord_immanentize fi
         JOIN immanentize i ON i.id = fi.immanentize_id
         WHERE LOWER(i.name) LIKE ?1",
    )?;
    let results: HashSet<i64> = stmt
        .query_map(params![pattern], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(results)
}

/// Load articles for a specific theme
fn load_theme_articles(
    conn: &rusqlite::Connection,
    theme_id: i64,
) -> Result<Vec<ThemeArticle>, rusqlite::Error> {
    // GROUP BY title + source to deduplicate articles that appear multiple times
    // in the same feed (RSS feeds sometimes publish the same article twice)
    let mut stmt = conn.prepare(
        "SELECT f.id, f.title, f.summary, COALESCE(p.title, p.url), f.political_bias,
                f.sachlichkeit, f.published_at, tra.topic_score
         FROM theme_report_articles tra
         JOIN fnords f ON f.id = tra.fnord_id
         JOIN pentacles p ON p.id = f.pentacle_id
         WHERE tra.theme_id = ?1
         GROUP BY f.title, COALESCE(p.title, p.url)
         ORDER BY f.published_at ASC",
    )?;

    let articles = stmt
        .query_map(params![theme_id], |row| {
            Ok(ThemeArticle {
                fnord_id: row.get(0)?,
                title: row.get(1)?,
                summary: row.get(2)?,
                source_name: row.get(3)?,
                political_bias: row.get(4)?,
                sachlichkeit: row.get(5)?,
                published_at: row.get::<_, String>(6).unwrap_or_default(),
                topic_score: row.get(7)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(articles)
}

// ============================================================
// PHASE 1: Multi-Signal Clustering (statistical, no LLM)
// ============================================================

fn run_phase1_clustering(
    conn: &rusqlite::Connection,
    articles: &[ArticleSignals],
    days: i32,
    min_sources: usize,
) -> CmdResult<Vec<ClusterCandidate>> {
    let article_ids: Vec<i64> = articles.iter().map(|a| a.fnord_id).collect();

    // Get ANN pairs (embedding pre-filter)
    let ann_pairs = get_ann_pairs(conn, &article_ids)?;

    info!(
        "Phase 1: {} ANN pairs for {} articles",
        ann_pairs.len(),
        articles.len()
    );

    // Build article lookup
    let article_map: HashMap<i64, &ArticleSignals> =
        articles.iter().map(|a| (a.fnord_id, a)).collect();
    let pentacle_map: HashMap<i64, i64> = articles
        .iter()
        .map(|a| (a.fnord_id, a.pentacle_id))
        .collect();

    let decay = decay_hours_for_days(days);

    // Calculate full topic scores for ANN-filtered pairs
    let mut distances: HashMap<(i64, i64), f64> = HashMap::new();
    for pair in &ann_pairs {
        let a = article_map.get(&pair.fnord_id_a);
        let b = article_map.get(&pair.fnord_id_b);
        if let (Some(a), Some(b)) = (a, b) {
            let score = topic_score(
                pair.embedding_similarity,
                &a.keyword_ids,
                &b.keyword_ids,
                &a.entity_ids,
                &b.entity_ids,
                &a.category_ids,
                &b.category_ids,
                &a.published_at,
                &b.published_at,
                decay,
            );
            let key = (pair.fnord_id_a, pair.fnord_id_b);
            distances.insert(key, 1.0 - score); // Convert score to distance
        }
    }

    let candidates = agglomerative_cluster(&article_ids, &distances, &pentacle_map, min_sources);
    info!("Phase 1: {} cluster candidates found", candidates.len());
    Ok(candidates)
}

// ============================================================
// PHASE 2: Batch Validation (Fast LLM)
// ============================================================

/// Per-cluster Phase-2 LLM response.
///
/// Phase 2 was reworked from one batched call (with `cluster_id` + `merge_with`
/// fields) to N parallel single-cluster calls — Fast models reliably mixed up
/// `cluster_id` ↔ `label` in the batched variant. With one cluster per call,
/// indexing is unambiguous.
#[derive(Debug, Deserialize)]
struct SingleValidation {
    valid: bool,
    label: Option<String>,
    #[allow(dead_code)]
    reason: Option<String>,
}

/// Validated cluster after Phase 2
#[derive(Debug, Clone)]
struct ValidatedTheme {
    label: String,
    article_ids: Vec<i64>,
    avg_topic_score: f64,
    source_count: usize,
}

/// Verify that an LLM-generated label is grounded in the cluster's content.
///
/// Returns true if at least one keyword name or entity name from the cluster's
/// articles appears (case-insensitive substring) in the label. This catches the
/// failure mode where the Fast LLM mixes up `cluster_id` ↔ `label` within the
/// batched validation prompt, attaching e.g. cluster 104's label to cluster 105.
///
/// Returns false if the cluster has no keywords/entities (Phase-2 fallback handles
/// that case) — naked clusters don't have grounding terms to match against.
fn label_matches_cluster(
    label: &str,
    article_ids: &[i64],
    article_map: &HashMap<i64, &ArticleSignals>,
    keyword_names: &HashMap<i64, String>,
    entity_names: &HashMap<i64, String>,
) -> bool {
    let label_lc = label.to_lowercase();
    if label_lc.trim().is_empty() {
        return false;
    }

    let mut terms: HashSet<String> = HashSet::new();
    for id in article_ids {
        let Some(a) = article_map.get(id) else {
            continue;
        };
        for kw_id in &a.keyword_ids {
            if let Some(name) = keyword_names.get(kw_id) {
                let n = name.trim().to_lowercase();
                if n.len() >= 3 {
                    terms.insert(n);
                }
            }
        }
        for (ent_id, _) in &a.entity_ids {
            if let Some(name) = entity_names.get(ent_id) {
                let n = name.trim().to_lowercase();
                if n.len() >= 3 {
                    terms.insert(n);
                }
            }
        }
    }

    if terms.is_empty() {
        return false;
    }

    terms.iter().any(|t| label_lc.contains(t))
}

/// Maximum number of parallel Phase-2 cluster validations.
///
/// Why 3: User runs ministral-3-class Fast LLMs on consumer hardware (typical
/// 12GB GPU). 3 concurrent calls keep latency low without saturating VRAM/CPU.
/// Effective parallelism is capped by `provider.suggested_concurrency()` so the
/// `ollama_concurrency` user setting stays the upper bound.
const PHASE2_PARALLELISM: usize = 3;

/// Render the prompt context for a single cluster (used by the per-cluster
/// Phase-2 prompt).
fn format_cluster_for_prompt(
    candidate: &ClusterCandidate,
    article_map: &HashMap<i64, &ArticleSignals>,
) -> String {
    let mut out = format!(
        "Cluster ({} articles, score {:.2}):\n",
        candidate.article_ids.len(),
        candidate.avg_topic_score
    );
    for (i, &id) in candidate.article_ids.iter().enumerate() {
        if let Some(a) = article_map.get(&id) {
            let summary_short = a
                .summary
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(150)
                .collect::<String>();
            let date_short = &a.published_at[..10.min(a.published_at.len())];
            out.push_str(&format!(
                "  [{}] \"{}\" ({}, {})\n      {}\n",
                i, a.title, a.source_name, date_short, summary_short
            ));
        }
    }
    out
}

/// Build a `ValidatedTheme` using the keyword-fallback label (first article
/// title, truncated). Used when the per-cluster LLM call fails, returns
/// `valid=false`, or produces a label that doesn't ground in the cluster.
fn fallback_theme(
    candidate: &ClusterCandidate,
    article_map: &HashMap<i64, &ArticleSignals>,
) -> ValidatedTheme {
    let label = candidate
        .article_ids
        .first()
        .and_then(|id| article_map.get(id))
        .map(|a| {
            let max_len = 60;
            if a.title.chars().count() > max_len {
                let truncated: String = a.title.chars().take(max_len).collect();
                format!("{}...", truncated)
            } else {
                a.title.clone()
            }
        })
        .unwrap_or_else(|| format!("Thema {}", candidate.cluster_id));

    ValidatedTheme {
        label,
        article_ids: candidate.article_ids.clone(),
        avg_topic_score: candidate.avg_topic_score,
        source_count: candidate.source_count,
    }
}

async fn run_phase2_validation(
    state: &State<'_, AppState>,
    candidates: &[ClusterCandidate],
    articles: &[ArticleSignals],
    keyword_names: &HashMap<i64, String>,
    entity_names: &HashMap<i64, String>,
    locale: &str,
) -> CmdResult<Vec<ValidatedTheme>> {
    use futures::stream::{self, StreamExt};
    use std::sync::Arc;

    if candidates.is_empty() {
        return Ok(vec![]);
    }

    let article_map: HashMap<i64, &ArticleSignals> =
        articles.iter().map(|a| (a.fnord_id, a)).collect();

    // Custom prompt template (or default) — read once before the parallel calls.
    let prompt_template = {
        let db = state.db_conn()?;
        db.conn()
            .query_row(
                "SELECT value FROM settings WHERE key = 'theme_validation_single_prompt'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
    }
    .unwrap_or_else(|| crate::ollama::DEFAULT_THEME_VALIDATION_SINGLE_PROMPT.to_string());

    let language = crate::ollama::get_language_for_locale(locale);

    // Create Fast provider once and share via Arc across all parallel calls.
    // `create_text_provider` already returns an `Arc<dyn AiTextProvider>`.
    let (provider, model) = {
        let db = state.db_conn()?;
        create_text_provider(&db, Some(&state.proxy_manager), TaskType::Fast)
    };
    let model: Arc<str> = Arc::from(model);

    let suggested = provider.suggested_concurrency();
    let parallelism = PHASE2_PARALLELISM.min(suggested.max(1));
    info!(
        "Phase 2: validating {} clusters per-cluster with parallelism={} (suggested={}, hard cap={})",
        candidates.len(),
        parallelism,
        suggested,
        PHASE2_PARALLELISM
    );

    // Pre-render each cluster's prompt and keep the candidate beside it.
    let prepared: Vec<(ClusterCandidate, String)> = candidates
        .iter()
        .map(|c| {
            let cluster_text = format_cluster_for_prompt(c, &article_map);
            let prompt = prompt_template
                .replace("{cluster}", &cluster_text)
                .replace("{language}", language);
            (c.clone(), prompt)
        })
        .collect();

    let schema = crate::ollama::theme_validation_single_schema();

    // Per-cluster validations in parallel.
    let results = stream::iter(prepared.into_iter())
        .map(|(candidate, prompt)| {
            let provider = provider.clone();
            let model = model.clone();
            let schema = schema.clone();
            async move {
                match provider.generate_text(&model, &prompt, Some(schema)).await {
                    Ok(result) => {
                        let preview = &result.text[..result.text.len().min(500)];
                        match serde_json::from_str::<SingleValidation>(&result.text) {
                            Ok(parsed) => (candidate, Ok(parsed), preview.to_string()),
                            Err(e) => (
                                candidate,
                                Err(format!("JSON parse error: {}", e)),
                                preview.to_string(),
                            ),
                        }
                    }
                    Err(e) => (candidate, Err(format!("LLM error: {}", e)), String::new()),
                }
            }
        })
        .buffer_unordered(parallelism)
        .collect::<Vec<_>>()
        .await;

    let mut themes: Vec<ValidatedTheme> = Vec::new();
    for (candidate, parsed, preview) in results {
        let cluster_id = candidate.cluster_id;
        match parsed {
            Ok(sv) => {
                if !sv.valid {
                    info!(
                        "Phase 2 cluster {}: valid=false — skipping (reason: {})",
                        cluster_id,
                        sv.reason.as_deref().unwrap_or("none")
                    );
                    continue;
                }
                let llm_label = sv
                    .label
                    .clone()
                    .unwrap_or_else(|| format!("Thema {}", cluster_id));
                let label = if label_matches_cluster(
                    &llm_label,
                    &candidate.article_ids,
                    &article_map,
                    keyword_names,
                    entity_names,
                ) {
                    llm_label
                } else {
                    warn!(
                        "Phase 2: label '{}' doesn't match cluster {} top keywords/entities — falling back",
                        llm_label, cluster_id
                    );
                    fallback_theme(&candidate, &article_map).label
                };
                info!(
                    "Phase 2 cluster {}: valid=true, label='{}'",
                    cluster_id, label
                );
                themes.push(ValidatedTheme {
                    label,
                    article_ids: candidate.article_ids.clone(),
                    avg_topic_score: candidate.avg_topic_score,
                    source_count: candidate.source_count,
                });
            }
            Err(err) => {
                warn!(
                    "Phase 2 cluster {}: {} — using keyword fallback (raw preview: {})",
                    cluster_id, err, preview
                );
                themes.push(fallback_theme(&candidate, &article_map));
            }
        }
    }

    // Result ordering: buffer_unordered yields in arbitrary order. Restore
    // a stable order by avg_topic_score so downstream sorting/cluster_id
    // assignment is deterministic.
    themes.sort_by(|a, b| {
        b.avg_topic_score
            .partial_cmp(&a.avg_topic_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(themes)
}

// ============================================================
// PHASE 2.5: Orphan-Attach Pass
// ============================================================

/// Attach in-period articles that did not land in any validated theme to the
/// nearest theme — provided the max pairwise `topic_score` against that theme's
/// articles exceeds `ORPHAN_ATTACH_THRESHOLD`.
///
/// Why: Phase 1 / Phase 2 strict cluster boundaries can drop late-arriving or
/// minority-source articles even when their topical fit is obvious (e.g. a
/// Sterlitamak refinery strike report with 4 keywords overlapping the
/// "Ukrainian drone strikes" theme).
///
/// Why this is safe to run after Phase 2:
/// - All themes already passed `MIN_SOURCE_COUNT` validation; we only grow
///   article_count, never shrink source_count.
/// - We never create new themes — orphans either find a strong host or stay
///   unassigned. `min_sources` cannot be circumvented.
/// - Threshold is conservative (0.5) — well above the ANN pre-filter (0.3).
///
/// Returns a map `theme_index_in_themes -> Vec<(orphan_id, attach_score)>`.
fn run_orphan_attach(
    conn: &rusqlite::Connection,
    themes: &mut [ValidatedTheme],
    articles: &[ArticleSignals],
    days: i32,
) -> HashMap<usize, Vec<(i64, f64)>> {
    if themes.is_empty() {
        return HashMap::new();
    }

    // Collect orphan ids: in `articles` but not in any theme.
    let assigned: HashSet<i64> = themes
        .iter()
        .flat_map(|t| t.article_ids.iter().copied())
        .collect();
    let orphans: Vec<&ArticleSignals> = articles
        .iter()
        .filter(|a| !assigned.contains(&a.fnord_id))
        .collect();

    if orphans.is_empty() {
        info!("Orphan-Attach: no orphans to consider");
        return HashMap::new();
    }

    // Load embeddings for orphans + all theme articles (one-shot).
    let needed_ids: HashSet<i64> = orphans
        .iter()
        .map(|a| a.fnord_id)
        .chain(themes.iter().flat_map(|t| t.article_ids.iter().copied()))
        .collect();

    let mut embeddings: HashMap<i64, Vec<f32>> = HashMap::new();
    if let Ok(mut stmt) =
        conn.prepare("SELECT embedding FROM fnords WHERE id = ?1 AND embedding IS NOT NULL")
    {
        for &fid in &needed_ids {
            let blob: Option<Vec<u8>> = stmt.query_row(params![fid], |row| row.get(0)).ok();
            if let Some(blob) = blob {
                let emb = blob_to_embedding(&blob);
                if emb.len() == 1024 {
                    embeddings.insert(fid, emb);
                }
            }
        }
    }

    attach_orphans_with_embeddings(themes, articles, &orphans, &embeddings, days)
}

/// Pure orphan-attach logic (no DB access) — exposed for unit testing.
///
/// For each orphan, find the theme with the highest max-pairwise `topic_score`
/// across that theme's articles. Attach iff that max score is at least
/// `ORPHAN_ATTACH_THRESHOLD`.
fn attach_orphans_with_embeddings(
    themes: &mut [ValidatedTheme],
    articles: &[ArticleSignals],
    orphans: &[&ArticleSignals],
    embeddings: &HashMap<i64, Vec<f32>>,
    days: i32,
) -> HashMap<usize, Vec<(i64, f64)>> {
    let mut attached_per_theme: HashMap<usize, Vec<(i64, f64)>> = HashMap::new();
    let decay = decay_hours_for_days(days);
    let total_orphans = orphans.len();
    let mut attached = 0usize;

    for orphan in orphans {
        let Some(orphan_emb) = embeddings.get(&orphan.fnord_id) else {
            continue;
        };

        let mut best: Option<(usize, f64)> = None;
        for (t_idx, theme) in themes.iter().enumerate() {
            let mut max_score = 0.0_f64;
            for theme_aid in &theme.article_ids {
                let Some(other) = articles.iter().find(|a| a.fnord_id == *theme_aid) else {
                    continue;
                };
                let Some(other_emb) = embeddings.get(theme_aid) else {
                    continue;
                };
                let emb_sim = cosine_similarity(orphan_emb, other_emb);
                let score = topic_score(
                    emb_sim,
                    &orphan.keyword_ids,
                    &other.keyword_ids,
                    &orphan.entity_ids,
                    &other.entity_ids,
                    &orphan.category_ids,
                    &other.category_ids,
                    &orphan.published_at,
                    &other.published_at,
                    decay,
                );
                if score > max_score {
                    max_score = score;
                }
            }
            if max_score >= ORPHAN_ATTACH_THRESHOLD
                && best.map(|(_, s)| max_score > s).unwrap_or(true)
            {
                best = Some((t_idx, max_score));
            }
        }

        if let Some((t_idx, score)) = best {
            themes[t_idx].article_ids.push(orphan.fnord_id);
            attached_per_theme
                .entry(t_idx)
                .or_default()
                .push((orphan.fnord_id, score));
            attached += 1;
        }
    }

    info!(
        "Orphan-Attach: {} of {} orphans attached (threshold {})",
        attached, total_orphans, ORPHAN_ATTACH_THRESHOLD
    );

    attached_per_theme
}

// ============================================================
// PHASE 3: Theme Report Generation (Reasoning LLM, per theme)
// ============================================================

async fn generate_single_report(
    state: &State<'_, AppState>,
    theme: &ValidatedTheme,
    articles: &[ArticleSignals],
    period_label: &str,
    locale: &str,
) -> CmdResult<String> {
    let article_map: HashMap<i64, &ArticleSignals> =
        articles.iter().map(|a| (a.fnord_id, a)).collect();

    // Build chronologically sorted article list grouped by day
    let mut theme_articles: Vec<&ArticleSignals> = theme
        .article_ids
        .iter()
        .filter_map(|id| article_map.get(id).copied())
        .collect();
    theme_articles.sort_by(|a, b| a.published_at.cmp(&b.published_at));

    let mut articles_text = String::new();
    let mut current_day = String::new();
    for (i, a) in theme_articles.iter().enumerate() {
        let day = a.published_at.get(..10).unwrap_or(&a.published_at);
        if day != current_day {
            current_day = day.to_string();
            articles_text.push_str(&format!("\n--- {} ---\n", day));
        }
        let summary = a.summary.as_deref().unwrap_or("(keine Zusammenfassung)");
        let bias = bias_label(a.political_bias);
        let sach = a.sachlichkeit.unwrap_or(0);
        articles_text.push_str(&format!(
            "[{}] \"{}\" ({}, {}, Bias: {}, Sachlichkeit: {}/4)\n    {}\n",
            i, a.title, a.source_name, &a.published_at, bias, sach, summary
        ));
    }

    // Get prompt template
    let prompt_template = {
        let db = state.db_conn()?;
        db.conn()
            .query_row(
                "SELECT value FROM settings WHERE key = 'theme_report_prompt'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
    };

    let language = crate::ollama::get_language_for_locale(locale);
    let prompt = prompt_template
        .unwrap_or_else(|| crate::ollama::DEFAULT_THEME_REPORT_PROMPT.to_string())
        .replace("{label}", &theme.label)
        .replace("{period}", period_label)
        .replace("{articles}", &articles_text)
        .replace("{language}", language);

    // Create Reasoning provider (short lock, then release)
    let (provider, model) = {
        let db = state.db_conn()?;
        create_text_provider(&db, Some(&state.proxy_manager), TaskType::Reasoning)
    };
    // DB lock released

    let schema = crate::ollama::theme_report_schema();
    let result = provider
        .generate_text(&model, &prompt, Some(schema))
        .await
        .map_err(|e| FuckupError::Generic(format!("Theme report LLM error: {}", e)))?;

    Ok(result.text)
}

// ============================================================
// TAURI COMMANDS
// ============================================================

/// Generate a full theme report for the given time range
#[tauri::command]
pub async fn generate_theme_report(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    days: i32,
    search_query: Option<String>,
    min_sources: Option<usize>,
) -> CmdResult<ThemeReportDetail> {
    let days = days.clamp(1, 14);
    let min_sources = min_sources.unwrap_or(MIN_SOURCE_COUNT).clamp(1, 5);
    info!(
        "Generating theme report (days={}, search={:?}, min_sources={})",
        days, search_query, min_sources
    );

    // Calculate period
    let now = chrono::Utc::now();
    let period_start = now - chrono::Duration::days(days as i64);
    let period_start_str = period_start.format("%Y-%m-%d %H:%M:%S").to_string();
    let period_end_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let period_label = if days == 1 {
        format!("{} (24h)", now.format("%d.%m.%Y"))
    } else {
        format!(
            "{} – {}",
            period_start.format("%d.%m.%Y"),
            now.format("%d.%m.%Y")
        )
    };

    // Get locale
    let locale = {
        let db = state.db_conn()?;
        db.conn()
            .query_row(
                "SELECT value FROM settings WHERE key = 'locale'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_else(|_| "de".to_string())
    };

    // Optional: hybrid keyword + semantic search filter
    let search_ids = if let Some(ref query) = search_query {
        if !query.trim().is_empty() {
            // 1. Keyword-based search (fast, exact match via immanentize)
            let keyword_ids = {
                let db = state.db_conn()?;
                keyword_search_filter(db.conn(), query).unwrap_or_default()
            };
            info!(
                "Search: {} keyword matches for '{}'",
                keyword_ids.len(),
                query
            );

            // 2. Semantic embedding search (catches near-matches)
            let embedding_ids = {
                let embedding_provider = {
                    let db = state.db_conn()?;
                    create_embedding_provider_from_db(&db, Some(&state.proxy_manager))
                };
                match embedding_provider.generate_embedding(query).await {
                    Ok(emb) => {
                        let db = state.db_conn()?;
                        semantic_search_filter(db.conn(), &emb, 0.3).unwrap_or_default()
                    }
                    Err(e) => {
                        warn!(
                            "Search embedding failed: {}. Using keyword matches only.",
                            e
                        );
                        HashSet::new()
                    }
                }
            };
            info!(
                "Search: {} semantic matches for '{}'",
                embedding_ids.len(),
                query
            );

            // 3. Union both sets
            let mut combined = keyword_ids;
            combined.extend(embedding_ids);
            info!(
                "Search: {} combined unique matches for '{}'",
                combined.len(),
                query
            );

            if combined.is_empty() {
                None
            } else {
                Some(combined)
            }
        } else {
            None
        }
    } else {
        None
    };

    // Load articles with signals (short lock)
    let articles = {
        let db = state.db_conn()?;
        load_articles_with_signals(
            db.conn(),
            &period_start_str,
            &period_end_str,
            search_ids.as_ref(),
        )?
    };

    if articles.len() < MIN_ARTICLES_FOR_REPORT {
        return Err(FuckupError::Validation(format!(
            "Nicht genügend analysierte Artikel im Zeitraum ({} gefunden, {} benötigt).",
            articles.len(),
            MIN_ARTICLES_FOR_REPORT
        )));
    }

    // Phase 1: Statistical clustering (short lock for ANN queries)
    // Also load keyword/entity names upfront so Phase 2 can validate LLM labels
    // without re-acquiring the DB lock.
    let (candidates, keyword_names, entity_names) = {
        let db = state.db_conn()?;
        let candidates = run_phase1_clustering(db.conn(), &articles, days, min_sources)?;
        let (kw_names, ent_names) = load_keyword_entity_names(db.conn(), &articles)?;
        (candidates, kw_names, ent_names)
    };

    if candidates.is_empty() {
        return Err(FuckupError::Validation("Keine Themen-Cluster erkannt. Versuche einen längeren Zeitraum oder einen anderen Suchbegriff.".to_string()));
    }

    // Phase 2: LLM validation (async, no lock held)
    let mut themes = run_phase2_validation(
        &state,
        &candidates,
        &articles,
        &keyword_names,
        &entity_names,
        &locale,
    )
    .await?;

    if themes.is_empty() {
        return Err(FuckupError::Validation(
            "Keine validen Themen nach LLM-Validierung.".to_string(),
        ));
    }

    // Phase 2.5: Orphan-Attach pass — attach in-period articles that did not
    // land in any theme but show strong topical fit to one. Updates source_count
    // for each affected theme.
    let orphan_attachments = {
        let db = state.db_conn()?;
        let attached = run_orphan_attach(db.conn(), &mut themes, &articles, days);
        let pentacle_map: HashMap<i64, i64> =
            articles.iter().map(|a| (a.fnord_id, a.pentacle_id)).collect();
        for theme in themes.iter_mut() {
            theme.source_count = theme
                .article_ids
                .iter()
                .filter_map(|id| pentacle_map.get(id).copied())
                .collect::<HashSet<_>>()
                .len();
        }
        attached
    };

    // Get model name for DB
    let model_used = {
        let db = state.db_conn()?;
        let (_, model) = create_text_provider(&db, Some(&state.proxy_manager), TaskType::Reasoning);
        Some(model)
    };

    // Save report + themes to DB (short lock)
    let report_id = {
        let db = state.db_conn()?;
        let conn = db.conn();

        conn.execute(
            "INSERT OR REPLACE INTO theme_reports
             (period_start, period_end, search_query, theme_count, model_used, locale)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                period_start_str,
                period_end_str,
                search_query.as_deref().unwrap_or(""),
                themes.len() as i32,
                model_used,
                locale
            ],
        )?;

        conn.last_insert_rowid()
    };

    // Save themes and articles (short lock).
    // Orphan-attached articles use their actual computed attach_score instead
    // of the cluster avg, which preserves the per-article quality signal.
    let theme_ids: Vec<i64> = {
        let db = state.db_conn()?;
        let conn = db.conn();
        let mut ids = Vec::new();

        for (t_idx, theme) in themes.iter().enumerate() {
            conn.execute(
                "INSERT INTO theme_report_themes
                 (report_id, label, report_status, cluster_score, article_count, source_count)
                 VALUES (?1, ?2, 'pending', ?3, ?4, ?5)",
                params![
                    report_id,
                    theme.label,
                    theme.avg_topic_score,
                    theme.article_ids.len() as i32,
                    theme.source_count as i32,
                ],
            )?;

            let theme_id = conn.last_insert_rowid();
            ids.push(theme_id);

            let orphan_scores: HashMap<i64, f64> = orphan_attachments
                .get(&t_idx)
                .map(|v| v.iter().copied().collect())
                .unwrap_or_default();

            for &fnord_id in &theme.article_ids {
                let score = orphan_scores
                    .get(&fnord_id)
                    .copied()
                    .unwrap_or(theme.avg_topic_score);
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO theme_report_articles (theme_id, fnord_id, topic_score)
                     VALUES (?1, ?2, ?3)",
                    params![theme_id, fnord_id, score],
                );
            }
        }
        ids
    };

    // Phase 3: Generate reports per theme (sequential, async)
    for (i, (theme, &theme_id)) in themes.iter().zip(theme_ids.iter()).enumerate() {
        // Emit progress
        let _ = app_handle.emit(
            "theme-report-progress",
            ThemeProgress {
                report_id,
                themes_complete: i,
                themes_total: themes.len(),
                current_theme: theme.label.clone(),
            },
        );

        // Update status to generating (short lock)
        {
            let db = state.db_conn()?;
            let _ = db.conn().execute(
                "UPDATE theme_report_themes SET report_status = 'generating' WHERE id = ?1",
                params![theme_id],
            );
        }

        match generate_single_report(&state, theme, &articles, &period_label, &locale).await {
            Ok(json) => {
                let headline = serde_json::from_str::<serde_json::Value>(&json)
                    .ok()
                    .and_then(|v| v["headline"].as_str().map(|s| s.to_string()));

                let db = state.db_conn()?;
                let _ = db.conn().execute(
                    "UPDATE theme_report_themes
                     SET report_json = ?1, report_status = 'complete', headline = ?2
                     WHERE id = ?3",
                    params![json, headline, theme_id],
                );
            }
            Err(e) => {
                error!("Theme report failed for '{}': {}", theme.label, e);
                let db = state.db_conn()?;
                let _ = db.conn().execute(
                    "UPDATE theme_report_themes SET report_status = 'failed' WHERE id = ?1",
                    params![theme_id],
                );
            }
        }
    }

    // Emit final progress
    let _ = app_handle.emit(
        "theme-report-progress",
        ThemeProgress {
            report_id,
            themes_complete: themes.len(),
            themes_total: themes.len(),
            current_theme: String::new(),
        },
    );

    // Return full report detail
    get_theme_report_detail(state, report_id).await
}

/// List all theme reports, newest first
#[tauri::command]
pub async fn get_theme_reports(
    state: State<'_, AppState>,
    limit: Option<i32>,
) -> CmdResult<Vec<ThemeReportSummary>> {
    let limit = limit.unwrap_or(20).min(50);
    let db = state.db_conn()?;
    let conn = db.conn();

    let mut stmt = conn.prepare(
        "SELECT id, period_start, period_end, search_query, theme_count,
                model_used, locale, created_at
         FROM theme_reports ORDER BY created_at DESC LIMIT ?1",
    )?;

    let reports: Vec<ThemeReportSummary> = stmt
        .query_map(params![limit], |row| {
            Ok(ThemeReportSummary {
                id: row.get(0)?,
                period_start: row.get(1)?,
                period_end: row.get(2)?,
                search_query: row.get(3)?,
                theme_count: row.get(4)?,
                model_used: row.get(5)?,
                locale: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(reports)
}

/// Get a single theme report with all themes and articles
#[tauri::command]
pub async fn get_theme_report_detail(
    state: State<'_, AppState>,
    report_id: i64,
) -> CmdResult<ThemeReportDetail> {
    let db = state.db_conn()?;
    let conn = db.conn();

    let report = conn.query_row(
        "SELECT id, period_start, period_end, search_query, theme_count,
                model_used, locale, created_at
         FROM theme_reports WHERE id = ?1",
        params![report_id],
        |row| {
            Ok(ThemeReportSummary {
                id: row.get(0)?,
                period_start: row.get(1)?,
                period_end: row.get(2)?,
                search_query: row.get(3)?,
                theme_count: row.get(4)?,
                model_used: row.get(5)?,
                locale: row.get(6)?,
                created_at: row.get(7)?,
            })
        },
    )?;

    // Load themes with their articles
    let mut stmt = conn.prepare(
        "SELECT id, label, headline, report_json, report_status, cluster_score,
                article_count, source_count
         FROM theme_report_themes WHERE report_id = ?1
         ORDER BY cluster_score DESC",
    )?;

    #[allow(clippy::type_complexity)]
    let theme_rows: Vec<(
        i64,
        String,
        Option<String>,
        Option<String>,
        String,
        f64,
        i32,
        i32,
    )> = stmt
        .query_map(params![report_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let themes: Vec<ThemeReportTheme> = theme_rows
        .into_iter()
        .map(
            |(
                theme_id,
                label,
                headline,
                report_json,
                report_status,
                cluster_score,
                article_count,
                source_count,
            )| {
                let articles = load_theme_articles(conn, theme_id).unwrap_or_default();
                ThemeReportTheme {
                    id: theme_id,
                    label,
                    headline,
                    report_json,
                    report_status,
                    cluster_score,
                    article_count,
                    source_count,
                    articles,
                }
            },
        )
        .collect();

    Ok(ThemeReportDetail { report, themes })
}

/// Retry report generation for a specific theme
#[tauri::command]
pub async fn retry_theme_analysis(
    state: State<'_, AppState>,
    theme_id: i64,
) -> CmdResult<ThemeReportTheme> {
    // Load theme info and articles (short lock)
    let (label, article_ids, period_start, period_end, locale) = {
        let db = state.db_conn()?;
        let conn = db.conn();

        let (report_id, label) = conn.query_row(
            "SELECT report_id, label FROM theme_report_themes WHERE id = ?1",
            params![theme_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )?;

        let (period_start, period_end, locale) = conn.query_row(
            "SELECT period_start, period_end, locale FROM theme_reports WHERE id = ?1",
            params![report_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;

        let articles_raw = load_theme_articles(conn, theme_id)?;
        let article_ids: Vec<i64> = articles_raw.iter().map(|a| a.fnord_id).collect();

        (label, article_ids, period_start, period_end, locale)
    };
    // DB lock released

    // Load full signals (short lock)
    let theme_articles = {
        let db = state.db_conn()?;
        let all_articles = load_articles_with_signals(db.conn(), &period_start, &period_end, None)?;
        all_articles
            .into_iter()
            .filter(|a| article_ids.contains(&a.fnord_id))
            .collect::<Vec<_>>()
    };
    // DB lock released

    let period_label = format!("{} – {}", &period_start[..10], &period_end[..10]);
    let validated = ValidatedTheme {
        label,
        article_ids,
        avg_topic_score: 0.0,
        source_count: 0,
    };

    // Update status to generating (short lock)
    {
        let db = state.db_conn()?;
        let _ = db.conn().execute(
            "UPDATE theme_report_themes SET report_status = 'generating' WHERE id = ?1",
            params![theme_id],
        );
    }

    match generate_single_report(&state, &validated, &theme_articles, &period_label, &locale).await
    {
        Ok(json) => {
            let headline = serde_json::from_str::<serde_json::Value>(&json)
                .ok()
                .and_then(|v| v["headline"].as_str().map(|s| s.to_string()));

            let db = state.db_conn()?;
            let _ = db.conn().execute(
                "UPDATE theme_report_themes
                 SET report_json = ?1, report_status = 'complete', headline = ?2
                 WHERE id = ?3",
                params![json, headline, theme_id],
            );
        }
        Err(e) => {
            let db = state.db_conn()?;
            let _ = db.conn().execute(
                "UPDATE theme_report_themes SET report_status = 'failed' WHERE id = ?1",
                params![theme_id],
            );
            return Err(FuckupError::Generic(format!("Retry failed: {}", e)));
        }
    }

    // Return updated theme (short lock)
    let db = state.db_conn()?;
    let conn = db.conn();
    let articles = load_theme_articles(conn, theme_id)?;

    conn.query_row(
        "SELECT id, label, headline, report_json, report_status, cluster_score,
                article_count, source_count
         FROM theme_report_themes WHERE id = ?1",
        params![theme_id],
        |row| {
            Ok(ThemeReportTheme {
                id: row.get(0)?,
                label: row.get(1)?,
                headline: row.get(2)?,
                report_json: row.get(3)?,
                report_status: row.get(4)?,
                cluster_score: row.get(5)?,
                article_count: row.get(6)?,
                source_count: row.get(7)?,
                articles,
            })
        },
    )
    .map_err(FuckupError::from)
}

/// Delete a theme report and all its themes/articles (CASCADE)
#[tauri::command]
pub async fn delete_theme_report(state: State<'_, AppState>, report_id: i64) -> CmdResult<bool> {
    let db = state.db_conn()?;
    let deleted = db.conn().execute(
        "DELETE FROM theme_reports WHERE id = ?1",
        params![report_id],
    )?;

    if deleted > 0 {
        info!("Deleted theme report {}", report_id);
    }

    Ok(deleted > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_article(
        id: i64,
        keyword_ids: Vec<i64>,
        entity_ids: Vec<(i64, &str)>,
    ) -> ArticleSignals {
        ArticleSignals {
            fnord_id: id,
            pentacle_id: id,
            title: format!("Article {}", id),
            summary: None,
            published_at: "2026-04-16 12:00:00".to_string(),
            political_bias: None,
            sachlichkeit: None,
            source_name: format!("Source {}", id),
            category_ids: vec![],
            keyword_ids,
            entity_ids: entity_ids
                .into_iter()
                .map(|(eid, t)| (eid, t.to_string()))
                .collect(),
        }
    }

    #[test]
    fn single_validation_parses_valid_response() {
        let json = r#"{"valid": true, "label": "Russische Angriffe auf Ukraine", "reason": null}"#;
        let parsed: SingleValidation = serde_json::from_str(json).unwrap();
        assert!(parsed.valid);
        assert_eq!(parsed.label.as_deref(), Some("Russische Angriffe auf Ukraine"));
    }

    #[test]
    fn single_validation_parses_invalid_with_reason() {
        let json = r#"{"valid": false, "reason": "articles cover unrelated topics"}"#;
        let parsed: SingleValidation = serde_json::from_str(json).unwrap();
        assert!(!parsed.valid);
        assert!(parsed.label.is_none());
    }

    #[test]
    fn single_validation_tolerates_missing_optional_fields() {
        // LLM returns only the required `valid` field.
        let json = r#"{"valid": true}"#;
        let parsed: SingleValidation = serde_json::from_str(json).unwrap();
        assert!(parsed.valid);
        assert!(parsed.label.is_none());
        assert!(parsed.reason.is_none());
    }

    fn make_theme(article_ids: Vec<i64>) -> ValidatedTheme {
        ValidatedTheme {
            label: "Test".to_string(),
            article_ids,
            avg_topic_score: 0.7,
            source_count: 2,
        }
    }

    #[test]
    fn orphan_attaches_when_score_meets_threshold() {
        // Theme article (id=1) and orphan (id=99) share 3 of 3 keywords,
        // same published_at, same category → topic_score well above 0.5
        // even with modest embedding similarity.
        let host = make_article(1, vec![10, 11, 12], vec![]);
        let orphan = make_article(99, vec![10, 11, 12], vec![]);
        let articles = vec![host, orphan];
        let orphan_refs: Vec<&ArticleSignals> =
            articles.iter().filter(|a| a.fnord_id == 99).collect();

        let mut embeddings: HashMap<i64, Vec<f32>> = HashMap::new();
        embeddings.insert(1, vec![1.0, 0.0]);
        embeddings.insert(99, vec![1.0, 0.0]); // identical → emb_sim = 1.0

        let mut themes = vec![make_theme(vec![1])];
        let attached =
            attach_orphans_with_embeddings(&mut themes, &articles, &orphan_refs, &embeddings, 1);

        assert_eq!(themes[0].article_ids, vec![1, 99]);
        let entries = attached.get(&0).expect("theme 0 should have attachments");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, 99);
        assert!(
            entries[0].1 >= ORPHAN_ATTACH_THRESHOLD,
            "attach score {} below threshold {}",
            entries[0].1,
            ORPHAN_ATTACH_THRESHOLD
        );
    }

    #[test]
    fn orphan_skipped_when_score_below_threshold() {
        // Orthogonal embeddings, no shared keywords/entities/categories,
        // and a large temporal gap → topic_score well below 0.5.
        let mut host = make_article(1, vec![10], vec![]);
        host.published_at = "2026-04-01 00:00:00".to_string();
        let mut orphan = make_article(99, vec![42], vec![]);
        orphan.published_at = "2026-04-16 12:00:00".to_string();
        let articles = vec![host, orphan];
        let orphan_refs: Vec<&ArticleSignals> =
            articles.iter().filter(|a| a.fnord_id == 99).collect();

        let mut embeddings: HashMap<i64, Vec<f32>> = HashMap::new();
        embeddings.insert(1, vec![1.0, 0.0]);
        embeddings.insert(99, vec![0.0, 1.0]); // orthogonal → emb_sim = 0.0

        let mut themes = vec![make_theme(vec![1])];
        let attached =
            attach_orphans_with_embeddings(&mut themes, &articles, &orphan_refs, &embeddings, 1);

        assert_eq!(
            themes[0].article_ids,
            vec![1],
            "orphan must not be attached"
        );
        assert!(
            attached.is_empty(),
            "attached map must be empty, got {:?}",
            attached
        );
    }

    #[test]
    fn orphan_picks_best_of_multiple_themes() {
        // Two themes: theme 0 shares a keyword with the orphan, theme 1 does not.
        // Orphan must land in theme 0, even though both have identical embeddings.
        let host_a = make_article(1, vec![10, 11], vec![]);
        let host_b = make_article(2, vec![20, 21], vec![]);
        let orphan = make_article(99, vec![10, 11], vec![]);
        let articles = vec![host_a, host_b, orphan];
        let orphan_refs: Vec<&ArticleSignals> =
            articles.iter().filter(|a| a.fnord_id == 99).collect();

        let mut embeddings: HashMap<i64, Vec<f32>> = HashMap::new();
        embeddings.insert(1, vec![1.0, 0.0]);
        embeddings.insert(2, vec![1.0, 0.0]);
        embeddings.insert(99, vec![1.0, 0.0]);

        let mut themes = vec![make_theme(vec![1]), make_theme(vec![2])];
        let attached =
            attach_orphans_with_embeddings(&mut themes, &articles, &orphan_refs, &embeddings, 1);

        assert_eq!(themes[0].article_ids, vec![1, 99]);
        assert_eq!(themes[1].article_ids, vec![2]);
        assert!(attached.contains_key(&0));
        assert!(!attached.contains_key(&1));
    }

    #[test]
    fn label_matches_when_keyword_appears() {
        let a = make_article(1, vec![100], vec![]);
        let b = make_article(2, vec![100, 101], vec![]);
        let articles = vec![a, b];
        let map: HashMap<i64, &ArticleSignals> =
            articles.iter().map(|a| (a.fnord_id, a)).collect();

        let mut keyword_names = HashMap::new();
        keyword_names.insert(100, "Russland".to_string());
        keyword_names.insert(101, "Ukraine".to_string());
        let entity_names = HashMap::new();

        assert!(label_matches_cluster(
            "Russische Angriffe auf Ukraine",
            &[1, 2],
            &map,
            &keyword_names,
            &entity_names,
        ));
    }

    #[test]
    fn label_rejected_when_disjoint() {
        // FIFA-Cluster bekommt fälschlich ein Russland-Label zugewiesen.
        let a = make_article(1, vec![300], vec![]);
        let b = make_article(2, vec![301], vec![]);
        let articles = vec![a, b];
        let map: HashMap<i64, &ArticleSignals> =
            articles.iter().map(|a| (a.fnord_id, a)).collect();

        let mut keyword_names = HashMap::new();
        keyword_names.insert(300, "FIFA".to_string());
        keyword_names.insert(301, "Weltmeisterschaft".to_string());
        let entity_names = HashMap::new();

        assert!(!label_matches_cluster(
            "Russische Angriffe auf Ukraine",
            &[1, 2],
            &map,
            &keyword_names,
            &entity_names,
        ));
    }

    #[test]
    fn label_rejected_when_empty_terms() {
        // Cluster ohne Keywords/Entities — keine Grundlage für Labelprüfung,
        // muss in den Fallback fallen.
        let a = make_article(1, vec![], vec![]);
        let articles = vec![a];
        let map: HashMap<i64, &ArticleSignals> =
            articles.iter().map(|a| (a.fnord_id, a)).collect();

        let keyword_names = HashMap::new();
        let entity_names = HashMap::new();

        assert!(!label_matches_cluster(
            "Irgendein Label",
            &[1],
            &map,
            &keyword_names,
            &entity_names,
        ));
    }
}
