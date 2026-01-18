# RECS_PRODUCT_BRIEF.md — Produkt-Spezifikation Empfehlungssystem

**Erstellt:** 2026-01-18
**Status:** Phase 2 — MVP Definition

---

## 1. Vision

> Nutzer erhalten **relevante, erklärbare Leseempfehlungen**, die ihre Interessen erweitern ohne sie in Filterblasen zu verstärken.

---

## 2. Zielgruppe (Personas)

### Persona A: "Der Informierte"
- Liest täglich 5-10 Artikel
- Hat klare Interessen (z.B. Technik, Politik)
- Will **tiefer** in Themen einsteigen
- Erwartet: "Mehr davon" + ähnliche Artikel

### Persona B: "Der Entdecker"
- Liest gelegentlich
- Offen für neue Themen
- Will **Horizonterweiterung**
- Erwartet: Überraschende aber relevante Vorschläge

### Persona C: "Der Kritische"
- Achtet auf Bias und Quellen
- Will verschiedene Perspektiven
- Erwartet: Counter-Perspectives + Quellenvielfalt

---

## 3. MVP-Definition

### 3.1 Was ist ein "Artikel" im System?

| Kriterium | Wert |
|-----------|------|
| Quelle | RSS-Feeds (pentacles) |
| Volltext | Erforderlich (content_full NOT NULL) |
| Embedding | Erforderlich für Similarity |
| Mindest-Keywords | ≥ 3 |

### 3.2 MVP-Scope

| Feature | In MVP | Begründung |
|---------|--------|------------|
| Embedding-basierte Similarity | ✅ | Primäres Signal, bereits vorhanden |
| Keyword-basierte Erklärungen | ✅ | Transparenz, kein Aufwand |
| Freshness-Boost | ✅ | Aktuelle Artikel bevorzugen |
| Source-Diversity | ✅ | Nicht nur eine Quelle |
| Hide/Dismiss | ✅ | Negatives Feedback |
| Save/Bookmark | ✅ | Positives Feedback |
| User-Profil aus Lese-Historie | ✅ | Personalisierung light |
| Collaborative Filtering | ❌ | Single-User App |
| A/B Testing | ❌ | Overkill für MVP |
| LLM-generierte Erklärungen | ❌ | Deterministische Fallbacks first |

### 3.3 Nicht-MVP (Ausbaupfad)

**Phase 2:**
- "More like this" Button
- Detaillierte Erklärungen mit LLM
- Keyword-Graph Expansion

**Phase 3:**
- Themen-Radar (Trending Keywords)
- Explizite Interessens-Profile
- Filter nach Sachlichkeit/Bias

---

## 4. Empfehlungs-Logik (MVP)

### 4.1 Candidate Generation

```
1. User-Profil erstellen
   - Keywords aus gelesenen Artikeln aggregieren (Top 20)
   - Kategorien aus gelesenen Artikeln
   - Optional: Golden-Apple Artikel stark gewichten

2. Kandidaten finden
   - Embedding-Similarity zu User-Profil-Artikeln
   - ODER: Artikel mit überlappenden Keywords
   - Filter: read_at IS NULL, embedding IS NOT NULL

3. Pool: 100 Kandidaten
```

### 4.2 Scoring

```python
score = (
    0.4 * embedding_similarity +      # Semantische Nähe
    0.3 * keyword_overlap_score +     # Thematische Nähe
    0.2 * freshness_score +           # Aktualität
    0.1 * source_quality_score        # Quellen-Qualität
)
```

### 4.3 Reranking (Diversity)

```python
def rerank_for_diversity(candidates, limit=10):
    results = []
    seen_sources = set()
    seen_categories = set()

    for c in sorted(candidates, key=lambda x: x.score, reverse=True):
        # Source Diversity
        if c.source in seen_sources:
            c.score *= 0.8

        # Category Diversity
        if c.categories.issubset(seen_categories):
            c.score *= 0.9

        results.append(c)
        seen_sources.add(c.source)
        seen_categories.update(c.categories)

        if len(results) >= limit:
            break

    return results
```

### 4.4 Explanation Generation

```python
def generate_explanation(article, user_profile):
    shared_keywords = article.keywords & user_profile.keywords

    if len(shared_keywords) >= 2:
        return f"Basierend auf: {', '.join(shared_keywords[:3])}"
    elif article.category in user_profile.top_categories:
        return f"Aus deinem Interessenbereich: {article.category}"
    else:
        return "Erweitere deinen Horizont"
```

---

## 5. UI-Spezifikation

### 5.1 Empfehlungs-Card (erweitertes ArticleCard)

```
┌──────────────────────────────────────────────────┐
│ [Source Icon] Quelle · vor 2 Stunden      [Save]│
├──────────────────────────────────────────────────┤
│ Artikel-Titel                                    │
│                                                  │
│ Kurze Beschreibung oder Summary...               │
├──────────────────────────────────────────────────┤
│ 💡 Basierend auf: Trump, NATO, Außenpolitik     │
│                                         [Hide]   │
└──────────────────────────────────────────────────┘
```

### 5.2 Neue UI-Elemente

| Element | Funktion | Priorität |
|---------|----------|-----------|
| Save Button | Artikel merken | P0 |
| Hide Button | Nicht mehr anzeigen | P0 |
| Explanation Row | Warum empfohlen | P0 |
| "More like this" | Ähnliche suchen | P1 |
| Similarity Badge | Score anzeigen | P2 |

### 5.3 Empty State (überarbeitet)

Wenn keine Empfehlungen:
1. Mindestens 5 Artikel lesen für Profil
2. Feeds mit mehr Inhalt hinzufügen
3. Ollama prüfen (Embeddings)

---

## 6. Erfolgsmetriken (KPIs)

### 6.1 Offline-Metriken

| Metrik | Ziel | Messung |
|--------|------|---------|
| Precision@5 | ≥ 60% | % angeklickter Top-5 |
| Recall@10 | ≥ 40% | % relevanter in Top-10 |
| Diversity | ≥ 3 | Quellen in Top-10 |
| Freshness | ≥ 50% | Artikel < 48h in Top-10 |

### 6.2 Online-Metriken

| Metrik | Ziel | Messung |
|--------|------|---------|
| Click-Through Rate | ≥ 15% | Klicks / Impressions |
| Save Rate | ≥ 5% | Saves / Impressions |
| Hide Rate | ≤ 10% | Hides / Impressions |
| Session Engagement | +10% | Artikel gelesen / Session |

### 6.3 Qualitative Metriken

| Metrik | Ziel |
|--------|------|
| Erklärbarkeit | Nutzer versteht "warum" |
| Überraschung | Mind. 1 unerwarteter aber guter Vorschlag |
| Keine Wiederholungen | Keine bereits gelesenen |

---

## 7. Definition of Done (DoD)

### MVP Complete wenn:

- [ ] Empfehlungen basieren auf echten Embeddings
- [ ] Mindestens 10 Artikel werden empfohlen
- [ ] Jede Empfehlung hat eine Erklärung
- [ ] Save-Button funktioniert
- [ ] Hide-Button funktioniert
- [ ] Diversity: Min. 3 verschiedene Quellen in Top-10
- [ ] Performance: < 500ms für Recommendation-Query
- [ ] UI respektiert Theme (Dark/Light)
- [ ] Keine bereits gelesenen Artikel
- [ ] Logging für Debugging vorhanden
- [ ] Unit Tests für Scoring-Logik
- [ ] Integration Test für API

---

## 8. Risiken & Mitigationen

| Risiko | Wahrscheinlichkeit | Impact | Mitigation |
|--------|-------------------|--------|------------|
| Zu wenig gelesene Artikel | Hoch | Hoch | Fallback auf globale Popularity |
| Embeddings fehlen | Mittel | Mittel | Keyword-only Fallback |
| Performance-Probleme | Niedrig | Mittel | Caching, Batch-Queries |
| Irrelevante Empfehlungen | Mittel | Hoch | User-Feedback + Iteration |

---

## 9. Technische Entscheidungen

### 9.1 Architektur

```
┌─────────────────────────────────────────────────────┐
│                    Frontend                          │
│   MindfuckView.svelte → RecommendationCard.svelte   │
└────────────────────────┬────────────────────────────┘
                         │ invoke()
┌────────────────────────┴────────────────────────────┐
│                    Backend                           │
│   recommendations.rs → RecommendationEngine         │
│        ↓                                            │
│   ┌─────────────┐  ┌─────────────┐  ┌────────────┐ │
│   │ vec_fnords  │  │ immanentize │  │ user_prefs │ │
│   └─────────────┘  └─────────────┘  └────────────┘ │
└─────────────────────────────────────────────────────┘
```

### 9.2 Neue Tabellen

```sql
-- User-Feedback speichern
CREATE TABLE recommendation_feedback (
    id INTEGER PRIMARY KEY,
    fnord_id INTEGER NOT NULL,
    action TEXT NOT NULL,  -- 'save', 'hide', 'click'
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (fnord_id) REFERENCES fnords(id) ON DELETE CASCADE
);

-- Optional: User-Präferenzen
CREATE TABLE user_preferences (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 9.3 Neue Tauri Commands

```rust
#[tauri::command]
fn get_recommendations(limit: Option<i32>) -> Vec<Recommendation>;

#[tauri::command]
fn save_recommendation(fnord_id: i64) -> Result<(), String>;

#[tauri::command]
fn hide_recommendation(fnord_id: i64) -> Result<(), String>;

#[tauri::command]
fn get_saved_articles() -> Vec<Fnord>;
```

---

## 10. Implementierungs-Reihenfolge

### Sprint 1: Foundation
1. ✅ Daten-Analyse (diese Docs)
2. Neue DB-Tabellen anlegen
3. `get_recommendations` Command implementieren
4. Basic Scoring (Embedding + Freshness)

### Sprint 2: UI & Feedback
5. RecommendationCard Komponente
6. Save/Hide Buttons + Commands
7. UI in MindfuckView integrieren
8. Theme-Integration prüfen

### Sprint 3: Polish & Testing
9. Erklärungen generieren
10. Diversity-Reranking
11. Unit & Integration Tests
12. Performance-Optimierung

### Sprint 4: Iteration
13. Metriken-Logging
14. User-Feedback einbauen
15. Scoring-Weights tunen

---

## Anhang: Abgrenzung zu Counter-Perspectives

Das bestehende `get_counter_perspectives()` bleibt erhalten als **separates Feature** für bewusste Horizonterweiterung.

| Feature | Counter-Perspectives | Recommendations |
|---------|---------------------|-----------------|
| Ziel | Gegenperspektiven | Relevante Artikel |
| Basis | Nur Bias | Embeddings + Keywords |
| Erklärung | Statisch | Dynamisch |
| Personalisierung | Bias-Balance | Interessen-basiert |
| Tab | Eigener Tab | Eigener Tab oder merged |

**Empfehlung:** Langfristig mergen, kurzfristig parallel betreiben.
