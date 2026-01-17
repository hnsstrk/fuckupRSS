# MIGRATION_NOTES.md

## Fnord-Statistiken Wiederherstellung & Ausbau

**Datum:** 17. Januar 2026
**Commit-Kontext:** Wiederherstellung der regressierten "Nach Quelle" Funktion

---

## Zusammenfassung der Änderungen

### 1. Wiederhergestellt: "Nach Quelle" (By Source) Sektion

**Was:** All-time Übersicht der Revisionen pro Feed mit Fortschrittsbalken

**Wo:** `src/lib/components/FnordView.svelte`

**Daten-Quelle:** `stats.by_source` aus `get_fnord_stats()` (Backend war nie entfernt!)

**Visualisierung:**
- Sortiert nach `revision_count` (absteigend)
- Fortschrittsbalken relativ zum Maximum
- Zeigt `article_count` als Zusatzinfo
- Begrenzt auf Top 10 mit "+X weitere Quellen" Hinweis

### 2. Neustrukturierung: Layout in zwei Bereiche

**Vorher:**
- Zeitraum-Selektor im Header (wirkte global)
- Alle Statistiken scheinbar zeitgefiltert
- Verwirrende Semantik

**Nachher:**
```
┌─────────────────────────────────────────┐
│ GESAMT-ÜBERSICHT (ohne Zeitfilter)      │
│ - Greyface Index (all-time)             │
│ - Nach Quelle (all-time) [NEU]          │
│ - Bias Heatmap (all-time)               │
│ - Nach Kategorie (all-time)             │
├─────────────────────────────────────────┤
│ TRENDS & AKTIVITÄT                      │
│ Zeitraum: [7 Tage] [30 Tage] [90 Tage]  │
│ - Eris-Chronik (Timeline)               │
│ - Top Keywords                          │
│ - Feed-Aktivität                        │
│ - Keyword-Wolke                         │
└─────────────────────────────────────────┘
```

### 3. Zeitfilter: Position verschoben

**Vorher:** Im Header, vor den Tabs
**Nachher:** In der "Trends & Aktivität" Sektion, klar abgegrenzt

---

## Geänderte Dateien

| Datei | Änderung |
|-------|----------|
| `src/lib/components/FnordView.svelte` | Layout, "Nach Quelle" Sektion, CSS |
| `src/lib/i18n/de.json` | 3 neue Keys: `overallStats`, `trendsAndActivity`, `moreSources` |
| `src/lib/i18n/en.json` | 3 neue Keys (englische Übersetzungen) |

## Neue i18n Keys

```json
{
  "fnordView": {
    "overallStats": "Gesamt-Übersicht",
    "trendsAndActivity": "Trends & Aktivität",
    "moreSources": "weitere Quellen"
  }
}
```

---

## Backend: Keine Änderungen nötig

Das Backend in `src-tauri/src/commands/fnords.rs` liefert weiterhin:
- `get_fnord_stats()` mit `by_source: Vec<SourceRevisionStats>`
- Alle relevanten Daten waren nie entfernt

Die Types in `src/lib/stores/state.svelte.ts` sind ebenfalls unverändert:
- `FnordStats.by_source` existiert und funktioniert

---

## CSS: Neue Klassen

```css
/* Section Headers */
.stats-section-header
.stats-section-header.trends-header
.section-header-title

/* Source Card (Nach Quelle) */
.source-card
.source-list
.source-item
.source-header
.source-name
.source-count
.source-progress
.source-progress-fill
.source-meta
.source-articles
.source-more
```

---

## Regressions-Prävention

### Tests (empfohlen hinzuzufügen)

1. **E2E Test:** Fnord-Statistiken Seite öffnen, prüfen dass:
   - "Nach Quelle" Sektion sichtbar ist
   - Mindestens ein Feed mit Revisions angezeigt wird
   - Zeitfilter NUR im "Trends" Bereich ist

2. **Component Test:** `FnordView` Unit Test:
   - Mock `stats.by_source` mit Testdaten
   - Prüfen dass alle Quellen gerendert werden
   - Prüfen dass Fortschrittsbalken korrekte Breite haben

### Review-Checkliste für zukünftige Änderungen

- [ ] Entfernt diese Änderung `by_source` aus der UI? → Regressionsalarm!
- [ ] Verschiebt diese Änderung den Zeitfilter global? → Semantik prüfen!
- [ ] Sind Gesamt- und Zeitraum-Statistiken klar getrennt?

---

## Hintergrunddokumentation

Für Details zur Analyse und Entscheidungsfindung siehe:
- `FNORD_STATS_HISTORY.md` - Timeline und Regression-Analyse
- `FNORD_STATS_SEMANTICS.md` - Fachliche Begründung
- `FNORD_STATS_SCHEMA.md` - Datenmodell
- `FNORD_STATS_UX.md` - UI-Designentscheidungen
