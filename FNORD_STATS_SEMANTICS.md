# FNORD_STATS_SEMANTICS.md

## Begriffsdefinitionen

### Fnord
Im Kontext von fuckupRSS bezeichnet "Fnord" einen **Artikel mit Änderungen**:
- Ein Artikel, dessen Inhalt sich seit dem ersten Abruf verändert hat
- Wird in `fnords.has_changes = TRUE` gespeichert
- Revisionen werden in `fnord_revisions` protokolliert
- Ursprung: Illuminatus!-Trilogie - unsichtbare Wörter, die Angst erzeugen

### Quelle (Pentacle)
Ein **RSS/Atom Feed** als Nachrichtenquelle:
- Gespeichert in `pentacles` Tabelle
- Identifiziert durch URL
- Liefert Artikel (`fnords`)
- Jeder Artikel hat genau eine Quelle (`fnords.pentacle_id`)

### Revision
Eine **protokollierte Änderung** eines Artikels:
- Gespeichert in `fnord_revisions`
- Enthält den vorherigen Zustand (Titel, Inhalt, Autor)
- Ermöglicht Diff-Ansicht zwischen Versionen
- `fnords.revision_count` zählt die Gesamtzahl

---

## Sinnvolle Dimensionen für Fnord-Statistiken

### 1. Nach Quelle (Pentacle) - PRIMÄR
**Frage:** "Welche Quellen ändern ihre Artikel am häufigsten?"

**Relevanz:**
- Identifiziert unzuverlässige/dynamische Quellen
- Ermöglicht Qualitätsbewertung von Feeds
- Zeigt redaktionelle Praktiken unterschiedlicher Medien

**Metriken:**
- `revision_count`: Absolute Anzahl Revisionen
- `article_count`: Anzahl Artikel mit mindestens einer Revision
- Verhältnis: Revisionen pro geändertem Artikel

**ZEITFILTER:** Nicht sinnvoll. Die Gesamthistorie zeigt verlässlichere Muster als kurzfristige Aktivität.

### 2. Nach Kategorie (Sephiroth)
**Frage:** "In welchen Themenbereichen werden Artikel am häufigsten geändert?"

**Relevanz:**
- Politik/Nachrichten: Häufige Updates erwartet
- Technik/Wissenschaft: Weniger dynamisch
- Zeigt Nachrichtenfluss pro Bereich

**Metriken:**
- Revisionen pro Hauptkategorie
- Revisionen pro Unterkategorie

**ZEITFILTER:** Optional. Kann Trends zeigen, aber Grundstatistik sollte all-time sein.

### 3. Nach Status
**Frage:** "Wie viele Artikel sind gelesen/ungelesen/favorisiert?"

**Werte:**
- `concealed` (Ungelesen)
- `illuminated` (Gelesen)
- `golden_apple` (Favorisiert)

**ZEITFILTER:** Nicht relevant. Zeigt aktuellen Zustand.

### 4. Nach Bias-Level
**Frage:** "Wie verteilt sich die politische Tendenz der Artikel?"

**Werte:** -2 bis +2 (Extrem Links bis Extrem Rechts)

**ZEITFILTER:** Kann interessant sein für Trend-Analyse, aber Grundverteilung ist all-time.

---

## Begründung GEGEN Zeitfilter bei Grundstatistiken

### Das Problem
Der Zeitraum-Selektor (7/30/90 Tage) wurde für die **gesamten** Fnord-Statistiken eingeführt. Das führt zu:

1. **Verzerrung bei kleinen Zeiträumen:**
   - 7 Tage: Nur sehr aktive Feeds sichtbar
   - Stille, aber zuverlässige Feeds verschwinden

2. **Verlust der Gesamtperspektive:**
   - "Welche Quelle ist generell am unzuverlässigsten?" nicht beantwortbar
   - Nur "Welche Quelle war letzte Woche aktiv?"

3. **Inkonsistenz:**
   - Kategorie-Statistiken zeigen all-time Daten
   - Feed-Aktivität zeigt nur Zeitraum-Daten
   - Nutzer vergleicht Äpfel mit Birnen

### Die Lösung
**Klare Trennung:**

| Statistik-Typ | Zeitfilter | Begründung |
|---------------|------------|------------|
| Nach Quelle (Gesamt) | NEIN | Zeigt Gesamtverhalten |
| Nach Kategorie | NEIN | Zeigt Gesamtverteilung |
| Greyface Index | NEIN | Zeigt Gesamtbild der Bias-Verteilung |
| Timeline (Eris-Chronik) | JA | Explizit zeitabhängig |
| Keyword-Trends | JA | Explizit zeitabhängig |
| Feed-Aktivität | JA | Explizit für aktuelle Dynamik |

### Umsetzung
1. **Grundstatistiken** oben ohne Zeitfilter
2. **Zeitraum-Selektor** nur für den "Trends & Aktivität" Bereich
3. Klare visuelle Trennung zwischen beiden Bereichen

---

## Erwünschte vs. Unerwünschte Funktionen

### ERWÜNSCHT (wiederherstellen/beibehalten):
- [x] Aufteilung nach Quelle (all-time)
- [x] Aufteilung nach Kategorie (all-time)
- [x] Greyface Index (all-time)
- [x] Bias Heatmap (all-time)
- [ ] Geänderte Artikel Tab

### UNERWÜNSCHT (entfernen/einschränken):
- [ ] Globaler Zeitfilter für Grundstatistiken
- [ ] "Feed Activity" als Ersatz für "Nach Quelle"

### BEHALTEN (aber klar abgrenzen):
- [x] Zeitfilter für Timeline
- [x] Zeitfilter für Keyword-Trends
- [x] Zeitfilter für Keyword-Cloud
