# fuckupRSS

## First Universal Cybernetic-Kinetic RSS Processor

> *"The only truly free person is the one who can read all the feeds without being programmed by them."*  
> — Hagbard Celine (probably)

An RSS aggregator and reader with local AI integration via Ollama. Just you and the truth behind the Fnords.

> Also supports OpenAI-compatible APIs as an alternative AI backend.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS-lightgrey.svg)
![Status](https://img.shields.io/badge/status-experimental-orange.svg)

---

## What is this?

fuckupRSS is a **personal hobby project** — an experiment in combining RSS reading with local AI analysis. Named after **F.U.C.K.U.P.** (First Universal Cybernetic-Kinetic Ultra-micro Programmer) from the [Illuminatus! Trilogy](https://en.wikipedia.org/wiki/The_Illuminatus!_Trilogy).

**This is not production-ready software.** It's a playground for exploring what's possible when you throw local LLMs at an RSS reader: automatic summaries, bias detection, keyword networks, article clustering, daily briefings. Some of it works well, some of it is rough around the edges, and the whole thing is in constant flux.

If you're looking for a polished RSS reader, use [Miniflux](https://miniflux.app/) or [Newsboat](https://newsboat.org/). If you're curious about what happens when AI meets RSS — read on.

---

## What it does

- **AI-powered article analysis** — summaries in your language (regardless of source language), categorization, keyword extraction, article type classification, named entity recognition
- **Bias detection** — political tendency, objectivity rating, source quality
- **Full-text retrieval** — fetches complete article text, not just RSS snippets
- **Semantic search** — find articles by meaning using vector embeddings
- **Daily briefings** — AI-generated overview of the day's most relevant articles
- **Theme reports** — multi-signal topic detection across sources
- **Personalized recommendations** — learns from your reading behavior
- **Keyboard-driven UI** — Vim-style navigation
- **Local-first** — runs fully offline with Ollama, or connect to OpenAI-compatible APIs
- **Your data stays yours** — reading habits and articles are stored locally

---

## Illuminatus! Terminology

fuckupRSS uses terms from the Illuminatus! Trilogy throughout:

| Term    | Meaning  |
|---------|----------|
| **Fnord** | Modified article (with revisions) |
| **Concealed** / **Illuminated** | Unread / Read |
| **Golden Apple** | Favorite |
| **Pentacle** | Feed source |
| **Sephiroth** | Category |
| **Immanentize** | Keyword / tag |
| **Greyface Alert** | Bias warning |
| **Discordian Analysis** | AI summary |

---

## Screenshots

### Article Detail with AI Analysis

![fuckupRSS main view — article with AI analysis, bias detection, and categories](docs/screenshots/fuckuprss-artikel-detail.png)

### Daily Briefing

![fuckupRSS Briefing — AI-generated daily overview](docs/screenshots/fuckuprss-briefing.png)

---

## Tech Stack

| Component  | Technology   |
|------------|--------------|
| Framework | [Tauri](https://tauri.app/) 2.x |
| Backend | Rust |
| Frontend | Svelte 5 |
| Database | SQLite + sqlite-vec |
| AI | [Ollama](https://ollama.com/) + OpenAI-compatible APIs |
| Models | ministral-3 (text), snowflake-arctic-embed2 (embeddings) |
| Icons | Font Awesome Free 6.7.2 |

---

## Getting Started

### Requirements

- **Ollama** — for local AI models
- **GPU** — NVIDIA with 12 GB VRAM (recommended) or Apple Silicon
- **RAM** — 16 GB minimum, 32 GB recommended
- **OS** — Linux (Ubuntu 22.04+, Fedora 38+, Arch) or macOS 13+ (Apple Silicon)

### Setup

```bash
# 1. Install Ollama (https://ollama.com)
# Linux:
curl -fsSL https://ollama.com/install.sh | sh
# macOS:
brew install ollama

# 2. Pull models
ollama pull ministral-3:latest
ollama pull snowflake-arctic-embed2:latest

# 3. Clone and build
git clone https://github.com/yourusername/fuckuprss.git
cd fuckuprss
npm install
cargo tauri dev    # Development
cargo tauri build  # Production build
```

### Quick Start

1. Launch fuckupRSS
2. Add feeds (import OPML or enter URLs)
3. Wait for the first sync
4. Articles are automatically analyzed and categorized

---

## Project Structure

```text
fuckupRSS/
├── src/              # Svelte 5 frontend
├── src-tauri/        # Rust backend
├── docs/             # Technical documentation
├── scripts/          # Build and utility scripts
└── static/           # Static assets (Font Awesome)
```

See [CLAUDE.md](CLAUDE.md) for detailed project structure, conventions, and architecture notes.

---

## Contributing

This is a hobby project, but contributions and ideas are welcome. The codebase has tests and linting set up:

```bash
npm run test                                     # Frontend (Vitest)
cargo test --manifest-path src-tauri/Cargo.toml  # Backend (Rust)
npm run lint                                     # ESLint
npm run rust:clippy                              # Clippy
```

See [docs/](docs/) for architecture documentation and [CLAUDE.md](CLAUDE.md) for development guidelines.

---

## Why Local-First?

- **Privacy** — your reading habits and data stay on your machine
- **Offline** — works without internet when using Ollama (after first sync)
- **Control** — you decide which models run and where
- **Optional cloud** — connect to OpenAI-compatible APIs if you prefer, but it's never required

---

## License

MIT License — see [LICENSE](LICENSE)

## Acknowledgments

- Robert Shea and Robert Anton Wilson for the [Illuminatus! Trilogy](https://en.wikipedia.org/wiki/The_Illuminatus!_Trilogy). fuckupRSS is an independent project — a tribute, not a derivative work.
- The [Ollama](https://ollama.com/) team for making local LLMs accessible
- The [Tauri](https://tauri.app/) team for the framework

---

```text
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

> *"Think for yourself. Question everything. Read the Fnords."*
