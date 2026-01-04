# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

fuckupRSS is an RSS aggregator/reader with local AI integration, named after F.U.C.K.U.P. from the Illuminatus! trilogy. It uses Ollama for local AI processing with no cloud dependencies.

**Status:** Pre-development (specification complete, no code yet)

## Technology Stack

- **Framework:** Tauri 2.x (Rust backend + Svelte frontend)
- **Database:** SQLite + SQLite-VSS (vector search)
- **AI Backend:** Ollama (local) with qwen3-vl:8b and nomic-embed-text models
- **Target Platforms:** Linux (primary), macOS (secondary)

## Build Commands

```bash
# Install dependencies
npm install

# Development mode
cargo tauri dev

# Production build
cargo tauri build
```

## Key Rust Crates

| Purpose | Crate |
|---------|-------|
| RSS/Atom Parsing | `feed-rs` |
| HTTP Client | `reqwest` |
| Async Runtime | `tokio` |
| SQLite | `rusqlite` |
| Vector Search | `sqlite-vss` |
| HTML Parsing | `scraper` |
| Readability | `readability` |
| Ollama API | `ollama-rs` |
| OPML Parsing | `opml` |

## Architecture

```
┌─────────────────────────────────────────┐
│         Tauri Frontend (Svelte)         │
└─────────────────────┬───────────────────┘
                      │ Tauri Commands (IPC)
                      ▼
┌─────────────────────────────────────────┐
│           Rust Backend                   │
│  ┌───────────┐ ┌──────────┐ ┌─────────┐ │
│  │feed-rs    │ │ollama-rs │ │readabil.│ │
│  └───────────┘ └──────────┘ └─────────┘ │
│  ┌─────────────────────────────────────┐│
│  │       SQLite + SQLite-VSS           ││
│  └─────────────────────────────────────┘│
└─────────────────────────────────────────┘
```

## Illuminatus! Terminology

The codebase uses terms from the Illuminatus! trilogy:

| Code Term | Meaning |
|-----------|---------|
| Fnord | Unread article |
| Illuminated | Read article |
| Golden Apple | Favorited article |
| Pentacle | Feed source |
| Sephiroth | Category |
| Immanentize | Keyword/tag |
| Greyface Alert | Bias warning |
| Discordian Analysis | AI summary |
| Operation Mindfuck | User interests profile |
| Hagbard's Retrieval | Full-text fetching |

## Database Schema Key Tables

- `pentacles` - Feed sources
- `fnords` - Articles
- `sephiroth` - Categories
- `immanentize` - Keywords/tags
- `fnords_vss` - Vector embeddings for similarity search
- `operation_mindfuck` - User interest preferences

## AI Processing Pipeline

1. **Hagbard's Retrieval** - Fetch full text for truncated feeds
2. **Immanentizing** - Generate embeddings via nomic-embed-text
3. **Discordian Analysis** - Summarize, categorize, extract keywords via qwen3-vl:8b
4. **Greyface Alert** - Bias detection (political_bias: -2 to +2, sachlichkeit: 0-4)

## Ollama Setup

```bash
# Install models
ollama pull qwen3-vl:8b
ollama pull nomic-embed-text

# Configure for dual model loading (Linux)
sudo systemctl edit ollama.service
# Add:
# [Service]
# Environment="OLLAMA_MAX_LOADED_MODELS=2"
# Environment="OLLAMA_FLASH_ATTENTION=1"
```

## Data Paths

- **Linux:** `~/.local/share/fuckupRSS/`
- **macOS:** `~/Library/Application Support/fuckupRSS/`
