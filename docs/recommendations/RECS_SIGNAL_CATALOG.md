# RECS_SIGNAL_CATALOG.md — Signal-Katalog für Empfehlungssystem

**Erstellt:** 2026-01-18
**Status:** Phase 1 Complete

---

## Signal-Übersicht

| Signal-Typ | Verfügbar | Genutzt heute | Impact | Aufwand |
|------------|-----------|---------------|--------|---------|
| Article Embeddings | ✅ | ❌ | Hoch | Niedrig |
| Keyword Overlap | ✅ | ❌ | Mittel | Niedrig |
| Category Match | ✅ | ❌ | Mittel | Niedrig |
| Freshness | ✅ | ❌ | Mittel | Niedrig |
| Political Bias | ✅ | ✅ | Niedrig | - |
| Source Diversity | ✅ | ❌ | Mittel | Niedrig |
| Read History | ✅ | Teilweise | Mittel | Niedrig |
| User Feedback | ❌ | - | Hoch | Mittel |
| Graph Expansion | ✅ | ❌ | Mittel | Mittel |

---

## 1. Content-Signale

### 1.1 Article Embeddings (vec_fnords)

**Beschreibung:** 1024-dimensionale Vektoren für semantische Ähnlichkeit

**Abfrage:**
```sql
SELECT fnord_id, distance
FROM vec_fnords
WHERE embedding MATCH (SELECT embedding FROM fnords WHERE id = ?)
  AND k = 20
ORDER BY distance ASC;
```

**Verfügbarkeit:** 655 von 770 Artikeln (85%)

**Stärken:**
- Semantische Ähnlichkeit über Wortgrenzen hinweg
- O(log n) Performance durch sqlite-vec
- Bereits berechnet und indiziert

**Schwächen:**
- 15% Artikel ohne Embedding
- Keine Erklärbarkeit ("warum ähnlich?")

**Empfehlung:** Primäres Signal für Candidate Generation

---

### 1.2 Keyword Overlap

**Beschreibung:** Jaccard-Ähnlichkeit basierend auf gemeinsamen Keywords

**Berechnung:**
```sql
-- Keywords für Artikel A
SELECT immanentize_id FROM fnord_immanentize WHERE fnord_id = ?

-- Jaccard: |A ∩ B| / |A ∪ B|
```

**Stärken:**
- 100% Coverage (alle Artikel haben Keywords)
- Erklärbar ("Gemeinsame Themen: X, Y, Z")
- Durchschnittlich 12 Keywords pro Artikel

**Schwächen:**
- Nur exakte Matches (keine Synonyme ohne Graph)
- Kann zu eng sein bei wenig Overlap

**Empfehlung:** Sekundäres Signal + Erklärungen generieren

---

### 1.3 Category Match

**Beschreibung:** Artikel der gleichen Kategorie(n)

**Abfrage:**
```sql
SELECT f.id, COUNT(*) as shared_categories
FROM fnords f
JOIN fnord_sephiroth fs ON f.id = fs.fnord_id
WHERE fs.sephiroth_id IN (
    SELECT sephiroth_id FROM fnord_sephiroth WHERE fnord_id = ?
)
GROUP BY f.id
ORDER BY shared_categories DESC;
```

**Stärken:**
- 100% Coverage
- Thematisch kohärent
- Erklärbar ("Auch in Kategorie: Politik")

**Schwächen:**
- Zu grob (nur 13 Kategorien)
- Viele Artikel pro Kategorie

**Empfehlung:** Filter/Boost, nicht primäres Signal

---

### 1.4 Freshness Score

**Beschreibung:** Aktuellere Artikel bevorzugen

**Berechnung:**
```python
def freshness_score(published_at):
    age_hours = (now - published_at).total_hours()
    # Exponential decay: 50% nach 48h
    return math.exp(-age_hours / 69.3)  # ln(2)/48h ≈ 69.3
```

**Anpassungen:**
- Breaking News: Stärkerer Decay (24h half-life)
- Evergreen Content: Schwächerer Decay (7d half-life)

**Empfehlung:** Multiplikator auf finale Scores

---

## 2. User-Signale

### 2.1 Read History (fnords.read_at)

**Beschreibung:** Welche Artikel hat der User gelesen?

**Verfügbarkeit:** 43 gelesene Artikel

**Nutzung:**
```sql
-- User-Interessenprofil aus Keywords gelesener Artikel
SELECT i.name, COUNT(*) as weight
FROM fnords f
JOIN fnord_immanentize fi ON f.id = fi.fnord_id
JOIN immanentize i ON fi.immanentize_id = i.id
WHERE f.read_at IS NOT NULL
GROUP BY i.id
ORDER BY weight DESC;
```

**Stärken:**
- Implizites Interesse
- Keine User-Aktion nötig

**Schwächen:**
- Wenig Daten (43 Artikel)
- Unterscheidet nicht gut/schlecht

**Empfehlung:** User-Profil aufbauen, nicht filtern

---

### 2.2 Article Status (fnords.status)

**Beschreibung:** concealed → illuminated → golden_apple

**Signalstärke:**
| Status | Bedeutung | Gewichtung |
|--------|-----------|------------|
| concealed | Nicht gesehen | 0 (neutral) |
| illuminated | Gelesen | 1x (positiv) |
| golden_apple | Favorit | 3x (stark positiv) |

**Aktuell:** 0 golden_apple Artikel

**Empfehlung:** golden_apple als starkes Signal nutzen

---

### 2.3 User Feedback (FEHLT!)

**Benötigte Signale:**

| Signal | UI-Element | DB-Feld | Stärke |
|--------|------------|---------|--------|
| Save | "Speichern" Button | `fnords.saved` | +2 |
| Like | "Gefällt mir" | `fnords.liked` | +1 |
| Dislike | "Gefällt nicht" | `fnords.disliked` | -1 |
| Hide | "Nicht mehr anzeigen" | `fnords.hidden` | -3 |
| "More like this" | Button/Link | implicit | +2 |

**Priorität:** Hoch — Ohne Feedback kein Lernen

---

## 3. Diversity-Signale

### 3.1 Source Diversity (pentacle_id)

**Beschreibung:** Nicht nur Artikel einer Quelle empfehlen

**Berechnung:**
```python
def source_penalty(recommendations, pentacle_id):
    same_source = sum(1 for r in recommendations if r.pentacle_id == pentacle_id)
    return 0.8 ** same_source  # 20% Penalty pro gleiche Quelle
```

**Empfehlung:** Bei Reranking anwenden

---

### 3.2 Political Bias Diversity

**Beschreibung:** Verschiedene Perspektiven in Ergebnissen

**Berechnung:**
```python
def bias_diversity_score(recommendations):
    biases = [r.political_bias for r in recommendations]
    return len(set(biases)) / len(biases)
```

**Empfehlung:** Diversität als Ziel, nicht nur Balancing

---

### 3.3 Category Diversity

**Beschreibung:** Nicht nur eine Kategorie empfehlen

**Berechnung:**
```python
def category_diversity(recommendations):
    categories = set()
    for r in recommendations:
        categories.update(r.categories)
    return len(categories) / expected_categories
```

**Empfehlung:** Mindestens 3 verschiedene Kategorien in Top-10

---

## 4. Graph-Signale

### 4.1 Keyword Co-occurrence (immanentize_neighbors)

**Beschreibung:** Keywords die oft zusammen auftreten

**Abfrage:**
```sql
SELECT immanentize_id_b as related_keyword, cooccurrence
FROM immanentize_neighbors
WHERE immanentize_id_a = ?
ORDER BY cooccurrence DESC
LIMIT 10;
```

**Nutzung:**
- Query Expansion: User-Keywords → verwandte Keywords
- Erklärungen: "Auch interessiert an: X, Y"

---

### 4.2 Keyword Embedding Similarity (immanentize_neighbors.embedding_similarity)

**Beschreibung:** Semantisch ähnliche Keywords

**Abfrage:**
```sql
SELECT immanentize_id_b, embedding_similarity
FROM immanentize_neighbors
WHERE immanentize_id_a = ?
  AND embedding_similarity > 0.7
ORDER BY embedding_similarity DESC;
```

**Nutzung:**
- Synonym-Erweiterung
- Themen-Cluster

---

### 4.3 Keyword-Category Association (immanentize_sephiroth)

**Beschreibung:** Welche Keywords zu welchen Kategorien gehören

**Nutzung:**
- Kategorie-Inferenz aus Keywords
- Erklärungen ("Technik-Artikel wegen: API, Linux, Server")

---

## 5. Quality-Signale

### 5.1 Sachlichkeit (fnords.sachlichkeit)

**Beschreibung:** 0-4 Skala der Objektivität

| Wert | Label | Anteil |
|------|-------|--------|
| 0 | Stark emotional | ? |
| 1 | Emotional | ? |
| 2 | Gemischt | ? |
| 3 | Überwiegend sachlich | ? |
| 4 | Sachlich | ? |

**Nutzung:** Optional als Filter ("nur sachliche Artikel")

---

### 5.2 Keyword Quality Score (immanentize.quality_score)

**Beschreibung:** Wie "gut" ist ein Keyword?

**Faktoren:**
- Artikel-Frequenz (nicht zu selten, nicht zu häufig)
- Embedding vorhanden
- Nicht Stopword

**Nutzung:** Gewichtung bei Keyword-Matching

---

## 6. Signal-Kombinationen (Hybrid Scoring)

### 6.1 Empfohlene MVP-Formel

```python
def recommendation_score(candidate, user_profile, query=None):
    # Base: Embedding Similarity
    if query:
        embedding_sim = cosine_similarity(candidate.embedding, query.embedding)
    else:
        embedding_sim = avg_similarity_to_user_articles(candidate, user_profile)

    # Keyword Overlap
    keyword_overlap = jaccard(candidate.keywords, user_profile.keywords)

    # Freshness
    freshness = exp_decay(candidate.published_at, half_life=48h)

    # Penalties
    already_read = 0 if candidate.read_at else 1
    source_penalty = 0.8 ** same_source_count

    # Combine
    score = (
        0.5 * embedding_sim +
        0.3 * keyword_overlap +
        0.2 * freshness
    ) * already_read * source_penalty

    return score
```

### 6.2 Gewichtungs-Anpassungen

| Szenario | Embedding | Keywords | Freshness |
|----------|-----------|----------|-----------|
| Standard | 0.5 | 0.3 | 0.2 |
| Breaking News | 0.3 | 0.2 | 0.5 |
| Deep Dive | 0.6 | 0.3 | 0.1 |
| Exploration | 0.4 | 0.4 | 0.2 |

---

## 7. Signal-Prioritäten für MVP

### Muss (P0)
1. ✅ Article Embeddings (Candidate Generation)
2. ✅ Freshness (Recency Boost)
3. ✅ Read Filter (Keine bereits gelesenen)

### Soll (P1)
4. Keyword Overlap (Erklärungen)
5. Source Diversity (Reranking)
6. Category Match (Boost)

### Kann (P2)
7. User Feedback Integration
8. Graph Expansion
9. Personalized Weights
