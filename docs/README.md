# fuckupRSS Documentation

Technical documentation for fuckupRSS — the RSS aggregator with local AI integration.

---

## Overview

| Document | Description |
|----------|-------------|
| [README.md](../README.md) | Project overview, features, installation |
| [CLAUDE.md](../CLAUDE.md) | Developer context and conventions |

---

## Architecture

| Document | Description |
|----------|-------------|
| [AI_PROCESSING_PIPELINE.md](architecture/AI_PROCESSING_PIPELINE.md) | AI processing pipeline: retrieval, analysis, bias detection, prompt design |
| [DATABASE_SCHEMA.md](architecture/DATABASE_SCHEMA.md) | SQLite schema: tables, revision management, settings, embeddings |

---

## API Reference

| Document | Description |
|----------|-------------|
| [TAURI_COMMANDS_REFERENCE.md](api/TAURI_COMMANDS_REFERENCE.md) | All Tauri commands with parameters and return values |

---

## Guides

| Document | Description |
|----------|-------------|
| [TESTING.md](guides/TESTING.md) | Test strategy, commands, and patterns |
| [QUALITY_CHECKLIST.md](guides/QUALITY_CHECKLIST.md) | Frontend-backend communication checklist |

---

## Features

| Document | Description |
|----------|-------------|
| [KEYBOARD_SHORTCUTS.md](features/ui/KEYBOARD_SHORTCUTS.md) | Vim-style keyboard shortcuts |

---

## SQL Scripts

| Document | Description |
|----------|-------------|
| [cleanup-orphans.sql](sql/cleanup-orphans.sql) | Clean up orphaned records |
| [migration-22-performance.sql](sql/migration-22-performance.sql) | Performance optimization migration |

---

## Directory Structure

```
docs/
├── README.md                        # This file
├── api/
│   └── TAURI_COMMANDS_REFERENCE.md  # Frontend → Backend API
├── architecture/
│   ├── AI_PROCESSING_PIPELINE.md    # AI pipeline design
│   └── DATABASE_SCHEMA.md          # Database schema
├── features/
│   └── ui/
│       └── KEYBOARD_SHORTCUTS.md    # Keyboard shortcuts
├── guides/
│   ├── QUALITY_CHECKLIST.md         # Code quality patterns
│   └── TESTING.md                   # Test guide
└── sql/                             # Maintenance scripts
    ├── cleanup-orphans.sql
    └── migration-22-performance.sql
```
