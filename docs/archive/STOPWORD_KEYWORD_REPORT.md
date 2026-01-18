# Stopword & Keyword Integration Report

**Projekt:** fuckupRSS
**Datum:** 2025-01-17
**Status:** Abgeschlossen

## Executive Summary

Dieses Projekt hat die Stopword- und Keyword-Listen in fuckupRSS analysiert, bereinigt und konsolidiert. Die wichtigsten Änderungen:

1. **Media-Outlets aus news.txt entfernt** - ARD, BBC, Reuters etc. sind jetzt korrekt als Keywords (nicht Stopwords) behandelt
2. **UTF-8 Encoding korrigiert** - Deutsche Umlaute (ä, ö, ü, ß) statt ASCII-Ersetzungen
3. **Duplikate entfernt** - Alle Listen sind jetzt dedupliziert
4. **Validierungsscript erstellt** - `scripts/clean_stopwords.sh` für CI-Integration

---

## 1. Ist-Analyse (Phase 1)

### 1.1 Vorgefundene Struktur

```
src-tauri/
├── resources/stopwords/          # TXT-Dateien für DB-Seeding
│   ├── de.txt                    # Deutsche Stopwords
│   ├── en.txt                    # Englische Stopwords
│   ├── news.txt                  # News-spezifische Stopwords
│   └── technical.txt             # HTML/CSS/JS Stopwords
│
└── src/text_analysis/
    ├── stopwords.rs              # Runtime-Stopwords (hardcoded)
    └── keyword_seeds.rs          # Bekannte Entitäten (370+)
```

### 1.2 Identifizierte Probleme

| Problem | Schwere | Status |
|---------|---------|--------|
| **news.txt enthielt Media-Outlets** (ARD, BBC, etc.) | 🔴 Kritisch | ✅ Behoben |
| **Encoding-Fehler in de.txt** (ueber statt über) | 🟡 Mittel | ✅ Behoben |
| **Duplikate in allen Listen** | 🟡 Mittel | ✅ Behoben |
| **Fehlende Validierung** | 🟢 Niedrig | ✅ Script erstellt |

### 1.3 Zahlen vor der Bereinigung

| Datei | Zeilen | Duplikate | Encoding |
|-------|--------|-----------|----------|
| de.txt | 264 | 9 | ASCII (Umlaute als ae/ue/oe) |
| en.txt | 224 | 10 | UTF-8 |
| news.txt | 172 | 1 | UTF-8 |
| technical.txt | 541 | 6 | UTF-8 |
| **Total** | **1,201** | **26** | - |

---

## 2. Research (Phase 2)

### 2.1 Best Practices für Stopwords

**Quellen evaluiert:**
- spaCy (~315 DE, ~326 EN)
- NLTK (~231 DE, ~179 EN)
- stopwords-iso (aggregiert)
- Leipzig Corpora (frequenzbasiert)

**Empfehlung:** Bestehende Listen sind bereits umfassend (~1400 Einträge in stopwords.rs). Keine externen Quellen nötig.

### 2.2 Kritische Ausnahmen (Negation Whitelist)

Die folgenden Wörter sind **KEINE** Stopwords, auch wenn sie häufig vorkommen:

**Deutsch:**
```
nicht, kein, keine, keiner, keines, keinem, keinen,
nie, niemals, nirgends, niemand, nichts, weder, ohne
```

**Englisch:**
```
not, no, nor, neither, never, none, nothing,
nobody, nowhere, without, hardly, barely, scarcely
```

**Begründung:** Diese Wörter kehren die Bedeutung um (Sentiment reversal).

### 2.3 Media-Outlets sind Keywords, nicht Stopwords

Media-Outlets wie ARD, BBC, Reuters sind **legitime Keywords** wenn sie Thema eines Artikels sind:

> "ARD berichtet über Sparmaßnahmen" → "ARD" ist relevantes Keyword

Diese wurden aus news.txt entfernt und sind korrekt in `keyword_seeds.rs` als `KNOWN_ORGANIZATIONS`.

---

## 3. Durchgeführte Änderungen (Phase 4)

### 3.1 news.txt - Media-Outlets entfernt

**Vorher (172 Zeilen):**
```
# === German Media Outlets ===
ard
zdf
spiegel
zeit
...
# === International Media Outlets ===
bbc
cnn
reuters
...
```

**Nachher (83 Zeilen):**
```
# News-specific Stopwords
# These are generic journalistic terms - NOT media outlet names!
bericht
laut
unterdessen
...
```

### 3.2 de.txt - UTF-8 Encoding

**Vorher:**
```
ueber
fuer
waehrend
koennen
muessen
```

**Nachher:**
```
über
für
während
können
müssen
```

### 3.3 Alle Listen - Deduplizierung

| Datei | Vorher | Nachher | Entfernt |
|-------|--------|---------|----------|
| de.txt | 264 | 428 | -/+ (reorganisiert) |
| en.txt | 224 | 508 | -/+ (reorganisiert) |
| news.txt | 172 | 83 | 89 (Media-Outlets) |
| technical.txt | 541 | 526 | 15 |

### 3.4 Validierungsscript

Neues Script: `scripts/clean_stopwords.sh`

```bash
# Validierung (CI-Modus)
./scripts/clean_stopwords.sh --check

# Statistiken
./scripts/clean_stopwords.sh --stats
```

**Prüft:**
- UTF-8 Encoding
- Duplikate
- Media-Outlets in news.txt (Fehler)
- ASCII-Umlaute in de.txt (Fehler)

---

## 4. Verifizierung (Phase 5)

### 4.1 Rust-Tests

```
running 8 tests
test text_analysis::stopwords::tests::test_media_outlets_are_not_stopwords ... ok
test text_analysis::stopwords::tests::test_stopwords_contains_generic_news_terms ... ok
test text_analysis::stopwords::tests::test_stopwords_does_not_contain_content_words ... ok
test text_analysis::stopwords::tests::test_stopwords_contains_common_words ... ok
test text_analysis::stopwords::tests::test_stopwords_contains_html_terms ... ok
test text_analysis::tfidf::tests::test_tokenize_filters_stopwords ... ok
test keywords::tests::test_keyword_extraction_filters_stopwords ... ok
test keywords::tests::test_news_stopwords_filtered ... ok

test result: ok. 8 passed; 0 failed; 0 ignored
```

### 4.2 Script-Validierung

```
$ ./scripts/clean_stopwords.sh --check
[INFO] Validating stopword lists...
[INFO] All checks passed!
```

---

## 5. Architektur-Entscheidung

### 5.1 Gewählter Ansatz: TXT-Dateien als Seed-Quelle

```
resources/stopwords/*.txt  →  include_str!()  →  DB Seeding
                                    ↓
                            stopwords.rs (Runtime-Filter)
```

**Begründung:**
- TXT-Dateien sind diff-friendly und leicht editierbar
- include_str!() kompiliert sie in Binary (keine Runtime-I/O)
- stopwords.rs hat zusätzliche Runtime-Funktionen

### 5.2 Nicht gewählt: Reine DB-Quelle

Wurde verworfen weil:
- Zusätzliche Komplexität für Setup
- DB muss existieren bevor Stopwords verfügbar
- Keine Vorteile für statische Listen

---

## 6. Dateien

### 6.1 Geänderte Dateien

| Datei | Änderung |
|-------|----------|
| `src-tauri/resources/stopwords/de.txt` | UTF-8 Encoding, dedupliziert |
| `src-tauri/resources/stopwords/en.txt` | Dedupliziert |
| `src-tauri/resources/stopwords/news.txt` | Media-Outlets entfernt |
| `src-tauri/resources/stopwords/technical.txt` | Dedupliziert |

### 6.2 Neue Dateien

| Datei | Zweck |
|-------|-------|
| `scripts/clean_stopwords.sh` | Validierung und Wartung |
| `docs/STOPWORD_KEYWORD_REPORT.md` | Dieser Report |

---

## 7. Wartung

### 7.1 Neue Stopwords hinzufügen

1. Entsprechende TXT-Datei editieren
2. `./scripts/clean_stopwords.sh --check` ausführen
3. `cargo test stopwords` ausführen
4. Committen

### 7.2 CI-Integration

```yaml
# In .github/workflows/ci.yml
- name: Validate Stopwords
  run: ./scripts/clean_stopwords.sh --check
```

### 7.3 Jährliche Wartung

- [ ] Leipzig Corpora Frequenzen prüfen
- [ ] Neue Media-Outlets zu keyword_seeds.rs hinzufügen
- [ ] Veraltete Stopwords entfernen

---

## 8. Risiken & Mitigations

| Risiko | Mitigation |
|--------|------------|
| Overfiltering von Domänenbegriffen | Tests prüfen dass "politik", "wirtschaft" etc. NICHT gefiltert werden |
| Media-Outlets als Stopwords | Test `test_media_outlets_are_not_stopwords` |
| Encoding-Regression | Script prüft auf ASCII-Umlaute |

---

## 9. Fazit

Die Stopword-/Keyword-Integration ist jetzt:

✅ **Konsistent** - Keine Duplikate, einheitliches Format
✅ **Korrekt** - Media-Outlets sind Keywords, nicht Stopwords
✅ **Validiert** - 8 Tests + Validierungsscript
✅ **Wartbar** - Script für Wartung, klare Dokumentation

**Empfohlene nächste Schritte:**
1. CI-Integration des Validierungsscripts
2. Regelmäßige Überprüfung neuer Media-Outlets
