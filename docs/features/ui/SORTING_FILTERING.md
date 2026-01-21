# Sortierung und Filterung

Die Artikel-Ansicht in fuckupRSS bietet flexible Sortier- und Filteroptionen, um schnell relevante Inhalte zu finden.

## Sortieroptionen

| Sortierung | Beschreibung | SQL |
|------------|--------------|-----|
| Neueste zuerst | Nach Veröffentlichungsdatum | `ORDER BY published_at DESC` |
| Älteste zuerst | Umgekehrt chronologisch | `ORDER BY published_at ASC` |
| Relevanz | Nach User-Interessen | `ORDER BY relevance_score DESC` |
| Quelle A-Z | Alphabetisch nach Feed | `ORDER BY pentacle_title ASC` |
| Sachlichkeit | Sachlichste zuerst | `ORDER BY sachlichkeit DESC` |

## Filteroptionen

| Filter | Werte | Mehrfachauswahl |
|--------|-------|-----------------|
| Status | Concealed, Illuminated, Golden Apple | Ja |
| Zeitraum | Heute, Diese Woche, Dieser Monat, Benutzerdefiniert | Nein |
| Quelle (Pentacle) | Alle abonnierten Feeds | Ja |
| Kategorie (Sephiroth) | Alle verfügbaren Kategorien | Ja |
| Stichworte (Immanentize) | Alle extrahierten Tags | Ja |
| Artikeltyp | news, analysis, opinion, etc. | Ja |
| Politische Tendenz | Links, Mitte, Rechts (Slider) | Nein |
| Sachlichkeit | Min-Wert (0-4) | Nein |
| Quellenqualität | Min-Sterne (1-5) | Nein |

## Suche

| Suchmodus | Beschreibung |
|-----------|--------------|
| **Volltext** | Klassische Keyword-Suche in Titel + Content |
| **Semantisch** | Via Embeddings, findet auch verwandte Begriffe |
| **Kombiniert** | Volltext + Semantisch, beste Ergebnisse |

## Relevante Quelldateien

- `src/lib/components/ArticleList.svelte` - Artikel-Liste mit Sortierung
- `src/lib/stores/state.svelte.ts` - Filter-State Management
- `src-tauri/src/commands/fnords.rs` - Backend-Filter-Logik
