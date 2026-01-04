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
- Automatisches Nachladen bei gekürzten Feeds
- Readability-Extraktion
- Keine Paywall-Umgehung – nur öffentlich zugängliche Inhalte

### 🎯 Personalisierung (Operation Mindfuck)
- Definiere deine Interessen
- Relevanz-Scoring basierend auf Leseverhalten
- Priorisierte Übersichten

### 🔗 Ähnliche Artikel
- Vektorbasierte Ähnlichkeitssuche
- Thematisch verwandte Artikel entdecken
- Auch ohne gemeinsame Keywords

---

## Illuminatus!-Terminologie

fuckupRSS verwendet durchgängig Begriffe aus der Illuminatus!-Trilogie:

| Begriff | Bedeutung |
|---------|-----------|
| **Fnord** | Ungelesener Artikel |
| **Illuminated** | Gelesener Artikel |
| **Golden Apple** 🍎 | Favorit |
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
ollama pull qwen3-vl:8b
ollama pull nomic-embed-text
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

### 4. fuckupRSS installieren

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

Die Konfigurationsdatei liegt unter:
- **Linux:** `~/.local/share/fuckupRSS/config.toml`
- **macOS:** `~/Library/Application Support/fuckupRSS/config.toml`

```toml
[sync]
interval_minutes = 30
sync_on_start = true

[ollama]
host = "http://localhost:11434"
main_model = "qwen3-vl:8b"
embedding_model = "nomic-embed-text"

[ui]
theme = "dark"
show_greyface = true
```

---

## Technologie

| Komponente | Technologie |
|------------|-------------|
| Framework | [Tauri](https://tauri.app/) 2.x |
| Backend | Rust |
| Frontend | Svelte |
| Datenbank | SQLite + [SQLite-VSS](https://github.com/asg017/sqlite-vss) |
| KI | [Ollama](https://ollama.com/) (lokal) |
| Modelle | qwen3-vl:8b, nomic-embed-text |

---

## Warum lokal?

- **Privatsphäre** – Deine Lesegewohnheiten bleiben bei dir
- **Offline** – Funktioniert ohne Internet (nach erstem Sync)
- **Kontrolle** – Du entscheidest, welche Modelle laufen
- **Keine Kosten** – Keine API-Gebühren, keine Abos
- **Keine Zensur** – Keine Cloud-ToS, die bestimmen was du lesen darfst

---

## Roadmap

- [x] Anforderungsdokument
- [x] **Phase 1** – Grundgerüst (Tauri + Svelte, SQLite, Basis-UI)
- [ ] **v0.1** – Basis-Reader mit Feed-Sync
- [ ] **v0.2** – KI-Integration (Zusammenfassung, Kategorien)
- [ ] **v0.3** – Greyface Alert (Bias-Erkennung)
- [ ] **v0.4** – Semantische Suche + Ähnliche Artikel
- [ ] **v0.5** – Operation Mindfuck (Personalisierung)
- [ ] **v1.0** – Stabiler Release

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
