# Keyboard-Shortcuts

fuckupRSS bietet Vim-ähnliche Tastenkombinationen für eine effiziente Navigation ohne Maus.

## Navigation

| Taste | Aktion |
|-------|--------|
| `j` | Nächster Artikel |
| `k` | Vorheriger Artikel |
| `g g` | Zum Anfang der Liste |
| `G` | Zum Ende der Liste |
| `h` | Vorheriger Feed/Ordner |
| `l` | Nächster Feed/Ordner |

## Aktionen

| Taste | Aktion |
|-------|--------|
| `o` / `Enter` | Artikel öffnen/schließen |
| `v` | Im Browser öffnen |
| `r` | Als gelesen markieren |
| `u` | Als ungelesen markieren |
| `s` | Golden Apple (Favorit) toggle |
| `a` | Alle als gelesen markieren |

## Suche und Filter

| Taste | Aktion |
|-------|--------|
| `/` | Suche öffnen |
| `Esc` | Suche/Dialog schließen |
| `f` | Filter-Panel toggle |

## System

| Taste | Aktion |
|-------|--------|
| `Ctrl+R` | Feeds synchronisieren |
| `Ctrl+,` | Einstellungen öffnen |
| `?` | Shortcut-Hilfe anzeigen |

## Relevante Quelldateien

- `src/App.svelte` - Globale Keyboard-Event-Handler
- `src/lib/components/ArticleList.svelte` - Artikel-Navigation
- `src/lib/stores/state.svelte.ts` - Keyboard-State Management
