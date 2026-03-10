# Design: Ollama-Potenzial Tier 1 + Tier 2

**Datum:** 2026-03-10
**Scope:** 7 Features, Full-Stack (Backend + Frontend)
**Basis:** [fuckupRSS Ollama-Potenzialanalyse](../../Notes/Entwicklung/fuckupRSS%20Ollama-Potenzialanalyse.md)

## Agent-Team-Struktur

6 Agenten arbeiten parallel in isolierten Git-Worktrees:

| Agent | Features | Branch | Migration |
|-------|----------|--------|-----------|
| A: Ollama Modernization | 1 (Structured Outputs) + 3 (/api/chat) | `feature/ollama-modernization` | keine |
| B: Article Type | 2 (article_type persistieren) | `feature/article-type` | 26 |
| C: Embedding Enhancement | 7 (Embedding-Kontext ausnutzen) | `feature/embedding-enhancement` | keine |
| D: Briefings | 4 (Tages-/Wochen-Briefings) | `feature/briefings` | 27 |
| E: Story Clustering | 5 (Perspektiven-Vergleich) | `feature/story-clustering` | 28 |
| F: NER | 6 (Named Entity Recognition) | `feature/ner-extraction` | 29 |

Merge-Reihenfolge: A → C → B → D → E → F (Migrationen sequenziell renummerieren).

---

## Feature-Spezifikationen

### Feature 1: Structured Outputs mit JSON-Schema (Agent A)

**Ziel:** `format: "json"` durch explizites JSON-Schema ersetzen.

**Backend-Änderungen:**
- `ollama/mod.rs`: `GenerateRequest.format` von `Option<String>` auf `Option<serde_json::Value>` ändern
- JSON-Schemas für alle Response-Typen definieren:
  - `DiscordianAnalysis`: `{political_bias, sachlichkeit, summary, keywords[], categories[], rejected_keywords[], rejected_categories[]}`
  - `BiasAnalysis`: `{political_bias, sachlichkeit}`
- `ai_provider/mod.rs`: `AiTextProvider::generate_text()` Signatur erweitern: `json_schema: Option<serde_json::Value>` statt `json_mode: bool`
- Beide Provider (Ollama + OpenAI) anpassen

**Kein Frontend nötig.**

### Feature 3: /api/chat mit System-Messages (Agent A)

**Ziel:** Von `/api/generate` auf `/api/chat` umstellen.

**Backend-Änderungen:**
- `ollama/mod.rs`: Neuer `ChatRequest` Struct mit `messages: Vec<ChatMessage>`, `ChatMessage { role, content }`
- Neue Methode `OllamaClient::chat()` die `/api/chat` aufruft
- System-Message enthält Rollenanweisung, User-Message enthält den Artikeltext
- `OllamaTextProvider::generate_text()` nutzt `chat()` statt `generate_simple()`/`summarize_with_prompt()`
- Bestehende Prompts aufteilen: System-Teil + User-Teil

### Feature 2: article_type persistieren (Agent B)

**Ziel:** Artikeltyp (news/analysis/opinion/satire/ad/unknown) in DB speichern und im UI filtern.

**DB-Migration 26:**
```sql
ALTER TABLE fnords ADD COLUMN article_type TEXT DEFAULT 'unknown';
CREATE INDEX idx_fnords_article_type ON fnords(article_type);
```

**Backend-Änderungen:**
- `ollama/mod.rs`: `RawDiscordianAnalysis` hat bereits `article_type` → sicherstellen dass es geparsed wird
- `commands/ai/article_processor.rs`: `article_type` in UPDATE-Statement aufnehmen
- `commands/fnords.rs`: `article_type` in Fnord-Struct und Queries aufnehmen
- Neuer Tauri-Command `get_article_types` für Filter-Dropdown

**Frontend-Änderungen:**
- `ArticleList.svelte`: Filter-Dropdown für article_type
- `ArticleView.svelte`: article_type als Badge anzeigen
- `ArticleGreyfaceAlert.svelte`: article_type neben Bias-Info
- i18n-Keys: `articleView.type.news`, `.analysis`, `.opinion`, `.satire`, `.ad`, `.unknown`

### Feature 7: Embedding-Kontext ausnutzen (Agent C)

**Ziel:** Mehr Artikeltext für Embeddings nutzen (aktuell: Titel + 500 Zeichen, möglich: 8.192 Tokens).

**Backend-Änderungen:**
- `commands/ai/similarity.rs` oder `commands/ai/helpers.rs`: `build_embedding_text()` Funktion anpassen
- Neuer Embedding-Input: `Titel + Summary + erste 2000 Zeichen content_full`
- Bestehende Artikel-Embeddings als veraltet markieren → Batch-Re-Embedding ermöglichen
- Settings: `embedding_text_length` konfigurierbar (Standard: 2000)

**Kein Frontend nötig** (Re-Embedding über bestehende Settings/Wartung UI auslösen).

### Feature 4: Automatische Briefings (Agent D)

**Ziel:** KI-generierte Tages-/Wochen-Zusammenfassungen der wichtigsten Themen.

**DB-Migration 27:**
```sql
CREATE TABLE briefings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    period_type TEXT NOT NULL CHECK(period_type IN ('daily', 'weekly')),
    period_start DATETIME NOT NULL,
    period_end DATETIME NOT NULL,
    content TEXT NOT NULL,
    top_keywords TEXT,
    article_count INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(period_type, period_start)
);
```

**Backend-Änderungen:**
- Neues Modul `src-tauri/src/commands/briefings.rs`
- Tauri-Commands: `generate_briefing`, `get_briefings`, `get_latest_briefing`
- Prompt: Top-10 Artikel (nach Relevanz/Recency) mit Summaries → "Erstelle ein Briefing der wichtigsten 5 Themen"
- Trending-Keywords aus `immanentize_daily` einbeziehen

**Frontend-Änderungen:**
- Neue Komponente `BriefingView.svelte` als Tab in `FnordView.svelte` oder eigener View
- Briefing-Card mit Zeitraum, Themen, verlinkten Artikeln
- Button "Briefing generieren" mit Loading-State
- i18n-Keys: `briefing.title`, `.daily`, `.weekly`, `.generate`, `.generating`, `.noArticles`, `.topKeywords`

### Feature 5: Story-Clustering + Perspektiven-Vergleich (Agent E)

**Ziel:** Artikel über dasselbe Thema gruppieren und Berichterstattung vergleichen.

**DB-Migration 28:**
```sql
CREATE TABLE story_clusters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    summary TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE story_cluster_articles (
    cluster_id INTEGER NOT NULL REFERENCES story_clusters(id) ON DELETE CASCADE,
    fnord_id INTEGER NOT NULL REFERENCES fnords(id) ON DELETE CASCADE,
    similarity_score REAL NOT NULL DEFAULT 0.0,
    PRIMARY KEY (cluster_id, fnord_id)
);
```

**Backend-Änderungen:**
- Neues Modul `src-tauri/src/commands/story_clusters.rs`
- Clustering-Algorithmus: Artikel mit Embedding-Similarity > 0.75 und >= 2 gemeinsamen Keywords gruppieren
- Tauri-Commands: `discover_story_clusters`, `get_story_clusters`, `get_story_cluster`, `compare_perspectives`
- Vergleichs-Prompt: "Vergleiche die Berichterstattung dieser Quellen über [Thema]: [Summaries]"

**Frontend-Änderungen:**
- Neue Komponente `StoryClusterView.svelte` als Tab in `FnordView.svelte`
- Cluster-Liste mit Titel, Artikelanzahl, Quellen-Badges
- Detail-Ansicht: Artikel nebeneinander mit Bias-Indikatoren + KI-Vergleichstext
- i18n-Keys: `storyClusters.title`, `.discover`, `.discovering`, `.noResults`, `.perspectives`, `.sources`, `.comparison`

### Feature 6: Named Entity Recognition (Agent F)

**Ziel:** Personen, Orte, Organisationen als strukturierte Entitäten extrahieren.

**DB-Migration 29:**
```sql
CREATE TABLE entities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    entity_type TEXT NOT NULL CHECK(entity_type IN ('person', 'organization', 'location', 'event')),
    normalized_name TEXT NOT NULL,
    UNIQUE(normalized_name, entity_type)
);

CREATE TABLE fnord_entities (
    fnord_id INTEGER NOT NULL REFERENCES fnords(id) ON DELETE CASCADE,
    entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    mention_count INTEGER NOT NULL DEFAULT 1,
    confidence REAL NOT NULL DEFAULT 0.0,
    PRIMARY KEY (fnord_id, entity_id)
);

CREATE INDEX idx_entities_type ON entities(entity_type);
CREATE INDEX idx_entities_normalized ON entities(normalized_name);
CREATE INDEX idx_fnord_entities_entity ON fnord_entities(entity_id);
```

**Backend-Änderungen:**
- Neues Modul `src-tauri/src/commands/entities.rs`
- NER-Extraktion: Erweitere Discordian-Analysis-Prompt um `entities[]` Feld, oder separater NER-Prompt
- Tauri-Commands: `get_article_entities`, `get_entity`, `search_entities`, `get_entity_articles`, `get_top_entities`
- Entity-Normalisierung: "Angela Merkel" = "Merkel" = "A. Merkel"

**Frontend-Änderungen:**
- `ArticleView.svelte`: Entity-Tags unter Keywords (farblich nach Typ unterschieden)
- Neue Komponente `EntityBadge.svelte` (klickbar → zeigt alle Artikel mit dieser Entität)
- Entity-Sidebar oder -Panel in bestehender Navigation
- i18n-Keys: `entities.title`, `.person`, `.organization`, `.location`, `.event`, `.mentions`, `.relatedArticles`

---

## Technische Konventionen

### Alle Agenten müssen beachten:

1. **DB-Pattern:** `state.db_conn()?` für Lock-Akquisition, kurze Locks, Transactions für Multi-Statement-Ops
2. **Tauri-Commands:** `#[tauri::command]` mit `Result<T, String>`, registrieren in `lib.rs` `.invoke_handler()`
3. **i18n:** Keys in `de.json` UND `en.json`, Verwendung via `$_('namespace.key')`
4. **Error Handling:** `CmdResult<T>` Typ-Alias aus `error.rs`
5. **Frontend Events:** `batch-complete` und `keywords-changed` CustomEvents beachten
6. **Rust Format:** `max_width=100` (rustfmt.toml), Clippy clean
7. **Svelte 5:** Runes-Syntax ($state, $derived, $effect), keine legacy stores
