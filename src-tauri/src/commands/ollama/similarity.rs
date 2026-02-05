//! Similar articles and semantic search commands

use crate::commands::settings::get_embedding_model_from_db;
use crate::embeddings::embedding_to_blob;
use crate::ollama::OllamaClient;
use crate::AppState;
use log::info;
use tauri::{Emitter, State, Window};

use super::data_persistence::generate_and_save_article_embedding;
use super::types::{
    ArticleEmbeddingBatchResult, ArticleEmbeddingCount, ArticleEmbeddingProgress, SearchResult,
    SemanticSearchResponse, SimilarArticle, SimilarArticleCategory, SimilarArticleTag,
    SimilarArticlesResponse,
};

/// Type alias for basic article data with similarity score
/// (id, title, summary, image_url, similarity)
type BasicArticleData = (i64, String, Option<String>, Option<String>, f64);

/// Helper function to get tags for an article
fn get_article_tags(conn: &rusqlite::Connection, fnord_id: i64) -> Vec<SimilarArticleTag> {
    let mut stmt = match conn.prepare(
        r#"SELECT i.id, i.name
           FROM immanentize i
           JOIN fnord_immanentize fi ON fi.immanentize_id = i.id
           WHERE fi.fnord_id = ?
           ORDER BY i.article_count DESC
           LIMIT 5"#,
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    stmt.query_map([fnord_id], |row| {
        Ok(SimilarArticleTag {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })
    .map(|iter| iter.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

/// Helper function to get main categories for an article
fn get_article_main_categories(
    conn: &rusqlite::Connection,
    fnord_id: i64,
) -> Vec<SimilarArticleCategory> {
    let mut stmt = match conn.prepare(
        r#"SELECT DISTINCT m.id, m.name, m.icon, m.color
           FROM sephiroth m
           JOIN sephiroth s ON (s.parent_id = m.id OR s.id = m.id)
           JOIN fnord_sephiroth fs ON fs.sephiroth_id = s.id
           WHERE fs.fnord_id = ? AND m.level = 0
           ORDER BY m.name"#,
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    stmt.query_map([fnord_id], |row| {
        Ok(SimilarArticleCategory {
            id: row.get(0)?,
            name: row.get(1)?,
            icon: row.get(2)?,
            color: row.get(3)?,
        })
    })
    .map(|iter| iter.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

/// Find similar articles based on embedding similarity
#[tauri::command]
pub fn find_similar_articles(
    state: State<AppState>,
    fnord_id: i64,
    limit: Option<i64>,
) -> Result<SimilarArticlesResponse, String> {
    let limit = limit.unwrap_or(5);
    let db = state.db_conn()?;

    let embedding: Option<Vec<u8>> = db
        .conn()
        .query_row(
            "SELECT embedding FROM fnords WHERE id = ?",
            [fnord_id],
            |row| row.get(0),
        )
        .ok();

    let embedding = match embedding {
        Some(e) if !e.is_empty() => e,
        _ => {
            return Ok(SimilarArticlesResponse {
                fnord_id,
                similar: vec![],
            });
        }
    };

    let mut stmt = db
        .conn()
        .prepare(
            r#"SELECT
                v.fnord_id,
                v.distance,
                f.title,
                p.title as pentacle_title,
                f.published_at
            FROM vec_fnords v
            JOIN fnords f ON f.id = v.fnord_id
            LEFT JOIN pentacles p ON p.id = f.pentacle_id
            WHERE v.embedding MATCH ?1
            AND k = ?2
            AND v.fnord_id != ?3
            ORDER BY v.distance ASC"#,
        )
        .map_err(|e| e.to_string())?;

    let basic_articles: Vec<BasicArticleData> = stmt
        .query_map(rusqlite::params![embedding, limit + 1, fnord_id], |row| {
            let distance: f64 = row.get(1)?;
            let similarity = 1.0 - (distance / 2.0);
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                similarity,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .filter(|(_, _, _, _, similarity)| *similarity >= 0.5)
        .take(limit as usize)
        .collect();

    let similar: Vec<SimilarArticle> = basic_articles
        .into_iter()
        .map(|(article_id, title, pentacle_title, published_at, similarity)| {
            let tags = get_article_tags(db.conn(), article_id);
            let categories = get_article_main_categories(db.conn(), article_id);
            SimilarArticle {
                fnord_id: article_id,
                title,
                pentacle_title,
                published_at,
                similarity,
                tags,
                categories,
            }
        })
        .collect();

    Ok(SimilarArticlesResponse { fnord_id, similar })
}

/// Perform semantic search by embedding the query
#[tauri::command]
pub async fn semantic_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<i64>,
) -> Result<SemanticSearchResponse, String> {
    let limit = limit.unwrap_or(20);

    if query.trim().is_empty() {
        return Ok(SemanticSearchResponse {
            query,
            results: vec![],
        });
    }

    let (embedding_model, client) = {
        let db = state.db_conn()?;
        let model = get_embedding_model_from_db(db.conn());
        (model, OllamaClient::new(None))
    };

    let query_embedding = client
        .generate_embedding(&embedding_model, &query)
        .await
        .map_err(|e| format!("Failed to generate query embedding: {}", e))?;

    let query_blob = embedding_to_blob(&query_embedding);

    let results: Vec<SearchResult> = {
        let db = state.db_conn()?;

        let mut stmt = db
            .conn()
            .prepare(
                r#"SELECT
                    v.fnord_id,
                    v.distance,
                    f.title,
                    p.title as pentacle_title,
                    f.published_at,
                    f.summary
                FROM vec_fnords v
                JOIN fnords f ON f.id = v.fnord_id
                LEFT JOIN pentacles p ON p.id = f.pentacle_id
                WHERE v.embedding MATCH ?1
                AND k = ?2
                ORDER BY v.distance ASC"#,
            )
            .map_err(|e| e.to_string())?;

        let result: Vec<SearchResult> = stmt
            .query_map(rusqlite::params![query_blob, limit], |row| {
                let distance: f64 = row.get(1)?;
                let similarity = 1.0 - (distance / 2.0);
                Ok(SearchResult {
                    fnord_id: row.get(0)?,
                    title: row.get(2)?,
                    pentacle_title: row.get(3)?,
                    published_at: row.get(4)?,
                    summary: row.get(5)?,
                    similarity,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .filter(|r| r.similarity >= 0.3)
            .collect();
        result
    };

    info!(
        "Semantic search for '{}' found {} results",
        query,
        results.len()
    );

    Ok(SemanticSearchResponse { query, results })
}

/// Get count of articles with and without embeddings
#[tauri::command]
pub fn get_article_embedding_stats(state: State<AppState>) -> Result<ArticleEmbeddingCount, String> {
    let db = state.db_conn()?;

    let total_articles: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM fnords", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let with_embedding: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM fnords WHERE embedding IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let processable: i64 = db
        .conn()
        .query_row(
            r#"SELECT COUNT(*) FROM fnords
               WHERE embedding IS NULL
               AND processed_at IS NOT NULL
               AND content_full IS NOT NULL
               AND LENGTH(content_full) >= 100"#,
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(ArticleEmbeddingCount {
        total_articles,
        with_embedding,
        without_embedding: total_articles - with_embedding,
        processable,
    })
}

/// Generate embeddings for processed articles that don't have one
#[tauri::command]
pub async fn generate_article_embeddings_batch(
    window: Window,
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<ArticleEmbeddingBatchResult, String> {
    let limit = limit.unwrap_or(1000);

    let articles: Vec<(i64, String, String)> = {
        let db = state.db_conn()?;
        let mut stmt = db
            .conn()
            .prepare(
                r#"SELECT id, title, content_full
                   FROM fnords
                   WHERE embedding IS NULL
                   AND processed_at IS NOT NULL
                   AND content_full IS NOT NULL
                   AND LENGTH(content_full) >= 100
                   ORDER BY processed_at DESC
                   LIMIT ?"#,
            )
            .map_err(|e| e.to_string())?;

        let result: Vec<(i64, String, String)> = stmt
            .query_map([limit], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        result
    };

    let total = articles.len() as i64;
    if total == 0 {
        return Ok(ArticleEmbeddingBatchResult {
            processed: 0,
            succeeded: 0,
            failed: 0,
        });
    }

    let _ = window.emit(
        "article-embedding-progress",
        ArticleEmbeddingProgress {
            current: 0,
            total,
            fnord_id: 0,
            title: "Starting...".to_string(),
            success: true,
            error: None,
        },
    );

    let client = OllamaClient::new(None);
    let mut succeeded = 0i64;
    let mut failed = 0i64;

    for (idx, (fnord_id, title, content)) in articles.into_iter().enumerate() {
        let result =
            generate_and_save_article_embedding(&client, &state.db, fnord_id, &title, &content).await;

        let (success, error) = match result {
            Ok(()) => {
                succeeded += 1;
                (true, None)
            }
            Err(e) => {
                failed += 1;
                (false, Some(e))
            }
        };

        let _ = window.emit(
            "article-embedding-progress",
            ArticleEmbeddingProgress {
                current: (idx + 1) as i64,
                total,
                fnord_id,
                title: title.clone(),
                success,
                error,
            },
        );
    }

    info!(
        "Article embedding batch complete: {} succeeded, {} failed",
        succeeded, failed
    );

    Ok(ArticleEmbeddingBatchResult {
        processed: total,
        succeeded,
        failed,
    })
}
