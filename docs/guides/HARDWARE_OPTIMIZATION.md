# Hardware-Optimierung

> Entscheidungsdokumentation und Konfigurationsrichtlinien fuer die KI-Parallelisierung in fuckupRSS.

## Entscheidung (2026-01-10)

### Problem

Mit `ministral-3:latest` (8.9B Parameter) und `OLLAMA_NUM_PARALLEL=4` werden ca. 9.3 GB VRAM belegt. Das laesst keinen Platz fuer das zweite Modell `snowflake-arctic-embed2`, das fuer Embedding-Generierung benoetigt wird.

### Loesung: Context-Length-Optimierung

Das ministral-3:latest Modell hat `num_ctx=32768` als Default. Durch Reduzierung auf 4096 Tokens wird der VRAM-Verbrauch drastisch gesenkt, ohne die Qualitaet zu beeintraechtigen.

**Begruendung:** Content wird ohnehin auf 6000 Zeichen gekuerzt (ca. 1500 Tokens), daher ist 4K Context ausreichend.

**Status:** Implementiert in `src-tauri/src/ollama/mod.rs` mit `num_ctx: 4096`

---

## Hardware-Profile

| Profil | VRAM | ai_parallelism | Beschreibung |
|--------|------|----------------|--------------|
| Standard | 8 GB | 1 | Sicher fuer alle Systeme |
| Moderat | 12 GB | 4 | RTX 3060/3070, M1/M2 |
| Hohe Leistung | 16+ GB | 8 | RTX 3080 Ti, M1 Pro/Max, M4 |

---

## Benchmark-Ergebnisse

### Getestete Modelle

| Modell | Parameter | Disk | Quantization |
|--------|-----------|------|--------------|
| ministral-3:3b | 3.8B | 3.0 GB | Q4_K_M |
| ministral-3:latest | 8.9B | 6.0 GB | Q4_K_M |
| qwen3-vl:8b | 8.8B | 6.1 GB | Q4_K_M |

### Context-Length vs. Performance

| num_ctx | VRAM | GPU% | Zeit (warm) |
|---------|------|------|-------------|
| 32768 (Default) | 9.5 GB | 100% | ~22s |
| 8192 | 11 GB | 84% | ~6.5s |
| **4096** | **9.5 GB** | **100%** | **~1.5s** |

### Qualitaetsvergleich

| Modell | JSON-Zuverlaessigkeit | Summary-Qualitaet | Gesamt |
|--------|----------------------|-------------------|--------|
| ministral-3:3b | 2/3 | Gut | 3/5 |
| ministral-3:latest | 3/3 | Sehr gut | 5/5 |
| qwen3-vl:8b | 3/3 | Gut | 4/5 |

### Empfehlung nach Hardware

| GPU | Modell | num_ctx | NUM_PARALLEL | Erwartete Leistung |
|-----|--------|---------|--------------|-------------------|
| **12 GB** | ministral-3:latest | 4096 | 2-4 | ~1.5s/Artikel, Platz fuer Embedding-Modell |
| 16+ GB | ministral-3:latest | 4096 | 4-8 | ~1.5s/Artikel, sehr hoher Durchsatz |
| 8 GB | ministral-3:3b | 4096 | 2-4 | ~1s/Artikel, evtl. Qualitaetseinbussen |

---

## Ollama-Konfiguration

### Linux (systemd)

```bash
# Override-Datei erstellen
sudo systemctl edit ollama.service
```

Inhalt hinzufuegen:

```ini
[Service]
Environment="OLLAMA_MAX_LOADED_MODELS=2"
Environment="OLLAMA_FLASH_ATTENTION=1"
Environment="OLLAMA_NUM_PARALLEL=4"
```

Dann neu starten:

```bash
sudo systemctl daemon-reload
sudo systemctl restart ollama
```

### macOS (Terminal)

Vor dem Start von Ollama:

```bash
export OLLAMA_NUM_PARALLEL=4
export OLLAMA_FLASH_ATTENTION=1
export OLLAMA_MAX_LOADED_MODELS=2
ollama serve
```

### macOS (launchctl - permanent)

```bash
launchctl setenv OLLAMA_NUM_PARALLEL 4
launchctl setenv OLLAMA_FLASH_ATTENTION 1
launchctl setenv OLLAMA_MAX_LOADED_MODELS 2
```

---

## Referenzen

- **Implementierung:** `src-tauri/src/ollama/mod.rs`
- **Settings UI:** `src/lib/components/SettingsView.svelte`
- **Hardware-Profile API:** `src-tauri/src/commands/settings.rs`
- **Originale Analyse:** `TODO.md` (Zeilen 11-56)
