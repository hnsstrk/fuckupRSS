//! Bias learning system for statistical analysis
//!
//! Learns from user corrections to improve keyword and category suggestions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rusqlite::{Connection, params};

/// Types of corrections that can be recorded
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionType {
    /// User removed a keyword
    KeywordRemoved,
    /// User added a keyword
    KeywordAdded,
    /// User removed a category
    CategoryRemoved,
    /// User added a category
    CategoryAdded,
}

/// A correction record for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionRecord {
    pub fnord_id: i64,
    pub correction_type: CorrectionType,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    /// For category corrections: the terms that matched this category
    /// Used to learn which terms should be down/up-weighted for this category
    #[serde(default)]
    pub matching_terms: Vec<String>,
    /// For category corrections: the category ID
    #[serde(default)]
    pub category_id: Option<i64>,
}

/// Weight types stored in the database (for future use)
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightType {
    /// Boost/penalty for specific keywords
    KeywordBoost,
    /// Weight for category-term associations
    CategoryTerm,
    /// Weight for different sources (ai, statistical, manual)
    SourceWeight,
}

#[allow(dead_code)]
impl WeightType {
    fn as_str(&self) -> &'static str {
        match self {
            WeightType::KeywordBoost => "keyword_boost",
            WeightType::CategoryTerm => "category_term",
            WeightType::SourceWeight => "source_weight",
        }
    }
}

/// Bias weights for statistical analysis
#[derive(Debug, Clone, Default)]
pub struct BiasWeights {
    /// Keyword boost factors: keyword_name -> weight multiplier
    pub keyword_boosts: HashMap<String, f64>,
    /// Category-term weights: (category_id, term) -> weight multiplier
    pub category_term_weights: HashMap<(i64, String), f64>,
    /// Source weights: source_type -> weight multiplier
    pub source_weights: HashMap<String, f64>,
}

impl BiasWeights {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load bias weights from database
    pub fn load_from_db(conn: &Connection) -> Result<Self, rusqlite::Error> {
        let mut weights = Self::new();

        let mut stmt = conn.prepare(
            "SELECT weight_type, context_key, term, weight FROM bias_weights"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })?;

        for row in rows {
            let (weight_type, context_key, term, weight) = row?;
            match weight_type.as_str() {
                "keyword_boost" => {
                    weights.keyword_boosts.insert(context_key, weight);
                }
                "category_term" => {
                    if let (Ok(cat_id), Some(term)) = (context_key.parse::<i64>(), term) {
                        weights.category_term_weights.insert((cat_id, term), weight);
                    }
                }
                "source_weight" => {
                    weights.source_weights.insert(context_key, weight);
                }
                _ => {}
            }
        }

        Ok(weights)
    }

    /// Get keyword boost factor (default: 1.0)
    pub fn get_keyword_boost(&self, keyword: &str) -> f64 {
        self.keyword_boosts.get(keyword).copied().unwrap_or(1.0)
    }

    /// Get category-term weight factor (default: 1.0)
    pub fn get_category_term_weight(&self, category_id: i64, term: &str) -> f64 {
        self.category_term_weights
            .get(&(category_id, term.to_lowercase()))
            .copied()
            .unwrap_or(1.0)
    }

    /// Get source weight factor
    /// Default weights: manual=1.2 (trusted), ai=1.0 (standard), statistical=0.9 (needs validation)
    pub fn get_source_weight(&self, source: &str) -> f64 {
        self.source_weights.get(source).copied().unwrap_or_else(|| {
            // Default source weights
            match source {
                "manual" => 1.2,      // Manual entries are most trusted
                "ai" => 1.0,          // AI is baseline
                "statistical" => 0.9, // Statistical needs validation
                _ => 1.0,
            }
        })
    }

    /// Apply source weight to a confidence score
    pub fn apply_source_weight(&self, source: &str, base_confidence: f64) -> f64 {
        (base_confidence * self.get_source_weight(source)).clamp(0.0, 1.0)
    }

    /// Apply keyword boost to a score
    pub fn apply_to_keyword(&self, keyword: &str, base_score: f64) -> f64 {
        base_score * self.get_keyword_boost(keyword)
    }

    /// Apply category-term weight to a score
    #[allow(dead_code)] // Public API for category score adjustment
    pub fn apply_to_category(&self, category_id: i64, term: &str, base_score: f64) -> f64 {
        base_score * self.get_category_term_weight(category_id, term)
    }
}

/// Adjustment amount for bias weights
const WEIGHT_ADJUSTMENT: f64 = 0.1;

/// Minimum weight (prevents going to zero)
const MIN_WEIGHT: f64 = 0.1;

/// Maximum weight (prevents extreme values)
const MAX_WEIGHT: f64 = 3.0;

/// Record a correction and update bias weights
pub fn record_correction(
    conn: &Connection,
    correction: &CorrectionRecord,
) -> Result<(), rusqlite::Error> {
    match correction.correction_type {
        CorrectionType::KeywordRemoved => {
            // User removed a keyword -> decrease its boost
            if let Some(keyword) = &correction.old_value {
                adjust_keyword_boost(conn, keyword, -WEIGHT_ADJUSTMENT)?;
            }
        }
        CorrectionType::KeywordAdded => {
            // User added a keyword -> increase its boost
            if let Some(keyword) = &correction.new_value {
                adjust_keyword_boost(conn, keyword, WEIGHT_ADJUSTMENT)?;
            }
        }
        CorrectionType::CategoryRemoved => {
            // User removed a category -> decrease weights for matching terms
            if let Some(category_id) = correction.category_id {
                // Decrease weight for each matching term that led to this category
                for term in &correction.matching_terms {
                    adjust_category_term_weight(conn, category_id, term, -WEIGHT_ADJUSTMENT)?;
                }
                // Also record a general category correction for stats
                record_category_correction(conn, category_id, false)?;
            }
        }
        CorrectionType::CategoryAdded => {
            // User added a category -> increase weights for matching terms
            if let Some(category_id) = correction.category_id {
                // Increase weight for each matching term
                for term in &correction.matching_terms {
                    adjust_category_term_weight(conn, category_id, term, WEIGHT_ADJUSTMENT)?;
                }
                // Also record a general category correction for stats
                record_category_correction(conn, category_id, true)?;
            }
        }
    }

    Ok(())
}

/// Record a category correction for statistics (without term-level adjustment)
fn record_category_correction(
    conn: &Connection,
    category_id: i64,
    was_added: bool,
) -> Result<(), rusqlite::Error> {
    let context_key = format!("cat_{}", category_id);
    let adjustment = if was_added { WEIGHT_ADJUSTMENT } else { -WEIGHT_ADJUSTMENT };

    // Try to update existing weight
    let updated = conn.execute(
        "UPDATE bias_weights
         SET weight = MAX(?, MIN(?, weight + ?)),
             correction_count = correction_count + 1,
             last_updated = CURRENT_TIMESTAMP
         WHERE weight_type = 'category_boost' AND context_key = ?",
        params![MIN_WEIGHT, MAX_WEIGHT, adjustment, &context_key],
    )?;

    // If no existing weight, insert new one
    if updated == 0 {
        let new_weight = (1.0 + adjustment).clamp(MIN_WEIGHT, MAX_WEIGHT);
        conn.execute(
            "INSERT INTO bias_weights (weight_type, context_key, weight, correction_count, last_updated)
             VALUES ('category_boost', ?, ?, 1, CURRENT_TIMESTAMP)",
            params![&context_key, new_weight],
        )?;
    }

    Ok(())
}

/// Adjust keyword boost weight
fn adjust_keyword_boost(
    conn: &Connection,
    keyword: &str,
    adjustment: f64,
) -> Result<(), rusqlite::Error> {
    let keyword_lower = keyword.to_lowercase();

    // Try to update existing weight
    let updated = conn.execute(
        "UPDATE bias_weights
         SET weight = MAX(?, MIN(?, weight + ?)),
             correction_count = correction_count + 1,
             last_updated = CURRENT_TIMESTAMP
         WHERE weight_type = 'keyword_boost' AND context_key = ?",
        params![MIN_WEIGHT, MAX_WEIGHT, adjustment, &keyword_lower],
    )?;

    // If no existing weight, insert new one
    if updated == 0 {
        let new_weight = (1.0 + adjustment).clamp(MIN_WEIGHT, MAX_WEIGHT);
        conn.execute(
            "INSERT INTO bias_weights (weight_type, context_key, weight, correction_count, last_updated)
             VALUES ('keyword_boost', ?, ?, 1, CURRENT_TIMESTAMP)",
            params![&keyword_lower, new_weight],
        )?;
    }

    Ok(())
}

/// Adjust category-term weight
fn adjust_category_term_weight(
    conn: &Connection,
    category_id: i64,
    term: &str,
    adjustment: f64,
) -> Result<(), rusqlite::Error> {
    let term_lower = term.to_lowercase();
    let context_key = category_id.to_string();

    // Try to update existing weight
    let updated = conn.execute(
        "UPDATE bias_weights
         SET weight = MAX(?, MIN(?, weight + ?)),
             correction_count = correction_count + 1,
             last_updated = CURRENT_TIMESTAMP
         WHERE weight_type = 'category_term' AND context_key = ? AND term = ?",
        params![MIN_WEIGHT, MAX_WEIGHT, adjustment, &context_key, &term_lower],
    )?;

    // If no existing weight, insert new one
    if updated == 0 {
        let new_weight = (1.0 + adjustment).clamp(MIN_WEIGHT, MAX_WEIGHT);
        conn.execute(
            "INSERT INTO bias_weights (weight_type, context_key, term, weight, correction_count, last_updated)
             VALUES ('category_term', ?, ?, ?, 1, CURRENT_TIMESTAMP)",
            params![&context_key, &term_lower, new_weight],
        )?;
    }

    Ok(())
}

/// Get bias weight statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasStats {
    pub total_weights: i64,
    pub total_corrections: i64,
    pub keyword_boost_count: i64,
    pub category_term_count: i64,
    /// Number of articles with political_bias data (for recommendations)
    pub articles_with_bias: i64,
}

pub fn get_bias_stats(conn: &Connection) -> Result<BiasStats, rusqlite::Error> {
    let total_weights: i64 = conn.query_row(
        "SELECT COUNT(*) FROM bias_weights",
        [],
        |row| row.get(0),
    )?;

    let total_corrections: i64 = conn.query_row(
        "SELECT COALESCE(SUM(correction_count), 0) FROM bias_weights",
        [],
        |row| row.get(0),
    )?;

    let keyword_boost_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM bias_weights WHERE weight_type = 'keyword_boost'",
        [],
        |row| row.get(0),
    )?;

    let category_term_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM bias_weights WHERE weight_type = 'category_term'",
        [],
        |row| row.get(0),
    )?;

    // Count articles with political_bias data (needed for recommendations feature)
    let articles_with_bias: i64 = conn.query_row(
        "SELECT COUNT(*) FROM fnords WHERE political_bias IS NOT NULL",
        [],
        |row| row.get(0),
    )?;

    Ok(BiasStats {
        total_weights,
        total_corrections,
        keyword_boost_count,
        category_term_count,
        articles_with_bias,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE bias_weights (
                id INTEGER PRIMARY KEY,
                weight_type TEXT NOT NULL,
                context_key TEXT NOT NULL,
                term TEXT,
                weight REAL DEFAULT 1.0,
                correction_count INTEGER DEFAULT 0,
                last_updated DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE UNIQUE INDEX idx_bias_weights_unique ON bias_weights(weight_type, context_key, COALESCE(term, ''));",
        ).unwrap();
        conn
    }

    #[test]
    fn test_adjust_keyword_boost_new() {
        let conn = setup_test_db();

        // First adjustment creates new entry
        adjust_keyword_boost(&conn, "Politik", 0.1).unwrap();

        let (weight, count): (f64, i64) = conn.query_row(
            "SELECT weight, correction_count FROM bias_weights WHERE context_key = 'politik'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();

        assert!((weight - 1.1).abs() < 0.001);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_adjust_keyword_boost_existing() {
        let conn = setup_test_db();

        // Create initial entry
        adjust_keyword_boost(&conn, "Politik", 0.1).unwrap();
        // Adjust again
        adjust_keyword_boost(&conn, "Politik", 0.1).unwrap();

        let (weight, count): (f64, i64) = conn.query_row(
            "SELECT weight, correction_count FROM bias_weights WHERE context_key = 'politik'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();

        assert!((weight - 1.2).abs() < 0.001);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_adjust_keyword_boost_clamp() {
        let conn = setup_test_db();

        // Many negative adjustments should not go below MIN_WEIGHT
        for _ in 0..20 {
            adjust_keyword_boost(&conn, "BadKeyword", -0.2).unwrap();
        }

        let weight: f64 = conn.query_row(
            "SELECT weight FROM bias_weights WHERE context_key = 'badkeyword'",
            [],
            |row| row.get(0),
        ).unwrap();

        assert!(weight >= MIN_WEIGHT);
    }

    #[test]
    fn test_load_from_db() {
        let conn = setup_test_db();

        // Insert test data
        conn.execute(
            "INSERT INTO bias_weights (weight_type, context_key, term, weight) VALUES
             ('keyword_boost', 'politik', NULL, 1.2),
             ('category_term', '201', 'regierung', 1.5),
             ('source_weight', 'ai', NULL, 0.9)",
            [],
        ).unwrap();

        let weights = BiasWeights::load_from_db(&conn).unwrap();

        assert!((weights.get_keyword_boost("politik") - 1.2).abs() < 0.001);
        assert!((weights.get_category_term_weight(201, "regierung") - 1.5).abs() < 0.001);
        assert!((weights.get_source_weight("ai") - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_bias_weights_defaults() {
        let weights = BiasWeights::new();

        // Unknown entries should return 1.0
        assert_eq!(weights.get_keyword_boost("unknown"), 1.0);
        assert_eq!(weights.get_category_term_weight(999, "unknown"), 1.0);
        assert_eq!(weights.get_source_weight("unknown"), 1.0);
    }
}
