# Synonym-Erkennung Refactoring Plan

## Status: ENTWURF - Warte auf Freigabe
**Erstellt:** 2026-01-23
**Autor:** Claude Opus 4.5 (Recherche-Team)

---

## 1. Problemanalyse

### 1.1 Root Cause

Das Frontend verwendet `get_similar_keywords` aus `article_analysis.rs`, die **NUR** die `immanentize_neighbors` Tabelle abfragt. Da "Trump" keinen Neighbor-Eintrag zu "Donald Trump" hat, wird es nie gefunden.

Die korrekt implementierte Funktion `find_similar_keywords` in `immanentize.rs` mit Name-Variant-Erkennung ist **NICHT im lib.rs registriert** und wird daher vom Frontend nie aufgerufen.

### 1.2 Aktuelle Architektur-Probleme

| Problem | Beschreibung | Auswirkung |
|---------|--------------|------------|
| **Doppelte Implementierung** | `get_similar_keywords` und `find_similar_keywords` existieren parallel | Inkonsistente Ergebnisse |
| **Ungenutzte Logik** | `find_similar_keywords` mit Name-Variant-Erkennung ist nicht registriert | String-Similarity wird nicht genutzt |
| **Fehlende Token Set Ratio** | Nur Levenshtein/Jaro-Winkler verfuegbar | Schlechte Erkennung von "Trump" ↔ "Donald Trump" |

### 1.3 Code-Standorte

```
src-tauri/src/commands/
├── article_analysis.rs
│   └── get_similar_keywords()      # Zeile 1022 - FRONTEND NUTZT DIESE
│
├── immanentize.rs
│   ├── find_similar_keywords()     # Zeile 1494 - NICHT REGISTRIERT!
│   ├── find_true_synonyms()        # Zeile 1912 - Registriert
│   ├── calculate_string_similarity()
│   ├── calculate_abbreviation_score()
│   └── calculate_exact_token_match_score()
```

---

## 2. Ziel-Architektur

### 2.1 Neues Modul: `src-tauri/src/similarity/`

Zentrales Modul fuer alle Similarity-Funktionen:

```
src-tauri/src/similarity/
├── mod.rs              # Exports und Trait-Definitionen
├── string.rs           # String-basierte Similarity (Token Set Ratio, etc.)
├── embedding.rs        # Embedding-basierte Similarity (Cosine)
├── hybrid.rs           # Kombinierte Ansaetze
└── tests.rs            # Unit-Tests
```

### 2.2 API-Konsolidierung

**Ziel:** Eine einheitliche Funktion fuer das Frontend

```rust
#[tauri::command]
pub fn find_similar_keywords_v2(
    state: State<AppState>,
    keyword_id: i64,
    options: Option<SimilarityOptions>,
) -> Result<Vec<SimilarKeywordResult>, String>

pub struct SimilarityOptions {
    pub method: SimilarityMethod,   // StringOnly, EmbeddingOnly, Hybrid
    pub string_threshold: f64,       // Default: 0.6
    pub embedding_threshold: f64,    // Default: 0.7
    pub limit: i64,                  // Default: 20
    pub include_name_variants: bool, // Default: true
}
```

### 2.3 Token Set Ratio Implementierung

```rust
/// Token Set Ratio: Findet Schnittmenge der Tokens
/// "Donald Trump" vs "Trump" -> 1.0 (Trump ist Subset)
/// "Donald Trump" vs "Trump, Donald" -> 1.0 (identische Tokens)
pub fn token_set_ratio(a: &str, b: &str) -> f64 {
    let a_tokens: HashSet<&str> = a.to_lowercase().split_whitespace().collect();
    let b_tokens: HashSet<&str> = b.to_lowercase().split_whitespace().collect();

    // Wenn einer Subset des anderen ist -> 1.0
    if a_tokens.is_subset(&b_tokens) || b_tokens.is_subset(&a_tokens) {
        return 1.0;
    }

    // Sonst: Jaccard Similarity der Tokens
    let intersection = a_tokens.intersection(&b_tokens).count();
    let union = a_tokens.union(&b_tokens).count();

    if union > 0 {
        intersection as f64 / union as f64
    } else {
        0.0
    }
}
```

---

## 3. Implementierungs-Reihenfolge

### Phase 1: Grundlagen (Backend)

| Schritt | Datei | Aenderung |
|---------|-------|-----------|
| 1.1 | `src-tauri/src/similarity/mod.rs` | Neues Modul erstellen |
| 1.2 | `src-tauri/src/similarity/string.rs` | Token Set Ratio + bestehende Funktionen |
| 1.3 | `src-tauri/src/similarity/embedding.rs` | Embedding-Funktionen extrahieren |
| 1.4 | `src-tauri/src/similarity/hybrid.rs` | Kombinierte Logik |
| 1.5 | `src-tauri/src/main.rs` | Modul importieren |

### Phase 2: API-Konsolidierung (Backend)

| Schritt | Datei | Aenderung |
|---------|-------|-----------|
| 2.1 | `src-tauri/src/commands/immanentize.rs` | `find_similar_keywords_v2` implementieren |
| 2.2 | `src-tauri/src/commands/article_analysis.rs` | `get_similar_keywords` auf neue API umstellen |
| 2.3 | `src-tauri/src/lib.rs` | Neue Commands registrieren |

### Phase 3: Frontend-Anpassung

| Schritt | Datei | Aenderung |
|---------|-------|-----------|
| 3.1 | `src/lib/types.ts` | `SimilarityOptions` Interface |
| 3.2 | `src/lib/components/KeywordNetwork.svelte` | Auf neue API umstellen |
| 3.3 | `src/lib/components/network/KeywordNetworkDetail.svelte` | Aehnliche Keywords anzeigen |

### Phase 4: Tests und Dokumentation

| Schritt | Datei | Aenderung |
|---------|-------|-----------|
| 4.1 | `src-tauri/src/similarity/tests.rs` | Unit-Tests |
| 4.2 | `docs/api/TAURI_COMMANDS_REFERENCE.md` | Dokumentation |

---

## 4. Dev-Team Aufteilung

| Agent | Aufgaben | Phase |
|-------|----------|-------|
| **Rust-Agent** | Similarity-Modul erstellen | Phase 1 |
| **Rust-Agent** | API-Konsolidierung | Phase 2 |
| **Svelte-Agent** | Frontend-Anpassung | Phase 3 |
| **Test-Agent** | Unit-Tests schreiben | Phase 4 |

---

## 5. Testplan

### 5.1 Kritische Test-Cases

| Test | Input | Erwartetes Ergebnis |
|------|-------|---------------------|
| Name-Variante | "Trump", "Donald Trump" | Similarity >= 0.9 |
| Identisch | "NATO", "NATO" | Similarity = 1.0 |
| Abbreviation | "EU", "European Union" | Similarity >= 0.9 |
| Unterschiedlich | "Trump", "Merkel" | Similarity < 0.3 |
| Token-Reihenfolge | "Donald Trump", "Trump Donald" | Similarity >= 0.9 |

### 5.2 Integration-Test

```
1. Starte App
2. Navigiere zu "Donald Trump" im Keyword Network
3. Pruefe ob "Trump" in "Aehnliche Keywords" erscheint
4. Pruefe ob Similarity >= 90% angezeigt wird
```

---

## 6. Risiken und Mitigationen

| Risiko | Mitigation |
|--------|------------|
| API-Breaking-Change | Alte Funktion als deprecated belassen |
| Performance | Token Set Ratio nur fuer Top-N Kandidaten |
| Falsche Positive | LLM-Verifikation als optionaler Schritt |

---

## 7. Erfolgs-Metriken

| Metrik | Vorher | Ziel |
|--------|--------|------|
| "Trump" bei "Donald Trump" erkannt | Nein | Ja (>90%) |
| "EU" bei "European Union" erkannt | Ja | Ja |
| API-Aufrufe | 2 verschiedene | 1 einheitliche |
| Code-Duplikation | Hoch | Minimal |

---

## 8. Kritische Dateien

1. **`src-tauri/src/commands/article_analysis.rs`** - `get_similar_keywords` (Zeile 1022)
2. **`src-tauri/src/commands/immanentize.rs`** - String-Similarity-Funktionen
3. **`src-tauri/src/lib.rs`** - Command-Registrierung
4. **`src/lib/components/KeywordNetwork.svelte`** - Frontend-Aufruf

---

## Freigabe

- [ ] Plan mit User abgestimmt
- [ ] Implementierung gestartet
- [ ] Tests bestanden
- [ ] Dokumentation aktualisiert
