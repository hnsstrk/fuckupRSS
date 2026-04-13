//! Multi-Signal Topic Clustering for Theme Reports
//!
//! Combines 5 signals (embedding similarity, keyword overlap, NER entity overlap,
//! category match, temporal proximity) into a weighted topic score.
//! Uses agglomerative clustering with average-linkage and dynamic cut.

use chrono::NaiveDateTime;
use std::collections::{HashMap, HashSet};

// ============================================================
// CONSTANTS
// ============================================================

/// Minimum articles with embeddings required to generate a report
pub const MIN_ARTICLES_FOR_REPORT: usize = 5;

/// Minimum articles per cluster candidate
pub const MIN_CLUSTER_SIZE: usize = 2;

/// Minimum unique sources per cluster candidate.
/// A theme reported by only one source is not significant enough.
pub const MIN_SOURCE_COUNT: usize = 2;

/// Signal weights for topic score calculation
const W_EMBEDDING: f64 = 0.35;
const W_KEYWORD: f64 = 0.25;
const W_ENTITY: f64 = 0.20;
const W_CATEGORY: f64 = 0.10;
const W_TEMPORAL: f64 = 0.10;

/// Distance threshold for cluster merging (1.0 - min_topic_score)
const MERGE_DISTANCE_THRESHOLD: f64 = 0.55;

/// Dynamic cut: stop merging if distance jump exceeds this factor
const DYNAMIC_CUT_FACTOR: f64 = 1.5;

/// Minimum embedding similarity for ANN pre-filter
pub const ANN_PREFILTER_THRESHOLD: f64 = 0.3;

/// Bonus multiplier when both person AND org entities overlap
const PERSON_ORG_BONUS: f64 = 1.3;

// ============================================================
// TYPES
// ============================================================

/// An article with all signals needed for topic clustering
#[derive(Debug, Clone)]
pub struct ArticleSignals {
    pub fnord_id: i64,
    pub pentacle_id: i64,
    pub title: String,
    pub summary: Option<String>,
    pub published_at: String,
    pub political_bias: Option<i32>,
    pub sachlichkeit: Option<i32>,
    pub source_name: String,
    pub category_ids: Vec<i64>,
    pub keyword_ids: Vec<i64>,
    pub entity_ids: Vec<(i64, String)>, // (entity_id, entity_type)
}

/// A pre-computed pair similarity from ANN search
#[derive(Debug, Clone)]
pub struct ArticlePair {
    pub fnord_id_a: i64,
    pub fnord_id_b: i64,
    pub embedding_similarity: f64,
}

/// Result of the clustering phase
#[derive(Debug, Clone)]
pub struct ClusterCandidate {
    pub cluster_id: usize,
    pub article_ids: Vec<i64>,
    pub avg_topic_score: f64,
    pub source_count: usize,
}

/// Decay hours for temporal proximity based on report range
pub fn decay_hours_for_days(days: i32) -> f64 {
    match days {
        1 => 12.0,
        2..=3 => 24.0,
        4..=7 => 48.0,
        _ => 96.0,
    }
}

// ============================================================
// SIGNAL CALCULATIONS
// ============================================================

/// Jaccard index of keyword ID sets. Returns 0.0 if both empty.
pub fn keyword_overlap(kw_a: &[i64], kw_b: &[i64]) -> f64 {
    if kw_a.is_empty() && kw_b.is_empty() {
        return 0.0;
    }

    let set_a: HashSet<i64> = kw_a.iter().copied().collect();
    let set_b: HashSet<i64> = kw_b.iter().copied().collect();

    let intersection = set_a.intersection(&set_b).count() as f64;
    let union = set_a.union(&set_b).count() as f64;

    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

/// Jaccard index of entity IDs with person+org bonus.
/// If shared entities include both a "person" AND an "organization" type,
/// multiply by PERSON_ORG_BONUS (capped at 1.0). Returns 0.0 if both empty.
pub fn entity_overlap(ents_a: &[(i64, String)], ents_b: &[(i64, String)]) -> f64 {
    if ents_a.is_empty() && ents_b.is_empty() {
        return 0.0;
    }

    let ids_a: HashSet<i64> = ents_a.iter().map(|(id, _)| *id).collect();
    let ids_b: HashSet<i64> = ents_b.iter().map(|(id, _)| *id).collect();

    let intersection: HashSet<i64> = ids_a.intersection(&ids_b).copied().collect();
    let union = ids_a.union(&ids_b).count() as f64;

    if union == 0.0 {
        return 0.0;
    }

    let jaccard = intersection.len() as f64 / union;

    // Check for person+org bonus: shared entities must include both types
    let all_entities: HashMap<i64, String> = ents_a
        .iter()
        .chain(ents_b.iter())
        .map(|(id, t)| (*id, t.clone()))
        .collect();

    let shared_types: HashSet<&str> = intersection
        .iter()
        .filter_map(|id| all_entities.get(id).map(|t| t.as_str()))
        .collect();

    let has_person = shared_types.contains("person");
    let has_org = shared_types.contains("organization");

    if has_person && has_org {
        (jaccard * PERSON_ORG_BONUS).min(1.0)
    } else {
        jaccard
    }
}

/// Category match score.
/// 1.0 if any exact category ID match.
/// 0.5 if same hundreds-group (e.g. 201 and 202 -> parent 200).
/// 0.0 otherwise.
pub fn category_match(cats_a: &[i64], cats_b: &[i64]) -> f64 {
    if cats_a.is_empty() || cats_b.is_empty() {
        return 0.0;
    }

    let set_a: HashSet<i64> = cats_a.iter().copied().collect();
    let set_b: HashSet<i64> = cats_b.iter().copied().collect();

    // Check exact match
    if set_a.intersection(&set_b).next().is_some() {
        return 1.0;
    }

    // Check same hundreds-group
    let parents_a: HashSet<i64> = set_a.iter().map(|c| (c / 100) * 100).collect();
    let parents_b: HashSet<i64> = set_b.iter().map(|c| (c / 100) * 100).collect();

    if parents_a.intersection(&parents_b).next().is_some() {
        return 0.5;
    }

    0.0
}

/// Parse a datetime string in multiple supported formats.
fn parse_datetime(s: &str) -> Option<NaiveDateTime> {
    // Try "%Y-%m-%d %H:%M:%S"
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(dt);
    }
    // Try "%Y-%m-%dT%H:%M:%S"
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt);
    }
    // Try "%Y-%m-%dT%H:%M:%S%.fZ" — strip trailing Z and parse with fractional seconds
    let trimmed = s.trim_end_matches('Z');
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(dt);
    }
    // Also try without fractional seconds after stripping Z
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt);
    }
    None
}

/// Temporal proximity using exponential decay.
/// `exp(-hours_apart / decay_hours)`.
/// Returns 0.5 as fallback if dates can't be parsed.
pub fn temporal_proximity(published_a: &str, published_b: &str, decay_hours: f64) -> f64 {
    let dt_a = match parse_datetime(published_a) {
        Some(dt) => dt,
        None => return 0.5,
    };
    let dt_b = match parse_datetime(published_b) {
        Some(dt) => dt,
        None => return 0.5,
    };

    let diff = dt_a.signed_duration_since(dt_b);
    let hours_apart = diff.num_seconds().unsigned_abs() as f64 / 3600.0;

    (-hours_apart / decay_hours).exp()
}

/// Weighted topic score combining all 5 signals.
///
/// 0.35*embedding + 0.25*keyword + 0.20*entity + 0.10*category + 0.10*temporal
#[allow(clippy::too_many_arguments)]
pub fn topic_score(
    embedding_sim: f64,
    kw_a: &[i64],
    kw_b: &[i64],
    ents_a: &[(i64, String)],
    ents_b: &[(i64, String)],
    cats_a: &[i64],
    cats_b: &[i64],
    pub_a: &str,
    pub_b: &str,
    decay_hours: f64,
) -> f64 {
    let kw = keyword_overlap(kw_a, kw_b);
    let ent = entity_overlap(ents_a, ents_b);
    let cat = category_match(cats_a, cats_b);
    let temp = temporal_proximity(pub_a, pub_b, decay_hours);

    W_EMBEDDING * embedding_sim
        + W_KEYWORD * kw
        + W_ENTITY * ent
        + W_CATEGORY * cat
        + W_TEMPORAL * temp
}

// ============================================================
// AGGLOMERATIVE CLUSTERING
// ============================================================

/// Agglomerative clustering with average-linkage and dynamic cut.
///
/// # Arguments
/// * `article_ids` - All article IDs to cluster
/// * `distances` - Pre-computed pairwise distances: key = (smaller_id, larger_id), value = distance
/// * `pentacle_map` - fnord_id -> pentacle_id mapping for source counting
///
/// # Algorithm
/// 1. Initialize: each article = own cluster
/// 2. Loop: find closest pair of clusters using average-linkage
/// 3. Stop conditions: (a) best distance > MERGE_DISTANCE_THRESHOLD, (b) dynamic cut
/// 4. Merge the two closest clusters
/// 5. Build results: only clusters with >= MIN_CLUSTER_SIZE articles
pub fn agglomerative_cluster(
    article_ids: &[i64],
    distances: &HashMap<(i64, i64), f64>,
    pentacle_map: &HashMap<i64, i64>,
) -> Vec<ClusterCandidate> {
    if article_ids.is_empty() {
        return vec![];
    }

    // Initialize: each article is its own cluster
    let mut clusters: HashMap<usize, Vec<i64>> = HashMap::new();
    let mut id_to_cluster: HashMap<i64, usize> = HashMap::new();

    for (i, &aid) in article_ids.iter().enumerate() {
        clusters.insert(i, vec![aid]);
        id_to_cluster.insert(aid, i);
    }

    let mut next_cluster_id = article_ids.len();
    let mut prev_merge_distance = 0.0_f64;

    loop {
        // Find the closest pair of clusters using average-linkage
        let active_cluster_ids: Vec<usize> = clusters.keys().copied().collect();

        if active_cluster_ids.len() <= 1 {
            break;
        }

        let mut best_distance = f64::MAX;
        let mut best_pair: Option<(usize, usize)> = None;

        for i in 0..active_cluster_ids.len() {
            for j in (i + 1)..active_cluster_ids.len() {
                let c1 = active_cluster_ids[i];
                let c2 = active_cluster_ids[j];

                let members_1 = &clusters[&c1];
                let members_2 = &clusters[&c2];

                // Average-linkage: average of all pairwise distances
                let mut total_dist = 0.0;
                let pair_count = members_1.len() * members_2.len();

                for &a in members_1 {
                    for &b in members_2 {
                        let key = (a.min(b), a.max(b));
                        let dist = distances.get(&key).copied().unwrap_or(1.0);
                        total_dist += dist;
                    }
                }

                let avg_dist = total_dist / pair_count as f64;

                if avg_dist < best_distance {
                    best_distance = avg_dist;
                    best_pair = Some((c1, c2));
                }
            }
        }

        let (c1, c2) = match best_pair {
            Some(pair) => pair,
            None => break,
        };

        // Stop condition (a): distance exceeds threshold
        if best_distance > MERGE_DISTANCE_THRESHOLD {
            break;
        }

        // Stop condition (b): dynamic cut — distance jump exceeds factor
        if prev_merge_distance > 0.0 && best_distance > prev_merge_distance * DYNAMIC_CUT_FACTOR {
            break;
        }

        prev_merge_distance = best_distance;

        // Merge c2 into c1 under a new cluster id
        let mut merged = clusters.remove(&c1).unwrap();
        let c2_members = clusters.remove(&c2).unwrap();
        merged.extend(c2_members);

        clusters.insert(next_cluster_id, merged);
        next_cluster_id += 1;
    }

    // Build results: only clusters with >= MIN_CLUSTER_SIZE
    let mut results: Vec<ClusterCandidate> = Vec::new();
    let mut cluster_counter = 0;

    for members in clusters.values() {
        if members.len() < MIN_CLUSTER_SIZE {
            continue;
        }

        // Compute avg_topic_score = average of (1.0 - distance) for all member pairs
        let mut total_score = 0.0;
        let mut pair_count = 0;

        for i in 0..members.len() {
            for j in (i + 1)..members.len() {
                let key = (members[i].min(members[j]), members[i].max(members[j]));
                let dist = distances.get(&key).copied().unwrap_or(1.0);
                total_score += 1.0 - dist;
                pair_count += 1;
            }
        }

        let avg_topic_score = if pair_count > 0 {
            total_score / pair_count as f64
        } else {
            0.0
        };

        // Count unique sources (pentacle_ids)
        let source_set: HashSet<i64> = members
            .iter()
            .filter_map(|id| pentacle_map.get(id).copied())
            .collect();

        // Skip clusters with too few unique sources
        if source_set.len() < MIN_SOURCE_COUNT {
            continue;
        }

        results.push(ClusterCandidate {
            cluster_id: cluster_counter,
            article_ids: members.clone(),
            avg_topic_score,
            source_count: source_set.len(),
        });

        cluster_counter += 1;
    }

    // Sort by avg_topic_score descending
    results.sort_by(|a, b| {
        b.avg_topic_score
            .partial_cmp(&a.avg_topic_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Reassign cluster IDs after sorting
    for (idx, cluster) in results.iter_mut().enumerate() {
        cluster.cluster_id = idx;
    }

    results
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_overlap_identical() {
        let kw = vec![1, 2, 3];
        assert!((keyword_overlap(&kw, &kw) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_keyword_overlap_disjoint() {
        let a = vec![1, 2, 3];
        let b = vec![4, 5, 6];
        assert!((keyword_overlap(&a, &b) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_keyword_overlap_partial() {
        let a = vec![1, 2, 3];
        let b = vec![2, 3, 4];
        // Intersection: {2, 3} = 2, Union: {1, 2, 3, 4} = 4 -> 2/4 = 0.5
        assert!((keyword_overlap(&a, &b) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_keyword_overlap_empty() {
        let empty: Vec<i64> = vec![];
        assert!((keyword_overlap(&empty, &empty) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_entity_overlap_with_person_org_bonus() {
        // Entity 1 = person, Entity 2 = organization — shared between both
        let a = vec![
            (1, "person".to_string()),
            (2, "organization".to_string()),
            (3, "location".to_string()),
        ];
        let b = vec![
            (1, "person".to_string()),
            (2, "organization".to_string()),
            (4, "location".to_string()),
        ];
        // Intersection: {1, 2} = 2, Union: {1, 2, 3, 4} = 4 -> Jaccard = 0.5
        // Bonus: person + org shared -> 0.5 * 1.3 = 0.65
        let result = entity_overlap(&a, &b);
        assert!((result - 0.65).abs() < 1e-10);
    }

    #[test]
    fn test_entity_overlap_no_bonus() {
        // Only person shared, no org
        let a = vec![(1, "person".to_string()), (3, "location".to_string())];
        let b = vec![(1, "person".to_string()), (4, "location".to_string())];
        // Intersection: {1} = 1, Union: {1, 3, 4} = 3 -> Jaccard = 1/3
        // No bonus (no shared org)
        let result = entity_overlap(&a, &b);
        assert!((result - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_category_match_exact() {
        let a = vec![201];
        let b = vec![201, 305];
        assert!((category_match(&a, &b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_category_match_parent() {
        // 201 and 202 -> parent 200
        let a = vec![201];
        let b = vec![202];
        assert!((category_match(&a, &b) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_category_match_different() {
        let a = vec![201];
        let b = vec![305];
        assert!((category_match(&a, &b) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_temporal_proximity_same_time() {
        let ts = "2026-04-10 12:00:00";
        let result = temporal_proximity(ts, ts, 12.0);
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_temporal_proximity_decay() {
        let a = "2026-04-10 12:00:00";
        let b = "2026-04-11 00:00:00"; // 12 hours later
        let result = temporal_proximity(a, b, 12.0);
        // exp(-12/12) = exp(-1) ≈ 0.36788
        assert!((result - (-1.0_f64).exp()).abs() < 1e-4);
    }

    #[test]
    fn test_topic_score_range() {
        let score = topic_score(
            0.8,
            &[1, 2, 3],
            &[2, 3, 4],
            &[(1, "person".to_string())],
            &[(2, "organization".to_string())],
            &[201],
            &[202],
            "2026-04-10 12:00:00",
            "2026-04-10 18:00:00",
            12.0,
        );
        assert!(score >= 0.0 && score <= 1.0, "score={} out of range", score);
    }

    #[test]
    fn test_agglomerative_cluster_basic() {
        // 4 articles: (1,2) are close, (3,4) are close, (1,2) far from (3,4)
        let article_ids = vec![1, 2, 3, 4];

        let mut distances: HashMap<(i64, i64), f64> = HashMap::new();
        // Pair (1,2): very close
        distances.insert((1, 2), 0.1);
        // Pair (3,4): very close
        distances.insert((3, 4), 0.1);
        // Cross-pairs: far apart
        distances.insert((1, 3), 0.9);
        distances.insert((1, 4), 0.9);
        distances.insert((2, 3), 0.9);
        distances.insert((2, 4), 0.9);

        let mut pentacle_map: HashMap<i64, i64> = HashMap::new();
        pentacle_map.insert(1, 100);
        pentacle_map.insert(2, 101);
        pentacle_map.insert(3, 102);
        pentacle_map.insert(4, 103);

        let result = agglomerative_cluster(&article_ids, &distances, &pentacle_map);

        assert_eq!(result.len(), 2, "Expected 2 clusters, got {}", result.len());

        // Each cluster should have 2 articles
        for cluster in &result {
            assert_eq!(cluster.article_ids.len(), 2);
        }
    }

    #[test]
    fn test_agglomerative_cluster_empty() {
        let article_ids: Vec<i64> = vec![];
        let distances: HashMap<(i64, i64), f64> = HashMap::new();
        let pentacle_map: HashMap<i64, i64> = HashMap::new();

        let result = agglomerative_cluster(&article_ids, &distances, &pentacle_map);
        assert!(result.is_empty());
    }

    #[test]
    fn test_decay_hours_for_days() {
        assert!((decay_hours_for_days(1) - 12.0).abs() < 1e-10);
        assert!((decay_hours_for_days(7) - 48.0).abs() < 1e-10);
        assert!((decay_hours_for_days(14) - 96.0).abs() < 1e-10);
    }
}
