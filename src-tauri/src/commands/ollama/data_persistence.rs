//! Database persistence functions for article categories, keywords, and embeddings

use crate::commands::settings::get_embedding_model_from_db;
use crate::db::Database;
use crate::embeddings::embedding_to_blob;
use crate::ollama::OllamaClient;
use crate::{find_canonical_keyword_with_db, normalize_keyword, split_compound_keyword};
use log::{debug, warn};
use rusqlite::Connection;

use super::types::{CategoryWithSource, KeywordWithSource};

// ============================================================
// CATEGORY PERSISTENCE
// ============================================================

/// Save categories (Sephiroth) for an article with default 'ai' source
pub fn save_article_categories(
    conn: &Connection,
    fnord_id: i64,
    categories: &[String],
) -> Vec<String> {
    let cats_with_source: Vec<CategoryWithSource> = categories
        .iter()
        .map(|c| CategoryWithSource {
            name: c.clone(),
            source: "ai".to_string(),
            confidence: 1.0,
        })
        .collect();
    save_article_categories_with_source(conn, fnord_id, &cats_with_source)
}

/// Save categories with source tracking (statistical vs ai)
pub fn save_article_categories_with_source(
    conn: &Connection,
    fnord_id: i64,
    categories: &[CategoryWithSource],
) -> Vec<String> {
    let mut saved = Vec::new();

    conn.execute("DELETE FROM fnord_sephiroth WHERE fnord_id = ?", [fnord_id])
        .ok();

    for cat in categories {
        if let Ok(sephiroth_id) = conn.query_row::<i64, _, _>(
            "SELECT id FROM sephiroth WHERE LOWER(name) = LOWER(?)",
            [&cat.name],
            |row| row.get(0),
        ) {
            conn.execute(
                r#"INSERT OR IGNORE INTO fnord_sephiroth
                   (fnord_id, sephiroth_id, confidence, source, assigned_at)
                   VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)"#,
                rusqlite::params![fnord_id, sephiroth_id, cat.confidence, &cat.source],
            )
            .ok();

            conn.execute(
                "UPDATE sephiroth SET article_count = (SELECT COUNT(*) FROM fnord_sephiroth WHERE sephiroth_id = ?) WHERE id = ?",
                rusqlite::params![sephiroth_id, sephiroth_id],
            )
            .ok();

            saved.push(cat.name.clone());
        }
    }

    saved
}

// ============================================================
// KEYWORD PERSISTENCE
// ============================================================

/// Save keywords (Immanentize) for an article with default 'ai' source
pub fn save_article_keywords_and_network(
    conn: &Connection,
    fnord_id: i64,
    keywords: &[String],
    categories_saved: &[String],
    article_date: Option<&str>,
) -> (Vec<String>, Vec<i64>) {
    use crate::keywords::types::KeywordSource;
    let kws_with_source: Vec<KeywordWithSource> = keywords
        .iter()
        .map(|k| KeywordWithSource {
            name: k.clone(),
            source: KeywordSource::Ai,
            confidence: 1.0,
        })
        .collect();
    save_article_keywords_with_source(conn, fnord_id, &kws_with_source, categories_saved, article_date)
}

/// Save keywords with source tracking (statistical vs ai)
pub fn save_article_keywords_with_source(
    conn: &Connection,
    fnord_id: i64,
    keywords: &[KeywordWithSource],
    categories_saved: &[String],
    article_date: Option<&str>,
) -> (Vec<String>, Vec<i64>) {
    let mut tags_saved = Vec::new();
    let mut tag_ids: Vec<i64> = Vec::new();
    let mut new_keyword_ids: Vec<i64> = Vec::new();

    let existing_tag_ids: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT immanentize_id FROM fnord_immanentize WHERE fnord_id = ?")
            .unwrap();
        stmt.query_map([fnord_id], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    };

    conn.execute(
        "DELETE FROM fnord_immanentize WHERE fnord_id = ?",
        [fnord_id],
    )
    .ok();

    // Expand compound keywords (e.g., "Trump-Zölle" → ["Trump-Zölle", "Trump", "Zölle"])
    let expanded_keywords: Vec<KeywordWithSource> = keywords
        .iter()
        .flat_map(|kw| {
            let split_parts = split_compound_keyword(&kw.name);
            let original_name = kw.name.clone();
            split_parts.into_iter().map(move |part| KeywordWithSource {
                confidence: if part != original_name {
                    kw.confidence * 0.8
                } else {
                    kw.confidence
                },
                name: part,
                source: kw.source.clone(),
            })
        })
        .collect();

    for kw in &expanded_keywords {
        let keyword = match normalize_keyword(&kw.name) {
            Some(k) => k,
            None => continue,
        };

        let canonical = find_canonical_keyword_with_db(&keyword);
        let store_keyword = canonical.as_deref().unwrap_or(&keyword);

        let existing_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM immanentize WHERE LOWER(name) = LOWER(?)",
                [store_keyword],
                |row| row.get(0),
            )
            .ok();

        let is_new_keyword = existing_id.is_none();
        let is_new_for_article = existing_id
            .map(|id| !existing_tag_ids.contains(&id))
            .unwrap_or(true);

        if is_new_for_article {
            if existing_id.is_some() {
                conn.execute(
                    r#"UPDATE immanentize SET
                           count = count + 1,
                           article_count = article_count + 1,
                           last_used = CURRENT_TIMESTAMP
                       WHERE LOWER(name) = LOWER(?1)"#,
                    [store_keyword],
                )
                .ok();
            } else {
                conn.execute(
                    r#"INSERT INTO immanentize (name, count, article_count, first_seen, last_used, is_canonical)
                       VALUES (?1, 1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, TRUE)"#,
                    [store_keyword],
                )
                .ok();
            }
        } else {
            conn.execute(
                r#"UPDATE immanentize SET
                       count = count + 1,
                       last_used = CURRENT_TIMESTAMP
                   WHERE LOWER(name) = LOWER(?1)"#,
                [store_keyword],
            )
            .ok();
        }

        if let Ok(tag_id) = conn.query_row::<i64, _, _>(
            "SELECT id FROM immanentize WHERE LOWER(name) = LOWER(?)",
            [store_keyword],
            |row| row.get(0),
        ) {
            conn.execute(
                "INSERT OR IGNORE INTO fnord_immanentize (fnord_id, immanentize_id, source, confidence) VALUES (?, ?, ?, ?)",
                rusqlite::params![fnord_id, tag_id, kw.source_str(), kw.confidence],
            )
            .ok();

            if let Some(date) = article_date {
                conn.execute(
                    r#"INSERT INTO immanentize_daily (immanentize_id, date, count)
                       VALUES (?1, ?2, 1)
                       ON CONFLICT(immanentize_id, date) DO UPDATE SET count = count + 1"#,
                    rusqlite::params![tag_id, date],
                )
                .ok();
            }

            if is_new_keyword {
                new_keyword_ids.push(tag_id);
            }

            tags_saved.push(keyword.to_string());
            tag_ids.push(tag_id);
        }
    }

    // Queue new keywords for embedding generation
    for keyword_id in &new_keyword_ids {
        conn.execute(
            r#"INSERT OR IGNORE INTO embedding_queue (immanentize_id, priority, queued_at)
               VALUES (?1, 0, CURRENT_TIMESTAMP)"#,
            [keyword_id],
        )
        .ok();
    }

    // Update keyword-category associations
    for tag_id in &tag_ids {
        for cat_name in categories_saved {
            if let Ok(sephiroth_id) = conn.query_row::<i64, _, _>(
                "SELECT id FROM sephiroth WHERE LOWER(name) = LOWER(?)",
                [cat_name],
                |row| row.get(0),
            ) {
                conn.execute(
                    r#"INSERT INTO immanentize_sephiroth
                       (immanentize_id, sephiroth_id, weight, article_count, first_seen, updated_at)
                       VALUES (?1, ?2, 1.0, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                       ON CONFLICT(immanentize_id, sephiroth_id) DO UPDATE SET
                           article_count = article_count + 1,
                           updated_at = CURRENT_TIMESTAMP"#,
                    rusqlite::params![tag_id, sephiroth_id],
                )
                .ok();
            }
        }
    }

    // Update keyword co-occurrence network
    for i in 0..tag_ids.len() {
        for j in (i + 1)..tag_ids.len() {
            let (id_a, id_b) = if tag_ids[i] < tag_ids[j] {
                (tag_ids[i], tag_ids[j])
            } else {
                (tag_ids[j], tag_ids[i])
            };

            conn.execute(
                r#"INSERT INTO immanentize_neighbors
                   (immanentize_id_a, immanentize_id_b, cooccurrence, first_seen, last_seen)
                   VALUES (?1, ?2, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                   ON CONFLICT(immanentize_id_a, immanentize_id_b) DO UPDATE SET
                       cooccurrence = cooccurrence + 1,
                       last_seen = CURRENT_TIMESTAMP"#,
                rusqlite::params![id_a, id_b],
            )
            .ok();
        }
    }

    (tags_saved, tag_ids)
}

/// Recalculate keyword weights after saving
pub fn recalculate_keyword_weights(conn: &Connection, tag_ids: &[i64]) {
    for tag_id in tag_ids {
        conn.execute(
            r#"UPDATE immanentize_sephiroth
               SET weight = CAST(article_count AS REAL) / (
                   SELECT MAX(article_count) FROM immanentize_sephiroth
                   WHERE immanentize_id = ?1
               )
               WHERE immanentize_id = ?1"#,
            [tag_id],
        )
        .ok();
    }

    conn.execute(
        r#"UPDATE immanentize_neighbors
           SET combined_weight = CAST(cooccurrence AS REAL) / (
               SELECT MAX(cooccurrence) FROM immanentize_neighbors
           )
           WHERE immanentize_id_a IN (SELECT value FROM json_each(?1))
              OR immanentize_id_b IN (SELECT value FROM json_each(?1))"#,
        [serde_json::to_string(&tag_ids).unwrap_or_default()],
    )
    .ok();
}

// ============================================================
// EMBEDDING PERSISTENCE
// ============================================================

/// Save an article embedding to the database (fnords.embedding + vec_fnords)
pub fn save_article_embedding(
    conn: &Connection,
    fnord_id: i64,
    embedding: &[f32],
) -> Result<(), String> {
    let blob = embedding_to_blob(embedding);

    conn.execute(
        "UPDATE fnords SET embedding = ?1, embedding_at = datetime('now') WHERE id = ?2",
        rusqlite::params![blob, fnord_id],
    )
    .map_err(|e| format!("Failed to save article embedding: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO vec_fnords (fnord_id, embedding) VALUES (?1, ?2)",
        rusqlite::params![fnord_id, blob],
    )
    .map_err(|e| {
        warn!("Failed to update vec_fnords: {}", e);
        e.to_string()
    })
    .ok();

    Ok(())
}

/// Generate and save embedding for an article
pub async fn generate_and_save_article_embedding(
    client: &OllamaClient,
    db: &std::sync::Arc<std::sync::Mutex<Database>>,
    fnord_id: i64,
    title: &str,
    content: &str,
) -> Result<(), String> {
    let model = {
        let db_guard = db.lock().map_err(|e| e.to_string())?;
        get_embedding_model_from_db(db_guard.conn())
    };

    let content_preview: String = content.chars().take(500).collect();
    let embedding_text = format!("{}\n\n{}", title, content_preview);

    let embedding = client
        .generate_embedding(&model, &embedding_text)
        .await
        .map_err(|e| format!("Embedding generation failed: {}", e))?;

    {
        let db_guard = db.lock().map_err(|e| e.to_string())?;
        save_article_embedding(db_guard.conn(), fnord_id, &embedding)?;
    }

    debug!("Generated embedding for article {}", fnord_id);
    Ok(())
}
