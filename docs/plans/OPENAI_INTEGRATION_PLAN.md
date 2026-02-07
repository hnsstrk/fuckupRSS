# OpenAI Integration - Analyse & Implementierungsplan

**Datum:** 2026-02-07
**Status:** Implementiert (2026-02-07)

---

## 1. Zusammenfassung der Analyse

Ein Team aus 5 spezialisierten Agenten hat die aktuelle OpenAI-Integration analysiert:
- **code-analyst:** Provider-Abstraktion und Code-Trennung
- **bug-analyst:** Root-Cause-Analyse des kritischen Bugs
- **api-researcher:** OpenAI Best Practices und Modellvergleich
- **prompt-analyst:** Prompt-Struktur und Response-Parsing
- **settings-analyst:** Konfiguration und Settings

---

## 2. Kritische Bugs (Datenbank-Evidenz)

Die Datenbank zeigt **130 fehlgeschlagene Artikel** mit folgenden Fehlern:

| Error | Anzahl | Root Cause |
|-------|--------|------------|
| `model 'gpt-5-nano' not found` (404) | **57** | Default-Modell existiert nicht |
| `model 'ministral-3:latest' not found` | **48** | Ollama-Modellname wird an OpenAI gesendet |
| `JSON parse error: EOF` | **24** | `max_completion_tokens: 1024` zu niedrig, Response abgeschnitten |
| `Ollama not available` | **1** | Ollama nicht gestartet |

### Root Causes:

**Bug 1 - Falsches Default-Modell:**
- `helpers.rs:69,96` verwenden `"gpt-5-nano"` als Default
- `settings.rs:116` verwendet `"gpt-4.1-nano"` als Default
- Frontend (`SettingsOllama.svelte:13`) verwendet `"gpt-5-nano"` als Default
- **Keines dieser Modelle ist garantiert verfuegbar!**

**Bug 2 - Ollama-Modellname an OpenAI:**
- Das Frontend sendet immer den Ollama-Modellnamen als `model`-Parameter
- Die Model-Override-Logik (`provider_name == "Ollama"`) ignoriert den Frontend-Parameter nur fuer OpenAI
- In manchen Code-Pfaden wird das Modell aber VOR der Provider-Pruefung gesetzt

**Bug 3 - Response-Truncation:**
- OpenAI: `max_completion_tokens: 1024` fuer JSON-Mode
- Ollama: `num_predict: 4096` (4x mehr)
- Discordian-Analyse braucht ~500-800 Tokens fuer vollstaendiges JSON
- Deutsche Zusammenfassungen sind token-intensiver → 1024 reicht oft nicht
- `finish_reason: "length"` wird komplett ignoriert

---

## 3. Identifizierte Probleme (priorisiert)

### KRITISCH (verhindert Funktionalitaet)

| # | Problem | Dateien |
|---|---------|---------|
| K1 | Inkonsistente Default-Modellnamen (`gpt-5-nano` vs `gpt-4.1-nano`) | `helpers.rs:69,96`, `settings.rs:116`, `SettingsOllama.svelte:13` |
| K2 | `max_completion_tokens: 1024` zu niedrig fuer JSON-Mode | `openai_provider.rs:135-136` |
| K3 | `finish_reason: "length"` wird ignoriert → stille Truncation | `openai_provider.rs:218-222` |
| K4 | Analyse-Typen (`DiscordianAnalysis` etc.) im `ollama`-Modul definiert | `ollama/mod.rs`, `commands/ai/types.rs`, `helpers.rs` |

### WICHTIG (Funktionalitaet eingeschraenkt)

| # | Problem | Dateien |
|---|---------|---------|
| W1 | Default-Prompts enthalten `/no_think` + doppelte Stripping-Logik | `ollama/mod.rs:220-280`, `helpers.rs:1264-1268` (4 Stellen) |
| W2 | Model-Override-Logik 6x dupliziert mit fragilem String-Vergleich | `article_processor.rs` (4x), `batch_processor.rs` (2x) |
| W3 | `check_cost_limit()` ist Dead Code - Kostenlimit nie enforced | `model_management.rs:485-514` |
| W4 | Cost-Logging nie aufgerufen trotz vorhandener Funktion | `model_management.rs:465-481` |
| W5 | Retry-Logik erstellt direkt `OllamaTextProvider`, bypassed Factory | `batch_processor.rs:936-941, 1506-1511` |
| W6 | Generische System-Message bei OpenAI (kein Aufgabenkontext) | `openai_provider.rs:112-118` |
| W7 | Temperature nicht konfigurierbar (hardcoded `None`) | `openai_provider.rs:140` |

### NIEDRIG (Verbesserungen)

| # | Problem | Dateien |
|---|---------|---------|
| N1 | Keine Eingabevalidierung fuer API-Key, URL, Modellname | `settings.rs` |
| N2 | Modell-Presets im Frontend nur `gpt-5-nano`/`gpt-5-mini` | `SettingsOllama.svelte:17-20` |
| N3 | Pipeline-Doku (`AI_PROCESSING_PIPELINE.md`) veraltet | `docs/architecture/` |
| N4 | `ProviderConfig` laedt immer alle Provider-Settings | `ai_provider/mod.rs:42-57` |

---

## 4. Modellempfehlung

### Default: `gpt-5-nano`

| Kriterium | Bewertung |
|-----------|-----------|
| Preis | $0.05/$0.40 pro 1M Tokens (Cached: $0.005) |
| Kosten/Artikel | ~$0.0003 |
| Kosten/1000 Artikel | ~$0.30 |
| Reasoning | Average |
| Structured Outputs | Ja |
| JSON-Zuverlaessigkeit | Sehr gut (`json_object` + Structured Outputs) |
| Kontext-Fenster | 400K Tokens |
| Max Output | 128K Tokens |
| Geschwindigkeit | Sehr schnell |
| Beschreibung (OpenAI) | "Great for summarization and classification tasks" |

### Quality-Alternative: `gpt-5-mini`

| Kriterium | Bewertung |
|-----------|-----------|
| Preis | $0.25/$2.00 pro 1M Tokens (Cached: $0.025) |
| Kosten/Artikel | ~$0.0015 |
| Kosten/1000 Artikel | ~$1.50 |
| Reasoning | High |
| Qualitaet | Besser bei Bias-Erkennung und nuancierten Aufgaben |

### NICHT empfohlen:
- **gpt-4o/gpt-4.1:** Zu teuer (5-20x) fuer RSS-Batch-Verarbeitung
- **o4-mini:** Zu teuer, Reasoning-Overhead unnoetig fuer Textanalyse

### Frontend-Presets (implementiert):
```typescript
const openaiModelPresets = [
    { value: "gpt-5-nano", label: "GPT-5 nano", price: "$0.05/$0.40 per 1M tokens" },
    { value: "gpt-5-mini", label: "GPT-5 mini", price: "$0.25/$2.00 per 1M tokens" },
    { value: "gpt-4.1-mini", label: "GPT-4.1 mini", price: "$0.40/$1.60 per 1M tokens" },
    { value: "gpt-4.1-nano", label: "GPT-4.1 nano", price: "$0.10/$0.40 per 1M tokens" },
];
```

---

## 5. Implementierungsplan

### Phase 1: Kritische Bug-Fixes (sofort)

#### 1.1 Default-Modell vereinheitlichen
**Dateien:** `helpers.rs`, `settings.rs`, `SettingsOllama.svelte`, `ai_provider/mod.rs`
- Alle Defaults auf `"gpt-4.1-mini"` aendern
- Eine einzige Konstante `DEFAULT_OPENAI_MODEL` definieren (z.B. in `ai_provider/mod.rs`)
- Frontend-Presets aktualisieren

#### 1.2 `max_completion_tokens` erhoehen
**Datei:** `openai_provider.rs:135-136`
- JSON-Mode: Von 1024 auf 4096 erhoehen (konsistent mit Ollama `num_predict`)
- Non-JSON-Mode: 4096 beibehalten

#### 1.3 `finish_reason` pruefen
**Datei:** `openai_provider.rs:218-240`
- `finish_reason: "length"` erkennen und als spezifischen Error zurueckgeben
- Neuer Error-Typ: `AiProviderError::ResponseTruncated`
- Bei Truncation: Automatischer Retry mit erhoehtem Token-Limit (optional)

#### 1.4 Fehlgeschlagene Artikel zuruecksetzen
**Einmalige DB-Migration:**
```sql
UPDATE fnords SET analysis_attempts = 0, analysis_error = NULL, analysis_hopeless = 0
WHERE analysis_error LIKE '%gpt-5-nano%'
   OR analysis_error LIKE '%ministral-3:latest%not found%'
   OR analysis_error LIKE '%JSON parse error: EOF%';
```

### Phase 2: Saubere Provider-Trennung

#### 2.1 Analyse-Typen extrahieren
**Von:** `ollama/mod.rs`
**Nach:** `ai_provider/types.rs` (neues Modul)
- `DiscordianAnalysis`, `DiscordianAnalysisWithRejections`, `RawDiscordianAnalysisWithRejections`
- `BiasAnalysis`, `RawBiasAnalysis`
- Flexible Deserializer (`flexible_string`, `flexible_string_vec`, etc.)
- Imports in allen betroffenen Dateien aktualisieren

#### 2.2 Prompts von `/no_think` bereinigen
**Datei:** `ollama/mod.rs:220-280`
- `/no_think` aus allen Default-Prompts entfernen
- `OllamaTextProvider.generate_text()` fuegt `/no_think` bereits automatisch hinzu
- Manuelles Stripping in `helpers.rs` (4 Stellen) entfernen

#### 2.3 Model-Override-Logik zentralisieren
**Neue Funktion:** `resolve_effective_model()` in `ai_provider/mod.rs`
```rust
pub fn resolve_effective_model(
    provider: &dyn AiTextProvider,
    frontend_model: &str,
    config_model: &str,
) -> String {
    // Nur fuer Ollama: Frontend-Override erlauben
    if provider.provider_name() == "Ollama" && !frontend_model.is_empty() {
        frontend_model.to_string()
    } else {
        config_model.to_string()
    }
}
```
- 6 duplizierte Stellen durch Aufruf dieser Funktion ersetzen

#### 2.4 System-Message verbessern (OpenAI)
**Datei:** `openai_provider.rs:112-118`
- Aufgaben-spezifische System-Message statt generischer
- JSON-Schema im System-Prompt einbetten
```
"You are a professional news analyst. Analyze articles for content, bias, and categorization.
Always respond with valid JSON matching the exact schema specified in the user message."
```

### Phase 3: Cost-Tracking aktivieren

#### 3.1 Cost-Logging integrieren
**Dateien:** `helpers.rs`, `batch_processor.rs`
- Nach jedem erfolgreichen `generate_text()`-Call: `log_ai_cost()` aufrufen
- `GenerationResult` Token-Counts durch die Pipeline reichen
- `discordian_analysis_via_provider()` muss Token-Counts zurueckgeben

#### 3.2 Cost-Limit enforcing
**Datei:** `model_management.rs:485-514`
- `#[allow(dead_code)]` entfernen
- Vor jedem API-Call: `check_cost_limit()` aufrufen
- Bei Ueberschreitung: `AiProviderError::CostLimitReached` zurueckgeben

### Phase 4: Retry-Logik verbessern

#### 4.1 Provider-agnostische Retry-Logik
**Datei:** `batch_processor.rs:936-941, 1506-1511`
- Direkte `OllamaTextProvider::new()` Instanziierung entfernen
- Stattdessen Factory `create_provider()` verwenden
- Exponential Backoff mit Jitter implementieren (1s → 2s → 4s → 8s)
- Max 5 Versuche

#### 4.2 HTTP-Status-Code-basiertes Retry
- 429 (Rate Limit): Retry mit Backoff
- 500/502/503 (Server Error): Retry mit Backoff
- 400 (Bad Request): KEIN Retry
- 401/403 (Auth Error): KEIN Retry

### Phase 5: Frontend & Settings

#### 5.1 Modell-Presets aktualisieren
**Datei:** `SettingsOllama.svelte:17-20`
- `gpt-4.1-mini` als empfohlenes Default
- `gpt-4.1-nano` als Budget-Option
- `gpt-4o-mini` als Alternative

#### 5.2 Temperature-Setting exponieren (optional)
**Dateien:** `SettingsOllama.svelte`, `settings.rs`, `openai_provider.rs`
- Slider 0.0-1.0, Default 0.3 fuer JSON-Analyse
- Nur fuer OpenAI-Provider sichtbar

### Phase 6: Dokumentation

#### 6.1 AI_PROCESSING_PIPELINE.md aktualisieren
- Prompt-Design Sektion mit aktuellen Prompts aktualisieren
- JSON-Schema korrigieren
- OpenAI-spezifische Hinweise hinzufuegen

#### 6.2 CLAUDE.md aktualisieren
- Neue Default-Modelle dokumentieren
- Provider-Trennung beschreiben

---

## 6. Geschaetzte Auswirkungen

### Kosten (bei 1000 Artikeln/Monat):
- **gpt-4.1-mini (Default):** ~$1.60/Monat
- **gpt-4.1-nano (Budget):** ~$0.40/Monat
- Mit Prompt Caching (90% Rabatt auf gecachte Inputs): 30-50% weniger

### Performance:
- Erhoehtes `max_completion_tokens` (1024 → 4096): Kein Geschwindigkeitsverlust, aber zuverlaessigere Ergebnisse
- Parallele Verarbeitung (bis zu 50 fuer OpenAI): Schnellere Batch-Verarbeitung

### Zuverlaessigkeit:
- Korrektes Default-Modell: 57 Artikel (404-Fehler) sofort behebbar
- Erhoehte Token-Limits: 24 Artikel (JSON-Truncation) behebbar
- Besseres Error-Handling: Klarere Fehlermeldungen fuer Benutzer
