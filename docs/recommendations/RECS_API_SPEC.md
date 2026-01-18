# RECS_API_SPEC.md — Backend API Spezifikation

**Erstellt:** 2026-01-18
**Status:** Phase 2 — Technical Design

---

## 1. Übersicht

### Neue Tauri Commands

| Command | Parameter | Return | Beschreibung |
|---------|-----------|--------|--------------|
| `get_recommendations` | `limit?` | `Vec<Recommendation>` | Personalisierte Empfehlungen |
| `save_article` | `fnord_id` | `Result<()>` | Artikel merken |
| `unsave_article` | `fnord_id` | `Result<()>` | Merkung aufheben |
| `hide_recommendation` | `fnord_id` | `Result<()>` | Empfehlung ausblenden |
| `get_saved_articles` | `limit?` | `Vec<SavedArticle>` | Gemerkte Artikel |
| `get_recommendation_stats` | - | `RecommendationStats` | Debug-Statistiken |

---

## 2. Datenstrukturen

### 2.1 Recommendation

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Recommendation {
    /// Artikel-ID
    pub fnord_id: i64,

    /// Artikel-Metadaten
    pub title: String,
    pub summary: Option<String>,
    pub url: String,
    pub image_url: Option<String>,

    /// Quelle
    pub pentacle_id: i64,
    pub pentacle_title: Option<String>,
    pub pentacle_icon: Option<String>,

    /// Zeitstempel
    pub published_at: Option<String>,

    /// Scores (für Debugging/UI)
    pub relevance_score: f64,
    pub freshness_score: f64,

    /// KI-Analyse (falls vorhanden)
    pub political_bias: Option<i32>,
    pub sachlichkeit: Option<i32>,

    /// Kategorien
    pub categories: Vec<CategoryInfo>,

    /// Keywords (für Erklärung)
    pub matching_keywords: Vec<String>,

    /// Erklärung (menschenlesbar)
    pub explanation: String,

    /// UI-Status
    pub is_saved: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryInfo {
    pub id: i64,
    pub name: String,
    pub icon: Option<String>,
}
```

### 2.2 SavedArticle

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SavedArticle {
    pub fnord_id: i64,
    pub title: String,
    pub pentacle_title: Option<String>,
    pub saved_at: String,
    pub published_at: Option<String>,
}
```

### 2.3 RecommendationStats

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct RecommendationStats {
    /// Profil-Statistiken
    pub articles_read: i64,
    pub articles_saved: i64,
    pub articles_hidden: i64,

    /// Keyword-Profil
    pub top_keywords: Vec<KeywordWeight>,

    /// Kategorie-Profil
    pub top_categories: Vec<CategoryWeight>,

    /// Engine-Metriken
    pub candidate_pool_size: i64,
    pub embedding_coverage: f64,  // % Artikel mit Embedding
    pub last_profile_update: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeywordWeight {
    pub name: String,
    pub weight: f64,
    pub article_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryWeight {
    pub name: String,
    pub weight: f64,
}
```

---

## 3. Command Implementierungen

### 3.1 get_recommendations

```rust
/// Generiere personalisierte Empfehlungen
///
/// # Arguments
/// * `limit` - Maximale Anzahl Empfehlungen (default: 10, max: 50)
///
/// # Returns
/// * `Vec<Recommendation>` - Sortiert nach Relevanz, mit Erklärungen
#[tauri::command]
pub async fn get_recommendations(
    state: State<'_, AppState>,
    limit: Option<i32>,
) -> Result<Vec<Recommendation>, String> {
    let limit = limit.unwrap_or(10).min(50) as usize;
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // 1. User-Profil erstellen
    let profile = UserProfile::build(&db).map_err(|e| e.to_string())?;

    // Cold Start Check
    if profile.total_read == 0 {
        return get_cold_start_recommendations(&db, limit);
    }

    // 2. Kandidaten generieren
    let candidates = CandidateGenerator::generate(&db, &profile, 100)
        .map_err(|e| e.to_string())?;

    // 3. Scoring
    let mut scored: Vec<Candidate> = candidates;
    for c in &mut scored {
        Scorer::score(c, &profile, &db).map_err(|e| e.to_string())?;
    }

    // 4. Reranking (MMR mit λ=0.7)
    let reranked = Reranker::rerank(scored, limit, 0.7);

    // 5. Zu Recommendation konvertieren
    let mut recommendations = Vec::new();
    for candidate in reranked {
        let rec = build_recommendation(&db, candidate, &profile)?;
        recommendations.push(rec);
    }

    Ok(recommendations)
}

fn get_cold_start_recommendations(
    db: &Database,
    limit: usize,
) -> Result<Vec<Recommendation>, String> {
    // Fallback: Aktuelle Artikel von beliebten Quellen
    let query = r#"
        SELECT
            f.id, f.title, f.summary, f.url, f.image_url,
            f.published_at, f.political_bias, f.sachlichkeit,
            p.id as pentacle_id, p.title as pentacle_title, p.icon_url
        FROM fnords f
        JOIN pentacles p ON p.id = f.pentacle_id
        WHERE f.read_at IS NULL
          AND f.summary IS NOT NULL
          AND f.published_at > datetime('now', '-48 hours')
        ORDER BY f.published_at DESC
        LIMIT ?
    "#;

    let results = db.query(query, &[limit])?;

    let mut recommendations = Vec::new();
    for row in results {
        recommendations.push(Recommendation {
            fnord_id: row.get("id")?,
            title: row.get("title")?,
            summary: row.get("summary")?,
            url: row.get("url")?,
            image_url: row.get("image_url")?,
            pentacle_id: row.get("pentacle_id")?,
            pentacle_title: row.get("pentacle_title")?,
            pentacle_icon: row.get("icon_url")?,
            published_at: row.get("published_at")?,
            relevance_score: 0.5,
            freshness_score: 1.0,
            political_bias: row.get("political_bias")?,
            sachlichkeit: row.get("sachlichkeit")?,
            categories: get_article_categories(db, row.get("id")?)?,
            matching_keywords: vec![],
            explanation: "Aktuelle Nachrichten".to_string(),
            is_saved: false,
        });
    }

    Ok(recommendations)
}
```

### 3.2 save_article / unsave_article

```rust
/// Artikel als gemerkt markieren
#[tauri::command]
pub fn save_article(
    state: State<'_, AppState>,
    fnord_id: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.execute(
        r#"
        INSERT INTO recommendation_feedback (fnord_id, action)
        VALUES (?1, 'save')
        ON CONFLICT (fnord_id, action) DO NOTHING
        "#,
        params![fnord_id],
    ).map_err(|e| e.to_string())?;

    // Invalidiere Cache
    state.recommendation_cache.invalidate();

    Ok(())
}

/// Merkung aufheben
#[tauri::command]
pub fn unsave_article(
    state: State<'_, AppState>,
    fnord_id: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.execute(
        "DELETE FROM recommendation_feedback WHERE fnord_id = ?1 AND action = 'save'",
        params![fnord_id],
    ).map_err(|e| e.to_string())?;

    state.recommendation_cache.invalidate();

    Ok(())
}
```

### 3.3 hide_recommendation

```rust
/// Empfehlung ausblenden (wird nicht mehr vorgeschlagen)
#[tauri::command]
pub fn hide_recommendation(
    state: State<'_, AppState>,
    fnord_id: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.execute(
        r#"
        INSERT INTO recommendation_feedback (fnord_id, action)
        VALUES (?1, 'hide')
        ON CONFLICT (fnord_id, action) DO NOTHING
        "#,
        params![fnord_id],
    ).map_err(|e| e.to_string())?;

    state.recommendation_cache.invalidate();

    Ok(())
}
```

### 3.4 get_saved_articles

```rust
/// Alle gemerkten Artikel abrufen
#[tauri::command]
pub fn get_saved_articles(
    state: State<'_, AppState>,
    limit: Option<i32>,
) -> Result<Vec<SavedArticle>, String> {
    let limit = limit.unwrap_or(50).min(100);
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let query = r#"
        SELECT
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
        LIMIT ?
    "#;

    let results = db.query(query, &[limit])?;

    let mut articles = Vec::new();
    for row in results {
        articles.push(SavedArticle {
            fnord_id: row.get("fnord_id")?,
            title: row.get("title")?,
            pentacle_title: row.get("pentacle_title")?,
            saved_at: row.get("saved_at")?,
            published_at: row.get("published_at")?,
        });
    }

    Ok(articles)
}
```

### 3.5 get_recommendation_stats

```rust
/// Debug-Statistiken für das Empfehlungssystem
#[tauri::command]
pub fn get_recommendation_stats(
    state: State<'_, AppState>,
) -> Result<RecommendationStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let articles_read: i64 = db.query_row(
        "SELECT COUNT(*) FROM fnords WHERE read_at IS NOT NULL",
        [],
    )?;

    let articles_saved: i64 = db.query_row(
        "SELECT COUNT(*) FROM recommendation_feedback WHERE action = 'save'",
        [],
    )?;

    let articles_hidden: i64 = db.query_row(
        "SELECT COUNT(*) FROM recommendation_feedback WHERE action = 'hide'",
        [],
    )?;

    let total_articles: i64 = db.query_row("SELECT COUNT(*) FROM fnords", [])?;
    let with_embedding: i64 = db.query_row(
        "SELECT COUNT(*) FROM fnords WHERE embedding IS NOT NULL",
        [],
    )?;

    let embedding_coverage = if total_articles > 0 {
        with_embedding as f64 / total_articles as f64
    } else {
        0.0
    };

    // Top Keywords
    let top_keywords = get_top_user_keywords(&db, 10)?;

    // Top Categories
    let top_categories = get_top_user_categories(&db, 5)?;

    Ok(RecommendationStats {
        articles_read,
        articles_saved,
        articles_hidden,
        top_keywords,
        top_categories,
        candidate_pool_size: total_articles - articles_read - articles_hidden,
        embedding_coverage,
        last_profile_update: None, // TODO: aus Cache
    })
}
```

---

## 4. Neue Datenbank-Tabelle

```sql
-- Feedback für Empfehlungen
CREATE TABLE IF NOT EXISTS recommendation_feedback (
    id INTEGER PRIMARY KEY,
    fnord_id INTEGER NOT NULL,
    action TEXT NOT NULL CHECK(action IN ('save', 'hide', 'click')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(fnord_id, action),
    FOREIGN KEY (fnord_id) REFERENCES fnords(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_feedback_action ON recommendation_feedback(action);
CREATE INDEX IF NOT EXISTS idx_feedback_fnord ON recommendation_feedback(fnord_id);
CREATE INDEX IF NOT EXISTS idx_feedback_created ON recommendation_feedback(created_at DESC);
```

---

## 5. Integration in lib.rs

```rust
// In src-tauri/src/lib.rs

mod commands {
    // ... existing modules ...
    pub mod recommendations;
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // ... existing commands ...

            // Recommendations
            commands::recommendations::get_recommendations,
            commands::recommendations::save_article,
            commands::recommendations::unsave_article,
            commands::recommendations::hide_recommendation,
            commands::recommendations::get_saved_articles,
            commands::recommendations::get_recommendation_stats,
        ])
        // ...
}
```

---

## 6. Frontend TypeScript Types

```typescript
// src/lib/types.ts

export interface Recommendation {
  fnord_id: number;
  title: string;
  summary: string | null;
  url: string;
  image_url: string | null;

  pentacle_id: number;
  pentacle_title: string | null;
  pentacle_icon: string | null;

  published_at: string | null;

  relevance_score: number;
  freshness_score: number;

  political_bias: number | null;
  sachlichkeit: number | null;

  categories: CategoryInfo[];
  matching_keywords: string[];
  explanation: string;

  is_saved: boolean;
}

export interface CategoryInfo {
  id: number;
  name: string;
  icon: string | null;
}

export interface SavedArticle {
  fnord_id: number;
  title: string;
  pentacle_title: string | null;
  saved_at: string;
  published_at: string | null;
}

export interface RecommendationStats {
  articles_read: number;
  articles_saved: number;
  articles_hidden: number;
  top_keywords: KeywordWeight[];
  top_categories: CategoryWeight[];
  candidate_pool_size: number;
  embedding_coverage: number;
  last_profile_update: string | null;
}

export interface KeywordWeight {
  name: string;
  weight: number;
  article_count: number;
}

export interface CategoryWeight {
  name: string;
  weight: number;
}
```

---

## 7. Frontend API Wrapper

```typescript
// src/lib/api/recommendations.ts

import { invoke } from '@tauri-apps/api/core';
import type { Recommendation, SavedArticle, RecommendationStats } from '$lib/types';

export async function getRecommendations(limit?: number): Promise<Recommendation[]> {
  return invoke('get_recommendations', { limit });
}

export async function saveArticle(fnordId: number): Promise<void> {
  return invoke('save_article', { fnordId });
}

export async function unsaveArticle(fnordId: number): Promise<void> {
  return invoke('unsave_article', { fnordId });
}

export async function hideRecommendation(fnordId: number): Promise<void> {
  return invoke('hide_recommendation', { fnordId });
}

export async function getSavedArticles(limit?: number): Promise<SavedArticle[]> {
  return invoke('get_saved_articles', { limit });
}

export async function getRecommendationStats(): Promise<RecommendationStats> {
  return invoke('get_recommendation_stats');
}
```

---

## 8. Logging & Observability

### 8.1 Logging-Events

```rust
// Bei jeder Empfehlungsanfrage
log::info!(
    target: "recommendations",
    "Generated {} recommendations for user (profile: {} read, {} saved)",
    recommendations.len(),
    profile.total_read,
    profile.saved_article_ids.len()
);

// Bei Feedback
log::debug!(
    target: "recommendations",
    "User {} article {} (action: {})",
    if action == "save" { "saved" } else { "hid" },
    fnord_id,
    action
);

// Performance-Metriken
log::debug!(
    target: "recommendations::perf",
    "Recommendation pipeline: profile={}ms, candidates={}ms, scoring={}ms, rerank={}ms, total={}ms",
    profile_time, candidate_time, scoring_time, rerank_time, total_time
);
```

### 8.2 Metriken für spätere Analyse

```sql
-- Recommendation-Log Tabelle (optional, für A/B Testing später)
CREATE TABLE IF NOT EXISTS recommendation_log (
    id INTEGER PRIMARY KEY,
    session_id TEXT,
    fnord_id INTEGER,
    position INTEGER,
    relevance_score REAL,
    explanation_type TEXT,
    shown_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    clicked_at DATETIME,
    saved_at DATETIME,
    hidden_at DATETIME
);
```

---

## 9. Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum RecommendationError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("No embeddings available for recommendations")]
    NoEmbeddings,

    #[error("Profile build failed: {0}")]
    ProfileBuildFailed(String),

    #[error("Article not found: {0}")]
    ArticleNotFound(i64),
}

impl From<RecommendationError> for String {
    fn from(e: RecommendationError) -> Self {
        e.to_string()
    }
}
```

---

## 10. Testing

### 10.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freshness_score() {
        // 0 Stunden alt → ~1.0
        let now = Utc::now();
        assert!(Scorer::compute_freshness(&Some(now)) > 0.95);

        // 48 Stunden alt → ~0.5
        let two_days_ago = now - Duration::hours(48);
        let score = Scorer::compute_freshness(&Some(two_days_ago));
        assert!(score > 0.45 && score < 0.55);

        // 7 Tage alt → sehr niedrig
        let week_ago = now - Duration::days(7);
        assert!(Scorer::compute_freshness(&Some(week_ago)) < 0.1);
    }

    #[test]
    fn test_mmr_reranking() {
        // Test dass MMR Diversity erhöht
        let candidates = vec![
            Candidate { fnord_id: 1, pentacle_id: 1, final_score: 0.9, .. },
            Candidate { fnord_id: 2, pentacle_id: 1, final_score: 0.85, .. },
            Candidate { fnord_id: 3, pentacle_id: 2, final_score: 0.8, .. },
        ];

        let reranked = Reranker::rerank(candidates, 3, 0.7);

        // Artikel 3 sollte vor Artikel 2 sein wegen Source-Diversity
        assert_eq!(reranked[0].fnord_id, 1);
        assert_eq!(reranked[1].fnord_id, 3); // Andere Quelle
        assert_eq!(reranked[2].fnord_id, 2);
    }
}
```

### 10.2 Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_recommendations_cold_start() {
        let db = setup_test_db();

        // Keine gelesenen Artikel
        let recs = get_recommendations(State::new(db), Some(5)).await.unwrap();

        // Sollte trotzdem Empfehlungen liefern (Popular)
        assert!(!recs.is_empty());

        // Alle sollten Erklärungen haben
        for rec in &recs {
            assert!(!rec.explanation.is_empty());
        }
    }
}
```
