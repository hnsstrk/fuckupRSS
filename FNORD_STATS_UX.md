# FNORD_STATS_UX.md

## Aktueller Zustand (Probleme)

### Was fehlt:
1. **"Nach Quelle" Sektion** - All-time Revisions pro Feed
2. Klare Trennung zwischen Gesamt- und Zeitraum-Statistiken

### Was verwirrt:
1. **Zeitfilter oben** wirkt so, als würde er alles filtern
2. "Feed Activity" zeigt andere Daten als erwartet (nur Zeitraum)
3. Kategorien zeigen all-time, Feeds zeigen Zeitraum - inkonsistent

---

## Zielbild: Neues Layout

### Struktur (von oben nach unten)

```
┌─────────────────────────────────────────────────────────────┐
│ HEADER                                                       │
│ Fnord-Statistiken    [123 Revisionen] [45 Geänderte Artikel]│
│                                                              │
│ [Stats-Tab] [Geänderte Artikel]                             │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ GESAMT-ÜBERSICHT (OHNE Zeitfilter)                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────────┐  ┌──────────────────────────────┐ │
│  │ GREYFACE INDEX       │  │ BIAS HEATMAP                 │ │
│  │ [Gauge: 42]          │  │ Feed × Bias Matrix           │ │
│  │ Avg Bias: -0.12      │  │ ████░░░░░░ Quelle A          │ │
│  │ Sachlichkeit: 2.8    │  │ ░░████░░░░ Quelle B          │ │
│  │ [Bias-Verteilung]    │  │ ░░░░░░████ Quelle C          │ │
│  └──────────────────────┘  └──────────────────────────────┘ │
│                                                              │
│  ┌──────────────────────┐  ┌──────────────────────────────┐ │
│  │ NACH QUELLE          │  │ NACH KATEGORIE               │ │
│  │ (Gesamt)             │  │ (erweiterbar)                │ │
│  │                      │  │                              │ │
│  │ Quelle A      42 Rev │  │ ┌────────────────────────┐   │ │
│  │ ████████████████░░░░ │  │ │🔬 Wissen & Technik    │   │ │
│  │                      │  │ │   Rev: 28  Art: 12    │   │ │
│  │ Quelle B      31 Rev │  │ └────────────────────────┘   │ │
│  │ ████████████░░░░░░░░ │  │ ┌────────────────────────┐   │ │
│  │                      │  │ │🏛️ Politik & Gesellsch. │   │ │
│  │ Quelle C      18 Rev │  │ │   Rev: 45  Art: 23    │   │ │
│  │ ███████░░░░░░░░░░░░░ │  │ └────────────────────────┘   │ │
│  │                      │  │                              │ │
│  │ [Alle N Quellen...]  │  │ [Klick = Unterkategorien]   │ │
│  └──────────────────────┘  └──────────────────────────────┘ │
│                                                              │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ TRENDS & AKTIVITÄT                                           │
│ Zeitraum: [7 Tage] [30 Tage] [90 Tage]                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ ERIS-CHRONIK (Timeline)                                │ │
│  │ ▁▂▅▇█▆▄▂▁▂▃▅▆█▇▅▃▁▂▄▆▇█▆▄▂▁                           │ │
│  │ 01-10        01-13        01-16                        │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌──────────────────────┐  ┌──────────────────────────────┐ │
│  │ TOP KEYWORDS         │  │ FEED-AKTIVITÄT               │ │
│  │ (im Zeitraum)        │  │ (im Zeitraum)                │ │
│  │                      │  │                              │ │
│  │ #1 Trump       45 ↑  │  │ Quelle A: 12 Art, 5 Rev     │ │
│  │ #2 Ukraine     38 ↓  │  │ Quelle B:  8 Art, 3 Rev     │ │
│  │ #3 KI         32 NEW │  │ Quelle C:  5 Art, 2 Rev     │ │
│  └──────────────────────┘  └──────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ KEYWORD-WOLKE                                          │ │
│  │   Trump  Ukraine  KI  Wirtschaft  Klima  Biden  ...    │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Komponenten-Details

### "Nach Quelle" Sektion (WIEDERHERZUSTELLEN)

**Position:** Im "Gesamt-Übersicht" Bereich, neben "Nach Kategorie"

**Inhalt:**
```svelte
<div class="stats-section">
  <h3>Nach Quelle</h3>
  {#each stats.by_source as source}
    <div class="source-item">
      <span class="source-name">{source.title}</span>
      <span class="source-count">{source.revision_count}</span>
      <div class="progress-bar">
        <div class="progress-fill" style="width: {barWidth}%"></div>
      </div>
      <span class="article-count">{source.article_count} Artikel</span>
    </div>
  {/each}
</div>
```

**Visualisierung:**
- Fortschrittsbalken relativ zum Maximum aller Quellen
- Quellen absteigend nach revision_count sortiert
- Bei > 10 Quellen: "Alle anzeigen" Expander

### Zeitraum-Selektor

**Aktuelle Position:** Im Header, wirkt global

**Neue Position:** Nur im "Trends & Aktivität" Abschnitt
- Optisch vom Gesamt-Bereich getrennt
- Klare Beschriftung "Zeitraum für Trends"

---

## Begründung der Visualisierungswahl

### Fortschrittsbalken für "Nach Quelle"
- **Warum:** Schnelle visuelle Erfassung der Verhältnisse
- **Alternative verworfen:** Kreisdiagramm (zu viele Segmente bei vielen Feeds)
- **Alternative verworfen:** Tabelle (weniger intuitiv)

### Karten für "Nach Kategorie"
- **Warum:** Interaktivität (erweiterbar), visuelle Hierarchie
- **Passt zu:** 6 Hauptkategorien = überschaubar

### Trennung Gesamt/Trends
- **Warum:** Unterschiedliche Fragen, unterschiedliche Zeitbezüge
- **Verhindert:** Verwirrung über Zeitfilter-Reichweite

---

## MVP vs. Erweiterungen

### MVP (sofort umsetzen)
1. [x] "Nach Quelle" Sektion wiederherstellen
2. [x] Zeitfilter aus Header in "Trends" Bereich verschieben
3. [x] Klare visuelle Trennung Gesamt/Trends

### Erweiterungen (später)
- [ ] "Alle Quellen anzeigen" mit Modal/Overlay
- [ ] Quellen anklickbar → Filter auf diese Quelle
- [ ] Export der Statistiken als CSV
- [ ] Vergleich verschiedener Zeiträume

---

## Implementierungs-Checkliste

### Frontend (FnordView.svelte)

- [ ] `stats.by_source` rendern (war: entfernt, jetzt: wiederherstellen)
- [ ] Fortschrittsbalken für Quellen (wie vor 16.01.)
- [ ] Zeitraum-Selektor verschieben (aus Header in Trends-Bereich)
- [ ] Optische Trennung: `<hr>` oder Container mit Überschrift

### Keine Backend-Änderungen nötig
- `get_fnord_stats()` liefert bereits `by_source`
- Types sind definiert
