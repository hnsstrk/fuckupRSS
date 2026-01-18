# RECS_ALGO_SPEC.md — Algorithmus-Spezifikation

**Erstellt:** 2026-01-18
**Status:** Phase 2 — Technical Design

---

## 1. Architektur-Übersicht

```
┌─────────────────────────────────────────────────────────────┐
│                     RECOMMENDATION ENGINE                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌───────────────┐    ┌───────────────┐    ┌────────────┐  │
│  │ User Profile  │    │   Candidate   │    │  Reranker  │  │
│  │   Builder     │ ──►│   Generator   │ ──►│            │  │
│  └───────────────┘    └───────────────┘    └────────────┘  │
│         │                    │                    │         │
│         ▼                    ▼                    ▼         │
│  ┌───────────────┐    ┌───────────────┐    ┌────────────┐  │
│  │ Read History  │    │  vec_fnords   │    │  Diversity │  │
│  │ Keywords      │    │  Keywords     │    │  Freshness │  │
│  │ Categories    │    │  Categories   │    │  Explain   │  │
│  └───────────────┘    └───────────────┘    └────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │ Top-N Results   │
                    │ + Explanations  │
                    └─────────────────┘
```

---

## 2. User Profile Builder

### 2.1 Profil-Struktur

```rust
pub struct UserProfile {
    /// Top-N Keywords aus gelesenen Artikeln (gewichtet)
    pub keyword_weights: HashMap<i64, f64>,  // immanentize_id -> weight

    /// Kategorie-Präferenzen
    pub category_weights: HashMap<i64, f64>, // sephiroth_id -> weight

    /// Durchschnittliches Embedding der gelesenen Artikel
    pub centroid_embedding: Option<Vec<f32>>,

    /// Gelesene Artikel-IDs (für Filterung)
    pub read_article_ids: HashSet<i64>,

    /// Hidden Artikel-IDs
    pub hidden_article_ids: HashSet<i64>,

    /// Saved Artikel-IDs (starkes Signal)
    pub saved_article_ids: HashSet<i64>,

    /// Statistiken
    pub total_read: i64,
    pub profile_freshness: DateTime<Utc>,
}
```

### 2.2 Profil-Berechnung

```rust
impl UserProfile {
    pub fn build(db: &Database) -> Result<Self, Error> {
        // 1. Gelesene Artikel laden
        let read_articles = db.query(
            "SELECT id FROM fnords WHERE read_at IS NOT NULL"
        )?;

        // 2. Saved Artikel (3x Gewichtung)
        let saved_articles = db.query(
            "SELECT fnord_id FROM recommendation_feedback WHERE action = 'save'"
        )?;

        // 3. Hidden Artikel
        let hidden_articles = db.query(
            "SELECT fnord_id FROM recommendation_feedback WHERE action = 'hide'"
        )?;

        // 4. Keywords aggregieren
        let keyword_weights = Self::aggregate_keywords(db, &read_articles, &saved_articles)?;

        // 5. Kategorien aggregieren
        let category_weights = Self::aggregate_categories(db, &read_articles)?;

        // 6. Centroid berechnen (optional, wenn genug Embeddings)
        let centroid = Self::compute_centroid(db, &read_articles)?;

        Ok(UserProfile {
            keyword_weights,
            category_weights,
            centroid_embedding: centroid,
            read_article_ids: read_articles.into_iter().collect(),
            hidden_article_ids: hidden_articles.into_iter().collect(),
            saved_article_ids: saved_articles.into_iter().collect(),
            total_read: read_articles.len() as i64,
            profile_freshness: Utc::now(),
        })
    }
}
```

### 2.3 Keyword-Aggregation

```rust
fn aggregate_keywords(
    db: &Database,
    read_ids: &[i64],
    saved_ids: &[i64],
) -> HashMap<i64, f64> {
    let mut weights: HashMap<i64, f64> = HashMap::new();

    // Basis-Gewicht aus Lese-Historie
    let query = r#"
        SELECT fi.immanentize_id, COUNT(*) as count, i.quality_score
        FROM fnord_immanentize fi
        JOIN immanentize i ON i.id = fi.immanentize_id
        WHERE fi.fnord_id IN (SELECT id FROM fnords WHERE read_at IS NOT NULL)
        GROUP BY fi.immanentize_id
        ORDER BY count DESC
        LIMIT 50
    "#;

    for row in db.query(query)? {
        let keyword_id = row.get("immanentize_id")?;
        let count = row.get::<f64>("count")?;
        let quality = row.get::<f64>("quality_score")?.unwrap_or(0.5);

        // TF-IDF-artiges Gewicht
        let weight = (1.0 + count.ln()) * quality;
        weights.insert(keyword_id, weight);
    }

    // Boost für Saved-Artikel Keywords (3x)
    for saved_id in saved_ids {
        let keywords = get_article_keywords(db, *saved_id)?;
        for kw_id in keywords {
            *weights.entry(kw_id).or_insert(0.0) *= 3.0;
        }
    }

    // Normalisieren
    let max_weight = weights.values().cloned().fold(0.0, f64::max);
    if max_weight > 0.0 {
        for w in weights.values_mut() {
            *w /= max_weight;
        }
    }

    weights
}
```

### 2.4 Centroid-Berechnung

```rust
fn compute_centroid(db: &Database, read_ids: &[i64]) -> Option<Vec<f32>> {
    // Nur wenn mindestens 3 Artikel mit Embeddings
    let embeddings: Vec<Vec<f32>> = db.query(
        "SELECT embedding FROM fnords WHERE id IN (?) AND embedding IS NOT NULL",
        read_ids
    )?.collect();

    if embeddings.len() < 3 {
        return None;
    }

    // Durchschnitt berechnen
    let dim = 1024;
    let mut centroid = vec![0.0f32; dim];

    for emb in &embeddings {
        for (i, v) in emb.iter().enumerate() {
            centroid[i] += v;
        }
    }

    let n = embeddings.len() as f32;
    for v in &mut centroid {
        *v /= n;
    }

    // Normalisieren für Cosine-Similarity
    let norm: f32 = centroid.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut centroid {
            *v /= norm;
        }
    }

    Some(centroid)
}
```

---

## 3. Candidate Generator

### 3.1 Multi-Source Candidate Generation

```rust
pub struct CandidateGenerator;

impl CandidateGenerator {
    pub fn generate(
        db: &Database,
        profile: &UserProfile,
        limit: usize,
    ) -> Result<Vec<Candidate>, Error> {
        let mut candidates: HashMap<i64, Candidate> = HashMap::new();

        // Source 1: Embedding Similarity (40 candidates)
        if let Some(centroid) = &profile.centroid_embedding {
            let embedding_candidates = Self::from_embedding_similarity(db, centroid, 40)?;
            Self::merge_candidates(&mut candidates, embedding_candidates, "embedding");
        }

        // Source 2: Keyword Overlap (40 candidates)
        let keyword_candidates = Self::from_keyword_overlap(db, profile, 40)?;
        Self::merge_candidates(&mut candidates, keyword_candidates, "keyword");

        // Source 3: Category Match (20 candidates)
        let category_candidates = Self::from_category_match(db, profile, 20)?;
        Self::merge_candidates(&mut candidates, category_candidates, "category");

        // Source 4: Recent Popular (Fallback, 20 candidates)
        let popular_candidates = Self::from_recent_popular(db, 20)?;
        Self::merge_candidates(&mut candidates, popular_candidates, "popular");

        // Filter bereits gelesene und hidden
        let filtered: Vec<Candidate> = candidates
            .into_values()
            .filter(|c| !profile.read_article_ids.contains(&c.fnord_id))
            .filter(|c| !profile.hidden_article_ids.contains(&c.fnord_id))
            .collect();

        Ok(filtered)
    }
}
```

### 3.2 Embedding-basierte Kandidaten

```rust
fn from_embedding_similarity(
    db: &Database,
    centroid: &[f32],
    limit: usize,
) -> Result<Vec<Candidate>, Error> {
    // sqlite-vec Query für KNN
    let query = r#"
        SELECT
            fnord_id,
            distance
        FROM vec_fnords
        WHERE embedding MATCH ?
          AND k = ?
        ORDER BY distance ASC
    "#;

    let results = db.query(query, &[centroid.as_blob(), limit * 2])?;

    let mut candidates = Vec::new();
    for row in results {
        let fnord_id: i64 = row.get("fnord_id")?;
        let distance: f64 = row.get("distance")?;

        // Cosine Distance → Similarity
        let similarity = 1.0 - distance;

        candidates.push(Candidate {
            fnord_id,
            embedding_score: Some(similarity),
            source: CandidateSource::Embedding,
            ..Default::default()
        });
    }

    Ok(candidates)
}
```

### 3.3 Keyword-basierte Kandidaten

```rust
fn from_keyword_overlap(
    db: &Database,
    profile: &UserProfile,
    limit: usize,
) -> Result<Vec<Candidate>, Error> {
    // Top User-Keywords
    let user_keywords: Vec<i64> = profile
        .keyword_weights
        .iter()
        .sorted_by(|a, b| b.1.partial_cmp(a.1).unwrap())
        .take(20)
        .map(|(id, _)| *id)
        .collect();

    if user_keywords.is_empty() {
        return Ok(vec![]);
    }

    // Artikel mit überlappenden Keywords
    let query = r#"
        SELECT
            f.id as fnord_id,
            COUNT(DISTINCT fi.immanentize_id) as overlap_count,
            GROUP_CONCAT(i.name) as matching_keywords
        FROM fnords f
        JOIN fnord_immanentize fi ON f.id = fi.fnord_id
        JOIN immanentize i ON i.id = fi.immanentize_id
        WHERE fi.immanentize_id IN (?)
          AND f.read_at IS NULL
          AND f.embedding IS NOT NULL
        GROUP BY f.id
        HAVING overlap_count >= 2
        ORDER BY overlap_count DESC
        LIMIT ?
    "#;

    let results = db.query(query, &[user_keywords.as_slice(), limit * 2])?;

    let mut candidates = Vec::new();
    for row in results {
        let fnord_id: i64 = row.get("fnord_id")?;
        let overlap: i64 = row.get("overlap_count")?;
        let matching: String = row.get("matching_keywords")?;

        // Jaccard-artiger Score
        let user_kw_count = user_keywords.len() as f64;
        let overlap_score = overlap as f64 / (user_kw_count + 5.0); // Damping

        candidates.push(Candidate {
            fnord_id,
            keyword_score: Some(overlap_score),
            matching_keywords: Some(matching),
            source: CandidateSource::Keyword,
            ..Default::default()
        });
    }

    Ok(candidates)
}
```

### 3.4 Kategorie-basierte Kandidaten

```rust
fn from_category_match(
    db: &Database,
    profile: &UserProfile,
    limit: usize,
) -> Result<Vec<Candidate>, Error> {
    // Top User-Kategorien
    let top_categories: Vec<i64> = profile
        .category_weights
        .iter()
        .sorted_by(|a, b| b.1.partial_cmp(a.1).unwrap())
        .take(5)
        .map(|(id, _)| *id)
        .collect();

    if top_categories.is_empty() {
        return Ok(vec![]);
    }

    let query = r#"
        SELECT
            f.id as fnord_id,
            s.name as category_name
        FROM fnords f
        JOIN fnord_sephiroth fs ON f.id = fs.fnord_id
        JOIN sephiroth s ON s.id = fs.sephiroth_id
        WHERE fs.sephiroth_id IN (?)
          AND f.read_at IS NULL
          AND f.published_at > datetime('now', '-7 days')
        ORDER BY f.published_at DESC
        LIMIT ?
    "#;

    let results = db.query(query, &[top_categories.as_slice(), limit])?;

    let mut candidates = Vec::new();
    for row in results {
        candidates.push(Candidate {
            fnord_id: row.get("fnord_id")?,
            category_name: Some(row.get("category_name")?),
            source: CandidateSource::Category,
            ..Default::default()
        });
    }

    Ok(candidates)
}
```

### 3.5 Recent Popular (Fallback)

```rust
fn from_recent_popular(db: &Database, limit: usize) -> Result<Vec<Candidate>, Error> {
    // Fallback: Aktuelle Artikel von beliebten Quellen
    let query = r#"
        SELECT
            f.id as fnord_id,
            p.title as source_name
        FROM fnords f
        JOIN pentacles p ON p.id = f.pentacle_id
        WHERE f.read_at IS NULL
          AND f.embedding IS NOT NULL
          AND f.published_at > datetime('now', '-48 hours')
        ORDER BY p.article_count DESC, f.published_at DESC
        LIMIT ?
    "#;

    let results = db.query(query, &[limit])?;

    let mut candidates = Vec::new();
    for row in results {
        candidates.push(Candidate {
            fnord_id: row.get("fnord_id")?,
            source: CandidateSource::Popular,
            ..Default::default()
        });
    }

    Ok(candidates)
}
```

---

## 4. Scoring & Ranking

### 4.1 Scoring-Funktion

```rust
pub struct Scorer;

impl Scorer {
    pub fn score(
        candidate: &mut Candidate,
        profile: &UserProfile,
        db: &Database,
    ) -> Result<(), Error> {
        // Lade Artikel-Metadaten falls nötig
        let article = db.get_fnord(candidate.fnord_id)?;

        // 1. Embedding Score (0.0 - 1.0)
        let embedding_score = candidate.embedding_score.unwrap_or_else(|| {
            Self::compute_embedding_score(&article, profile)
        });

        // 2. Keyword Score (0.0 - 1.0)
        let keyword_score = candidate.keyword_score.unwrap_or_else(|| {
            Self::compute_keyword_score(&article, profile, db)
        });

        // 3. Freshness Score (0.0 - 1.0)
        let freshness_score = Self::compute_freshness(&article.published_at);

        // 4. Source Quality Score (0.0 - 1.0)
        let source_score = Self::compute_source_score(&article, db);

        // 5. Diversity Bonus (später im Reranking)
        let diversity_bonus = 0.0;

        // Gewichtete Kombination
        let final_score =
            0.40 * embedding_score +
            0.30 * keyword_score +
            0.20 * freshness_score +
            0.10 * source_score +
            diversity_bonus;

        candidate.final_score = final_score;
        candidate.embedding_score = Some(embedding_score);
        candidate.keyword_score = Some(keyword_score);
        candidate.freshness_score = Some(freshness_score);

        Ok(())
    }

    fn compute_freshness(published_at: &Option<DateTime<Utc>>) -> f64 {
        match published_at {
            Some(dt) => {
                let age_hours = (Utc::now() - *dt).num_hours() as f64;
                // Half-life: 48 Stunden
                let decay = (-age_hours / 69.3).exp(); // ln(2) / 48 ≈ 69.3
                decay.clamp(0.0, 1.0)
            }
            None => 0.3, // Default für unbekanntes Datum
        }
    }

    fn compute_source_score(article: &Fnord, db: &Database) -> f64 {
        // Basis: Quellen-Popularität
        let source = db.get_pentacle(article.pentacle_id)?;
        let popularity = (source.article_count as f64).ln() / 10.0;

        popularity.clamp(0.0, 1.0)
    }
}
```

---

## 5. Reranker (Diversity)

### 5.1 MMR-basiertes Reranking

```rust
pub struct Reranker;

impl Reranker {
    /// Maximal Marginal Relevance Reranking
    pub fn rerank(
        candidates: Vec<Candidate>,
        limit: usize,
        lambda: f64, // 0.0 = nur Diversity, 1.0 = nur Relevanz
    ) -> Vec<Candidate> {
        let mut selected: Vec<Candidate> = Vec::new();
        let mut remaining: Vec<Candidate> = candidates;

        while selected.len() < limit && !remaining.is_empty() {
            // Finde Kandidat mit höchstem MMR-Score
            let mut best_idx = 0;
            let mut best_mmr = f64::NEG_INFINITY;

            for (i, candidate) in remaining.iter().enumerate() {
                let relevance = candidate.final_score;

                // Max Similarity zu bereits ausgewählten
                let max_sim = selected
                    .iter()
                    .map(|s| Self::similarity(candidate, s))
                    .fold(0.0, f64::max);

                // MMR = λ * Relevance - (1-λ) * max_similarity
                let mmr = lambda * relevance - (1.0 - lambda) * max_sim;

                if mmr > best_mmr {
                    best_mmr = mmr;
                    best_idx = i;
                }
            }

            selected.push(remaining.remove(best_idx));
        }

        selected
    }

    fn similarity(a: &Candidate, b: &Candidate) -> f64 {
        let mut sim = 0.0;

        // Gleiche Quelle = hohe Similarity
        if a.pentacle_id == b.pentacle_id {
            sim += 0.5;
        }

        // Gleiche Kategorie = moderate Similarity
        if a.category_ids.intersection(&b.category_ids).count() > 0 {
            sim += 0.3;
        }

        // Keyword Overlap
        let kw_overlap = a.keyword_ids.intersection(&b.keyword_ids).count();
        if kw_overlap > 3 {
            sim += 0.2;
        }

        sim
    }
}
```

### 5.2 Source-Diversity Constraint

```rust
fn apply_source_diversity(
    candidates: &mut Vec<Candidate>,
    max_per_source: usize,
) {
    let mut source_counts: HashMap<i64, usize> = HashMap::new();

    candidates.retain(|c| {
        let count = source_counts.entry(c.pentacle_id).or_insert(0);
        if *count < max_per_source {
            *count += 1;
            true
        } else {
            false
        }
    });
}
```

---

## 6. Explanation Generator

### 6.1 Erklärungstypen

```rust
pub enum ExplanationType {
    Keywords(Vec<String>),     // "Basierend auf: X, Y, Z"
    Category(String),          // "Aus deinem Interessenbereich: Politik"
    Source(String),            // "Von einer deiner Lieblingsquellen"
    Similar(String),           // "Ähnlich zu: Artikel-Titel"
    Exploration,               // "Erweitere deinen Horizont"
    Popular,                   // "Beliebt diese Woche"
}
```

### 6.2 Erklärungsgenerierung

```rust
pub fn generate_explanation(candidate: &Candidate, profile: &UserProfile) -> String {
    // Priorität 1: Keyword-basiert (am erklärbarsten)
    if let Some(ref keywords) = candidate.matching_keywords {
        let kw_list: Vec<&str> = keywords.split(',').take(3).collect();
        if kw_list.len() >= 2 {
            return format!("Basierend auf: {}", kw_list.join(", "));
        }
    }

    // Priorität 2: Kategorie-basiert
    if let Some(ref category) = candidate.category_name {
        if profile.category_weights.contains_key(&candidate.category_ids.iter().next().unwrap_or(&0)) {
            return format!("Aus deinem Interessenbereich: {}", category);
        }
    }

    // Priorität 3: Embedding-basiert (weniger erklärbar)
    if candidate.embedding_score.map(|s| s > 0.7).unwrap_or(false) {
        return "Thematisch ähnlich zu deinen Artikeln".to_string();
    }

    // Fallback
    match candidate.source {
        CandidateSource::Popular => "Beliebt diese Woche".to_string(),
        _ => "Könnte dich interessieren".to_string(),
    }
}
```

---

## 7. Cold Start Handling

### 7.1 Stufen

| Gelesene Artikel | Strategie |
|------------------|-----------|
| 0 | Nur Recent Popular |
| 1-4 | Keywords + Popular |
| 5-9 | Keywords + Categories |
| 10+ | Full Pipeline |

### 7.2 Implementation

```rust
fn get_recommendation_strategy(profile: &UserProfile) -> RecommendationStrategy {
    match profile.total_read {
        0 => RecommendationStrategy::OnlyPopular,
        1..=4 => RecommendationStrategy::KeywordsAndPopular,
        5..=9 => RecommendationStrategy::KeywordsAndCategories,
        _ => RecommendationStrategy::Full,
    }
}
```

---

## 8. Caching-Strategie

### 8.1 Cache-Ebenen

| Ebene | TTL | Invalidierung |
|-------|-----|---------------|
| User Profile | 5 min | Bei read_at/feedback Änderung |
| Candidate Pool | 10 min | Bei neuem Artikel |
| Final Recommendations | 2 min | Bei User-Aktion |

### 8.2 Implementation

```rust
pub struct RecommendationCache {
    profile_cache: LruCache<(), (UserProfile, Instant)>,
    candidate_cache: LruCache<(), (Vec<Candidate>, Instant)>,
}

impl RecommendationCache {
    fn get_or_compute_profile(&mut self, db: &Database) -> &UserProfile {
        if let Some((profile, cached_at)) = self.profile_cache.get(&()) {
            if cached_at.elapsed() < Duration::from_secs(300) {
                return profile;
            }
        }

        let profile = UserProfile::build(db).unwrap();
        self.profile_cache.put((), (profile, Instant::now()));
        &self.profile_cache.get(&()).unwrap().0
    }
}
```

---

## 9. Performance-Budget

| Operation | Ziel | Messung |
|-----------|------|---------|
| Profile Build | < 50ms | SQLite Queries |
| Candidate Generation | < 200ms | vec_fnords + Joins |
| Scoring | < 100ms | In-Memory |
| Reranking | < 50ms | In-Memory |
| **Total** | **< 500ms** | E2E |

---

## 10. Ausbaupfad (Phase 2+)

### Phase 2: Personalization
- User-Feedback in Scoring einbauen
- Personalized Weights lernen
- A/B Testing für Gewichte

### Phase 3: Advanced
- LLM-generierte Erklärungen
- Cross-Artikel Graph Expansion
- Trending Topics Integration
