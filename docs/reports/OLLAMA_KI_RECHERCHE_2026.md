# Recherche: Ollama & Lokale KI-Modelle fuer News-Verarbeitung

**Datum:** 2026-03-10
**Kontext:** fuckupRSS Phase 4+ Feature-Planung
**Hardware:** MacBook Pro M4 Pro, 48 GB RAM

---

## 1. Ollama API Features (Stand Maerz 2026)

### 1.1 Structured Output / JSON Mode

Ollama unterstuetzt seit Ende 2024 vollstaendige Structured Outputs via JSON Schema:

- **`format`-Parameter:** Akzeptiert entweder `"json"` (freies JSON) oder ein vollstaendiges JSON-Schema-Objekt
- **Pydantic/Zod-Integration:** Schema kann direkt aus Pydantic-Models (`model_json_schema()`) oder Zod-Schemas generiert werden
- **Deterministische Ausgabe:** Durch `temperature: 0` erhaelt man reproduzierbare, maschinenlesbare Outputs
- **Vision-Model-Support:** Structured Outputs funktionieren auch mit Vision-Modellen (z.B. LLaVA, Gemma3)
- **Empfehlungen:**
  - "return as JSON" zusaetzlich in den Prompt aufnehmen
  - Temperatur niedrig halten fuer zuverlaessigere Ergebnisse

**Relevanz fuer fuckupRSS:** Die aktuelle Pipeline nutzt freie Textgenerierung und parst das Ergebnis. Mit Structured Outputs koennten Zusammenfassungen, Kategorien, Keywords und Bias-Werte als typisiertes JSON-Schema erzwungen werden -- weniger Parsing-Fehler, zuverlaessigere Pipeline.

### 1.2 Tool/Function Calling

Ollama unterstuetzt Tool Calling ueber das `tools`-Feld in der API:

- **Unterstuetzte Modelle:** Qwen3, Llama 3.1/3.2/4, Mistral, GLM-4, DeepSeek-R1
- **Streaming:** Tool Calls koennen gestreamt werden (partielle Argumente)
- **Community-Empfehlung 2026:** Mindestens 14B-32B Modelle fuer zuverlaessiges Tool Calling, 32B+ fuer produktive Anwendungen
- **Best Performers:** Qwen3 (32B) und Llama 3.1 8B-Instruct werden am haeufigsten empfohlen

**Relevanz fuer fuckupRSS:** Ermoeglichen agentic workflows -- z.B. koennte das Modell selbst entscheiden, ob ein Artikel gecrosscheckt, eine Quelle verifiziert oder zusaetzliche Informationen abgerufen werden muessen.

### 1.3 Vision/Multimodal

Verfuegbare Vision-Modelle in Ollama:

| Modell | Parameter | Staerken |
|--------|-----------|----------|
| LLaVA 1.6 | 7B/13B | Visual QA, OCR, Bildbeschreibung |
| Llama 3.2 Vision | 11B/90B | Multimodale Analyse, Dokumentenverstehen |
| Gemma 3 | 4B/12B/27B | Vision + Text, effizient auf Apple Silicon |
| Qwen2.5-VL | 3B/7B/72B | Starke OCR, multilingual |
| Moondream | 2B | Extrem klein, Edge-Devices |
| MiniCPM-V | 3B/8B | Effizient, gut fuer mobile/eingebettete Systeme |

**Relevanz fuer fuckupRSS:** Bilder in Artikeln koennten analysiert werden -- Beschreibungen generieren, manipulierte Bilder erkennen, Infografiken auswerten, Alt-Text fuer Barrierefreiheit erzeugen.

### 1.4 Embedding-Modelle

| Modell | Dimensionen | Max. Kontext | Besonderheit |
|--------|-------------|--------------|--------------|
| snowflake-arctic-embed2 (aktuell in fuckupRSS) | 1024 | 512 Tokens | Multilingual (74 Sprachen), Apache 2.0 |
| nomic-embed-text | 1024 | 8.192 Tokens | Groesserer Kontext, Task-Prefixes |
| nomic-embed-text-v2-moe | 768 | 8.192 Tokens | MoE-Architektur, effizient |
| mxbai-embed-large | 1024 | 512 Tokens | BERT-large basiert, gute Qualitaet |
| snowflake-arctic-embed-l-v2.0 | 1024 | 8.192 Tokens | Neueste Version, multilingual verbessert |

**Relevanz fuer fuckupRSS:** Der aktuelle snowflake-arctic-embed2 ist solide. Ein Upgrade auf nomic-embed-text oder snowflake-arctic-embed-l-v2.0 wuerde den Kontext von 512 auf 8.192 Tokens erhoehen -- laengere Artikelabschnitte koennten in einem Embedding erfasst werden.

### 1.5 Batch-Processing / Parallelitaet

- **OLLAMA_NUM_PARALLEL:** Standard 4 (oder 1 bei wenig RAM), steuert parallele Requests pro Modell
- **OLLAMA_MAX_QUEUE:** Standard 512 Requests in der Warteschlange
- **OLLAMA_MAX_LOADED_MODELS:** Standard 3x GPU-Anzahl (oder 3 fuer CPU)
- **Performance-Realitaet:**
  - Single User: ~45ms TTFR (Time to First Response)
  - 10 Concurrent: Antwortzeiten steigen von 2s auf 45s+
  - Ollama ist fuer Single-/Low-Concurrency optimiert, nicht fuer Hochlast
- **vLLM-Vergleich:** Bei 50 Usern erreicht vLLM ~920 tok/s vs. Ollama ~155 tok/s
- **Empfehlung:** Fuer Batch-Verarbeitung von Artikeln: sequentiell oder niedrige Parallelitaet (2-4) nutzen

### 1.6 Context Window Groessen aktueller Modelle

| Modell | Default Kontext | Max. Kontext | Anmerkung |
|--------|-----------------|--------------|-----------|
| Qwen3 (0.8B-235B) | 4.096 | 131.072 (mit YaRN) | Bestes Preis-Leistungs-Verhaeltnis |
| Gemma 3 (4B-27B) | 2.048 (Ollama) | 131.072 | Ollama-Default niedrig! |
| Llama 4 Scout | 10M nativ | 10.000.000 | MoE, enorm grosser Kontext |
| Llama 3.3 (70B) | 8.192 | 128.000 | Community-Builds |
| Mistral Small 3.1 | 32.768 | 128.000 | Sehr guter Allrounder |
| Phi-4 (14B) | 4.096 | 16.384 | Edge-optimiert |
| SmolLM3 (3B) | 2.048 | 8.192 | 6 Sprachen, klein und effizient |
| Ministral (3B) | 4.096 | 32.768 | Aktuell in fuckupRSS |

**Wichtig:** Ollama setzt den Default-Kontext oft auf 2.048-4.096 Tokens. Fuer laengere Artikel muss `num_ctx` explizit erhoeht werden (Modelfile oder API-Parameter).

### 1.7 Web Search (Neu!)

Ollama bietet seit kurzem eine **Web Search Capability**:
- Modelle koennen aktuelle Informationen aus dem Web abrufen
- Reduziert Halluzinationen durch aktuelle Datenquellen
- Erfordert `OLLAMA_API_KEY`

**Relevanz fuer fuckupRSS:** Koennte fuer Fact-Checking genutzt werden -- Behauptungen in Artikeln gegen aktuelle Web-Quellen pruefen.

### 1.8 Thinking Mode

Ollama unterstuetzt einen **Thinking Mode** fuer erweitertes Reasoning:
- Chain-of-Thought Ausgabe vor der eigentlichen Antwort
- Nuetzlich fuer komplexe Analyse-Aufgaben (Bias-Erkennung, Argumentationsanalyse)

---

## 2. Modelle fuer News-Verarbeitung

### 2.1 Named Entity Recognition (NER)

| Ansatz | Modell | Groesse | Vorteil |
|--------|--------|---------|---------|
| **Spezialisiert** | GLiNER / GLiNER2 | 205M | Extrem schnell, CPU-tauglich, zero-shot, deterministische Offsets/Scores |
| **Spezialisiert** | NuNER | 125M | Nur 125M Parameter, few-shot NER, data-efficient |
| **General LLM** | Qwen3 (7B/14B) | 7-14B | Generativ, flexibel, multilingual |
| **General LLM** | Gemma3 (27B) | 27B | F1 ~41% zero-shot auf medizinischen NER-Tasks |
| **Fine-tuned** | Finance-Llama-8B | 8B | Spezialisiert auf NER + Sentiment im Finanzbereich |

**Empfehlung fuer fuckupRSS:** GLiNER2 als dediziertes NER-Modell (205M, CPU-tauglich, liefert Offsets und Confidence-Scores) in Kombination mit dem bestehenden LLM fuer kontextuelle Anreicherung. GLiNER laeuft nicht direkt in Ollama, sondern als eigenes Python-Package -- koennte als Microservice integriert werden.

### 2.2 Sentiment Analysis / Bias-Erkennung

| Modell | Groesse | Staerke |
|--------|---------|---------|
| Mistral 7B | 7B | 94% Genauigkeit bei Sentiment (Film-Reviews), bester lokaler Performer |
| Qwen3 (7B/14B) | 7-14B | Multilingual, starkes Reasoning |
| Falcon3 | 7B/10B | Gut dokumentierte Sentiment-Pipeline mit RSS-Feeds |
| Gemma 2 (9B) | 9B | sentiment_analysis_with_reasoning-Variante verfuegbar |

**Relevanz fuer fuckupRSS:** Die bestehende Greyface Alert Pipeline (political_bias) koennte durch Structured Outputs deutlich zuverlaessiger werden. Statt freier Text-Analyse ein typisiertes JSON-Schema mit definierten Bias-Kategorien.

### 2.3 Topic Modeling / Klassifikation

- **Qwen3 (7B):** Bestes Preis-Leistungs-Verhaeltnis, multilingual, 128K Kontext
- **Mistral Small 3.1:** 128K Kontext, guter Allrounder fuer Klassifikation
- **Phi-4 (14B):** Stark bei Reasoning-lastigen Klassifikationsaufgaben
- **GLiNER2:** Auch fuer Text Classification geeignet (neben NER), 205M Parameter

### 2.4 Zusammenfassungen in verschiedenen Sprachen

| Modell | Sprachen | Besonderheit |
|--------|----------|--------------|
| Qwen3 | 100+ Sprachen | Bester multilingualer Support unter kleinen Modellen |
| SmolLM3 (3B) | 6 Sprachen (inkl. DE) | Nur 3B, aber starke Multilingual-Performance |
| Gemma 3 | 140+ Sprachen | Google-Qualitaet, gute Deutsche Sprachunterstuetzung |
| Mistral Small 3.1 | Primaer EN/FR/DE | 128K Kontext, effizient |

### 2.5 Cross-lingual Analysis

- Qwen3 und Gemma 3 koennen Artikel in verschiedenen Sprachen vergleichen und Gemeinsamkeiten erkennen
- Embedding-Modelle wie snowflake-arctic-embed2 (74 Sprachen) ermoeglichen sprachuebergreifende Aehnlichkeitssuche
- Anwendung: Deutsche und englische Artikel zum selben Thema automatisch verknuepfen

---

## 3. Innovative Anwendungen fuer fuckupRSS

### 3.1 Fake News / Desinformations-Erkennung

**Stand der Forschung (2025/2026):**
- **STEEL-Framework:** Aggregiert Web-Quellen zu Behauptungen, LLM bewertet: true/false/insufficient info
- **ARG-Netzwerk:** Kombiniert kleine und grosse Modelle -- SLM filtert Rationale, LLM trifft Entscheidung
- **HEMT-Fake:** Multimodal (Text + Bild + Relationen) mit hierarchischer Erklaerbarkeit
- **Multi-Agent Fact-Checking (LoCal):** Zerlegungsagent + Reasoning-Agenten + Bewertungsagent

**Machbar fuer fuckupRSS:**
- Cross-Reference zwischen Artikeln verschiedener Quellen zum selben Thema
- Erkennung widerspruechtlicher Behauptungen
- Confidence-Score basierend auf Quellen-Diversitaet
- "Claim Detection" in Artikeln mit anschliessender Verifikation

### 3.2 Duplikat-/Aehnlichkeitserkennung

**Bereits implementiert in fuckupRSS:** Embedding-basierte Aehnlichkeitssuche (sqlite-vec, cosine similarity)

**Verbesserungsmoeglichkeiten:**
- Semantisches Clustering statt nur paarweiser Vergleich
- Erkennung von "Spin-Varianten" (gleiche Story, unterschiedliche Perspektive)
- Hierarchische Gruppierung: identisch > gleiche Story > verwandtes Thema
- LLM-gestuetzte Verifizierung der Cluster ("Handeln diese Artikel wirklich vom selben Event?")

### 3.3 Automatische Timeline-Erstellung

**Forschungsansatz:** LLM-Enhanced Social Event Detection:
1. LLM fasst Artikel zusammen
2. Zusammenfassungen werden mit Zeitstempeln vektorisiert
3. Clustering erkennt zusammengehoerige Events
4. Chronologische Sortierung ergibt Timeline

**Umsetzung fuer fuckupRSS:**
- Artikel zu Stories gruppieren (Embedding-Clustering)
- Zeitliche Entwicklung einer Story verfolgen
- Automatische "Story-Zeitleiste" im UI

### 3.4 Personalisierte Empfehlungen

**Aktuelle Forschung (PURE-Framework):**
- **Review Extractor:** Praeferenzen aus Interaktionen extrahieren
- **Profile Updater:** Profil kontinuierlich verfeinern
- **Recommender:** Empfehlungen basierend auf Profil generieren

**Bereits in fuckupRSS:** Empfehlungssystem mit Scoring-Weights (31 Konstanten in `recommendations.rs`)

**Erweiterungsmoeglichkeiten:**
- LLM-basiertes User-Profil aus Lesehistorie ableiten (statt nur Embedding-Aehnlichkeit)
- "Warum wird mir das empfohlen?"-Erklaerungen generieren
- Diversitaets-Score: bewusst Artikel aus anderen Perspektiven vorschlagen

### 3.5 Automatische Briefings/Digests

- Taegliche/woechentliche Zusammenfassungen der wichtigsten Themen
- Personalisiert nach Nutzer-Interessen
- Strukturiert nach Kategorien (Sephiroth in fuckupRSS)
- Koennte als eigener Tauri-Command implementiert werden

### 3.6 Argumentationsanalyse / Perspektiven-Vergleich

**Stand der Forschung:**
- Argument Mining mit LLMs: Erkennung von Claims, Premises, Rebuttals
- Perspektiven-Erkennung: Verschiedene Standpunkte zum selben Thema identifizieren
- Bekanntes Problem: LLMs haben inhaarente ideologische Verzerrungen in den Trainingsdaten

**Umsetzung fuer fuckupRSS:**
- Pro/Contra-Extraktion aus Artikeln zum selben Thema
- "Perspektiven-Karte": Welche Quellen berichten wie ueber ein Thema?
- Erweiterung der Greyface Alert Pipeline

### 3.7 Geo-Tagging

- NER fuer Ortsnennung in Artikeln (Laender, Staedte, Regionen)
- Kartendarstellung: "Wo passiert was?"
- Kombination mit Timeline: Raum-zeitliche Nachrichtenkarte

---

## 4. Technische Moeglichkeiten

### 4.1 RAG (Retrieval Augmented Generation) mit lokalen Modellen

**Aktuelle Best Practices:**
- **LangGraph + Ollama:** Produktionsreife RAG-Systeme mit Zustandsmanagement
- **ChromaDB/sqlite-vec + Ollama:** Lokale Vektordatenbank + LLM
- **Qwen3 (7B):** Empfohlen fuer RAG mit Tool Calling
- **Vorteile lokal:** Keine Rate Limits, deterministische Performance, volle Datenkontrolle

**Relevanz fuer fuckupRSS:** sqlite-vec ist bereits integriert. RAG koennte genutzt werden, um bei der Analyse eines neuen Artikels aehnliche bereits analysierte Artikel als Kontext einzubeziehen -- bessere Kategorisierung, konsistentere Keywords.

### 4.2 Agentic Workflows mit Ollama

**Frameworks:**
- **LangGraph:** Zustandsbasierte Multi-Step-Workflows, beste Kontrolle
- **Langflow:** Low-Code, visuelle Pipeline-Erstellung
- **Eigene Implementation:** Ollama Tool Calling API direkt nutzen

**Beispiel-Workflow fuer Artikel-Analyse:**
1. Agent erhaelt neuen Artikel
2. Entscheidet: Brauche ich Volltext? -> Tool: Volltext abrufen
3. Entscheidet: Ist der Artikel aehnlich zu bestehenden? -> Tool: Embedding-Suche
4. Analysiert: Zusammenfassung, Keywords, Bias
5. Entscheidet: Widerspricht dieser Artikel einem anderen? -> Tool: Vergleichsanalyse
6. Speichert Ergebnis

### 4.3 Fine-Tuning Moeglichkeiten

**Pipeline (2025/2026 Standard):**
1. **Training:** LoRA/QLoRA mit Unsloth (2x schneller, 70% weniger VRAM)
2. **Konvertierung:** LoRA-Adapter -> GGUF via `convert_lora_to_gguf.py` (llama.cpp)
3. **Deployment:** Ollama Modelfile mit `ADAPTER`-Instruktion
4. **Kosten:** Kostenlos auf Google Colab/Kaggle moeglich, oder lokal ab ~3 GB VRAM

**Relevanz fuer fuckupRSS:**
- Fine-Tuning von ministral auf fuckupRSS-spezifische Kategorien (Sephiroth)
- Training auf bestehende Artikel-Analyse-Daten fuer konsistentere Ergebnisse
- Spezialisiertes NER-Modell fuer Nachrichten-Entitaeten

### 4.4 Multimodale Analyse (Bilder in Artikeln)

**Verfuegbare Modelle (lokal auf M4 Pro):**
- **Gemma 3 (12B):** Vision + Text, effizient auf Apple Silicon
- **Llama 3.2 Vision (11B):** Meta's multimodale Variante
- **Qwen2.5-VL (7B):** Starke OCR, multilingual
- **Moondream (2B):** Extrem klein fuer schnelle Bild-Beschreibungen

**Anwendungen:**
- Artikelbilder beschreiben und in die Analyse einbeziehen
- Infografiken/Charts in Artikeln auswerten
- Thumbnail-Qualitaet bewerten
- Alt-Text fuer Barrierefreiheit generieren

---

## 5. Empfehlungen fuer fuckupRSS Roadmap

### Quick Wins (geringer Aufwand, hoher Nutzen)

| Feature | Aufwand | Nutzen | Abhaengigkeit |
|---------|---------|--------|---------------|
| Structured Outputs fuer Pipeline | Mittel | Hoch -- weniger Parsing-Fehler | Ollama format-Parameter |
| Embedding-Upgrade (nomic-embed-text) | Gering | Mittel -- 16x groesserer Kontext | Nur Modell-Wechsel + Re-Embedding |
| Automatische Briefings/Digests | Mittel | Hoch -- neues User-Feature | Bestehende Pipeline |
| num_ctx erhoehen fuer laengere Artikel | Gering | Mittel | Modelfile-Anpassung |

### Mittelfristig (Phase 5+)

| Feature | Aufwand | Nutzen |
|---------|---------|--------|
| Story-Clustering (Duplikat-Gruppen) | Hoch | Sehr hoch -- Kernfeature fuer News-Aggregator |
| Perspektiven-Vergleich pro Story | Hoch | Hoch -- Unique Selling Point |
| NER mit GLiNER2 Microservice | Mittel | Hoch -- strukturierte Entitaeten |
| Geo-Tagging ueber NER | Mittel | Mittel -- visuelle Nachrichtenkarte |
| Modell-Upgrade auf Qwen3 (7B) | Gering | Hoch -- besseres Reasoning, multilingual |

### Langfristig / Experimentell

| Feature | Aufwand | Nutzen |
|---------|---------|--------|
| Agentic Workflow fuer Artikel-Analyse | Sehr hoch | Transformativ |
| Multimodale Analyse (Bilder) | Hoch | Mittel |
| Fine-Tuning auf fuckupRSS-Daten | Hoch | Mittel |
| Fake-News-Detection Pipeline | Sehr hoch | Hoch |
| Story-Timeline-Erstellung | Hoch | Hoch |

---

## 6. Quellen

### Ollama Dokumentation
- [Ollama Structured Outputs Docs](https://docs.ollama.com/capabilities/structured-outputs)
- [Ollama Tool Calling Docs](https://docs.ollama.com/capabilities/tool-calling)
- [Ollama Context Length Docs](https://docs.ollama.com/context-length)
- [Ollama API Reference (GitHub)](https://github.com/ollama/ollama/blob/main/docs/api.md)
- [Ollama Blog: Structured Outputs](https://ollama.com/blog/structured-outputs)
- [Ollama Blog: Vision Models](https://ollama.com/blog/vision-models)
- [Ollama Blog: Streaming Tool Calling](https://ollama.com/blog/streaming-tool)
- [Ollama Embedding Models](https://ollama.com/search?c=embedding)

### Modell-Vergleiche
- [Best Ollama Models 2025 Performance Guide](https://collabnix.com/best-ollama-models-in-2025-complete-performance-comparison/)
- [Small Language Models 2026: Phi-4, Gemma 3, Qwen 3 Guide](https://localaimaster.com/blog/small-language-models-guide-2026)
- [Best Local LLMs for Offline Use 2026](https://iproyal.com/blog/best-local-llms/)
- [13 Best Embedding Models 2026](https://elephas.app/blog/best-embedding-models)
- [Ollama vs vLLM Performance Benchmark 2026](https://www.sitepoint.com/ollama-vs-vllm-performance-benchmark-2026/)
- [Snowflake Arctic Embed 2.0 Multilingual](https://www.snowflake.com/en/engineering-blog/snowflake-arctic-embed-2-multilingual/)

### NER / Spezialisierte Modelle
- [GLiNER: Generalist Model for NER (GitHub)](https://github.com/urchade/GLiNER)
- [GLiNER2: Unified Schema-Based Information Extraction (GitHub)](https://github.com/fastino-ai/GLiNER2)
- [NuNER: Entity Recognition Encoder (arXiv)](https://arxiv.org/html/2402.15343v1)
- [Case Study: Local LLM-Based NER with n8n and Ollama](https://drezil.de/Writing/ner4all-case-study.html)

### Fake News / Desinformation
- [LLMs in Fake News Detection Survey (MDPI)](https://www.mdpi.com/1999-5903/16/8/298)
- [LLM-for-Misinformation-Research (GitHub)](https://github.com/ICTMCG/LLM-for-misinformation-research)
- [HEMT-Fake: Multimodal Fake News Detection (Frontiers)](https://www.frontiersin.org/journals/artificial-intelligence/articles/10.3389/frai.2025.1690616/full)
- [LLMs Meet Misinformation Initiative](https://llm-misinformation.github.io/)

### Argument Mining / Fact-Checking
- [LLMs in Argument Mining Survey (arXiv)](https://arxiv.org/abs/2506.16383)
- [LoCal: Logical and Causal Fact-Checking (ACM)](https://dl.acm.org/doi/abs/10.1145/3696410.3714748)
- [FIRE: Fact-checking with Iterative Retrieval (NAACL)](https://aclanthology.org/2025.findings-naacl.158.pdf)

### News-Empfehlungen
- [Survey on LLM-based News Recommender Systems (arXiv)](https://arxiv.org/html/2502.09797v1)
- [PURE: LLM-based User Profile Management (arXiv)](https://arxiv.org/html/2502.14541)
- [LLM-Assisted News Discovery (arXiv)](https://arxiv.org/html/2509.25491v1)

### RAG / Agentic Workflows
- [Agentic RAG with LangGraph and Ollama (GitHub)](https://github.com/laxmimerit/Agentic-RAG-with-LangGraph-and-Ollama)
- [Build Local AI with Qwen 3 and Ollama (freeCodeCamp)](https://www.freecodecamp.org/news/build-a-local-ai/)
- [Using Ollama with Agents (Langflow)](https://www.langflow.org/blog/local-ai-using-ollama-with-agents)

### Fine-Tuning
- [Serve Fine-tuned LLMs with Ollama (Union.ai)](https://www.union.ai/blog-post/serve-fine-tuned-llms-with-ollama)
- [Fine-Tune SLMs for Free: Colab to Ollama (DZone)](https://dzone.com/articles/fine-tune-lms-for-free)
- [LoRA to GGUF for Ollama Pipeline (GitHub Gist)](https://gist.github.com/raoulbia-ai/f4a01596e080ef3fbb67c15d5e572d85)
- [How Ollama Handles Parallel Requests](https://www.glukhov.org/post/2025/05/how-ollama-handles-parallel-requests/)
