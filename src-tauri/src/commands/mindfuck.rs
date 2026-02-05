//! Operation Mindfuck - Bias Mirror & Filter Bubble Detection
//!
//! Analyzes user reading patterns to detect filter bubbles and suggest
//! counter-perspectives.

use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

// ============================================================
// Reading Statistics
// ============================================================

/// Category reading statistics (main category level)
#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryReadStats {
    pub sephiroth_id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub read_count: i64,
    pub total_count: i64,
    pub percentage: f64,
    pub subcategories: Vec<SubCategoryReadStats>,
}

/// Subcategory reading statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct SubCategoryReadStats {
    pub sephiroth_id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub read_count: i64,
    pub total_count: i64,
    pub percentage: f64,
}

/// Political bias reading statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct BiasReadStats {
    pub bias_value: i32,
    pub label: String,
    pub read_count: i64,
    pub percentage: f64,
}

/// Sachlichkeit (objectivity) reading statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct SachlichkeitReadStats {
    pub sachlichkeit_value: i32,
    pub label: String,
    pub read_count: i64,
    pub percentage: f64,
}

/// Overall reading profile
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadingProfile {
    pub total_read: i64,
    pub total_articles: i64,
    pub read_percentage: f64,
    pub avg_political_bias: Option<f64>,
    pub avg_sachlichkeit: Option<f64>,
    pub by_category: Vec<CategoryReadStats>,
    pub by_bias: Vec<BiasReadStats>,
    pub by_sachlichkeit: Vec<SachlichkeitReadStats>,
    pub first_read_at: Option<String>,
    pub last_read_at: Option<String>,
}

/// Get comprehensive reading profile
#[tauri::command]
pub fn get_reading_profile(state: State<AppState>) -> Result<ReadingProfile, String> {
    let db = state.db_conn()?;

    // Total counts
    let (total_read, total_articles): (i64, i64) = db
        .conn()
        .query_row(
            r#"SELECT
                COUNT(*) FILTER (WHERE read_at IS NOT NULL),
                COUNT(*)
               FROM fnords"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    let read_percentage = if total_articles > 0 {
        (total_read as f64 / total_articles as f64) * 100.0
    } else {
        0.0
    };

    // Average bias and sachlichkeit for read articles
    let (avg_political_bias, avg_sachlichkeit): (Option<f64>, Option<f64>) = db
        .conn()
        .query_row(
            r#"SELECT
                AVG(CAST(political_bias AS REAL)),
                AVG(CAST(sachlichkeit AS REAL))
               FROM fnords
               WHERE read_at IS NOT NULL
                 AND political_bias IS NOT NULL
                 AND sachlichkeit IS NOT NULL"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap_or((None, None));

    // First and last read timestamps
    let (first_read_at, last_read_at): (Option<String>, Option<String>) = db
        .conn()
        .query_row(
            "SELECT MIN(read_at), MAX(read_at) FROM fnords WHERE read_at IS NOT NULL",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap_or((None, None));

    // By category
    let by_category = get_category_stats(&db)?;

    // By political bias
    let by_bias = get_bias_stats(&db)?;

    // By sachlichkeit
    let by_sachlichkeit = get_sachlichkeit_stats(&db)?;

    Ok(ReadingProfile {
        total_read,
        total_articles,
        read_percentage,
        avg_political_bias,
        avg_sachlichkeit,
        by_category,
        by_bias,
        by_sachlichkeit,
        first_read_at,
        last_read_at,
    })
}

fn get_category_stats(
    db: &std::sync::MutexGuard<crate::db::Database>,
) -> Result<Vec<CategoryReadStats>, String> {
    // Get total read count for percentage calculation
    let total_read: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnords WHERE read_at IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Get main categories (level 0)
    let mut main_stmt = db
        .conn()
        .prepare(
            r#"
            SELECT id, name, icon, color
            FROM sephiroth
            WHERE level = 0
            ORDER BY id
            "#,
        )
        .map_err(|e| e.to_string())?;

    let main_cats: Vec<(i64, String, Option<String>, Option<String>)> = main_stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();

    for (main_id, main_name, main_icon, main_color) in main_cats {
        // Get subcategories with stats
        let mut sub_stmt = db
            .conn()
            .prepare(
                r#"
                SELECT
                    sub.id,
                    sub.name,
                    sub.icon,
                    COUNT(DISTINCT f.id) FILTER (WHERE f.read_at IS NOT NULL) as read_count,
                    COUNT(DISTINCT f.id) as total_count
                FROM sephiroth sub
                LEFT JOIN fnord_sephiroth fs ON fs.sephiroth_id = sub.id
                LEFT JOIN fnords f ON f.id = fs.fnord_id
                WHERE sub.parent_id = ?
                GROUP BY sub.id
                ORDER BY sub.name
                "#,
            )
            .map_err(|e| e.to_string())?;

        let subcategories: Vec<SubCategoryReadStats> = sub_stmt
            .query_map([main_id], |row| {
                let read_count: i64 = row.get(3)?;
                let total_count: i64 = row.get(4)?;
                Ok(SubCategoryReadStats {
                    sephiroth_id: row.get(0)?,
                    name: row.get(1)?,
                    icon: row.get(2)?,
                    read_count,
                    total_count,
                    percentage: if total_count > 0 {
                        (read_count as f64 / total_count as f64) * 100.0
                    } else {
                        0.0
                    },
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        // Aggregate stats for main category
        let total_count: i64 = subcategories.iter().map(|s| s.total_count).sum();
        let read_count: i64 = subcategories.iter().map(|s| s.read_count).sum();

        result.push(CategoryReadStats {
            sephiroth_id: main_id,
            name: main_name,
            icon: main_icon,
            color: main_color,
            read_count,
            total_count,
            percentage: if total_read > 0 {
                (read_count as f64 / total_read as f64) * 100.0
            } else {
                0.0
            },
            subcategories,
        });
    }

    // Sort by read_count descending
    result.sort_by(|a, b| b.read_count.cmp(&a.read_count));

    Ok(result)
}

fn get_bias_stats(
    db: &std::sync::MutexGuard<crate::db::Database>,
) -> Result<Vec<BiasReadStats>, String> {
    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                political_bias,
                COUNT(*) as read_count
            FROM fnords
            WHERE read_at IS NOT NULL AND political_bias IS NOT NULL
            GROUP BY political_bias
            ORDER BY political_bias
            "#,
        )
        .map_err(|e| e.to_string())?;

    let total_with_bias: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnords WHERE read_at IS NOT NULL AND political_bias IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let stats = stmt
        .query_map([], |row| {
            let bias_value: i32 = row.get(0)?;
            let read_count: i64 = row.get(1)?;
            let label = match bias_value {
                -2 => "Stark links",
                -1 => "Leicht links",
                0 => "Neutral",
                1 => "Leicht rechts",
                2 => "Stark rechts",
                _ => "Unbekannt",
            };
            Ok(BiasReadStats {
                bias_value,
                label: label.to_string(),
                read_count,
                percentage: if total_with_bias > 0 {
                    (read_count as f64 / total_with_bias as f64) * 100.0
                } else {
                    0.0
                },
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(stats)
}

fn get_sachlichkeit_stats(
    db: &std::sync::MutexGuard<crate::db::Database>,
) -> Result<Vec<SachlichkeitReadStats>, String> {
    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                sachlichkeit,
                COUNT(*) as read_count
            FROM fnords
            WHERE read_at IS NOT NULL AND sachlichkeit IS NOT NULL
            GROUP BY sachlichkeit
            ORDER BY sachlichkeit
            "#,
        )
        .map_err(|e| e.to_string())?;

    let total_with_sach: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnords WHERE read_at IS NOT NULL AND sachlichkeit IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let stats = stmt
        .query_map([], |row| {
            let sachlichkeit_value: i32 = row.get(0)?;
            let read_count: i64 = row.get(1)?;
            let label = match sachlichkeit_value {
                0 => "Stark emotional",
                1 => "Emotional",
                2 => "Gemischt",
                3 => "Überwiegend sachlich",
                4 => "Sachlich",
                _ => "Unbekannt",
            };
            Ok(SachlichkeitReadStats {
                sachlichkeit_value,
                label: label.to_string(),
                read_count,
                percentage: if total_with_sach > 0 {
                    (read_count as f64 / total_with_sach as f64) * 100.0
                } else {
                    0.0
                },
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(stats)
}

// ============================================================
// Blind Spots Detection
// ============================================================

/// A detected blind spot in reading habits
#[derive(Debug, Serialize, Deserialize)]
pub struct BlindSpot {
    pub spot_type: String, // "category", "bias", "sachlichkeit"
    pub name: String,
    pub icon: Option<String>, // Font Awesome icon class
    pub description: String,
    pub severity: String, // "low", "medium", "high"
    pub available_count: i64,
    pub read_count: i64,
    pub main_category: Option<String>,      // For subcategory blind spots
    pub main_category_color: Option<String>, // Color from main category
}

/// Detect blind spots in reading habits
#[tauri::command]
pub fn get_blind_spots(state: State<AppState>) -> Result<Vec<BlindSpot>, String> {
    let db = state.db_conn()?;
    let mut blind_spots = Vec::new();

    // Check for underrepresented categories
    let category_spots = detect_category_blind_spots(&db)?;
    blind_spots.extend(category_spots);

    // Check for missing political perspectives
    let bias_spots = detect_bias_blind_spots(&db)?;
    blind_spots.extend(bias_spots);

    Ok(blind_spots)
}

fn detect_category_blind_spots(
    db: &std::sync::MutexGuard<crate::db::Database>,
) -> Result<Vec<BlindSpot>, String> {
    // Detect blind spots at SUBCATEGORY level (level 1) for granular analysis
    // Include main category info for context
    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                sub.name,
                sub.icon,
                main.name as main_name,
                main.color as main_color,
                COUNT(DISTINCT f.id) FILTER (WHERE f.read_at IS NOT NULL) as read_count,
                COUNT(DISTINCT f.id) as total_count
            FROM sephiroth sub
            JOIN sephiroth main ON main.id = sub.parent_id
            LEFT JOIN fnord_sephiroth fs ON fs.sephiroth_id = sub.id
            LEFT JOIN fnords f ON f.id = fs.fnord_id
            WHERE sub.level = 1
            GROUP BY sub.id
            HAVING total_count > 5  -- Only consider categories with enough articles
            ORDER BY (CAST(read_count AS REAL) / NULLIF(total_count, 0)) ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let spots: Vec<BlindSpot> = stmt
        .query_map([], |row| {
            let name: String = row.get(0)?;
            let icon: Option<String> = row.get(1)?;
            let main_name: String = row.get(2)?;
            let main_color: Option<String> = row.get(3)?;
            let read_count: i64 = row.get(4)?;
            let total_count: i64 = row.get(5)?;

            let read_ratio = if total_count > 0 {
                read_count as f64 / total_count as f64
            } else {
                1.0
            };

            // Determine severity based on read ratio
            let severity = if read_ratio < 0.1 {
                "high"
            } else if read_ratio < 0.3 {
                "medium"
            } else {
                return Ok(None);
            };

            Ok(Some(BlindSpot {
                spot_type: "category".to_string(),
                name,
                icon,
                description: format!(
                    "Du hast nur {}% der verfügbaren Artikel gelesen",
                    (read_ratio * 100.0).round()
                ),
                severity: severity.to_string(),
                available_count: total_count,
                read_count,
                main_category: Some(main_name),
                main_category_color: main_color,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok().flatten())
        .collect();

    Ok(spots)
}

fn detect_bias_blind_spots(
    db: &std::sync::MutexGuard<crate::db::Database>,
) -> Result<Vec<BlindSpot>, String> {
    let mut spots = Vec::new();

    // Get average bias of read articles
    let avg_bias: Option<f64> = db
        .conn()
        .query_row(
            "SELECT AVG(CAST(political_bias AS REAL)) FROM fnords WHERE read_at IS NOT NULL AND political_bias IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .ok()
        .flatten();

    if let Some(bias) = avg_bias {
        // Check for missing perspectives based on average reading bias
        if bias < -0.5 {
            // User reads mostly left-leaning content
            let right_count: i64 = db
                .conn()
                .query_row(
                    "SELECT COUNT(*) FROM fnords WHERE political_bias > 0 AND read_at IS NULL",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            if right_count > 10 {
                spots.push(BlindSpot {
                    spot_type: "bias".to_string(),
                    name: "Konservative Perspektiven".to_string(),
                    icon: Some("fa-solid fa-landmark".to_string()),
                    description: format!(
                        "Du liest überwiegend links-orientierte Artikel. {} konservative Artikel warten auf dich.",
                        right_count
                    ),
                    severity: if bias < -1.0 { "high" } else { "medium" }.to_string(),
                    available_count: right_count,
                    read_count: 0,
                    main_category: None,
                    main_category_color: None,
                });
            }
        } else if bias > 0.5 {
            // User reads mostly right-leaning content
            let left_count: i64 = db
                .conn()
                .query_row(
                    "SELECT COUNT(*) FROM fnords WHERE political_bias < 0 AND read_at IS NULL",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            if left_count > 10 {
                spots.push(BlindSpot {
                    spot_type: "bias".to_string(),
                    name: "Progressive Perspektiven".to_string(),
                    icon: Some("fa-solid fa-seedling".to_string()),
                    description: format!(
                        "Du liest überwiegend rechts-orientierte Artikel. {} progressive Artikel warten auf dich.",
                        left_count
                    ),
                    severity: if bias > 1.0 { "high" } else { "medium" }.to_string(),
                    available_count: left_count,
                    read_count: 0,
                    main_category: None,
                    main_category_color: None,
                });
            }
        }
    }

    Ok(spots)
}

// ============================================================
// Counter-Perspective Recommendations
// ============================================================

/// A recommended article that provides a counter-perspective
#[derive(Debug, Serialize, Deserialize)]
pub struct CounterPerspective {
    pub fnord_id: i64,
    pub title: String,
    pub pentacle_title: Option<String>,
    pub published_at: Option<String>,
    pub political_bias: Option<i32>,
    pub reason: String,
}

/// Get articles that provide counter-perspectives to user's reading habits
#[tauri::command]
pub fn get_counter_perspectives(
    state: State<AppState>,
    limit: Option<i64>,
) -> Result<Vec<CounterPerspective>, String> {
    let db = state.db_conn()?;
    let limit = limit.unwrap_or(10);

    // Get user's average bias
    let avg_bias: Option<f64> = db
        .conn()
        .query_row(
            "SELECT AVG(CAST(political_bias AS REAL)) FROM fnords WHERE read_at IS NOT NULL AND political_bias IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .ok()
        .flatten();

    let avg_bias = avg_bias.unwrap_or(0.0);

    // Find articles with opposite bias that haven't been read
    let target_bias = if avg_bias < 0.0 {
        // User reads left, recommend right
        1
    } else if avg_bias > 0.0 {
        // User reads right, recommend left
        -1
    } else {
        // User is balanced, recommend articles with strong opinions (any direction)
        return get_diverse_recommendations(&db, limit);
    };

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT f.id, f.title, p.title, f.published_at, f.political_bias
            FROM fnords f
            LEFT JOIN pentacles p ON p.id = f.pentacle_id
            WHERE f.read_at IS NULL
              AND f.political_bias IS NOT NULL
              AND f.political_bias * ?1 > 0  -- Same sign as target_bias
              AND f.summary IS NOT NULL      -- Has been processed
            ORDER BY ABS(f.political_bias) DESC, f.published_at DESC
            LIMIT ?2
            "#,
        )
        .map_err(|e| e.to_string())?;

    let reason = if target_bias > 0 {
        "Bietet eine konservativere Perspektive"
    } else {
        "Bietet eine progressivere Perspektive"
    };

    let recommendations = stmt
        .query_map([target_bias, limit as i32], |row| {
            Ok(CounterPerspective {
                fnord_id: row.get(0)?,
                title: row.get(1)?,
                pentacle_title: row.get(2)?,
                published_at: row.get(3)?,
                political_bias: row.get(4)?,
                reason: reason.to_string(),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(recommendations)
}

fn get_diverse_recommendations(
    db: &std::sync::MutexGuard<crate::db::Database>,
    limit: i64,
) -> Result<Vec<CounterPerspective>, String> {
    // For balanced readers, recommend articles with strong opinions from both sides
    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT f.id, f.title, p.title, f.published_at, f.political_bias
            FROM fnords f
            LEFT JOIN pentacles p ON p.id = f.pentacle_id
            WHERE f.read_at IS NULL
              AND f.political_bias IS NOT NULL
              AND ABS(f.political_bias) >= 1  -- Strong opinions
              AND f.summary IS NOT NULL
            ORDER BY f.published_at DESC
            LIMIT ?1
            "#,
        )
        .map_err(|e| e.to_string())?;

    let recommendations = stmt
        .query_map([limit], |row| {
            let bias: Option<i32> = row.get(4)?;
            let reason = match bias {
                Some(b) if b < 0 => "Progressive Perspektive",
                Some(b) if b > 0 => "Konservative Perspektive",
                _ => "Starke Meinung",
            };
            Ok(CounterPerspective {
                fnord_id: row.get(0)?,
                title: row.get(1)?,
                pentacle_title: row.get(2)?,
                published_at: row.get(3)?,
                political_bias: bias,
                reason: reason.to_string(),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(recommendations)
}

// ============================================================
// Reading Trends Over Time
// ============================================================

/// Reading statistics for a specific time period
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadingTrend {
    pub date: String,
    pub read_count: i64,
    pub avg_bias: Option<f64>,
    pub avg_sachlichkeit: Option<f64>,
}

/// Get reading trends over time (last N days)
#[tauri::command]
pub fn get_reading_trends(
    state: State<AppState>,
    days: Option<i64>,
) -> Result<Vec<ReadingTrend>, String> {
    let db = state.db_conn()?;
    let days = days.unwrap_or(30);

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                DATE(read_at) as date,
                COUNT(*) as read_count,
                AVG(CAST(political_bias AS REAL)) as avg_bias,
                AVG(CAST(sachlichkeit AS REAL)) as avg_sachlichkeit
            FROM fnords
            WHERE read_at IS NOT NULL
              AND read_at >= DATE('now', '-' || ?1 || ' days')
            GROUP BY DATE(read_at)
            ORDER BY date ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let trends = stmt
        .query_map([days], |row| {
            Ok(ReadingTrend {
                date: row.get(0)?,
                read_count: row.get(1)?,
                avg_bias: row.get(2)?,
                avg_sachlichkeit: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(trends)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_bias_label() {
        assert_eq!(
            match -2 {
                -2 => "Stark links",
                -1 => "Leicht links",
                0 => "Neutral",
                1 => "Leicht rechts",
                2 => "Stark rechts",
                _ => "Unbekannt",
            },
            "Stark links"
        );
    }

    #[test]
    fn test_sachlichkeit_label() {
        assert_eq!(
            match 4 {
                0 => "Stark emotional",
                1 => "Emotional",
                2 => "Gemischt",
                3 => "Überwiegend sachlich",
                4 => "Sachlich",
                _ => "Unbekannt",
            },
            "Sachlich"
        );
    }

    #[test]
    fn test_severity_calculation() {
        let read_ratio = 0.05; // 5%
        let severity = if read_ratio < 0.1 {
            "high"
        } else if read_ratio < 0.3 {
            "medium"
        } else {
            "low"
        };
        assert_eq!(severity, "high");

        let read_ratio = 0.25;
        let severity = if read_ratio < 0.1 {
            "high"
        } else if read_ratio < 0.3 {
            "medium"
        } else {
            "low"
        };
        assert_eq!(severity, "medium");
    }
}
