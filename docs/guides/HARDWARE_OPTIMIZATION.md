# Hardware-Optimierung

> Konfigurationsrichtlinien fuer Ollama in fuckupRSS.

## Context-Length-Optimierung (2026-01-10)

Das ministral-3:latest Modell hat `num_ctx=32768` als Default. Durch Reduzierung auf 4096 Tokens wird der VRAM-Verbrauch drastisch gesenkt, ohne die Qualitaet zu beeintraechtigen.

**Begruendung:** Content wird ohnehin auf 6000 Zeichen gekuerzt (ca. 1500 Tokens), daher ist 4K Context ausreichend.

**Status:** Implementiert in `src-tauri/src/ollama/mod.rs` mit `num_ctx: 4096`

---

## Parallelisierung: Nicht erforderlich

**Benchmark-Ergebnis (2026-01-28):** Fuer Batch-Artikel-Analysen (lange Generierungsaufgaben) bringt `OLLAMA_NUM_PARALLEL` **keinen Geschwindigkeitsvorteil**.

| Konfiguration | 10 Artikel | Ergebnis |
|---------------|------------|----------|
| OLLAMA_NUM_PARALLEL=1 | 105.3s | Baseline |
| OLLAMA_NUM_PARALLEL=4 | 106.3s | Kein Vorteil |

**Grund:** Bei langen Generierungsaufgaben ist die GPU bereits voll ausgelastet. Parallele Anfragen werden intern sequentiell abgearbeitet.

**Empfehlung:** Ollama-Defaults verwenden. Keine spezielle Konfiguration erforderlich.

**UI-Aenderung (2026-01-28):** Die Hardware-Profile wurden vereinfacht. Statt drei Profilen (default/moderate/high) gibt es nur noch ein Profil "Standard (Empfohlen)". Die irreführenden Parallelisierungsoptionen wurden entfernt.

> Vollstaendiger Benchmark-Report: [`docs/reports/BENCHMARK_DISCORDIAN_PARALLEL_2026-01-28.md`](../reports/BENCHMARK_DISCORDIAN_PARALLEL_2026-01-28.md)

---

## Hardware-Anforderungen

| Komponente | Minimum | Empfohlen |
|------------|---------|-----------|
| VRAM | 8 GB | 12+ GB |
| RAM | 16 GB | 32 GB |
| GPU | - | NVIDIA RTX 3060+ oder Apple Silicon |

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

| GPU | Modell | num_ctx | Erwartete Leistung |
|-----|--------|---------|-------------------|
| **12 GB** | ministral-3:latest | 4096 | ~1.5s/Artikel |
| 16+ GB | ministral-3:latest | 4096 | ~1.5s/Artikel |
| 8 GB | ministral-3:3b | 4096 | ~1s/Artikel, evtl. Qualitaetseinbussen |

---

## Ollama-Konfiguration (optional)

Die Ollama-Defaults sind fuer fuckupRSS optimal. Die folgenden Einstellungen sind **optional** und nur bei speziellen Anforderungen noetig.

### Linux (systemd)

```bash
sudo systemctl edit ollama.service
```

```ini
[Service]
# Optional: Beide Modelle (LLM + Embedding) gleichzeitig laden
Environment="OLLAMA_MAX_LOADED_MODELS=2"

# Optional: Modelle laenger im VRAM halten
Environment="OLLAMA_KEEP_ALIVE=24h"
```

```bash
sudo systemctl daemon-reload
sudo systemctl restart ollama
```

### macOS (launchctl)

```bash
launchctl setenv OLLAMA_MAX_LOADED_MODELS 2
launchctl setenv OLLAMA_KEEP_ALIVE 24h
```

---

## Referenzen

- **Implementierung:** `src-tauri/src/ollama/mod.rs`
- **Benchmark-Report:** `docs/reports/BENCHMARK_DISCORDIAN_PARALLEL_2026-01-28.md`
