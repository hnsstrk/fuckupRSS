//! Database-based evaluation test for keyword extraction
//!
//! This test reads real articles from the fuckupRSS database and compares
//! OLD vs NEW keyword extraction methods.
//!
//! Run with: cargo test --package fuckup-rss --lib -- keywords::db_evaluation_test --nocapture

use super::advanced::{is_near_duplicate, levenshtein_distance};
use super::config::KeywordConfig;
use super::*;
use rusqlite::Connection;
use std::collections::HashMap;

/// Load articles from database
fn load_articles_from_db(limit: usize) -> Vec<(i64, String, String)> {
    let db_path = "data/fuckup.db";

    match Connection::open(db_path) {
        Ok(conn) => {
            let mut stmt = conn
                .prepare(
                    "SELECT id, title, COALESCE(content_full, content_raw, '') as content
                 FROM fnords
                 WHERE content_full IS NOT NULL AND content_full != ''
                 ORDER BY RANDOM()
                 LIMIT ?",
                )
                .expect("Failed to prepare statement");

            let articles = stmt
                .query_map([limit], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .expect("Failed to query articles")
                .filter_map(|r| r.ok())
                .collect();

            articles
        }
        Err(e) => {
            eprintln!("Could not open database: {}. Using fallback test data.", e);
            vec![]
        }
    }
}

/// Configuration for OLD method (baseline)
fn old_method_config() -> KeywordConfig {
    KeywordConfig::standard()
        .with_mmr(false)
        .with_trisum(false)
        .with_levenshtein_distance(0)
        .with_max_keywords(15)
}

/// Configuration for NEW method (improved)
fn new_method_config() -> KeywordConfig {
    KeywordConfig::standard()
        .with_mmr(true)
        .with_mmr_lambda(0.5)
        .with_trisum(true)
        .with_levenshtein_distance(2)
        .with_max_keywords(15)
}

/// Count near-duplicates in a keyword list using Levenshtein distance
fn count_near_duplicates(keywords: &[String], max_distance: usize) -> usize {
    let mut count = 0;
    for i in 0..keywords.len() {
        for j in (i + 1)..keywords.len() {
            if is_near_duplicate(&keywords[i], &keywords[j], max_distance) {
                count += 1;
            }
        }
    }
    count
}

/// Get pairs of near-duplicates
fn get_near_duplicate_pairs(
    keywords: &[String],
    max_distance: usize,
) -> Vec<(String, String, usize)> {
    let mut pairs = Vec::new();
    for i in 0..keywords.len() {
        for j in (i + 1)..keywords.len() {
            let dist =
                levenshtein_distance(&keywords[i].to_lowercase(), &keywords[j].to_lowercase());
            if dist <= max_distance && dist > 0 {
                pairs.push((keywords[i].clone(), keywords[j].clone(), dist));
            }
        }
    }
    pairs
}

/// Calculate diversity score based on average pairwise distance
fn calculate_diversity_score(keywords: &[String]) -> f64 {
    if keywords.len() < 2 {
        return 1.0;
    }

    let mut total_distance = 0;
    let mut pairs = 0;

    for i in 0..keywords.len() {
        for j in (i + 1)..keywords.len() {
            let dist =
                levenshtein_distance(&keywords[i].to_lowercase(), &keywords[j].to_lowercase());
            total_distance += dist;
            pairs += 1;
        }
    }

    if pairs == 0 {
        return 1.0;
    }

    let avg_len: f64 =
        keywords.iter().map(|k| k.len()).sum::<usize>() as f64 / keywords.len() as f64;
    let avg_distance = total_distance as f64 / pairs as f64;

    (avg_distance / (avg_len * 2.0)).min(1.0)
}

/// Check if keyword contains garbage patterns
fn contains_garbage(keyword: &str) -> bool {
    let lower = keyword.to_lowercase();
    let garbage = [
        "div",
        "span",
        "class",
        "href",
        "http",
        "https",
        "www",
        "img",
        "src",
        "alt",
        "style",
        "<",
        ">",
        "{",
        "}",
        "onclick",
        "onload",
        "javascript",
    ];
    garbage.iter().any(|g| lower.contains(g))
}

/// Count garbage keywords
fn count_garbage_keywords(keywords: &[String]) -> usize {
    keywords.iter().filter(|k| contains_garbage(k)).count()
}

/// Extract keywords with a config
fn extract_with_config(title: &str, content: &str, config: KeywordConfig) -> Vec<String> {
    let extractor = KeywordExtractor::with_config(config);
    extractor
        .extract(title, content)
        .into_iter()
        .map(|kw| kw.text)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Benoetigt lokale Datenbank mit Artikeln
    fn test_db_keyword_extraction_evaluation() {
        println!("\n");
        println!("==========================================================================");
        println!("  KEYWORD EXTRACTION EVALUATION WITH REAL DATABASE ARTICLES");
        println!("==========================================================================\n");

        let articles = load_articles_from_db(100);

        if articles.is_empty() {
            println!("No articles found in database. Skipping test.");
            return;
        }

        let old_config = old_method_config();
        let new_config = new_method_config();

        println!("Loaded {} articles from database\n", articles.len());
        println!("Configuration:");
        println!(
            "  OLD: MMR={}, TRISUM={}, Levenshtein={}",
            old_config.use_mmr, old_config.use_trisum, old_config.levenshtein_max_distance
        );
        println!(
            "  NEW: MMR={}, TRISUM={}, Levenshtein={}\n",
            new_config.use_mmr, new_config.use_trisum, new_config.levenshtein_max_distance
        );

        // Collect statistics
        let mut old_total_keywords = 0;
        let mut new_total_keywords = 0;
        let mut old_total_near_dups = 0;
        let mut new_total_near_dups = 0;
        let mut old_total_garbage = 0;
        let mut new_total_garbage = 0;
        let mut old_diversity_sum = 0.0;
        let mut new_diversity_sum = 0.0;
        let mut near_dup_examples: Vec<(String, Vec<(String, String, usize)>)> = Vec::new();
        let mut keyword_frequency: HashMap<String, usize> = HashMap::new();

        // Analyze first 10 articles in detail
        println!("==========================================================================");
        println!("  DETAILED ANALYSIS OF 10 SAMPLE ARTICLES");
        println!("==========================================================================\n");

        for (idx, (id, title, content)) in articles.iter().take(10).enumerate() {
            // Safely truncate at char boundary
            let truncated_content = if content.len() > 5000 {
                let mut end = 5000;
                while !content.is_char_boundary(end) && end > 0 {
                    end -= 1;
                }
                &content[..end]
            } else {
                content.as_str()
            };

            let old_keywords = extract_with_config(title, truncated_content, old_config.clone());
            let new_keywords = extract_with_config(title, truncated_content, new_config.clone());

            let old_near_dups = count_near_duplicates(&old_keywords, 2);
            let new_near_dups = count_near_duplicates(&new_keywords, 2);
            let old_dup_pairs = get_near_duplicate_pairs(&old_keywords, 2);
            let new_dup_pairs = get_near_duplicate_pairs(&new_keywords, 2);

            let old_garbage = count_garbage_keywords(&old_keywords);
            let new_garbage = count_garbage_keywords(&new_keywords);

            let old_diversity = calculate_diversity_score(&old_keywords);
            let new_diversity = calculate_diversity_score(&new_keywords);

            // Truncate title for display (UTF-8 safe)
            let display_title = if title.chars().count() > 60 {
                let truncated: String = title.chars().take(57).collect();
                format!("{}...", truncated)
            } else {
                title.clone()
            };

            println!("--------------------------------------------------------------------------");
            println!("Article #{} (ID: {}): {}", idx + 1, id, display_title);
            println!("--------------------------------------------------------------------------");

            println!(
                "\n  OLD Method ({} keywords, {} near-dups, {} garbage, diversity: {:.2}):",
                old_keywords.len(),
                old_near_dups,
                old_garbage,
                old_diversity
            );
            for (i, kw) in old_keywords.iter().take(10).enumerate() {
                let markers = if contains_garbage(kw) {
                    " [GARBAGE]"
                } else {
                    ""
                };
                println!("    {}. {}{}", i + 1, kw, markers);
            }
            if old_keywords.len() > 10 {
                println!("    ... and {} more", old_keywords.len() - 10);
            }
            if !old_dup_pairs.is_empty() {
                println!("    Near-duplicate pairs:");
                for (a, b, dist) in &old_dup_pairs {
                    println!("      - '{}' <-> '{}' (distance: {})", a, b, dist);
                }
            }

            println!(
                "\n  NEW Method ({} keywords, {} near-dups, {} garbage, diversity: {:.2}):",
                new_keywords.len(),
                new_near_dups,
                new_garbage,
                new_diversity
            );
            for (i, kw) in new_keywords.iter().take(10).enumerate() {
                let markers = if contains_garbage(kw) {
                    " [GARBAGE]"
                } else {
                    ""
                };
                println!("    {}. {}{}", i + 1, kw, markers);
            }
            if new_keywords.len() > 10 {
                println!("    ... and {} more", new_keywords.len() - 10);
            }
            if !new_dup_pairs.is_empty() {
                println!("    Near-duplicate pairs:");
                for (a, b, dist) in &new_dup_pairs {
                    println!("      - '{}' <-> '{}' (distance: {})", a, b, dist);
                }
            }

            // Assessment
            let improvement = if new_near_dups < old_near_dups {
                format!(
                    "IMPROVED: removed {} near-duplicates",
                    old_near_dups - new_near_dups
                )
            } else if new_near_dups == 0 && old_near_dups == 0 {
                "EQUAL: no near-duplicates".to_string()
            } else if new_near_dups > old_near_dups {
                format!(
                    "REGRESSED: {} more near-duplicates",
                    new_near_dups - old_near_dups
                )
            } else {
                "NO CHANGE".to_string()
            };
            println!("\n  => Assessment: {}\n", improvement);

            // Save examples of near-duplicates for report
            if !old_dup_pairs.is_empty() {
                near_dup_examples.push((title.clone(), old_dup_pairs));
            }
        }

        // Process all articles for statistics
        println!("\n==========================================================================");
        println!(
            "  PROCESSING ALL {} ARTICLES FOR STATISTICS",
            articles.len()
        );
        println!("==========================================================================\n");

        for (_, title, content) in &articles {
            // Safely truncate at a character boundary
            let truncated_content = if content.len() > 5000 {
                // Find the last valid character boundary before 5000
                let end_idx = content
                    .char_indices()
                    .take_while(|(idx, _)| *idx < 5000)
                    .last()
                    .map(|(idx, c)| idx + c.len_utf8())
                    .unwrap_or(5000.min(content.len()));
                &content[..end_idx]
            } else {
                content.as_str()
            };

            let old_keywords = extract_with_config(title, truncated_content, old_config.clone());
            let new_keywords = extract_with_config(title, truncated_content, new_config.clone());

            old_total_keywords += old_keywords.len();
            new_total_keywords += new_keywords.len();

            old_total_near_dups += count_near_duplicates(&old_keywords, 2);
            new_total_near_dups += count_near_duplicates(&new_keywords, 2);

            old_total_garbage += count_garbage_keywords(&old_keywords);
            new_total_garbage += count_garbage_keywords(&new_keywords);

            old_diversity_sum += calculate_diversity_score(&old_keywords);
            new_diversity_sum += calculate_diversity_score(&new_keywords);

            // Track keyword frequency for NEW method
            for kw in &new_keywords {
                *keyword_frequency.entry(kw.to_lowercase()).or_insert(0) += 1;
            }
        }

        let article_count = articles.len() as f64;

        println!("==========================================================================");
        println!("  SUMMARY STATISTICS");
        println!("==========================================================================\n");

        println!("Total articles analyzed: {}\n", articles.len());

        println!("Average keywords per article:");
        println!("  OLD: {:.1}", old_total_keywords as f64 / article_count);
        println!("  NEW: {:.1}", new_total_keywords as f64 / article_count);
        println!();

        println!("Total near-duplicates detected (distance <= 2):");
        println!("  OLD: {}", old_total_near_dups);
        println!("  NEW: {}", new_total_near_dups);
        if old_total_near_dups > 0 {
            let reduction = 100.0 * (1.0 - new_total_near_dups as f64 / old_total_near_dups as f64);
            println!("  Reduction: {:.1}%", reduction);
        }
        println!();

        println!("Total garbage keywords detected:");
        println!("  OLD: {}", old_total_garbage);
        println!("  NEW: {}", new_total_garbage);
        if old_total_garbage > 0 {
            let reduction = 100.0 * (1.0 - new_total_garbage as f64 / old_total_garbage as f64);
            println!("  Reduction: {:.1}%", reduction);
        }
        println!();

        println!("Average diversity score (0-1, higher = more diverse):");
        println!("  OLD: {:.3}", old_diversity_sum / article_count);
        println!("  NEW: {:.3}", new_diversity_sum / article_count);
        let old_div = old_diversity_sum / article_count;
        let new_div = new_diversity_sum / article_count;
        if old_div > 0.0 {
            let improvement = (new_div - old_div) / old_div * 100.0;
            println!("  Change: {:+.1}%", improvement);
        }
        println!();

        // Near-duplicate examples
        if !near_dup_examples.is_empty() {
            println!("Examples of near-duplicates found (OLD method):");
            for (title, pairs) in near_dup_examples.iter().take(5) {
                let short_title = if title.len() > 50 {
                    &title[..50]
                } else {
                    title
                };
                println!("  Article: '{}'", short_title);
                for (a, b, dist) in pairs.iter().take(3) {
                    println!("    - '{}' <-> '{}' (dist: {})", a, b, dist);
                }
            }
            println!();
        }

        // Most frequent keywords
        println!("Top 20 most frequent keywords (NEW method):");
        let mut freq_vec: Vec<_> = keyword_frequency.iter().collect();
        freq_vec.sort_by(|a, b| b.1.cmp(a.1));
        for (i, (kw, count)) in freq_vec.iter().take(20).enumerate() {
            println!("  {}. {} ({}x)", i + 1, kw, count);
        }
        println!();

        // Quality assessment
        println!("==========================================================================");
        println!("  QUALITY ASSESSMENT");
        println!("==========================================================================\n");

        let dup_score = if new_total_near_dups == 0 {
            10
        } else if new_total_near_dups < 10 {
            8
        } else if new_total_near_dups < 30 {
            6
        } else {
            4
        };

        let garbage_score = if new_total_garbage == 0 {
            10
        } else if new_total_garbage < 5 {
            8
        } else if new_total_garbage < 20 {
            6
        } else {
            4
        };

        let avg_diversity = new_diversity_sum / article_count;
        let diversity_score = if avg_diversity > 0.6 {
            9
        } else if avg_diversity > 0.5 {
            7
        } else if avg_diversity > 0.4 {
            5
        } else {
            3
        };

        // Estimate relevance and coverage (would need manual review)
        let relevance_score = 7;
        let coverage_score = 8;

        println!("Criteria (1-10 scale):");
        println!(
            "  - Near-duplicate avoidance: {}/10 ({} total near-dups)",
            dup_score, new_total_near_dups
        );
        println!(
            "  - Garbage filtering: {}/10 ({} garbage keywords)",
            garbage_score, new_total_garbage
        );
        println!(
            "  - Diversity: {}/10 (avg: {:.3})",
            diversity_score, avg_diversity
        );
        println!("  - Relevance: {}/10 (estimated)", relevance_score);
        println!("  - Topic coverage: {}/10 (estimated)", coverage_score);
        println!();

        let overall =
            (dup_score + garbage_score + diversity_score + relevance_score + coverage_score) as f64
                / 5.0;
        println!("==> OVERALL QUALITY SCORE: {:.1}/10", overall);
        println!();

        // Improvement summary
        println!("==========================================================================");
        println!("  IMPROVEMENT SUMMARY (NEW vs OLD)");
        println!("==========================================================================\n");

        if old_total_near_dups > new_total_near_dups {
            println!(
                "  [+] Near-duplicates reduced by {} ({:.1}%)",
                old_total_near_dups - new_total_near_dups,
                100.0 * (1.0 - new_total_near_dups as f64 / old_total_near_dups as f64)
            );
        } else if old_total_near_dups == new_total_near_dups {
            println!(
                "  [=] Near-duplicates unchanged ({} total)",
                new_total_near_dups
            );
        } else {
            println!(
                "  [-] Near-duplicates increased by {}",
                new_total_near_dups - old_total_near_dups
            );
        }

        if old_total_garbage > new_total_garbage {
            println!(
                "  [+] Garbage keywords reduced by {}",
                old_total_garbage - new_total_garbage
            );
        } else if old_total_garbage == new_total_garbage {
            println!(
                "  [=] Garbage keywords unchanged ({} total)",
                new_total_garbage
            );
        }

        if new_div > old_div {
            println!(
                "  [+] Diversity improved by {:.1}%",
                (new_div - old_div) / old_div * 100.0
            );
        } else if (new_div - old_div).abs() < 0.01 {
            println!("  [=] Diversity unchanged ({:.3})", new_div);
        } else {
            println!(
                "  [-] Diversity decreased by {:.1}%",
                (old_div - new_div) / old_div * 100.0
            );
        }

        println!("\n==========================================================================\n");

        // Test assertions
        assert!(articles.len() >= 10, "Should process at least 10 articles");
    }
}
