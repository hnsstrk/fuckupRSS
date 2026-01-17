# FNORD_STATS_HISTORY.md

## Timeline der Fnord-Statistiken Features

### 6. Januar 2026 - Initiale Implementierung (Commit 21ed832)
**"feat: Erweitertes Revisions-System mit Volltext-Diff und Fnord-Tab"**

**Eingeführte Features:**
- Fnord-Statistiken Tab mit zwei Hauptsektionen
- **"Nach Kategorie"** - Revision-Counts pro Sephiroth-Kategorie
- **"Nach Quelle"** - Revision-Counts pro Feed/Pentacle mit Fortschrittsbalken
- Backend liefert `FnordStats { by_category, by_source }`

### 14. Januar 2026 - UI Verbesserungen

**Commit 8936c6c - Card-basiertes Layout:**
- Einfache Tabellen durch Card-Design ersetzt
- Gradient-Hintergründe und Fortschrittsbalken
- Hover-Effekte
- **"Nach Quelle" Sektion weiterhin vorhanden**

**Commit a6c8af3 - Erweiterbare Kategorien:**
- Klickbare Kategorie-Karten mit Unterkategorien
- **"Nach Quelle" Sektion weiterhin vorhanden**

### 16. Januar 2026 - REGRESSION (Commit 9990aec)
**"feat: Add extended Fnord statistics with Greyface Index (Plan 4)"**

**Hinzugefügt:**
- Zeitraum-Selektor (7/30/90 Tage)
- Greyface Index Gauge
- Eris-Chronik Timeline
- Top Keywords mit Trend-Indikatoren
- **"Feed Activity"** (ersetzte "Nach Quelle")
- Bias Heatmap
- Keyword Cloud
- Easter Eggs (fnord, 23)

**ENTFERNT (Regression):**
- Einfache **"Nach Quelle"** Ansicht aus dem Frontend
- Backend liefert `by_source` weiterhin, wird aber nicht angezeigt

---

## Diff: Früher vs. Jetzt

| Aspekt | Vor 16. Jan (alt) | Nach 16. Jan (neu) |
|--------|-------------------|---------------------|
| **Quellen-Übersicht** | "Nach Quelle" - alle Feeds, all-time | "Feed Activity" - nur Top 5, zeitgefiltert |
| **Zeitfilter** | Keiner | 7/30/90 Tage global |
| **Datenquelle** | `stats.by_source` (all-time) | `feedActivity` (period-filtered) |
| **Anzahl Feeds** | Alle | Nur 5 |
| **Metriken** | revision_count, article_count | articles_period, revisions_period |
| **Visualisierung** | Fortschrittsbalken relativ zum Maximum | Kompakte Karten ohne Balken |

---

## Technische Details der Regression

### Was das Backend liefert (unverändert)
```rust
// src-tauri/src/commands/fnords.rs:478
pub struct FnordStats {
    pub total_revisions: i64,
    pub articles_with_changes: i64,
    pub by_category: Vec<CategoryRevisionStats>,
    pub by_source: Vec<SourceRevisionStats>,  // <-- NOCH VORHANDEN
}
```

### Was das Frontend lädt (unverändert)
```typescript
// FnordView.svelte:68-69
const statsData = await appState.getFnordStats();
stats = statsData;  // stats.by_source ist verfügbar!
```

### Was das Frontend NICHT mehr rendert
```svelte
<!-- FRÜHER VORHANDEN, JETZT GELÖSCHT -->
{#if stats.by_source.length > 0}
  <div class="stats-section">
    <h3>Nach Quelle</h3>
    <!-- Revision-Counts pro Feed mit Fortschrittsbalken -->
  </div>
{/if}
```

---

## Ursachen-Analyse

1. **Absicht vs. Versehen:**
   - Die Entfernung war wahrscheinlich **beabsichtigt** im Rahmen des UI-Umbaus
   - "Feed Activity" sollte "Nach Quelle" ersetzen
   - Aber: Die neue Komponente erfüllt einen **anderen Zweck**

2. **Semantischer Unterschied:**
   - **"Nach Quelle" (alt):** "Welche Quellen haben die meisten Änderungen insgesamt?"
   - **"Feed Activity" (neu):** "Welche Quellen waren im Zeitraum am aktivsten?"
   - Das sind zwei verschiedene Fragen!

3. **Datenverlust:**
   - All-time Statistiken nicht mehr sichtbar
   - Nur noch relative Aktivität im kurzen Zeitfenster

---

## Empfehlung

### Sofort wiederherstellen:
- **"Nach Quelle"** Sektion mit `stats.by_source` Daten
- All-time Revision-Counts pro Feed
- Fortschrittsbalken für visuelle Gewichtung

### Zeitfilter:
- Für grundlegende Statistiken (Kategorie, Quelle) NICHT verwenden
- Nur für erweiterte Analysen (Timeline, Trends) beibehalten
- Klare visuelle Trennung zwischen "Gesamt" und "Zeitraum"

### Strukturvorschlag:
1. **Grundlegende Statistiken** (ohne Zeitfilter)
   - Nach Kategorie (Sephiroth)
   - Nach Quelle (Pentacle)
2. **Erweiterte Analysen** (mit Zeitfilter)
   - Greyface Index
   - Timeline
   - Keywords & Trends
   - Bias Heatmap
