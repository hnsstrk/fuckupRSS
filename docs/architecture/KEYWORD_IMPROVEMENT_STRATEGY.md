# Keyword-Verbesserungsstrategie

**Status:** Entwurf
**Erstellt:** 2026-01-21
**Basiert auf:** Expertenteam-Analyse (Code, DB, Best Practices)

## Executive Summary

Das Keyword-System hat zwei grundlegende Probleme:
1. **Das Synonym-System ist inaktiv** - Keine `canonical_id` Zuweisungen trotz funktionierendem Code
2. **74% aller Keywords sind Einmal-Vorkommen** - Mangelnde Normalisierung bei der Extraktion

Diese Strategie adressiert beide Probleme mit einem Drei-Phasen-Ansatz.

---

## Phase 1: Sofortmassnahmen (Cleanup)

### 1.1 Datenbank-Bereinigung

**Ziel:** Reduzierung der Keywords von 12.562 auf ~3.000 hochwertige Keywords

| Aktion | Betroffene Keywords | Methode |
|--------|---------------------|---------|
| Verwaiste Keywords loeschen | 588 | `DELETE WHERE article_count = 0` |
| HTML-Fragmente entfernen | ~50 | Pattern-Match: `div`, `component`, `class` |
| Zahlen-Keywords entfernen | ~200 | Pattern: `^\d+` oder `Mrd\. USD` |
| Low-Quality prunen | ~2.500 | `quality_score < 0.2 AND article_count = 1` |

**SQL-Skript:**
```sql
-- 1. Verwaiste Keywords
DELETE FROM fnord_immanentize WHERE immanentize_id IN (
    SELECT id FROM immanentize WHERE article_count = 0
);
DELETE FROM immanentize WHERE article_count = 0;

-- 2. HTML-Fragmente (manuell pruefen!)
SELECT * FROM immanentize
WHERE LOWER(name) LIKE '%div%'
   OR LOWER(name) LIKE '%component%'
   OR LOWER(name) LIKE '%class%';

-- 3. Low-Quality
DELETE FROM fnord_immanentize WHERE immanentize_id IN (
    SELECT id FROM immanentize
    WHERE quality_score < 0.2 AND article_count = 1
);
DELETE FROM immanentize WHERE quality_score < 0.2 AND article_count = 1;
```

### 1.2 Case-Duplikate zusammenfuehren

**Betroffene Paare:**
- `nhs` / `NHS` -> `NHS`
- `internet` / `Internet` -> `Internet`
- `verbot` / `Verbot` -> `Verbot`

**Implementierung:** Erweiterung `merge_synonym_keywords()` um case-insensitive Matching.

### 1.3 Synonym-System aktivieren

**Problem:** 0 canonical_id Zuweisungen obwohl SYNONYM_GROUPS existiert.

**Loesung:** Einmaliger Sync der statischen Synonyme:
```rust
pub fn sync_static_synonyms(conn: &Connection) -> Result<usize> {
    for (canonical, variants) in SYNONYM_GROUPS.iter() {
        let canonical_id = get_or_create_keyword(conn, canonical)?;
        for variant in variants {
            if let Some(variant_id) = find_keyword(conn, variant) {
                conn.execute(
                    "UPDATE immanentize SET canonical_id = ? WHERE id = ?",
                    params![canonical_id, variant_id]
                )?;
            }
        }
    }
}
```

---

## Phase 2: Automatisches Compound-Splitting

### 2.1 Problem

Aktuell: `split_compound_keyword()` existiert, aber:
- Nur Whitelist-basiert (trump, ukraine, etc.)
- Keine Integration in Extraktion-Pipeline
- Keine Rueckverknuepfung zum Original

**Beispiel:**
- Input: "Ukraine-Krieg"
- Aktuell: Wird als einzelnes Keyword gespeichert
- Gewuenscht: "Ukraine-Krieg" + Neighbor-Relations zu "Ukraine" und "Krieg"

### 2.2 Loesung: Erweiterte Compound-Verarbeitung

```rust
pub struct CompoundResult {
    pub original: String,
    pub components: Vec<String>,
    pub should_split: bool,
}

pub fn analyze_compound(keyword: &str) -> CompoundResult {
    // Regel 1: Bindestrich -> Split-Kandidat
    if keyword.contains('-') {
        let parts: Vec<&str> = keyword.split('-').collect();
        if parts.len() >= 2 && parts.iter().all(|p| p.len() >= 3) {
            return CompoundResult {
                original: keyword.to_string(),
                components: parts.iter().map(|s| capitalize(s)).collect(),
                should_split: true,
            };
        }
    }

    // Regel 2: Klammern -> Abkuerzung extrahieren
    // "Europaeische Zentralbank (EZB)" -> ["Europaeische Zentralbank", "EZB"]
    if let Some(caps) = ABBREV_REGEX.captures(keyword) {
        return CompoundResult {
            original: caps[1].to_string(),
            components: vec![caps[2].to_string()],
            should_split: true,
        };
    }

    CompoundResult { original: keyword.to_string(), components: vec![], should_split: false }
}
```

### 2.3 Integration in Pipeline

**Speicherlogik erweitern:**
```rust
// In process_keywords()
for keyword in extracted_keywords {
    let compound = analyze_compound(&keyword.name);

    // Original speichern
    let main_id = store_keyword(conn, &compound.original)?;

    // Komponenten als Neighbors verknuepfen
    if compound.should_split {
        for component in &compound.components {
            let comp_id = store_keyword(conn, component)?;
            create_neighbor(conn, main_id, comp_id, "component")?;
        }
    }
}
```

### 2.4 Relation-Typ Schema-Erweiterung

```sql
ALTER TABLE immanentize_neighbors ADD COLUMN relation_type TEXT DEFAULT 'cooccurrence';
-- Werte: 'cooccurrence', 'synonym', 'component', 'hypernym'
```

---

## Phase 3: Verbesserte manuelle Verwaltung

### 3.1 UI-Anforderungen

**Keyword-Detail-Dialog:**

```
┌─────────────────────────────────────────────────┐
│ Keyword: Ukraine-Krieg                          │
├─────────────────────────────────────────────────┤
│ Artikel: 47  │  Quality: 0.72  │  Typ: concept  │
├─────────────────────────────────────────────────┤
│                                                 │
│ [Aktionen]                                      │
│ ┌─────────────────────────────────────────────┐ │
│ │ ○ In Komponenten aufteilen                  │ │
│ │   → Ukraine + Krieg (Original behalten)     │ │
│ │                                             │ │
│ │ ○ Mit anderem Keyword zusammenfuehren       │ │
│ │   [Suche: ________________]                 │ │
│ │                                             │ │
│ │ ○ Als Synonym markieren von:               │ │
│ │   [Dropdown: Aehnliche Keywords]            │ │
│ │                                             │ │
│ │ ○ Loeschen (Verknuepfungen uebertragen an:) │ │
│ │   [Dropdown: _____________]                 │ │
│ └─────────────────────────────────────────────┘ │
│                                                 │
│ [Abbrechen]  [Ausfuehren]                       │
└─────────────────────────────────────────────────┘
```

### 3.2 Backend-Commands

**Neue Tauri-Commands:**

```rust
#[tauri::command]
pub fn split_keyword_into_components(
    state: State<AppState>,
    keyword_id: i64,
    keep_original: bool,
) -> Result<SplitResult, String>;

#[tauri::command]
pub fn merge_keywords(
    state: State<AppState>,
    source_ids: Vec<i64>,
    target_id: i64,
    transfer_articles: bool,
) -> Result<MergeResult, String>;

#[tauri::command]
pub fn set_keyword_synonym(
    state: State<AppState>,
    keyword_id: i64,
    canonical_id: i64,
) -> Result<(), String>;
```

### 3.3 Bulk-Operationen

**Keyword-Liste mit Checkboxen:**
- [x] Ukraine-Krieg
- [x] Krieg in der Ukraine
- [x] Ukrainekrieg

**Aktion:** "Ausgewaehlte zusammenfuehren" -> Dialog mit Ziel-Keyword-Auswahl

---

## Best Practices (aus Recherche)

### Extraktion

| Empfehlung | Quelle |
|------------|--------|
| YAKE als Vorfilter, LLM fuer Validierung | Benchmark 2024 |
| Lemmatisierung statt Stemming fuer Deutsch | Stanford NLP |
| Named Entities separat extrahieren | BERT NER Research |

### Normalisierung

```
Eingabe: "Die Bundesregierung beschliesst neue Massnahmen"
    ↓ Tokenisierung
    ↓ Stopword-Entfernung
    ↓ Lemmatisierung (nicht Stemming!)
    ↓ Case-Normalisierung (ausser Eigennamen)
Ausgabe: ["Bundesregierung", "beschliessen", "Massnahme"]
```

### Synonym-Schwellwerte

| Beziehung | Embedding-Similarity |
|-----------|----------------------|
| Synonym (auto-merge) | > 0.92 |
| Synonym (Vorschlag) | 0.85-0.92 |
| Eng verwandt | 0.70-0.85 |

### Tools fuer Compound-Splitting (Deutsch)

| Tool | Ansatz | Genauigkeit |
|------|--------|-------------|
| CharSplit | Probabilistisch | ~95% |
| german_compound_splitter | Woerterbuch (2.1M) | ~98% |
| compound-split (PyPI) | N-Gram | ~90% |

---

## Implementierungs-Roadmap

### Woche 1: Cleanup
- [ ] SQL-Cleanup-Skript ausfuehren
- [ ] Case-Duplikate manuell pruefen und mergen
- [ ] `sync_static_synonyms()` implementieren und ausfuehren

### Woche 2: Compound-Splitting
- [ ] `analyze_compound()` implementieren
- [ ] Schema erweitern (`relation_type`)
- [ ] Pipeline-Integration

### Woche 3: UI
- [ ] Keyword-Detail-Dialog
- [ ] Bulk-Operationen
- [ ] Synonym-Vorschlaege verbessern

### Woche 4: Prompt-Optimierung
- [ ] Entity-Typen separat extrahieren
- [ ] Compound-Keywords im Prompt anfordern
- [ ] Qualitaets-Metriken tracken

---

## Erfolgsmetriken

| Metrik | Aktuell | Ziel |
|--------|---------|------|
| Keywords mit 1 Artikel | 74% | < 30% |
| Aktive Synonym-Relationen | 0 | > 500 |
| Durchschn. Quality Score | 0.42 | > 0.55 |
| Compound-Keywords gesplittet | 0 | > 80% |

---

## Entscheidungen (2026-01-21)

1. **Komponenten ersetzen Original:**
   - ✅ "Ukraine-Krieg" wird geloescht, Artikel zu "Ukraine" + "Krieg" zugewiesen
   - Kein Beibehalten von Compound-Keywords

2. **Aggressiver Cleanup:**
   - ✅ Alle Keywords mit quality < 0.2 UND article_count = 1 entfernen
   - ~3.300 Keywords werden geloescht

3. **Nur Bindestrich-Komposita splitten:**
   - ✅ "Bundesregierung" bleibt als einzelnes Keyword
   - ❌ Kein Woerterbuch-basiertes Splitting fuer durchgeschriebene Komposita
