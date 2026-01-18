# fuckupRSS

**First Universal Cybernetic-Kinetic RSS Processor**

> *"The only truly free person is the one who can read all the feeds without being programmed by them."*  
> — Hagbard Celine (probably)

Ein RSS-Aggregator und Reader mit lokaler KI-Integration. Keine Cloud. Keine Tracker. Nur du und die Wahrheit hinter den Fnords.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS-lightgrey.svg)
![Status](https://img.shields.io/badge/status-in%20development-orange.svg)

---

## Was ist fuckupRSS?

fuckupRSS ist ein moderner RSS/Atom-Reader, der lokale KI-Modelle nutzt, um dir beim Durchblicken der täglichen Informationsflut zu helfen. Benannt nach **F.U.C.K.U.P.** (First Universal Cybernetic-Kinetic Ultra-micro Programmer) aus der [Illuminatus!-Trilogie](https://de.wikipedia.org/wiki/Illuminatus!).

**Das Problem:** Du abonnierst 50 Feeds. Du hast keine Zeit, alles zu lesen. Du verpasst Wichtiges. Du ertränkst in Unwichtigem.

**Die Lösung:** fuckupRSS analysiert jeden Artikel lokal mit KI:
- Erstellt Zusammenfassungen
- Kategorisiert automatisch  
- Erkennt politischen Bias
- Findet ähnliche Artikel
- Lernt deine Interessen

Alles läuft **lokal** auf deinem Rechner. Deine Lesegewohnheiten gehören dir.

---

## Features

### 🔍 KI-gestützte Analyse (Discordian Analysis)
- **Automatische Zusammenfassungen** – 2-3 Sätze pro Artikel
- **Kategorisierung** – Artikel werden Themen zugeordnet
- **Stichwort-Extraktion** – Wichtige Begriffe, Personen, Orte
- **Semantische Suche** – Finde Artikel nach Bedeutung, nicht nur Keywords

### ⚠️ Bias-Erkennung (Greyface Alert)
- **Politische Tendenz** – Links ↔ Rechts Spektrum
- **Sachlichkeit** – Emotional vs. faktenbasiert
- **Quellenqualität** – Sterne-Bewertung
- **Artikeltyp** – Nachricht, Meinung, Analyse, Satire

### 📰 Volltext-Abruf (Hagbard's Retrieval)
- Automatisches Nachladen für alle neuen Artikel
- Readability-Extraktion (entfernt Werbung, Navigation etc.)
- KI-Analysen verwenden ausschließlich den Volltext
- Keine Paywall-Umgehung – nur öffentlich zugängliche Inhalte

### 🧠 Intelligente Kategorisierung
- **Dual-Analyse:** LLM + statistische Textanalyse (TF-IDF mit Corpus-Statistiken)
- **Lernfähig:** System verbessert sich durch Benutzer-Korrekturen und LLM-Feedback
- **Transparent:** Jedes Keyword/Kategorie zeigt Quelle (KI/Statistik/Manuell) und Konfidenz
- **Editierbar:** Keywords und Kategorien manuell anpassen
- **Bias-Gewichtungen:** Aus Korrekturen gelernte Präferenzen
- **Source-Gewichtungen:** Manuelle Einträge werden höher gewichtet als statistische

### 🎯 Personalisierung (Operation Mindfuck)
- Personalisierte Empfehlungen basierend auf Leseverhalten
- Relevanz-Scoring mit Embedding-Similarity und Keyword-Overlap
- Quellenvielfalt durch Diversity-Reranking
- Artikel speichern und ausblenden für Feedback-Loop

### 🔗 Ähnliche Artikel
- Vektorbasierte Ähnlichkeitssuche mit snowflake-arctic-embed2
- Thematisch verwandte Artikel entdecken
- Auch ohne gemeinsame Keywords

---

## Illuminatus!-Terminologie

fuckupRSS verwendet durchgängig Begriffe aus der Illuminatus!-Trilogie:

| Begriff | Bedeutung |
|---------|-----------|
| **Fnord** | Geänderter Artikel (mit Revisionen) |
| **Concealed** ● | Ungelesener Artikel |
| **Illuminated** ○ | Gelesener Artikel |
| **Golden Apple** ✦ | Favorit |
| **Pentacle** | Feed-Quelle |
| **Sephiroth** | Kategorie |
| **Immanentize** | Stichwort/Tag |
| **Greyface Alert** | Bias-Warnung |
| **Hagbard's Retrieval** | Volltext-Abruf |
| **Discordian Analysis** | KI-Zusammenfassung |
| **Operation Mindfuck** | Interessen-Profil |

---

## Screenshots

*Coming soon – das Projekt befindet sich in Entwicklung.*

```
┌────────────────────────────────────────────────────────────────┐
│ fuckupRSS                                    [−][□][×]         │
├────────────────────────────────────────────────────────────────┤
│ PENTACLES         │ FNORDS                                     │
│                   │                                            │
│ ▼ Tech            │ ● EU verabschiedet AI Act                  │
│   • Heise (12)    │   heise.de · vor 2 Stunden                 │
│   • Golem (5)     │   📰 Nachricht · ━━●━━ · ★★★★☆             │
│                   │                                            │
│ ▼ Politik         │ ○ Neue Entwicklungen in der...             │
│   • Tagesschau    │   tagesschau.de · vor 3 Stunden            │
│                   │                                            │
├───────────────────┴────────────────────────────────────────────┤
│ 🔄 Discordian Analysis: 12/47 │ ████████░░░░░░ │ [Details]     │
└────────────────────────────────────────────────────────────────┘
```

---

## Systemanforderungen

### Hardware
- **GPU:** NVIDIA mit 12 GB VRAM (empfohlen) oder Apple Silicon
- **RAM:** 16 GB (32 GB empfohlen)
- **Speicher:** 2 GB für App + Modelle + Datenbank

### Software
- **Linux:** Ubuntu 22.04+, Fedora 38+, Arch Linux
- **macOS:** 13.0+ (Ventura) auf Apple Silicon
- **Ollama:** Für lokale KI-Modelle

---

## Installation

### 1. Ollama installieren

**Linux:**
```bash
curl -fsSL https://ollama.com/install.sh | sh
```

**macOS:**
```bash
brew install ollama
```

### 2. KI-Modelle herunterladen

```bash
ollama pull ministral-3:latest
ollama pull snowflake-arctic-embed2
```

### 3. Ollama konfigurieren (Linux)

```bash
sudo systemctl edit ollama.service
```

Füge hinzu:
```ini
[Service]
Environment="OLLAMA_MAX_LOADED_MODELS=2"
Environment="OLLAMA_FLASH_ATTENTION=1"
```

```bash
sudo systemctl daemon-reload
sudo systemctl restart ollama
```

### 4. Optimale Hardware-Konfiguration (Parallelisierung)

Um die Leistung von High-End Hardware (wie RTX 3080 Ti oder MacBook Pro M4) zu nutzen, unterstützt fuckupRSS parallele KI-Analysen. Damit dies funktioniert, muss Ollama entsprechend konfiguriert werden.

**Wichtig:** Der Wert für `OLLAMA_NUM_PARALLEL` muss mindestens dem in der App gewählten Profil entsprechen (z.B. Profil "Desktop (4x)" -> `OLLAMA_NUM_PARALLEL=4`).

#### Linux Desktop (z.B. NVIDIA RTX 3080 Ti)

Hier wird Ollama als systemd-Service konfiguriert:

```bash
sudo systemctl edit ollama.service
```

Füge im `[Service]`-Block folgende Zeilen hinzu:

```ini
[Service]
# Erlaubt das Laden von Main- und Embedding-Modell gleichzeitig
Environment="OLLAMA_MAX_LOADED_MODELS=2"

# Hält Modelle 24h im VRAM (vermeidet Neuladen)
Environment="OLLAMA_KEEP_ALIVE=24h"

# Erlaubt 4 gleichzeitige Anfragen (für RTX 3080 Ti / 12GB VRAM)
Environment="OLLAMA_NUM_PARALLEL=4"
```

Einstellungen übernehmen:
```bash
sudo systemctl daemon-reload
sudo systemctl restart ollama
```

#### Apple Silicon (z.B. MacBook Pro M4)

Für macOS (Ollama.app) müssen Umgebungsvariablen anders gesetzt werden:

1. Beende Ollama (Icon in der Menüleiste -> Quit).
2. Öffne ein Terminal und setze die Variablen permanent via `launchctl`:

```bash
launchctl setenv OLLAMA_MAX_LOADED_MODELS 2
launchctl setenv OLLAMA_KEEP_ALIVE 24h
# M4 Pro/Max Chips haben Unified Memory und können oft mehr parallel verarbeiten
launchctl setenv OLLAMA_NUM_PARALLEL 8 
```

3. Starte Ollama neu.

**Hinweis:** Die App steuert, wie viele Anfragen *gesendet* werden. Diese Konfiguration steuert, wie viele Ollama *gleichzeitig verarbeitet*. Wenn Ollama auf 1 steht, bringt das App-Profil "8x" nichts (Warteschlange).

### 5. fuckupRSS installieren

**Aus Releases (empfohlen):**
```bash
# Linux (.deb)
sudo dpkg -i fuckuprss_0.1.0_amd64.deb

# Linux (.rpm)
sudo rpm -i fuckuprss-0.1.0.x86_64.rpm

# Linux (AppImage)
chmod +x fuckupRSS-0.1.0.AppImage
./fuckupRSS-0.1.0.AppImage
```

**macOS:**
```bash
# .dmg herunterladen und installieren
```

**Aus Source:**
```bash
git clone https://github.com/yourusername/fuckuprss.git
cd fuckuprss
cargo tauri build
```

---

## Schnellstart

1. **fuckupRSS starten**
2. **Feeds hinzufügen** – OPML importieren oder URLs eingeben
3. **Warten** – Die erste Synchronisation läuft automatisch
4. **Lesen** – Artikel werden analysiert und kategorisiert

### Keyboard-Shortcuts

| Taste | Aktion |
|-------|--------|
| `j` / `k` | Nächster / Vorheriger Artikel |
| `o` | Artikel öffnen |
| `r` | Als gelesen markieren |
| `s` | Golden Apple (Favorit) |
| `v` | Im Browser öffnen |
| `/` | Suche |
| `?` | Alle Shortcuts anzeigen |

---

## Konfiguration

Alle Einstellungen werden in der lokalen SQLite-Datenbank gespeichert:
- **Datenbank:** `./data/fuckup.db` (im Projektordner)

Die Einstellungen können direkt in der App unter "Einstellungen" geändert werden:

### Allgemein
- Sprache (Deutsch/English)
- Theme (Mocha, Macchiato, Frappé, Latte)
- Tooltips für Illuminatus!-Begriffe

### Ollama (KI)
- Ollama-Status anzeigen
- Modell-Auswahl (Hauptmodell, Embedding-Modell)
- Empfohlene Modelle direkt herunterladen

### Prompts
- Anpassbare KI-Prompts für Zusammenfassung und Analyse
- Reset auf Standard-Prompts
- Ausgabesprache folgt den Spracheinstellungen

---

## Technologie

| Komponente | Technologie |
|------------|-------------|
| Framework | [Tauri](https://tauri.app/) 2.x |
| Backend | Rust |
| Frontend | Svelte 5 |
| Datenbank | SQLite + sqlite-vec |
| i18n | [svelte-i18n](https://github.com/kaisermann/svelte-i18n) |
| KI | [Ollama](https://ollama.com/) (lokal) |
| Modelle | ministral-3:latest, snowflake-arctic-embed2 |

### Mehrsprachigkeit

fuckupRSS unterstützt mehrere Sprachen:
- Deutsch (Standard)
- English

Die Sprache kann in den Einstellungen gewechselt werden.

---

## Warum lokal?

- **Privatsphäre** – Deine Lesegewohnheiten bleiben bei dir
- **Offline** – Funktioniert ohne Internet (nach erstem Sync)
- **Kontrolle** – Du entscheidest, welche Modelle laufen
- **Keine Kosten** – Keine API-Gebühren, keine Abos
- **Keine Zensur** – Keine Cloud-ToS, die bestimmen was du lesen darfst

---

## Roadmap

**Aktueller Status:** Phase 3 abgeschlossen, Phase 4 (Polish) in Entwicklung

Detaillierte Planung und Phasen-Übersicht: siehe [`fuckupRSS-Anforderungen.md`](fuckupRSS-Anforderungen.md#20-nächste-schritte)

---

## Contributing

Contributions sind willkommen! Bitte lies zuerst [CONTRIBUTING.md](CONTRIBUTING.md).

```bash
# Entwicklungsumgebung aufsetzen
git clone https://github.com/yourusername/fuckuprss.git
cd fuckuprss
npm install
cargo tauri dev
```

### Voraussetzungen für Entwicklung
- Rust 1.75+
- Node.js 20+
- Tauri CLI (`cargo install tauri-cli`)

### Tests

Das Projekt hat umfangreiche Tests (260 insgesamt):

```bash
# Rust Backend Tests (160 Tests)
cargo test --manifest-path src-tauri/Cargo.toml

# Frontend Unit Tests (89 Tests)
npm run test

# E2E Tests (11 Tests)
npm run test:e2e
```

**Wichtig:** Alle neuen Features und Bugfixes müssen mit Tests abgedeckt werden.

---

## Lizenz

MIT License – siehe [LICENSE](LICENSE)

---

## Danksagungen

- Robert Shea und Robert Anton Wilson für die Illuminatus!-Trilogie
- Das [Ollama](https://ollama.com/)-Team für lokale LLMs
- Das [Tauri](https://tauri.app/)-Team für das Framework
- [Qwen](https://github.com/QwenLM/Qwen)-Team für die Modelle

---

## Fnord

```
        ▲
       ╱ ╲
      ╱   ╲
     ╱  ●  ╲
    ╱   │   ╲
   ╱    │    ╲
  ▔▔▔▔▔▔▔▔▔▔▔▔

  IMMANENTIZE THE ESCHATON
  ONE FEED AT A TIME
```

*"Denke selbst. Hinterfrage alles. Lies die Fnords."*
