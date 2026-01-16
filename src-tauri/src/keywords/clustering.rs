//! Article Clustering for Efficient LLM Usage
//!
//! This module provides clustering functionality for grouping similar articles
//! based on their embeddings. This enables efficient LLM usage by:
//! 1. Clustering similar articles together
//! 2. Running LLM analysis only on cluster representatives
//! 3. Transferring keywords to all articles in the cluster
//!
//! Clustering algorithm: Agglomerative Hierarchical Clustering with cosine distance

use std::collections::{HashMap, HashSet};

/// Configuration for article clustering
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// Maximum distance threshold for merging clusters (0.0-1.0)
    /// Lower values = tighter clusters, higher values = more inclusive
    pub distance_threshold: f64,
    /// Minimum cluster size to consider for representative selection
    pub min_cluster_size: usize,
    /// Maximum number of clusters (0 = unlimited)
    pub max_clusters: usize,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            distance_threshold: 0.4, // Cosine distance threshold (1 - similarity)
            min_cluster_size: 1,
            max_clusters: 0, // Unlimited
        }
    }
}

/// An article with its embedding for clustering
#[derive(Debug, Clone)]
pub struct ArticleForClustering {
    /// Unique identifier (fnord_id)
    pub id: i64,
    /// Title of the article
    pub title: String,
    /// Embedding vector (from snowflake-arctic-embed2)
    pub embedding: Vec<f32>,
    /// Optional summary (if already processed)
    pub summary: Option<String>,
    /// Whether this article has been processed
    pub is_processed: bool,
}

/// Result of clustering - a single cluster with its articles
#[derive(Debug, Clone)]
pub struct ArticleCluster {
    /// Cluster identifier
    pub cluster_id: usize,
    /// ID of the representative article (typically first/central article)
    pub representative_id: i64,
    /// All article IDs in this cluster
    pub article_ids: Vec<i64>,
    /// Centroid embedding of the cluster (average of all embeddings)
    pub centroid: Vec<f32>,
    /// Internal coherence score (average pairwise similarity)
    pub coherence: f64,
}

/// Result of the full clustering operation
#[derive(Debug, Clone)]
pub struct ClusteringResult {
    /// All clusters found
    pub clusters: Vec<ArticleCluster>,
    /// Articles that were not assigned to any cluster (outliers)
    pub unclustered_ids: Vec<i64>,
    /// Total number of articles processed
    pub total_articles: usize,
    /// Number of representatives (articles that need LLM processing)
    pub representatives_count: usize,
}

/// Calculate cosine distance between two embedding vectors
/// Returns 1.0 - cosine_similarity (so lower = more similar)
fn cosine_distance(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 1.0; // Maximum distance for invalid inputs
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 1.0;
    }

    let similarity = (dot / (norm_a * norm_b)) as f64;
    (1.0 - similarity).max(0.0) // Ensure non-negative distance
}

/// Calculate the centroid (average) of a set of embeddings
fn calculate_centroid(embeddings: &[&[f32]]) -> Vec<f32> {
    if embeddings.is_empty() {
        return vec![];
    }

    let dim = embeddings[0].len();
    let n = embeddings.len() as f32;

    let mut centroid = vec![0.0f32; dim];
    for emb in embeddings {
        for (i, val) in emb.iter().enumerate() {
            if i < dim {
                centroid[i] += val / n;
            }
        }
    }

    centroid
}

/// Calculate cluster coherence (average pairwise similarity)
fn calculate_coherence(embeddings: &[&[f32]]) -> f64 {
    if embeddings.len() < 2 {
        return 1.0; // Single-item cluster is perfectly coherent
    }

    let mut total_similarity = 0.0;
    let mut count = 0;

    for i in 0..embeddings.len() {
        for j in (i + 1)..embeddings.len() {
            total_similarity += 1.0 - cosine_distance(embeddings[i], embeddings[j]);
            count += 1;
        }
    }

    if count == 0 {
        1.0
    } else {
        total_similarity / count as f64
    }
}

/// Find the representative article in a cluster
/// Prefers: already processed > closest to centroid > first article
fn find_representative(
    articles: &[&ArticleForClustering],
    centroid: &[f32],
) -> i64 {
    if articles.is_empty() {
        return 0;
    }

    // First, try to find an already processed article (save LLM calls)
    let processed_indices: Vec<usize> = articles
        .iter()
        .enumerate()
        .filter(|(_, a)| a.is_processed)
        .map(|(i, _)| i)
        .collect();

    let candidate_indices: Vec<usize> = if processed_indices.is_empty() {
        (0..articles.len()).collect()
    } else {
        processed_indices
    };

    // Find the article closest to centroid
    candidate_indices
        .iter()
        .min_by(|&&i, &&j| {
            let dist_i = cosine_distance(&articles[i].embedding, centroid);
            let dist_j = cosine_distance(&articles[j].embedding, centroid);
            dist_i.partial_cmp(&dist_j).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|&i| articles[i].id)
        .unwrap_or(articles[0].id)
}

/// Perform agglomerative hierarchical clustering on articles
///
/// Uses single-linkage clustering with cosine distance.
/// Articles are merged into clusters when their distance is below the threshold.
///
/// # Arguments
/// * `articles` - List of articles with their embeddings
/// * `config` - Clustering configuration
///
/// # Returns
/// ClusteringResult with clusters and unclustered articles
pub fn cluster_articles(
    articles: Vec<ArticleForClustering>,
    config: &ClusterConfig,
) -> ClusteringResult {
    if articles.is_empty() {
        return ClusteringResult {
            clusters: vec![],
            unclustered_ids: vec![],
            total_articles: 0,
            representatives_count: 0,
        };
    }

    let n = articles.len();

    // Initialize: each article is its own cluster
    let mut cluster_assignments: Vec<usize> = (0..n).collect();
    let mut active_clusters: HashSet<usize> = (0..n).collect();

    // Build distance matrix (upper triangular)
    let mut distances: HashMap<(usize, usize), f64> = HashMap::new();
    for i in 0..n {
        for j in (i + 1)..n {
            let dist = cosine_distance(&articles[i].embedding, &articles[j].embedding);
            distances.insert((i, j), dist);
        }
    }

    // Agglomerative clustering: merge closest clusters until threshold exceeded
    loop {
        // Find the minimum distance pair among active clusters
        let mut min_dist = f64::MAX;
        let mut merge_pair: Option<(usize, usize)> = None;

        for &c1 in &active_clusters {
            for &c2 in &active_clusters {
                if c1 >= c2 {
                    continue;
                }

                // Find minimum distance between any two points in clusters c1 and c2
                // (single-linkage)
                for (i, &assign_i) in cluster_assignments.iter().enumerate() {
                    if assign_i != c1 {
                        continue;
                    }
                    for (j, &assign_j) in cluster_assignments.iter().enumerate() {
                        if assign_j != c2 || i >= j {
                            continue;
                        }

                        let key = (i.min(j), i.max(j));
                        if let Some(&dist) = distances.get(&key) {
                            if dist < min_dist {
                                min_dist = dist;
                                merge_pair = Some((c1, c2));
                            }
                        }
                    }
                }
            }
        }

        // Check if we should continue merging
        match merge_pair {
            Some((c1, c2)) if min_dist <= config.distance_threshold => {
                // Merge c2 into c1
                for assign in cluster_assignments.iter_mut() {
                    if *assign == c2 {
                        *assign = c1;
                    }
                }
                active_clusters.remove(&c2);

                // Check max clusters limit
                if config.max_clusters > 0 && active_clusters.len() <= config.max_clusters {
                    break;
                }
            }
            _ => break, // No more merges possible
        }
    }

    // Build final clusters
    let mut cluster_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for (idx, &cluster_id) in cluster_assignments.iter().enumerate() {
        cluster_map.entry(cluster_id).or_default().push(idx);
    }

    let mut clusters = Vec::new();
    let mut unclustered_ids = Vec::new();
    let mut cluster_id_counter = 0;

    for (_, article_indices) in cluster_map {
        if article_indices.len() < config.min_cluster_size {
            // Articles in small clusters become unclustered
            for idx in article_indices {
                unclustered_ids.push(articles[idx].id);
            }
            continue;
        }

        // Collect embeddings for centroid calculation
        let embeddings: Vec<&[f32]> = article_indices
            .iter()
            .map(|&idx| articles[idx].embedding.as_slice())
            .collect();

        let centroid = calculate_centroid(&embeddings);
        let coherence = calculate_coherence(&embeddings);

        // Get article references for representative selection
        let cluster_articles: Vec<&ArticleForClustering> = article_indices
            .iter()
            .map(|&idx| &articles[idx])
            .collect();

        let representative_id = find_representative(&cluster_articles, &centroid);
        let article_ids: Vec<i64> = article_indices.iter().map(|&idx| articles[idx].id).collect();

        clusters.push(ArticleCluster {
            cluster_id: cluster_id_counter,
            representative_id,
            article_ids,
            centroid,
            coherence,
        });

        cluster_id_counter += 1;
    }

    // Sort clusters by size (largest first)
    clusters.sort_by(|a, b| b.article_ids.len().cmp(&a.article_ids.len()));

    // Reassign cluster IDs after sorting
    for (idx, cluster) in clusters.iter_mut().enumerate() {
        cluster.cluster_id = idx;
    }

    let representatives_count = clusters.len() + unclustered_ids.len();

    ClusteringResult {
        clusters,
        unclustered_ids,
        total_articles: n,
        representatives_count,
    }
}

/// Transfer keywords from representative to all articles in a cluster
///
/// This is called after LLM analysis of the representative article.
/// All articles in the cluster receive the same keywords.
///
/// # Arguments
/// * `cluster` - The cluster containing article IDs
/// * `keywords` - Keywords extracted from the representative
///
/// # Returns
/// List of (article_id, keywords) tuples for database insertion
pub fn transfer_keywords_to_cluster<'a>(
    cluster: &'a ArticleCluster,
    keywords: &'a [String],
) -> Vec<(i64, &'a [String])> {
    cluster
        .article_ids
        .iter()
        .map(|&id| (id, keywords))
        .collect()
}

/// Get all representative article IDs from a clustering result
///
/// These are the articles that need LLM processing.
/// Other articles will receive keywords via transfer.
pub fn get_representatives(result: &ClusteringResult) -> Vec<i64> {
    let mut reps: Vec<i64> = result
        .clusters
        .iter()
        .map(|c| c.representative_id)
        .collect();

    // Unclustered articles also need processing
    reps.extend(&result.unclustered_ids);

    reps
}

/// Calculate estimated LLM cost savings from clustering
///
/// Returns (saved_calls, total_articles, savings_percentage)
pub fn calculate_savings(result: &ClusteringResult) -> (usize, usize, f64) {
    let total = result.total_articles;
    let needed = result.representatives_count;
    let saved = total.saturating_sub(needed);
    let percentage = if total > 0 {
        (saved as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    (saved, total, percentage)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_article(id: i64, embedding: Vec<f32>) -> ArticleForClustering {
        ArticleForClustering {
            id,
            title: format!("Article {}", id),
            embedding,
            summary: None,
            is_processed: false,
        }
    }

    #[test]
    fn test_cosine_distance() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![1.0f32, 0.0, 0.0];
        assert!((cosine_distance(&a, &b) - 0.0).abs() < 1e-6);

        let c = vec![0.0f32, 1.0, 0.0];
        assert!((cosine_distance(&a, &c) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_calculate_centroid() {
        let a = vec![1.0f32, 0.0];
        let b = vec![0.0f32, 1.0];
        let embeddings: Vec<&[f32]> = vec![a.as_slice(), b.as_slice()];

        let centroid = calculate_centroid(&embeddings);
        assert!((centroid[0] - 0.5).abs() < 1e-6);
        assert!((centroid[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_cluster_identical_articles() {
        // Two identical articles should cluster together
        let articles = vec![
            make_article(1, vec![1.0, 0.0, 0.0]),
            make_article(2, vec![1.0, 0.0, 0.0]),
        ];

        let config = ClusterConfig::default();
        let result = cluster_articles(articles, &config);

        assert_eq!(result.clusters.len(), 1);
        assert_eq!(result.clusters[0].article_ids.len(), 2);
        assert!(result.unclustered_ids.is_empty());
    }

    #[test]
    fn test_cluster_distant_articles() {
        // Two orthogonal articles should not cluster
        let articles = vec![
            make_article(1, vec![1.0, 0.0, 0.0]),
            make_article(2, vec![0.0, 1.0, 0.0]),
        ];

        let config = ClusterConfig {
            distance_threshold: 0.3, // Strict threshold
            min_cluster_size: 2,
            max_clusters: 0,
        };

        let result = cluster_articles(articles, &config);

        // Should have no clusters (both articles are unclustered)
        assert!(result.clusters.is_empty());
        assert_eq!(result.unclustered_ids.len(), 2);
    }

    #[test]
    fn test_cluster_similar_articles() {
        // Three similar articles should cluster together
        let articles = vec![
            make_article(1, vec![0.9, 0.1, 0.0]),
            make_article(2, vec![0.85, 0.15, 0.0]),
            make_article(3, vec![0.88, 0.12, 0.0]),
        ];

        let config = ClusterConfig {
            distance_threshold: 0.2,
            min_cluster_size: 1,
            max_clusters: 0,
        };

        let result = cluster_articles(articles, &config);

        assert_eq!(result.clusters.len(), 1);
        assert_eq!(result.clusters[0].article_ids.len(), 3);
    }

    #[test]
    fn test_representative_selection_processed() {
        // Processed article should be preferred as representative
        let mut articles = vec![
            make_article(1, vec![1.0, 0.0, 0.0]),
            make_article(2, vec![0.95, 0.05, 0.0]),
        ];
        articles[1].is_processed = true; // Mark article 2 as processed

        let config = ClusterConfig::default();
        let result = cluster_articles(articles, &config);

        assert_eq!(result.clusters.len(), 1);
        // Article 2 should be representative since it's processed
        assert_eq!(result.clusters[0].representative_id, 2);
    }

    #[test]
    fn test_calculate_savings() {
        let result = ClusteringResult {
            clusters: vec![
                ArticleCluster {
                    cluster_id: 0,
                    representative_id: 1,
                    article_ids: vec![1, 2, 3, 4, 5],
                    centroid: vec![],
                    coherence: 0.9,
                },
            ],
            unclustered_ids: vec![6],
            total_articles: 6,
            representatives_count: 2, // 1 cluster rep + 1 unclustered
        };

        let (saved, total, percentage) = calculate_savings(&result);

        assert_eq!(saved, 4); // 6 - 2 = 4 saved
        assert_eq!(total, 6);
        assert!((percentage - 66.666).abs() < 1.0);
    }

    #[test]
    fn test_transfer_keywords() {
        let cluster = ArticleCluster {
            cluster_id: 0,
            representative_id: 1,
            article_ids: vec![1, 2, 3],
            centroid: vec![],
            coherence: 0.9,
        };

        let keywords = vec!["Politik".to_string(), "Wirtschaft".to_string()];
        let transfers = transfer_keywords_to_cluster(&cluster, &keywords);

        assert_eq!(transfers.len(), 3);
        assert!(transfers.iter().all(|(_, kw)| kw.len() == 2));
    }

    #[test]
    fn test_get_representatives() {
        let result = ClusteringResult {
            clusters: vec![
                ArticleCluster {
                    cluster_id: 0,
                    representative_id: 1,
                    article_ids: vec![1, 2, 3],
                    centroid: vec![],
                    coherence: 0.9,
                },
                ArticleCluster {
                    cluster_id: 1,
                    representative_id: 5,
                    article_ids: vec![5, 6],
                    centroid: vec![],
                    coherence: 0.85,
                },
            ],
            unclustered_ids: vec![7, 8],
            total_articles: 8,
            representatives_count: 4,
        };

        let reps = get_representatives(&result);

        assert_eq!(reps.len(), 4);
        assert!(reps.contains(&1));
        assert!(reps.contains(&5));
        assert!(reps.contains(&7));
        assert!(reps.contains(&8));
    }

    #[test]
    fn test_empty_input() {
        let articles: Vec<ArticleForClustering> = vec![];
        let config = ClusterConfig::default();
        let result = cluster_articles(articles, &config);

        assert!(result.clusters.is_empty());
        assert!(result.unclustered_ids.is_empty());
        assert_eq!(result.total_articles, 0);
        assert_eq!(result.representatives_count, 0);
    }

    #[test]
    fn test_single_article() {
        let articles = vec![make_article(1, vec![1.0, 0.0, 0.0])];
        let config = ClusterConfig {
            min_cluster_size: 1,
            ..Default::default()
        };

        let result = cluster_articles(articles, &config);

        assert_eq!(result.clusters.len(), 1);
        assert_eq!(result.clusters[0].article_ids.len(), 1);
    }

    #[test]
    fn test_max_clusters_limit() {
        // 4 different articles
        let articles = vec![
            make_article(1, vec![1.0, 0.0, 0.0, 0.0]),
            make_article(2, vec![0.0, 1.0, 0.0, 0.0]),
            make_article(3, vec![0.0, 0.0, 1.0, 0.0]),
            make_article(4, vec![0.0, 0.0, 0.0, 1.0]),
        ];

        let config = ClusterConfig {
            distance_threshold: 1.0, // Allow all merges
            min_cluster_size: 1,
            max_clusters: 2, // Limit to 2 clusters
        };

        let result = cluster_articles(articles, &config);

        assert!(result.clusters.len() + result.unclustered_ids.len() <= 4);
    }

    #[test]
    fn test_coherence_calculation() {
        // Identical embeddings should have coherence = 1.0
        let emb = vec![1.0f32, 0.0];
        let embeddings: Vec<&[f32]> = vec![emb.as_slice(), emb.as_slice()];

        let coherence = calculate_coherence(&embeddings);
        assert!((coherence - 1.0).abs() < 1e-6);
    }
}
