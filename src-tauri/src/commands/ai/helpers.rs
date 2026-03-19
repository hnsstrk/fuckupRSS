//! Shared helper functions for AI commands

use crate::ai_provider::{
    self, AiTextProvider, EmbeddingProvider, EmbeddingProviderConfig, ProviderConfig, ProviderType,
    TaskType, DEFAULT_OPENAI_EMBEDDING_MODEL, DEFAULT_OPENAI_MODEL,
};
use crate::commands::settings::get_embedding_model_from_db;
use crate::db::Database;
use crate::ollama::{
    get_language_for_locale, BRIEFING_NUM_CTX, DEFAULT_ANALYSIS_PROMPT, DEFAULT_NUM_CTX,
    DEFAULT_SUMMARY_PROMPT, RECOMMENDED_MAIN_MODEL, RECOMMENDED_REASONING_MODEL,
};
use crate::AppState;
use crate::SEPHIROTH_CATEGORIES;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tauri::State;

use super::types::{CategoryWithSource, KeywordWithMetadata};

// ============================================================
// DATABASE SETTINGS HELPERS
// ============================================================

/// Get num_ctx setting from database, falling back to DEFAULT_NUM_CTX
pub fn get_num_ctx_setting(db: &Database) -> u32 {
    db.conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'ollama_num_ctx'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_NUM_CTX)
}

/// Get Ollama URL from database settings
pub fn get_ollama_url(db: &Database) -> String {
    db.conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'ollama_url'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "http://localhost:11434".to_string())
}

/// Get effective Ollama URL considering proxy state.
/// Returns the proxy local URL if the proxy is active, otherwise falls back to the DB setting.
pub fn get_effective_ollama_url(db: &Database, proxy: &crate::proxy::ProxyManager) -> String {
    if let Some(local_url) = proxy.get_local_url() {
        local_url
    } else {
        get_ollama_url(db)
    }
}

/// Get the effective Ollama URL, using proxy if available.
/// Helper that handles the `Option<&ProxyManager>` case.
fn resolve_ollama_url(db: &Database, proxy: Option<&crate::proxy::ProxyManager>) -> String {
    match proxy {
        Some(p) => get_effective_ollama_url(db, p),
        None => get_ollama_url(db),
    }
}

/// Get a string setting from database with a default
pub fn get_setting(db: &Database, key: &str, default: &str) -> String {
    db.conn()
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
            row.get::<_, String>(0)
        })
        .unwrap_or_else(|_| default.to_string())
}

/// Get the embedding_max_chars setting from database.
/// Returns the configured value or DEFAULT_EMBEDDING_MAX_CHARS.
pub fn get_embedding_max_chars(db: &Database) -> usize {
    use super::data_persistence::DEFAULT_EMBEDDING_MAX_CHARS;
    get_setting(
        db,
        "embedding_max_chars",
        &DEFAULT_EMBEDDING_MAX_CHARS.to_string(),
    )
    .parse()
    .unwrap_or(DEFAULT_EMBEDDING_MAX_CHARS)
}

/// Parse the openai_temperature setting from DB.
/// Returns None if the value is empty, "auto", or not a valid float.
fn parse_openai_temperature(db: &Database) -> Option<f32> {
    let val = get_setting(db, "openai_temperature", "");
    if val.is_empty() || val.eq_ignore_ascii_case("auto") {
        return None;
    }
    val.parse::<f32>().ok()
}

/// Get provider config from database settings.
/// If a ProxyManager is provided, the Ollama URL will use the proxy when active.
///
/// `task_type` steuert das Modell-Routing:
/// - `TaskType::Fast` → ollama_model, Standard num_ctx
/// - `TaskType::Reasoning` → reasoning_model, mindestens BRIEFING_NUM_CTX
pub fn get_provider_config(
    db: &Database,
    proxy: Option<&crate::proxy::ProxyManager>,
    task_type: TaskType,
) -> ProviderConfig {
    let provider_type_str = get_setting(db, "ai_text_provider", "ollama");
    let provider_type = ProviderType::from_str_setting(&provider_type_str);

    let main_model = get_setting(db, "ollama_model", RECOMMENDED_MAIN_MODEL);
    let reasoning_model = get_setting(db, "reasoning_model", RECOMMENDED_REASONING_MODEL);
    let mut num_ctx = get_num_ctx_setting(db);

    // Bei Reasoning: Reasoning-Modell verwenden und höheren num_ctx sicherstellen
    let effective_model = match task_type {
        TaskType::Fast => main_model.clone(),
        TaskType::Reasoning => {
            if num_ctx < BRIEFING_NUM_CTX {
                num_ctx = BRIEFING_NUM_CTX;
            }
            reasoning_model.clone()
        }
    };

    ProviderConfig {
        provider_type: provider_type.clone(),
        ollama_url: resolve_ollama_url(db, proxy),
        ollama_model: effective_model,
        ollama_reasoning_model: reasoning_model,
        ollama_num_ctx: num_ctx,
        ollama_concurrency: get_setting(db, "ollama_concurrency", "1")
            .parse()
            .unwrap_or(1),
        openai_base_url: get_setting(db, "openai_base_url", "https://api.openai.com"),
        openai_api_key: get_setting(db, "openai_api_key", ""),
        openai_model: get_setting(db, "openai_model", DEFAULT_OPENAI_MODEL),
        openai_temperature: parse_openai_temperature(db),
        task_type,
        claude_model: get_setting(db, "claude_model", ""),
        claude_max_budget_usd: get_setting(db, "claude_max_budget_usd", "0.0")
            .parse()
            .unwrap_or(0.0),
        cli_timeout_secs: get_setting(db, "cli_timeout_secs", "120")
            .parse()
            .unwrap_or(120),
    }
}

/// Create the configured text provider based on settings.
///
/// Returns a provider that implements AiTextProvider trait.
/// - If `ai_text_provider` is "openai_compatible", returns OpenAiCompatibleProvider
/// - Otherwise returns OllamaTextProvider (default)
///
/// `task_type` steuert das Modell-Routing (Fast vs. Reasoning).
/// If a ProxyManager is provided, the Ollama URL will use the proxy when active.
pub fn create_text_provider(
    db: &Database,
    proxy: Option<&crate::proxy::ProxyManager>,
    task_type: TaskType,
) -> (Arc<dyn AiTextProvider>, String) {
    let config = get_provider_config(db, proxy, task_type);

    let model = match config.provider_type {
        ProviderType::Ollama => config.ollama_model.clone(),
        ProviderType::OpenAiCompatible => config.openai_model.clone(),
        ProviderType::GeminiCli => "gemini-cli".to_string(),
        ProviderType::ClaudeCodeCli => {
            if config.claude_model.is_empty() {
                "claude-code-cli".to_string()
            } else {
                config.claude_model.clone()
            }
        }
    };

    (ai_provider::create_provider(&config), model)
}

/// Get embedding provider config from database settings.
/// If a ProxyManager is provided, the Ollama URL will use the proxy when active.
pub fn get_embedding_provider_config(
    db: &Database,
    proxy: Option<&crate::proxy::ProxyManager>,
) -> EmbeddingProviderConfig {
    let provider_type_str = get_setting(db, "embedding_provider", "ollama");
    let provider_type = ProviderType::from_str_setting(&provider_type_str);

    let dimensions: usize = get_setting(db, "embedding_dimensions", "1024")
        .parse()
        .unwrap_or(1024);

    EmbeddingProviderConfig {
        provider_type,
        ollama_url: resolve_ollama_url(db, proxy),
        ollama_embedding_model: get_embedding_model_from_db(db.conn()),
        openai_base_url: get_setting(db, "openai_base_url", "https://api.openai.com"),
        openai_api_key: get_setting(db, "openai_api_key", ""),
        openai_embedding_model: get_setting(
            db,
            "openai_embedding_model",
            DEFAULT_OPENAI_EMBEDDING_MODEL,
        ),
        embedding_dimensions: dimensions,
    }
}

/// Create the configured embedding provider based on settings.
/// If a ProxyManager is provided, the Ollama URL will use the proxy when active.
pub fn create_embedding_provider_from_db(
    db: &Database,
    proxy: Option<&crate::proxy::ProxyManager>,
) -> Arc<dyn EmbeddingProvider> {
    let config = get_embedding_provider_config(db, proxy);
    ai_provider::create_embedding_provider(&config)
}

/// Get locale from database settings
pub fn get_locale_from_db(state: &State<'_, AppState>) -> String {
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return "de".to_string(),
    };
    db.conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'locale'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "de".to_string())
}

// ============================================================
// PROMPT HELPERS
// ============================================================

/// Get summary prompt from database or default
pub fn get_summary_prompt(state: &State<'_, AppState>, locale: &str) -> String {
    let language = get_language_for_locale(locale);
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return DEFAULT_SUMMARY_PROMPT.replace("{language}", language),
    };

    let custom_prompt: Option<String> = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'summary_prompt'",
            [],
            |row| row.get(0),
        )
        .ok();

    match custom_prompt {
        Some(prompt) => prompt.replace("{language}", language),
        None => DEFAULT_SUMMARY_PROMPT.replace("{language}", language),
    }
}

/// Get analysis prompt from database or default
pub fn get_analysis_prompt(state: &State<'_, AppState>, locale: &str) -> String {
    let language = get_language_for_locale(locale);
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return DEFAULT_ANALYSIS_PROMPT.replace("{language}", language),
    };

    let custom_prompt: Option<String> = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'analysis_prompt'",
            [],
            |row| row.get(0),
        )
        .ok();

    match custom_prompt {
        Some(prompt) => prompt.replace("{language}", language),
        None => DEFAULT_ANALYSIS_PROMPT.replace("{language}", language),
    }
}

/// Get discordian prompt from database (returns None for default behavior)
/// The prompt is NOT pre-processed with {language} here, as that's done in the OllamaClient
pub fn get_discordian_prompt(state: &State<'_, AppState>) -> Option<String> {
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return None,
    };

    db.conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'discordian_prompt'",
            [],
            |row| row.get(0),
        )
        .ok()
}

// ============================================================
// KEYWORD/CATEGORY MERGE HELPERS
// ============================================================

/// Merge LLM keywords with local extraction, deduplicating by lowercase
pub fn merge_keywords(
    llm_keywords: &[String],
    local_keywords: Vec<String>,
    max_count: usize,
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();

    for kw in llm_keywords.iter().chain(local_keywords.iter()) {
        let normalized = kw.to_lowercase();
        if !normalized.is_empty() && normalized.len() >= 2 && seen.insert(normalized) {
            merged.push(kw.clone());
            if merged.len() >= max_count {
                break;
            }
        }
    }

    merged
}

/// Validate and merge categories from multiple sources
/// Priority: statistical > llm > local (statistical is most reliable)
pub fn validate_and_merge_categories(
    llm_categories: &[String],
    local_categories: Vec<String>,
) -> Vec<String> {
    let valid_llm: Vec<String> = llm_categories
        .iter()
        .filter(|c| {
            SEPHIROTH_CATEGORIES
                .iter()
                .any(|s| s.to_lowercase() == c.to_lowercase())
        })
        .cloned()
        .collect();

    if valid_llm.is_empty() {
        local_categories
    } else {
        let mut seen = HashSet::new();
        // LLM categories first, then local as supplement
        valid_llm
            .into_iter()
            .chain(local_categories)
            .filter(|c| seen.insert(c.to_lowercase()))
            .take(5)
            .collect()
    }
}

/// Merge categories with statistical categories as PRIMARY source
/// Statistical categories are deterministic and more reliable than LLM
pub fn merge_categories_stat_primary(
    stat_categories: &[(String, f64)],
    llm_categories: &[String],
    local_categories: Vec<String>,
    min_confidence: f64,
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    // 1. Statistical categories FIRST (if confident enough)
    for (name, confidence) in stat_categories {
        if *confidence >= min_confidence && seen.insert(name.to_lowercase()) {
            // Validate against known categories
            if SEPHIROTH_CATEGORIES
                .iter()
                .any(|s| s.to_lowercase() == name.to_lowercase())
            {
                result.push(name.clone());
            }
        }
    }

    // 2. LLM categories as supplement (validated)
    for cat in llm_categories {
        if seen.insert(cat.to_lowercase())
            && SEPHIROTH_CATEGORIES
                .iter()
                .any(|s| s.to_lowercase() == cat.to_lowercase())
        {
            result.push(cat.clone());
        }
    }

    // 3. Local categories as fallback
    for cat in local_categories {
        if seen.insert(cat.to_lowercase()) {
            result.push(cat);
        }
    }

    result.truncate(5);
    result
}

/// Statistical keyword info with type
#[allow(dead_code)] // Public API for type-aware keyword processing
pub struct StatKeywordInfo {
    pub name: String,
    pub keyword_type: String,
}

/// Determine source for each keyword by comparing with statistical suggestions
pub fn determine_keyword_sources(
    final_keywords: &[String],
    stat_keywords: &[String],
) -> Vec<KeywordWithMetadata> {
    use crate::keywords::types::KeywordSource;

    let stat_lower: HashSet<String> = stat_keywords.iter().map(|k| k.to_lowercase()).collect();

    final_keywords
        .iter()
        .map(|k| {
            let is_statistical = stat_lower.contains(&k.to_lowercase());
            KeywordWithMetadata {
                name: k.clone(),
                source: if is_statistical {
                    KeywordSource::Statistical
                } else {
                    KeywordSource::Ai
                },
                confidence: if is_statistical { 0.8 } else { 1.0 },
                keyword_type: "concept".to_string(),
            }
        })
        .collect()
}

/// Determine source for each keyword by comparing with statistical suggestions (with type info)
#[allow(dead_code)] // Public API for type-aware source determination
pub fn determine_keyword_sources_with_types(
    final_keywords: &[String],
    stat_keywords_with_types: &[StatKeywordInfo],
) -> Vec<KeywordWithMetadata> {
    use crate::keywords::types::KeywordSource;

    let stat_map: HashMap<String, &StatKeywordInfo> = stat_keywords_with_types
        .iter()
        .map(|k| (k.name.to_lowercase(), k))
        .collect();

    final_keywords
        .iter()
        .map(|k| {
            let lower = k.to_lowercase();
            if let Some(stat_info) = stat_map.get(&lower) {
                KeywordWithMetadata {
                    name: k.clone(),
                    source: KeywordSource::Statistical,
                    confidence: 0.8,
                    keyword_type: stat_info.keyword_type.clone(),
                }
            } else {
                // For AI-only keywords, try to detect type from patterns
                let keyword_type = detect_keyword_type(k);
                KeywordWithMetadata {
                    name: k.clone(),
                    source: KeywordSource::Ai,
                    confidence: 1.0,
                    keyword_type,
                }
            }
        })
        .collect()
}

/// Detect keyword type from heuristics
///
/// Priority order: acronym -> organization -> location -> person -> concept
///
/// Person detection is strict to avoid false positives. We require:
/// 1. 2-4 words, all title case
/// 2. NOT matching any exclusion patterns (common nouns, organizations, etc.)
/// 3. OR matching person title patterns (Dr., Prof., etc.)
pub fn detect_keyword_type(keyword: &str) -> String {
    // Check for acronyms (all caps, 2-6 chars)
    if keyword.len() >= 2
        && keyword.len() <= 6
        && keyword.chars().all(|c| c.is_uppercase() || c.is_numeric())
    {
        return "acronym".to_string();
    }

    let keyword_lower = keyword.to_lowercase();
    let words: Vec<&str> = keyword.split_whitespace().collect();

    // ================================================================
    // ORGANIZATION DETECTION (expanded patterns)
    // ================================================================

    // Legal entity suffixes (case-insensitive check)
    let org_suffixes = [
        "gmbh", "ag", "inc", "inc.", "corp", "corp.", "ltd", "ltd.", "co.", "co", "e.v.", "ev",
        "eg", "se", "sa", "kg", "ohg", "plc", "llc", "mbh",
    ];
    if org_suffixes
        .iter()
        .any(|suf| keyword_lower.ends_with(&format!(" {}", suf)) || keyword_lower == *suf)
    {
        return "organization".to_string();
    }

    // Organization indicators - must match as whole words or word boundaries
    // We check if the indicator appears as a distinct word component
    let org_word_indicators = [
        // German institutions (as whole words or compound endings) - including plural forms
        "verband",
        "verbände",
        "institut",
        "institute",
        "ministerium",
        "ministerien",
        "bundesamt",
        "behörde",
        "behörden",
        "bundesanstalt",
        "landesamt",
        "stiftung",
        "stiftungen",
        "verein",
        "vereine",
        "gewerkschaft",
        "gewerkschaften",
        "kammer",
        "kammern",
        "akademie",
        "akademien",
        "universität",
        "universitäten",
        "hochschule",
        "hochschulen",
        "zentrum",
        "zentren",
        "agentur",
        "agenturen",
        "anstalt",
        "anstalten",
        "gesellschaft",
        "gesellschaften",
        "genossenschaft",
        "genossenschaften",
        "bundesregierung",
        "landesregierung",
        // English institutions
        "foundation",
        "foundations",
        "institute",
        "institutes",
        "university",
        "universities",
        "college",
        "colleges",
        "council",
        "councils",
        "committee",
        "committees",
        "commission",
        "commissions",
        "agency",
        "agencies",
        "authority",
        "authorities",
        "board",
        "boards",
        "trust",
        "trusts",
        "association",
        "associations",
        "federation",
        "federations",
        "coalition",
        "coalitions",
        "alliance",
        "alliances",
        "organization",
        "organisations",
        "organisation",
        "organizations",
        "corporation",
        "corporations",
        "company",
        "companies",
        "holdings",
        "partners",
        "enterprises",
        // Political
        "partei",
        "parteien",
        "fraktion",
        "fraktionen",
        // Sports clubs/teams - specific patterns
        "vfb",
        "vfl",
        "fsv",
        "tsv",
        "rovers",
        "wanderers",
        "albion",
        "borussia",
        // Media
        "tribune",
        "gazette",
        "herald",
        "broadcasting",
        "television",
        // Tech companies
        "labs",
        "solutions",
        "technologies",
        // Military/Government
        "brigade",
        "brigaden",
        "division",
        "divisionen",
        "command",
        "corps",
        "department",
        "departments",
        "ministry",
        "ministries",
        "tribunal",
        "tribunals",
        // Other
        "airline",
        "airlines",
        "airways",
        "railway",
        "railways",
        "insurance",
        "versicherung",
        "versicherungen",
        "krankenkasse",
        "krankenkassen",
    ];

    // Check if org indicator appears as a whole word or word part
    for ind in org_word_indicators.iter() {
        // Check for whole word match
        if keyword_lower == *ind {
            return "organization".to_string();
        }
        // Check word boundaries (space before/after)
        if keyword_lower.ends_with(&format!(" {}", ind))
            || keyword_lower.starts_with(&format!("{} ", ind))
            || keyword_lower.contains(&format!(" {} ", ind))
        {
            return "organization".to_string();
        }
        // German compound word endings (e.g., "Pestel-Institut")
        // The indicator must be at least 5 chars and have a prefix
        if ind.len() >= 5 && keyword_lower.ends_with(ind) && keyword.len() > ind.len() + 2 {
            return "organization".to_string();
        }
        // Check for hyphenated compounds (e.g., "Pestel-Institut")
        if keyword_lower.contains(&format!("-{}", ind)) {
            return "organization".to_string();
        }
        // German compound words where indicator is in the middle (e.g., "Gewerkschaftsbund")
        // Only for indicators >= 8 chars to avoid false positives
        if ind.len() >= 8 && keyword_lower.contains(ind) {
            return "organization".to_string();
        }
    }

    // Additional German compound suffixes that indicate organizations
    let org_compound_suffixes = ["bund", "verband", "werk", "dienst", "wesen", "amt"];
    for suf in org_compound_suffixes.iter() {
        // Must end with this suffix and have substantial content before
        if keyword_lower.ends_with(suf) && keyword.len() > suf.len() + 5 {
            // Additional check: the part before should contain an org-related stem
            let prefix = &keyword_lower[..keyword_lower.len() - suf.len()];
            let org_stems = [
                "gewerk",
                "arbeit",
                "beamt",
                "polizei",
                "feuerwehr",
                "rettung",
                "bundes",
                "landes",
                "kranken",
                "versicher",
                "angestellt",
            ];
            if org_stems.iter().any(|stem| prefix.contains(stem)) {
                return "organization".to_string();
            }
        }
    }

    // Organization patterns with word boundaries (case-insensitive)
    let org_boundary_patterns = [
        // Sports clubs - need word boundary check
        ("fc ", true, false), // starts with
        (" fc", false, true), // ends with
        ("sc ", true, false),
        (" sc", false, true),
        ("sv ", true, false),
        (" sv", false, true),
        // Specific sports terms
        ("united", false, false), // contains
        ("athletic", false, false),
        ("palace", false, false),
        ("villa", false, false),
        ("milan", false, false),
        ("inter ", true, false),
        (" inter", false, true),
        ("real ", true, false),
        ("bayern", false, false),
        ("arsenal", false, false),
        ("chelsea", false, false),
        ("liverpool", false, false),
        ("everton", false, false),
        ("tottenham", false, false),
        // Media - only when clearly media
        (" news", false, true),
        ("news ", true, false),
        (" times", false, true),
        (" post", false, true),
        (" daily", false, true),
        ("daily ", true, false),
        // Military
        (" force", false, true),
        ("forces", false, false),
        (" guard", false, true),
        // Other
        (" bank", false, true),
        ("bank ", true, false),
        (" bahn", false, true),
        // Political parties with special handling
        ("party", false, false),
        ("bewegung", false, false),
        ("front ", true, false),
        (" front", false, true),
        // Groups
        (" group", false, true),
        ("group ", true, false),
        // Club (but not as generic word)
        (" club", false, true),
        ("club ", true, false),
        // Union (as organization)
        (" union", false, true),
        ("union ", true, false),
        // Office
        (" office", false, true),
        ("office ", true, false),
        // Court
        (" court", false, true),
        // Press/Media/Radio/TV
        (" press", false, true),
        (" radio", false, true),
        (" tv", false, true),
        ("media ", true, false),
        (" media", false, true),
        // City as sports team suffix
        (" city", false, true),
    ];

    for (pattern, is_start, is_end) in org_boundary_patterns.iter() {
        let matches = if *is_start {
            keyword_lower.starts_with(pattern)
        } else if *is_end {
            keyword_lower.ends_with(pattern)
        } else {
            keyword_lower.contains(pattern)
        };
        if matches {
            return "organization".to_string();
        }
    }

    // Specific organization patterns - longer patterns that can be matched with contains
    let org_long_patterns = [
        // Political parties
        "die grünen",
        "die linke",
        "freie wähler",
        // Specific organizations
        "federal reserve",
        "european commission",
        "world health",
        // Tech companies
        "google",
        "microsoft",
        "amazon",
        "facebook",
        "twitter",
        // NGOs
        "amnesty international",
        "greenpeace",
        "caritas",
        "diakonie",
        // Government/Security
        "bundeswehr",
        "polizei",
        "feuerwehr",
        "rettungsdienst",
    ];

    for pat in org_long_patterns.iter() {
        if keyword_lower == *pat || keyword_lower.contains(pat) {
            return "organization".to_string();
        }
    }

    // Short abbreviations/acronyms that need word boundary matching
    // These are often embedded in other words, so we need strict matching
    let org_abbrev_patterns = [
        // Political parties
        "cdu", "csu", "spd", "fdp", "afd", // International organizations
        "eu", "un", "uno", "who", "nato", "unicef", "unesco", "wto", "imf", // Media
        "bbc", "cnn", "nbc", "abc", "ard", "zdf", "rtl", "fox", "sat.1", // Sports
        "fifa", "uefa", "ioc", "nfl", "nba", "mlb", "nhl", "pga", "atp", "wta",
        // Tech (short names)
        "meta", // NGOs (short names)
        "amnesty", "oxfam",
    ];

    for pat in org_abbrev_patterns.iter() {
        // Must be exact match, or with word boundaries (space, hyphen)
        if keyword_lower == *pat
            || keyword_lower.starts_with(&format!("{} ", pat))
            || keyword_lower.ends_with(&format!(" {}", pat))
            || keyword_lower.contains(&format!(" {} ", pat))
            || keyword_lower.starts_with(&format!("{}-", pat))
            || keyword_lower.ends_with(&format!("-{}", pat))
            || keyword_lower.contains(&format!("-{}-", pat))
        {
            return "organization".to_string();
        }
    }

    // ================================================================
    // LOCATION DETECTION (expanded patterns)
    // ================================================================

    // Location suffixes - only for single-word keywords to avoid false positives
    // Multi-word phrases are handled by loc_indicators and known_locations
    let loc_suffixes = [
        "land", "reich", "istan", "abad", "burg", "berg", "dorf", "heim", "hausen", "stadt",
        "furt", "haven", "hafen", "see", "tal", "wald", "field", "town", "ville", "port", "bridge",
        "shire", "ford",
    ];

    // Only apply suffix matching to single words (no spaces)
    if !keyword.contains(' ') {
        for suf in loc_suffixes.iter() {
            if keyword_lower.ends_with(suf) && keyword.len() > suf.len() + 2 {
                let prefix = &keyword[..keyword.len() - suf.len()];
                if !prefix.is_empty() && prefix.chars().last().is_some_and(|c| c.is_alphabetic()) {
                    // Avoid false positives like "Homeland"
                    let non_location_with_suffix = [
                        "homeland",
                        "fatherland",
                        "motherland",
                        "wasteland",
                        "dreamland",
                    ];
                    if !non_location_with_suffix.iter().any(|w| keyword_lower == *w) {
                        return "location".to_string();
                    }
                }
            }
        }
    }

    // Country suffix "ien" - only for specific patterns (countries typically end with consonant + "ien")
    // e.g., "Spanien", "Italien", "Rumänien" but NOT "O'Brien", "alien"
    if !keyword.contains(' ') && keyword_lower.ends_with("ien") && keyword.len() >= 6 {
        let prefix = &keyword_lower[..keyword_lower.len() - 3];
        // Check if it looks like a country name (ends with n, l, r, t, k before "ien")
        let country_like_chars = ['n', 'l', 'r', 't', 'k', 'd', 's', 'b', 'm'];
        if prefix
            .chars()
            .last()
            .is_some_and(|c| country_like_chars.contains(&c))
        {
            // Additional check: not a person's name pattern (no apostrophe, no common surname endings)
            if !keyword.contains('\'') && !keyword_lower.ends_with("brien") {
                return "location".to_string();
            }
        }
    }

    // Location indicators (contains)
    let loc_indicators = [
        "stadt ",
        " stadt",
        "kreis",
        "bezirk",
        "region ",
        " region",
        "provinz",
        "bundesstaat",
        "bundesland",
        "kanton",
        "distrikt",
        "county",
        "state of",
        "republic of",
        "kingdom of",
        "emirate",
        "airport",
        "flughafen",
        "hafen",
        "bahnhof",
        "station",
        "straße",
        "strasse",
        "platz",
        "allee",
        "avenue",
        "street",
        "road",
        "square",
        "plaza",
    ];
    if loc_indicators.iter().any(|ind| keyword_lower.contains(ind)) {
        return "location".to_string();
    }

    // Known countries and major cities
    let known_locations = [
        // Countries (German names)
        "deutschland",
        "österreich",
        "schweiz",
        "frankreich",
        "italien",
        "spanien",
        "portugal",
        "griechenland",
        "türkei",
        "russland",
        "ukraine",
        "polen",
        "tschechien",
        "ungarn",
        "rumänien",
        "bulgarien",
        "kroatien",
        "serbien",
        "niederlande",
        "belgien",
        "luxemburg",
        "dänemark",
        "schweden",
        "norwegen",
        "finnland",
        "estland",
        "lettland",
        "litauen",
        "irland",
        "großbritannien",
        "england",
        "schottland",
        "wales",
        "nordirland",
        "china",
        "japan",
        "indien",
        "pakistan",
        "iran",
        "irak",
        "syrien",
        "israel",
        "ägypten",
        "marokko",
        "südafrika",
        "nigeria",
        "kenia",
        "brasilien",
        "argentinien",
        "mexiko",
        "kanada",
        "australien",
        "neuseeland",
        // Countries (English names)
        "germany",
        "austria",
        "switzerland",
        "france",
        "italy",
        "spain",
        "portugal",
        "greece",
        "turkey",
        "russia",
        "ukraine",
        "poland",
        "czechia",
        "hungary",
        "romania",
        "bulgaria",
        "croatia",
        "serbia",
        "netherlands",
        "belgium",
        "denmark",
        "sweden",
        "norway",
        "finland",
        "ireland",
        "britain",
        "scotland",
        "wales",
        "china",
        "japan",
        "india",
        "pakistan",
        "iran",
        "iraq",
        "syria",
        "israel",
        "egypt",
        "morocco",
        "south africa",
        "nigeria",
        "kenya",
        "brazil",
        "argentina",
        "mexico",
        "canada",
        "australia",
        "new zealand",
        "united states",
        "united kingdom",
        // Major cities
        "berlin",
        "münchen",
        "hamburg",
        "köln",
        "frankfurt",
        "stuttgart",
        "düsseldorf",
        "dortmund",
        "essen",
        "leipzig",
        "bremen",
        "dresden",
        "hannover",
        "nürnberg",
        "wien",
        "zürich",
        "genf",
        "bern",
        "paris",
        "london",
        "rom",
        "madrid",
        "barcelona",
        "amsterdam",
        "brüssel",
        "kopenhagen",
        "stockholm",
        "oslo",
        "helsinki",
        "warschau",
        "prag",
        "budapest",
        "athen",
        "istanbul",
        "moskau",
        "kiew",
        "kyjiw",
        "peking",
        "beijing",
        "shanghai",
        "tokio",
        "tokyo",
        "delhi",
        "mumbai",
        "teheran",
        "kairo",
        "kapstadt",
        "lagos",
        "new york",
        "los angeles",
        "chicago",
        "washington",
        "toronto",
        "sydney",
        "melbourne",
    ];
    if known_locations.iter().any(|loc| keyword_lower == *loc) {
        return "location".to_string();
    }

    // ================================================================
    // PERSON DETECTION (strict with exclusions)
    // ================================================================

    // First check for person title indicators (strong signal)
    let person_titles = [
        "dr.",
        "dr ",
        "prof.",
        "prof ",
        "herr ",
        "frau ",
        "mr.",
        "mr ",
        "mrs.",
        "mrs ",
        "ms.",
        "ms ",
        "sir ",
        "lord ",
        "lady ",
        "dame ",
        "graf ",
        "baron ",
        "prinz ",
        "könig ",
        "präsident ",
        "kanzler ",
        "minister ",
        "general ",
        "oberst ",
        "kapitän ",
        "direktor ",
        "chef ",
        "ceo ",
        "cfo ",
        "cto ",
    ];
    if person_titles
        .iter()
        .any(|title| keyword_lower.starts_with(title))
    {
        return "person".to_string();
    }

    // Check title case pattern for potential person names
    // Handle hyphenated names like "Jean-Pascal" and Irish/Scottish names like "O'Brien"
    let is_title_case_word = |w: &str| -> bool {
        // Handle hyphenated compound names (e.g., "Jean-Pascal")
        if w.contains('-') {
            w.split('-').all(|part| {
                let chars: Vec<char> = part.chars().collect();
                !chars.is_empty()
                    && chars[0].is_uppercase()
                    && chars.iter().skip(1).all(|c| c.is_lowercase())
            })
        }
        // Handle Irish/Scottish names with apostrophe (e.g., "O'Brien", "O'Connor", "McDonald")
        else if w.contains('\'') {
            // Split by apostrophe and check each part
            w.split('\'').all(|part| {
                if part.is_empty() {
                    return true; // Empty part after split is OK
                }
                let chars: Vec<char> = part.chars().collect();
                chars[0].is_uppercase() && chars.iter().skip(1).all(|c| c.is_lowercase())
            })
        } else {
            let chars: Vec<char> = w.chars().collect();
            !chars.is_empty()
                && chars[0].is_uppercase()
                && chars.iter().skip(1).all(|c| c.is_lowercase())
        }
    };

    if words.len() >= 2 && words.len() <= 4 {
        let all_title_case = words.iter().all(|w| is_title_case_word(w));

        if all_title_case {
            // Now apply strict exclusion rules

            // Exclusion: German articles at start (Der, Die, Das, Ein, Eine, etc.)
            let german_articles = [
                "der", "die", "das", "ein", "eine", "einem", "einen", "einer", "eines", "dem",
                "den", "am", "im", "vom", "zum", "zur", "beim", "als", "für", "mit", "aus", "bei",
                "nach", "vor", "seit", "von", "zu", "auch", "und", "oder", "aber", "wenn", "weil",
                "dass", "ob", "wie", "was", "wer", "wo", "wann", "warum", "welche", "welcher",
                "welches", "diese", "dieser", "dieses", "jede", "jeder", "jedes", "alle", "keine",
                "keiner", "keines", "meine", "mein", "sein", "seine", "ihre", "ihr", "unser",
                "unsere", "euer", "eure",
            ];
            if german_articles
                .iter()
                .any(|art| words[0].to_lowercase() == *art)
            {
                return "concept".to_string();
            }

            // Exclusion: English articles/prepositions at start
            let english_starters = [
                "the", "a", "an", "for", "from", "with", "by", "in", "on", "at", "to", "of", "is",
                "are", "was", "were", "be", "been", "being", "have", "has", "had", "do", "does",
                "did", "will", "would", "could", "should", "may", "might", "must", "shall", "can",
                "this", "that", "these", "those", "my", "your", "his", "her", "its", "our",
                "their", "some", "any", "no", "every", "all", "both", "each", "few", "more",
                "most", "other", "such", "how", "what", "when", "where", "why", "who", "which",
                "if", "then", "than", "so", "as", "like", "just", "only", "also", "very", "too",
                "not", "but", "and", "or",
            ];
            if english_starters
                .iter()
                .any(|art| words[0].to_lowercase() == *art)
            {
                return "concept".to_string();
            }

            // Exclusion: Common non-person compound words
            let non_person_words = [
                // Temporal
                "januar",
                "februar",
                "märz",
                "april",
                "mai",
                "juni",
                "juli",
                "august",
                "september",
                "oktober",
                "november",
                "dezember",
                "january",
                "february",
                "march",
                "may",
                "june",
                "july",
                "october",
                "december",
                "montag",
                "dienstag",
                "mittwoch",
                "donnerstag",
                "freitag",
                "samstag",
                "sonntag",
                "monday",
                "tuesday",
                "wednesday",
                "thursday",
                "friday",
                "saturday",
                "sunday",
                "jahr",
                "jahre",
                "monat",
                "woche",
                "tag",
                "stunde",
                "minute",
                "year",
                "years",
                "month",
                "week",
                "day",
                "hour",
                "minute",
                "weltkrieg",
                "neuzeit",
                "weltkriegs",
                // Geographic
                "norden",
                "süden",
                "osten",
                "westen",
                "north",
                "south",
                "east",
                "west",
                "central",
                "eastern",
                "western",
                "northern",
                "southern",
                // Nationalities/Adjectives (German)
                "britische",
                "britischer",
                "britisches",
                "britischen",
                "deutsche",
                "deutscher",
                "deutsches",
                "deutschen",
                "amerikanische",
                "amerikanischer",
                "amerikanisches",
                "amerikanischen",
                "französische",
                "französischer",
                "französisches",
                "französischen",
                "europäische",
                "europäischer",
                "europäisches",
                "europäischen",
                "internationale",
                "internationaler",
                "internationales",
                "internationalen",
                "nationale",
                "nationaler",
                "nationales",
                "nationalen",
                "russische",
                "russischer",
                "russisches",
                "russischen",
                "chinesische",
                "chinesischer",
                "chinesisches",
                "chinesischen",
                // Nationalities/Adjectives (English)
                "british",
                "german",
                "american",
                "french",
                "european",
                "international",
                "national",
                "russian",
                "chinese",
                "global",
                "local",
                "regional",
                // Political/Abstract
                "krieg",
                "frieden",
                "politik",
                "wirtschaft",
                "kultur",
                "gesellschaft",
                "war",
                "peace",
                "politics",
                "economy",
                "culture",
                "society",
                "reform",
                "krise",
                "crisis",
                "konflikt",
                "conflict",
                "zusammenarbeit",
                "cooperation",
                "integration",
                "einigung",
                "souveränität",
                "sovereignty",
                "sicherheit",
                "security",
                // Tech/Science
                "software",
                "hardware",
                "internet",
                "digital",
                "cyber",
                "tech",
                "science",
                "research",
                "study",
                "projekt",
                "project",
                "programm",
                "program",
                "programme",
                // Events
                "game",
                "games",
                "cup",
                "championship",
                "tournament",
                "festival",
                "conference",
                "summit",
                "meeting",
                "congress",
                "forum",
                "awards",
                "prize",
                "prix",
                "globes",
                "open",
                "classic",
                "masters",
                "slam",
                "slams",
                "tour",
                "series",
                "league",
                "leagues",
                "sentry",
                "operation",
                "exercise",
                // Sports terms
                "football",
                "fußball",
                "handball",
                "basketball",
                "volleyball",
                "tennis",
                "golf",
                "rugby",
                "cricket",
                "hockey",
                "soccer",
                "cycling",
                "swimming",
                "athletics",
                "racing",
                "team",
                "teams",
                "squad",
                "club",
                "clubs",
                "match",
                "final",
                "blacks",
                "stars",
                "giants",
                "lions",
                "eagles",
                // Media terms
                "news",
                "report",
                "interview",
                "article",
                "story",
                "video",
                "film",
                "movie",
                "show",
                "series",
                "episode",
                "season",
                "book",
                "books",
                "novel",
                "magazine",
                "journal",
                "paper",
                "funk",
                "radio",
                "tv",
                "television",
                // Common nouns
                "haus",
                "house",
                "gebäude",
                "building",
                "straße",
                "street",
                "road",
                "park",
                "garden",
                "museum",
                "theater",
                "cinema",
                "hotel",
                "hospital",
                "school",
                "church",
                "palace",
                "castle",
                "tower",
                "bridge",
                "station",
                "airport",
                "port",
                "harbor",
                "truck",
                "car",
                "vehicle",
                "fahrzeug",
                "cases",
                "cleaning",
                "health",
                "wealth",
                // Abstract concepts
                "system",
                "systems",
                "service",
                "services",
                "process",
                "method",
                "model",
                "models",
                "plan",
                "plans",
                "initiative",
                "campaign",
                "movement",
                "trend",
                "act",
                "law",
                "bill",
                "treaty",
                "agreement",
                "deal",
                "pact",
                "fashion",
                "finance",
                "business",
                "industry",
                "depression",
                "recession",
                "inflation",
                // Adjectives often in compounds
                "new",
                "neu",
                "alte",
                "old",
                "große",
                "great",
                "big",
                "small",
                "erste",
                "ersten",
                "erster",
                "erstes",
                "first",
                "zweite",
                "zweiten",
                "zweiter",
                "second",
                "dritte",
                "dritten",
                "dritter",
                "third",
                "letzte",
                "letzten",
                "letzter",
                "last",
                "nächste",
                "nächsten",
                "nächster",
                "next",
                "andere",
                "anderen",
                "anderer",
                "other",
                "black",
                "white",
                "red",
                "blue",
                "green",
                "golden",
                "silver",
                "schwarz",
                "weiß",
                "rot",
                "blau",
                "grün",
                "gold",
                "silber",
                "fast",
                "slow",
                "quick",
                "dry",
                "wet",
                "cold",
                "hot",
                "warm",
                "mental",
                "physical",
                "social",
                "political",
                "economic",
                "kalter",
                "kalte",
                "kaltes",
                "heißer",
                "heiße",
                "heißes",
                "frühe",
                "früher",
                "frühen",
                "späte",
                "später",
                "späten",
                "heilige",
                "heiliger",
                "heiligen",
                // Quantifiers and pronouns
                "mehr",
                "more",
                "weniger",
                "less",
                "viel",
                "many",
                "much",
                "alle",
                "alles",
                "allen",
                "aller",
                "all",
                "keine",
                "keiner",
                "keines",
                "keinen",
                "none",
                "einige",
                "some",
                "mehrere",
                "gute",
                "guter",
                "gutes",
                "guten",
                "million",
                "millionen",
                "milliarde",
                "milliarden",
                "billion",
                "tausend",
                "thousand",
                "hundert",
                "hundred",
                "fünf",
                "zehn",
                "zwanzig",
                "dreißig",
                "fünfzig",
            ];

            // Check if any word in the keyword matches non-person words
            let has_non_person_word = words
                .iter()
                .any(|w| non_person_words.iter().any(|npw| w.to_lowercase() == *npw));

            if has_non_person_word {
                return "concept".to_string();
            }

            // Exclusion: Patterns that indicate non-person
            let non_person_patterns = [
                // Dates and time references
                "jahr ",
                " jahr",
                "jahre",
                "monat",
                " tag",
                "tag ",
                // Financial/Legal
                "euro",
                "dollar",
                "prozent",
                "percent",
                "gesetz",
                "act ",
                "haft",
                "strafe",
                "urteil",
                "klage",
                "anklage",
                // Events (specific patterns)
                " cup",
                "cup ",
                " open",
                "open ",
                " slam",
                "slam ",
                " tour",
                "tour ",
                " prix",
                "prix ",
                " award",
                "awards",
                // Organizations patterns - only prefix patterns, not suffix
                // (suffix patterns like " sc", " fc" removed - they break names like "Olaf Scholz")
                "fc ",
                "sc ",
                "ac ",
                "united",
                "city ",
                " city",
                "palace",
                "villa",
                "milan",
                "rovers",
                "wanderers",
                "athletic",
                // News/Media
                " news",
                "news ",
                " daily",
                "daily ",
                " times",
                "times ",
                // Technology
                "linux",
                "windows",
                "android",
                "ios",
                "macos",
                "ubuntu",
                "firefox",
                "chrome",
                "safari",
                "edge",
                // Compound indicators
                "abkommen",
                "vereinbarung",
                "vertrag",
                "treaty",
                "gesundheit",
                "health",
                "sicherheit",
                "security",
                "wirtschaft",
                "economy",
                "wissenschaft",
                "science",
            ];

            if non_person_patterns
                .iter()
                .any(|pat| keyword_lower.contains(pat))
            {
                return "concept".to_string();
            }

            // Sports club suffixes - only match at end of keyword as whole word
            let sports_club_suffixes = [" fc", " sc", " ac"];
            if sports_club_suffixes
                .iter()
                .any(|suf| keyword_lower.ends_with(suf))
            {
                return "concept".to_string();
            }

            // Exclusion: Single word that could be a name but is actually common noun
            let ambiguous_single_words = [
                "china", "berlin", "paris", "london", "rom", "madrid", "wien", "tokio", "jordan",
                "georgia", "dakota", "montana", "virginia", "carolina", "phoenix", "aurora",
                "trinity", "liberty", "justice", "victory", "diamond", "crystal", "ruby", "amber",
                "jade", "pearl", "ivy", "rose", "lily", "violet", "daisy", "holly", "heather",
                "hunter", "mason", "carter", "cooper", "walker", "taylor", "cook", "baker",
                "fisher", "miller", "smith", "king", "prince",
            ];

            if words.len() == 1 && ambiguous_single_words.iter().any(|w| keyword_lower == *w) {
                // Single ambiguous word - default to concept
                return "concept".to_string();
            }

            // Additional check: if first word is a known first name pattern, more likely person
            // But we're being conservative, so we only accept clear person names at this point

            // If we passed all exclusions and have 2-4 title case words, it's likely a person
            // But let's add one more check: at least one word should be > 3 chars
            let has_substantial_word = words.iter().any(|w| w.len() > 3);
            if has_substantial_word {
                return "person".to_string();
            }
        }
    }

    "concept".to_string()
}

/// Determine source for each category by comparing with statistical suggestions
pub fn determine_category_sources(
    final_categories: &[String],
    stat_categories: &[(String, f64)],
) -> Vec<CategoryWithSource> {
    let stat_map: HashMap<String, f64> = stat_categories
        .iter()
        .map(|(name, conf)| (name.to_lowercase(), *conf))
        .collect();

    final_categories
        .iter()
        .map(|c| {
            let lower = c.to_lowercase();
            if let Some(&conf) = stat_map.get(&lower) {
                CategoryWithSource {
                    name: c.clone(),
                    source: "statistical".to_string(),
                    confidence: conf,
                }
            } else {
                CategoryWithSource {
                    name: c.clone(),
                    source: "ai".to_string(),
                    confidence: 1.0,
                }
            }
        })
        .collect()
}

// ============================================================
// CATEGORY DERIVATION FROM KEYWORD NETWORK
// ============================================================

/// Derive categories from keywords based on their historical category associations.
///
/// This function looks up keywords in the immanentize network and aggregates their
/// category associations from `immanentize_sephiroth`. Categories are weighted by:
/// - The `weight` field from `immanentize_sephiroth`
/// - Specificity: `1/number_of_categories` (keywords with fewer categories count more)
///
/// Keywords with more than 6 categories are ignored as too unspecific.
/// Only subcategories (level = 1 in sephiroth) are considered.
///
/// # Arguments
/// * `conn` - Database connection
/// * `keywords` - List of keyword names to analyze
/// * `min_score` - Minimum aggregated score for a category to be included (e.g., 0.15)
/// * `min_supporting_keywords` - Minimum number of keywords that must support a category
///
/// # Returns
/// Vector of (category_name, confidence) tuples, sorted by confidence descending
pub fn derive_categories_from_keywords(
    conn: &Connection,
    keywords: &[String],
    min_score: f64,
    min_supporting_keywords: usize,
) -> Vec<(String, f64)> {
    // Structure to track category scores and supporting keywords
    struct CategoryScore {
        score: f64,
        supporting_keywords: usize,
    }

    let mut category_scores: HashMap<String, CategoryScore> = HashMap::new();

    for keyword in keywords {
        // Look up the keyword in immanentize
        let keyword_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM immanentize WHERE LOWER(name) = LOWER(?1)",
                rusqlite::params![keyword],
                |row| row.get(0),
            )
            .ok();

        let Some(keyword_id) = keyword_id else {
            continue;
        };

        // Get category associations for this keyword (only subcategories with level = 1)
        let mut stmt = match conn.prepare(
            r#"
            SELECT s.name, iis.weight
            FROM immanentize_sephiroth iis
            JOIN sephiroth s ON s.id = iis.sephiroth_id
            WHERE iis.immanentize_id = ?1 AND s.level = 1
            "#,
        ) {
            Ok(stmt) => stmt,
            Err(_) => continue,
        };

        let categories: Vec<(String, f64)> = match stmt
            .query_map(rusqlite::params![keyword_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            }) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(_) => continue,
        };

        // Skip keywords with too many categories (unspecific)
        if categories.is_empty() || categories.len() > 6 {
            continue;
        }

        // Calculate specificity factor: keywords with fewer categories are more specific
        let specificity = 1.0 / (categories.len() as f64);

        // Add weighted scores to each category
        for (cat_name, weight) in categories {
            let weighted_score = weight * specificity;
            let entry = category_scores.entry(cat_name).or_insert(CategoryScore {
                score: 0.0,
                supporting_keywords: 0,
            });
            entry.score += weighted_score;
            entry.supporting_keywords += 1;
        }
    }

    // Filter and sort categories
    let mut results: Vec<(String, f64)> = category_scores
        .into_iter()
        .filter(|(_, cs)| {
            cs.score >= min_score && cs.supporting_keywords >= min_supporting_keywords
        })
        .map(|(name, cs)| (name, cs.score))
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    results
}

// ============================================================
// ANALYSIS CACHE (Content-Hash Based)
// ============================================================

use sha2::{Digest, Sha256};

/// Cached analysis result
#[derive(Debug, Clone)]
pub struct CachedAnalysis {
    pub summary: String,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    pub political_bias: i32,
    pub sachlichkeit: i32,
    pub article_type: Option<String>,
}

/// Compute a content hash for caching
/// Uses first 6000 chars of content (same truncation as LLM analysis)
pub fn compute_content_hash(title: &str, content: &str) -> String {
    let truncated_content: String = content.chars().take(6000).collect();
    let combined = format!("{}\n{}", title, truncated_content);
    let hash = Sha256::digest(combined.as_bytes());
    format!("{:x}", hash)
}

/// Check if we have a cached analysis for the given content hash
pub fn check_analysis_cache(conn: &Connection, content_hash: &str) -> Option<CachedAnalysis> {
    let result = conn.query_row(
        r#"SELECT summary, categories, keywords, political_bias,
                  sachlichkeit, article_type
           FROM analysis_cache WHERE content_hash = ?1"#,
        rusqlite::params![content_hash],
        |row| {
            let categories_json: String = row.get(1)?;
            let keywords_json: String = row.get(2)?;
            Ok(CachedAnalysis {
                summary: row.get(0)?,
                categories: serde_json::from_str(&categories_json).unwrap_or_default(),
                keywords: serde_json::from_str(&keywords_json).unwrap_or_default(),
                political_bias: row.get(3)?,
                sachlichkeit: row.get(4)?,
                article_type: row.get(5)?,
            })
        },
    );

    if result.is_ok() {
        // Increment hit count
        let _ = conn.execute(
            "UPDATE analysis_cache SET hit_count = hit_count + 1 WHERE content_hash = ?1",
            rusqlite::params![content_hash],
        );
    }

    result.ok()
}

/// Store analysis result in cache
pub fn store_analysis_cache(
    conn: &Connection,
    content_hash: &str,
    summary: &str,
    categories: &[String],
    keywords: &[String],
    political_bias: i32,
    sachlichkeit: i32,
    article_type: &str,
) -> Result<(), rusqlite::Error> {
    let categories_json = serde_json::to_string(categories).unwrap_or_else(|_| "[]".to_string());
    let keywords_json = serde_json::to_string(keywords).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        r#"INSERT OR REPLACE INTO analysis_cache
           (content_hash, summary, categories, keywords, political_bias,
            sachlichkeit, article_type, created_at, hit_count)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, CURRENT_TIMESTAMP, 0)"#,
        rusqlite::params![
            content_hash,
            summary,
            categories_json,
            keywords_json,
            political_bias,
            sachlichkeit,
            article_type
        ],
    )?;

    Ok(())
}

// ============================================================
// PROVIDER-AWARE TEXT GENERATION
// ============================================================

use crate::ai_provider::{AiProviderError, GenerationResult};
use crate::ollama::{
    BiasAnalysis, DiscordianAnalysis, DiscordianAnalysisWithRejections, RawBiasAnalysis,
    DEFAULT_DISCORDIAN_PROMPT, DEFAULT_DISCORDIAN_PROMPT_WITH_STATS,
};
use log::{debug, info, warn};

use super::model_management::log_ai_cost;

/// Get pricing (input_price_per_1m, output_price_per_1m) in USD for a given model name.
/// Returns conservative defaults for unknown models.
fn get_model_pricing(model: &str) -> (f64, f64) {
    match model {
        m if m.starts_with("gpt-5-nano") => (0.05, 0.40),
        m if m.starts_with("gpt-5-mini") => (0.25, 2.00),
        m if m.starts_with("gpt-5") => (1.25, 10.00),
        m if m.starts_with("gpt-4.1-nano") => (0.10, 0.40),
        m if m.starts_with("gpt-4.1-mini") => (0.40, 1.60),
        m if m.starts_with("gpt-4.1") => (2.00, 8.00),
        m if m.starts_with("gpt-4o-mini") => (0.15, 0.60),
        m if m.starts_with("gpt-4o") => (2.50, 10.00),
        _ => (0.50, 2.00), // Conservative default
    }
}

/// Token usage from an AI generation call.
/// Used to pass cost info from helper functions to callers for logging.
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

impl From<&GenerationResult> for TokenUsage {
    fn from(result: &GenerationResult) -> Self {
        TokenUsage {
            input_tokens: result.input_tokens,
            output_tokens: result.output_tokens,
        }
    }
}

/// Log the cost of an AI generation to the database.
/// Only logs if both token counts are available (i.e., provider reported them).
/// Safe to call with Ollama results (tokens will be None, nothing logged).
///
/// Callers should acquire a brief DB lock, call this, then release the lock.
pub fn log_generation_cost(
    conn: &rusqlite::Connection,
    provider_name: &str,
    model: &str,
    usage: &TokenUsage,
) {
    if let (Some(input_tokens), Some(output_tokens)) = (usage.input_tokens, usage.output_tokens) {
        let (input_price, output_price) = get_model_pricing(model);
        let cost = input_tokens as f64 * input_price / 1_000_000.0
            + output_tokens as f64 * output_price / 1_000_000.0;

        info!(
            "AI cost: {} / {} - {} in / {} out = ${:.6}",
            provider_name, model, input_tokens, output_tokens, cost
        );

        log_ai_cost(
            conn,
            provider_name,
            model,
            input_tokens,
            output_tokens,
            cost,
        );
    }
}

/// Safely truncate a string to a maximum byte length, respecting UTF-8 char boundaries.
fn truncate_str_helper(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Run Discordian analysis (summary + bias + categories + keywords) via the configured provider.
///
/// This is the provider-agnostic equivalent of OllamaClient::discordian_analysis_with_stats_custom.
/// It builds the prompt, calls the provider for text generation, and parses the JSON response.
///
/// The `/no_think` prefix is NOT added here - the OllamaTextProvider handles that automatically.
pub async fn discordian_analysis_via_provider(
    provider: &dyn AiTextProvider,
    model: &str,
    title: &str,
    content: &str,
    locale: &str,
    stat_keywords: &[String],
    stat_categories: &[(String, f64)],
    custom_prompt: Option<&str>,
) -> Result<(DiscordianAnalysisWithRejections, TokenUsage), AiProviderError> {
    debug!(
        "Starting Discordian analysis via provider for: {}",
        truncate_str_helper(title, 60)
    );
    let language = get_language_for_locale(locale);
    let truncated_content: String = content.chars().take(6000).collect();

    // Format statistical keywords
    let stat_keywords_str = if stat_keywords.is_empty() {
        "none".to_string()
    } else {
        stat_keywords.join(", ")
    };

    // Format statistical categories with confidence
    let stat_categories_str = if stat_categories.is_empty() {
        "none".to_string()
    } else {
        stat_categories
            .iter()
            .map(|(name, conf)| format!("{} ({:.0}%)", name, conf * 100.0))
            .collect::<Vec<_>>()
            .join(", ")
    };

    // Build prompt from template (without /no_think - provider handles that)
    let prompt_template = custom_prompt.unwrap_or(DEFAULT_DISCORDIAN_PROMPT_WITH_STATS);

    let prompt = prompt_template
        .replace("{language}", language)
        .replace("{title}", title)
        .replace("{content}", &truncated_content)
        .replace("{stat_keywords}", &stat_keywords_str)
        .replace("{stat_categories}", &stat_categories_str);

    // Call provider with Discordian JSON schema
    let schema = crate::ollama::discordian_schema();
    let result = provider.generate_text(model, &prompt, Some(schema)).await?;
    let usage = TokenUsage::from(&result);

    // Parse the JSON response (same parsing as OllamaClient)
    let raw: crate::ollama::RawDiscordianAnalysisWithRejections =
        serde_json::from_str(&result.text).map_err(|e| {
            warn!(
                "JSON parse error: {}. Response: {}",
                e,
                truncate_str_helper(&result.text, 300)
            );
            AiProviderError::JsonParseError {
                message: e.to_string(),
                raw_response: result.text.chars().take(500).collect(),
            }
        })?;

    let analysis: DiscordianAnalysisWithRejections = raw.into();

    debug!(
        "Analysis via provider complete: {} categories, {} keywords",
        analysis.categories.len(),
        analysis.keywords.len()
    );

    Ok((analysis, usage))
}

/// Summarize content via the configured provider.
///
/// Provider-agnostic equivalent of OllamaClient::summarize_with_prompt.
pub async fn summarize_via_provider(
    provider: &dyn AiTextProvider,
    model: &str,
    content: &str,
    prompt_template: &str,
) -> Result<(String, TokenUsage), AiProviderError> {
    let truncated_content: String = content.chars().take(8000).collect();

    let prompt = prompt_template.replace("{content}", &truncated_content);
    // Freetext: no JSON schema
    let result = provider.generate_text(model, &prompt, None).await?;
    let usage = TokenUsage::from(&result);

    Ok((result.text, usage))
}

/// Analyze article bias via the configured provider.
///
/// Provider-agnostic equivalent of OllamaClient::analyze_bias_with_prompt.
pub async fn analyze_bias_via_provider(
    provider: &dyn AiTextProvider,
    model: &str,
    title: &str,
    content: &str,
    prompt_template: &str,
) -> Result<(BiasAnalysis, TokenUsage), AiProviderError> {
    let truncated_content: String = content.chars().take(4000).collect();

    let prompt = prompt_template
        .replace("{title}", title)
        .replace("{content}", &truncated_content);

    // Call provider with Bias JSON schema
    let schema = crate::ollama::bias_schema();
    let result = provider.generate_text(model, &prompt, Some(schema)).await?;
    let usage = TokenUsage::from(&result);

    let raw: RawBiasAnalysis = serde_json::from_str(&result.text).map_err(|e| {
        warn!(
            "JSON parse error: {}. Response: {}",
            e,
            truncate_str_helper(&result.text, 300)
        );
        AiProviderError::JsonParseError {
            message: e.to_string(),
            raw_response: result.text.chars().take(500).collect(),
        }
    })?;

    Ok((raw.into(), usage))
}

/// Run simple Discordian analysis (without stats) via the configured provider.
#[allow(dead_code)]
pub async fn discordian_analysis_simple_via_provider(
    provider: &dyn AiTextProvider,
    model: &str,
    title: &str,
    content: &str,
    locale: &str,
    previous_error: Option<&str>,
) -> Result<(DiscordianAnalysis, TokenUsage), AiProviderError> {
    let language = get_language_for_locale(locale);
    let truncated_content: String = content.chars().take(6000).collect();

    let prompt = if let Some(error) = previous_error {
        format!(
            r#"Your previous response could not be parsed. Error: {}
Return ONLY valid JSON:
{{
  "political_bias": 0,
  "sachlichkeit": 2,
  "summary": "...",
  "categories": ["..."],
  "keywords": ["..."]
}}

Title: {}
Content: {}"#,
            error, title, truncated_content
        )
    } else {
        DEFAULT_DISCORDIAN_PROMPT
            .replace("{language}", language)
            .replace("{title}", title)
            .replace("{content}", &truncated_content)
    };

    let schema = crate::ollama::discordian_simple_schema();
    let result = provider.generate_text(model, &prompt, Some(schema)).await?;
    let usage = TokenUsage::from(&result);

    let raw: crate::ollama::RawDiscordianAnalysis =
        serde_json::from_str(&result.text).map_err(|e| {
            warn!(
                "JSON parse error: {}. Response: {}",
                e,
                truncate_str_helper(&result.text, 300)
            );
            AiProviderError::JsonParseError {
                message: e.to_string(),
                raw_response: result.text.chars().take(500).collect(),
            }
        })?;

    Ok((raw.into(), usage))
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // ACRONYM TESTS
    // ========================================

    #[test]
    fn test_detect_acronym_uppercase() {
        assert_eq!(detect_keyword_type("NATO"), "acronym");
        assert_eq!(detect_keyword_type("EU"), "acronym");
        assert_eq!(detect_keyword_type("USA"), "acronym");
        assert_eq!(detect_keyword_type("UN"), "acronym");
        assert_eq!(detect_keyword_type("BBC"), "acronym");
        assert_eq!(detect_keyword_type("FBI"), "acronym");
        assert_eq!(detect_keyword_type("CIA"), "acronym");
    }

    #[test]
    fn test_detect_acronym_with_numbers() {
        assert_eq!(detect_keyword_type("G7"), "acronym");
        assert_eq!(detect_keyword_type("G20"), "acronym");
        assert_eq!(detect_keyword_type("5G"), "acronym");
    }

    #[test]
    fn test_not_acronym_too_long() {
        // Too long for acronym detection
        assert_ne!(detect_keyword_type("ABCDEFGH"), "acronym");
    }

    // ========================================
    // ORGANIZATION TESTS
    // ========================================

    #[test]
    fn test_detect_organization_legal_suffix() {
        assert_eq!(detect_keyword_type("Siemens AG"), "organization");
        assert_eq!(detect_keyword_type("Google Inc"), "organization");
        assert_eq!(detect_keyword_type("HIL GmbH"), "organization");
        assert_eq!(detect_keyword_type("Microsoft Corp"), "organization");
        assert_eq!(detect_keyword_type("Shunjia Toys Co Ltd"), "organization");
    }

    #[test]
    fn test_detect_organization_german_institutions() {
        assert_eq!(
            detect_keyword_type("Bundesamt für Verfassungsschutz"),
            "organization"
        );
        assert_eq!(detect_keyword_type("Pestel-Institut"), "organization");
        assert_eq!(detect_keyword_type("Das Ministerium"), "organization");
        assert_eq!(detect_keyword_type("Lokale Behörden"), "organization");
        assert_eq!(
            detect_keyword_type("Deutscher Gewerkschaftsbund"),
            "organization"
        );
        assert_eq!(
            detect_keyword_type("Industrie- und Handelskammer"),
            "organization"
        );
    }

    #[test]
    fn test_detect_organization_english_institutions() {
        assert_eq!(detect_keyword_type("Cambridge University"), "organization");
        assert_eq!(
            detect_keyword_type("World Health Organization"),
            "organization"
        );
        assert_eq!(detect_keyword_type("European Commission"), "organization");
        assert_eq!(detect_keyword_type("Federal Reserve"), "organization");
    }

    #[test]
    fn test_detect_organization_political() {
        assert_eq!(detect_keyword_type("Conservative Party"), "organization");
        assert_eq!(detect_keyword_type("Die Grünen"), "organization");
        assert_eq!(detect_keyword_type("Die Linke"), "organization");
        assert_eq!(detect_keyword_type("Freie Wähler"), "organization");
        assert_eq!(detect_keyword_type("MAGA-Bewegung"), "organization");
    }

    #[test]
    fn test_detect_organization_sports_clubs() {
        assert_eq!(detect_keyword_type("Bayern München"), "organization");
        assert_eq!(detect_keyword_type("Manchester United"), "organization");
        assert_eq!(detect_keyword_type("Borussia Dortmund"), "organization");
        assert_eq!(detect_keyword_type("FC Augsburg"), "organization");
        assert_eq!(detect_keyword_type("Aston Villa"), "organization");
        assert_eq!(detect_keyword_type("Crystal Palace"), "organization");
        assert_eq!(detect_keyword_type("Inter Milan"), "organization");
        assert_eq!(detect_keyword_type("Chelsea"), "organization");
        assert_eq!(detect_keyword_type("Arsenal"), "organization");
    }

    #[test]
    fn test_detect_organization_media() {
        assert_eq!(detect_keyword_type("BBC News"), "organization");
        assert_eq!(detect_keyword_type("Fox News"), "organization");
        assert_eq!(detect_keyword_type("Apple Daily"), "organization");
        assert_eq!(detect_keyword_type("Deutsche Bahn"), "organization");
    }

    #[test]
    fn test_detect_organization_military() {
        assert_eq!(detect_keyword_type("Coast Guard"), "organization");
        assert_eq!(detect_keyword_type("Delta Force"), "organization");
        assert_eq!(detect_keyword_type("Allied Reaction Force"), "organization");
    }

    // ========================================
    // LOCATION TESTS
    // ========================================

    #[test]
    fn test_detect_location_countries() {
        assert_eq!(detect_keyword_type("Deutschland"), "location");
        assert_eq!(detect_keyword_type("Frankreich"), "location");
        assert_eq!(detect_keyword_type("Österreich"), "location");
        assert_eq!(detect_keyword_type("Großbritannien"), "location");
        assert_eq!(detect_keyword_type("Japan"), "location");
        assert_eq!(detect_keyword_type("China"), "location");
    }

    #[test]
    fn test_detect_location_cities() {
        assert_eq!(detect_keyword_type("Berlin"), "location");
        assert_eq!(detect_keyword_type("München"), "location");
        assert_eq!(detect_keyword_type("Hamburg"), "location");
        assert_eq!(detect_keyword_type("Paris"), "location");
        assert_eq!(detect_keyword_type("London"), "location");
        assert_eq!(detect_keyword_type("Wien"), "location");
        assert_eq!(detect_keyword_type("Zürich"), "location");
    }

    #[test]
    fn test_detect_location_with_suffix() {
        assert_eq!(detect_keyword_type("Heidelberg"), "location");
        assert_eq!(detect_keyword_type("Frankfurt"), "location");
        assert_eq!(detect_keyword_type("Nürnberg"), "location");
        assert_eq!(detect_keyword_type("Regensburg"), "location");
    }

    #[test]
    fn test_detect_location_with_indicators() {
        assert_eq!(detect_keyword_type("Landkreis München"), "location");
        assert_eq!(detect_keyword_type("Stadt Prokowsk"), "location");
        assert_eq!(detect_keyword_type("Region Saporischschja"), "location");
        assert_eq!(detect_keyword_type("Bundesstaat Minnesota"), "location");
    }

    #[test]
    fn test_detect_location_airports_stations() {
        assert_eq!(detect_keyword_type("Delhi Airport"), "location");
        assert_eq!(detect_keyword_type("Flughafen Wien"), "location");
    }

    // ========================================
    // PERSON TESTS (True Positives)
    // ========================================

    #[test]
    fn test_detect_person_with_title() {
        assert_eq!(detect_keyword_type("Dr. Susan Gilby"), "person");
        assert_eq!(detect_keyword_type("Prof. Max Mustermann"), "person");
        assert_eq!(detect_keyword_type("Sir David Attenborough"), "person");
    }

    #[test]
    fn test_detect_person_names() {
        assert_eq!(detect_keyword_type("Angela Merkel"), "person");
        assert_eq!(detect_keyword_type("Friedrich Merz"), "person");
        assert_eq!(detect_keyword_type("Emmanuel Macron"), "person");
        assert_eq!(detect_keyword_type("Donald Trump"), "person");
        assert_eq!(detect_keyword_type("Elon Musk"), "person");
        assert_eq!(detect_keyword_type("Keir Starmer"), "person");
        assert_eq!(detect_keyword_type("Markus Söder"), "person");
    }

    #[test]
    fn test_detect_person_international_names() {
        assert_eq!(detect_keyword_type("Ali Khamenei"), "person");
        assert_eq!(detect_keyword_type("Leonid Volkov"), "person");
        assert_eq!(detect_keyword_type("Kyrylo Budanov"), "person");
        assert_eq!(detect_keyword_type("Andriy Yermak"), "person");
    }

    #[test]
    fn test_detect_person_three_word_names() {
        assert_eq!(detect_keyword_type("Maria Corina Machado"), "person");
        assert_eq!(detect_keyword_type("Aung San Suu Kyi"), "person");
    }

    // ========================================
    // PERSON FALSE POSITIVE TESTS (Should NOT be person)
    // ========================================

    #[test]
    fn test_not_person_german_article_start() {
        assert_ne!(detect_keyword_type("Das Abkommen"), "person");
        assert_ne!(detect_keyword_type("Die Bundesregierung"), "person");
        assert_ne!(detect_keyword_type("Der Bundestag"), "person");
        assert_ne!(detect_keyword_type("Ein Vergleich"), "person");
        assert_ne!(detect_keyword_type("Aus Sicht"), "person");
        assert_ne!(detect_keyword_type("Am Samstag"), "person");
        assert_ne!(detect_keyword_type("Auch Bundeswehr"), "person");
        assert_ne!(detect_keyword_type("Als Präsident"), "person");
    }

    #[test]
    fn test_not_person_english_article_start() {
        assert_ne!(detect_keyword_type("The Summit"), "person");
        assert_ne!(detect_keyword_type("For Russia"), "person");
        assert_ne!(detect_keyword_type("From Fiji"), "person");
        assert_ne!(detect_keyword_type("By Saturday"), "person");
        assert_ne!(detect_keyword_type("How Jenrick"), "person");
        assert_ne!(detect_keyword_type("Is Farage"), "person");
        assert_ne!(detect_keyword_type("Like Alcaraz"), "person");
    }

    #[test]
    fn test_not_person_events_competitions() {
        assert_ne!(detect_keyword_type("Berlin Game"), "person");
        assert_ne!(detect_keyword_type("Australian Open"), "person");
        assert_ne!(detect_keyword_type("Grand Slam"), "person");
        assert_ne!(detect_keyword_type("Carabao Cup"), "person");
        assert_ne!(detect_keyword_type("Golden Globes"), "person");
        assert_ne!(detect_keyword_type("Love On Tour"), "person");
        assert_ne!(detect_keyword_type("Bahrain Masters"), "person");
    }

    #[test]
    fn test_not_person_sports_terms() {
        assert_ne!(detect_keyword_type("American Football"), "person");
        assert_ne!(detect_keyword_type("All Blacks"), "person");
        assert_ne!(detect_keyword_type("All Stars"), "person");
        assert_ne!(detect_keyword_type("British Cycling"), "person");
        assert_ne!(detect_keyword_type("Deutsches Handball"), "person");
    }

    #[test]
    fn test_not_person_organizations_clubs() {
        // These should be detected as organizations, not persons
        assert_ne!(detect_keyword_type("Bayern München"), "person");
        assert_ne!(detect_keyword_type("Manchester United"), "person");
        assert_ne!(detect_keyword_type("Crystal Palace"), "person");
        assert_ne!(detect_keyword_type("Aston Villa"), "person");
        assert_ne!(detect_keyword_type("Inter Milan"), "person");
    }

    #[test]
    fn test_not_person_political_concepts() {
        assert_ne!(detect_keyword_type("Kalter Krieg"), "person");
        assert_ne!(detect_keyword_type("Erste Einigung"), "person");
        assert_ne!(detect_keyword_type("Britische Zusammenarbeit"), "person");
        assert_ne!(detect_keyword_type("Europäische Union"), "person");
    }

    #[test]
    fn test_not_person_media_technology() {
        assert_ne!(detect_keyword_type("Linux Mint"), "person");
        assert_ne!(detect_keyword_type("Digital Services Act"), "person");
        assert_ne!(detect_keyword_type("Fake News"), "person");
        assert_ne!(detect_keyword_type("Creator Economy"), "person");
    }

    #[test]
    fn test_not_person_temporal() {
        assert_ne!(detect_keyword_type("Fünf Jahre Haft"), "person");
        assert_ne!(detect_keyword_type("Ersten Weltkrieg"), "person");
        assert_ne!(detect_keyword_type("Frühe Neuzeit"), "person");
        assert_ne!(detect_keyword_type("Heilige Jahr"), "person");
    }

    #[test]
    fn test_not_person_abstract_concepts() {
        assert_ne!(detect_keyword_type("Alles Gute"), "person");
        assert_ne!(detect_keyword_type("Große Depression"), "person");
        assert_ne!(detect_keyword_type("Blended Finance"), "person");
        assert_ne!(detect_keyword_type("Fast Fashion"), "person");
        assert_ne!(detect_keyword_type("Black Friday"), "person");
    }

    #[test]
    fn test_not_person_quantified_phrases() {
        assert_ne!(detect_keyword_type("Mehr Bundeswehr"), "person");
        assert_ne!(detect_keyword_type("Mehr Sicherheit"), "person");
        assert_ne!(detect_keyword_type("Millionen Menschen"), "person");
        assert_ne!(detect_keyword_type("Milliarden Euro"), "person");
    }

    // ========================================
    // CONCEPT TESTS
    // ========================================

    #[test]
    fn test_detect_concept_single_word() {
        assert_eq!(detect_keyword_type("Klimawandel"), "concept");
        assert_eq!(detect_keyword_type("Digitalisierung"), "concept");
        assert_eq!(detect_keyword_type("Inflation"), "concept");
    }

    #[test]
    fn test_detect_concept_lowercase() {
        // Lowercase words should be concepts
        assert_eq!(detect_keyword_type("künstliche intelligenz"), "concept");
    }

    // ========================================
    // EDGE CASES
    // ========================================

    #[test]
    fn test_edge_case_umlauts() {
        // German names with umlauts
        assert_eq!(detect_keyword_type("Björn Höcke"), "person");
        assert_eq!(detect_keyword_type("Jörg Müller"), "person");
        // Location with umlaut
        assert_eq!(detect_keyword_type("München"), "location");
        assert_eq!(detect_keyword_type("Zürich"), "location");
    }

    #[test]
    fn test_edge_case_hyphenated_names() {
        assert_eq!(detect_keyword_type("Jean-Pascal Hohm"), "person");
        assert_eq!(detect_keyword_type("Frank-Walter Steinmeier"), "person");
    }

    #[test]
    fn test_edge_case_apostrophe_names() {
        assert_eq!(detect_keyword_type("Liam O'Brien"), "person");
    }

    #[test]
    fn test_edge_case_empty_string() {
        assert_eq!(detect_keyword_type(""), "concept");
    }

    #[test]
    fn test_edge_case_single_char() {
        assert_eq!(detect_keyword_type("A"), "concept");
    }

    // ========================================
    // REGRESSION TESTS (Known False Positives)
    // ========================================

    #[test]
    fn test_regression_known_false_positives() {
        // These were incorrectly classified as "person" before
        assert_ne!(detect_keyword_type("Berlin Game"), "person");
        assert_ne!(detect_keyword_type("Programm Deutschlandfunk"), "person");
        assert_ne!(detect_keyword_type("Antenna Group"), "person");
        assert_ne!(detect_keyword_type("Daimler Truck"), "person");
        assert_ne!(detect_keyword_type("Baltic Sentry"), "person");
        assert_ne!(detect_keyword_type("Arctic Sentry"), "person");
        assert_ne!(detect_keyword_type("Cold Cases"), "person");
        assert_ne!(detect_keyword_type("Dry Cleaning"), "person");
        assert_ne!(detect_keyword_type("Mental Health"), "person");
    }

    // ========================================
    // MERGE_KEYWORDS TESTS
    // ========================================

    #[test]
    fn test_merge_keywords_deduplication_case_insensitive() {
        let llm = vec!["Rust".to_string(), "Python".to_string()];
        let local = vec!["rust".to_string(), "Java".to_string()];
        let result = merge_keywords(&llm, local, 10);
        // "rust" from local should be deduped against "Rust" from LLM
        assert_eq!(result, vec!["Rust", "Python", "Java"]);
    }

    #[test]
    fn test_merge_keywords_llm_first_then_local() {
        let llm = vec!["Alpha".to_string(), "Beta".to_string()];
        let local = vec!["Gamma".to_string(), "Delta".to_string()];
        let result = merge_keywords(&llm, local, 10);
        assert_eq!(result, vec!["Alpha", "Beta", "Gamma", "Delta"]);
    }

    #[test]
    fn test_merge_keywords_max_count_truncation() {
        let llm = vec!["A1".to_string(), "B2".to_string(), "C3".to_string()];
        let local = vec!["D4".to_string(), "E5".to_string()];
        let result = merge_keywords(&llm, local, 3);
        assert_eq!(result.len(), 3);
        assert_eq!(result, vec!["A1", "B2", "C3"]);
    }

    #[test]
    fn test_merge_keywords_empty_inputs() {
        let empty: Vec<String> = vec![];
        // Both empty
        assert!(merge_keywords(&empty, vec![], 10).is_empty());
        // Only LLM
        let llm = vec!["Rust".to_string()];
        assert_eq!(merge_keywords(&llm, vec![], 10), vec!["Rust"]);
        // Only local
        let local = vec!["Java".to_string()];
        assert_eq!(merge_keywords(&empty, local, 10), vec!["Java"]);
    }

    #[test]
    fn test_merge_keywords_excludes_single_char() {
        let llm = vec!["A".to_string(), "Rust".to_string()];
        let local = vec!["B".to_string(), "Go".to_string()];
        let result = merge_keywords(&llm, local, 10);
        // Single-char "A" and "B" should be excluded (min 2 chars)
        assert_eq!(result, vec!["Rust", "Go"]);
    }

    #[test]
    fn test_merge_keywords_excludes_empty_strings() {
        let llm = vec!["".to_string(), "Rust".to_string()];
        let local = vec!["Go".to_string()];
        let result = merge_keywords(&llm, local, 10);
        assert_eq!(result, vec!["Rust", "Go"]);
    }

    // ========================================
    // VALIDATE_AND_MERGE_CATEGORIES TESTS
    // ========================================

    #[test]
    fn test_validate_and_merge_categories_valid_llm() {
        let llm = vec!["Politik".to_string(), "Wirtschaft".to_string()];
        let local = vec!["Sport".to_string()];
        let result = validate_and_merge_categories(&llm, local);
        // LLM categories are valid SEPHIROTH, so they come first
        assert_eq!(result[0], "Politik");
        assert_eq!(result[1], "Wirtschaft");
        // Local supplements
        assert_eq!(result[2], "Sport");
    }

    #[test]
    fn test_validate_and_merge_categories_invalid_llm_filtered() {
        let llm = vec!["InvalidCategory".to_string(), "Politik".to_string()];
        let local = vec!["Sport".to_string()];
        let result = validate_and_merge_categories(&llm, local);
        // "InvalidCategory" should be filtered out
        assert!(result.contains(&"Politik".to_string()));
        assert!(!result.contains(&"InvalidCategory".to_string()));
    }

    #[test]
    fn test_validate_and_merge_categories_fallback_to_local() {
        let llm = vec!["NotACategory".to_string(), "AlsoInvalid".to_string()];
        let local = vec!["Technik".to_string(), "Kultur".to_string()];
        let result = validate_and_merge_categories(&llm, local);
        // All LLM invalid, falls back to local
        assert_eq!(result, vec!["Technik", "Kultur"]);
    }

    #[test]
    fn test_validate_and_merge_categories_max_5() {
        let llm = vec![
            "Politik".to_string(),
            "Wirtschaft".to_string(),
            "Technik".to_string(),
        ];
        let local = vec![
            "Sport".to_string(),
            "Kultur".to_string(),
            "Umwelt".to_string(),
            "Recht".to_string(),
        ];
        let result = validate_and_merge_categories(&llm, local);
        assert!(result.len() <= 5);
    }

    #[test]
    fn test_validate_and_merge_categories_dedup() {
        let llm = vec!["Politik".to_string(), "Wirtschaft".to_string()];
        let local = vec!["politik".to_string(), "Sport".to_string()];
        let result = validate_and_merge_categories(&llm, local);
        // "politik" should be deduped against "Politik" (case-insensitive)
        let politik_count = result
            .iter()
            .filter(|c| c.to_lowercase() == "politik")
            .count();
        assert_eq!(politik_count, 1);
    }

    #[test]
    fn test_validate_and_merge_categories_case_insensitive_validation() {
        // SEPHIROTH has "Politik" - test that "politik" (lowercase) is also accepted
        let llm = vec!["politik".to_string()];
        let local = vec![];
        let result = validate_and_merge_categories(&llm, local);
        assert_eq!(result, vec!["politik"]);
    }

    // ========================================
    // MERGE_CATEGORIES_STAT_PRIMARY TESTS
    // ========================================

    #[test]
    fn test_merge_categories_stat_primary_stat_first() {
        let stat = vec![
            ("Politik".to_string(), 0.8),
            ("Wirtschaft".to_string(), 0.7),
        ];
        let llm = vec!["Technik".to_string()];
        let local = vec!["Sport".to_string()];
        let result = merge_categories_stat_primary(&stat, &llm, local, 0.5);
        // Statistical categories should be first
        assert_eq!(result[0], "Politik");
        assert_eq!(result[1], "Wirtschaft");
    }

    #[test]
    fn test_merge_categories_stat_primary_min_confidence_filtering() {
        let stat = vec![
            ("Politik".to_string(), 0.8),
            ("Wirtschaft".to_string(), 0.3), // Below min_confidence
        ];
        let llm = vec!["Technik".to_string()];
        let local = vec![];
        let result = merge_categories_stat_primary(&stat, &llm, local, 0.5);
        assert!(result.contains(&"Politik".to_string()));
        assert!(!result.contains(&"Wirtschaft".to_string())); // Filtered out
        assert!(result.contains(&"Technik".to_string()));
    }

    #[test]
    fn test_merge_categories_stat_primary_llm_supplements() {
        let stat = vec![("Politik".to_string(), 0.9)];
        let llm = vec!["Wirtschaft".to_string(), "Technik".to_string()];
        let local = vec![];
        let result = merge_categories_stat_primary(&stat, &llm, local, 0.5);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "Politik"); // Stat first
        assert!(result.contains(&"Wirtschaft".to_string()));
        assert!(result.contains(&"Technik".to_string()));
    }

    #[test]
    fn test_merge_categories_stat_primary_local_fallback() {
        let stat: Vec<(String, f64)> = vec![];
        let llm: Vec<String> = vec![];
        let local = vec!["Sport".to_string(), "Kultur".to_string()];
        let result = merge_categories_stat_primary(&stat, &llm, local, 0.5);
        assert_eq!(result, vec!["Sport", "Kultur"]);
    }

    #[test]
    fn test_merge_categories_stat_primary_max_5() {
        let stat = vec![
            ("Politik".to_string(), 0.9),
            ("Wirtschaft".to_string(), 0.8),
            ("Technik".to_string(), 0.7),
        ];
        let llm = vec!["Sport".to_string(), "Kultur".to_string()];
        let local = vec!["Umwelt".to_string(), "Recht".to_string()];
        let result = merge_categories_stat_primary(&stat, &llm, local, 0.5);
        assert!(result.len() <= 5);
    }

    #[test]
    fn test_merge_categories_stat_primary_validates_against_sephiroth() {
        let stat = vec![("InvalidCat".to_string(), 0.9)];
        let llm = vec!["AlsoInvalid".to_string()];
        let local = vec!["Sport".to_string()];
        let result = merge_categories_stat_primary(&stat, &llm, local, 0.5);
        // Invalid stat and LLM categories should not appear
        assert!(!result.contains(&"InvalidCat".to_string()));
        assert!(!result.contains(&"AlsoInvalid".to_string()));
        // Local is used as fallback (not validated against SEPHIROTH in this function)
        assert!(result.contains(&"Sport".to_string()));
    }

    #[test]
    fn test_merge_categories_stat_primary_dedup_across_sources() {
        let stat = vec![("Politik".to_string(), 0.9)];
        let llm = vec!["Politik".to_string(), "Wirtschaft".to_string()];
        let local = vec!["politik".to_string()];
        let result = merge_categories_stat_primary(&stat, &llm, local, 0.5);
        let politik_count = result
            .iter()
            .filter(|c| c.to_lowercase() == "politik")
            .count();
        assert_eq!(politik_count, 1);
    }

    // ========================================
    // DETERMINE_KEYWORD_SOURCES TESTS
    // ========================================

    #[test]
    fn test_determine_keyword_sources_statistical() {
        use crate::keywords::types::KeywordSource;
        let final_kw = vec!["Rust".to_string(), "Python".to_string()];
        let stat_kw = vec!["Rust".to_string()];
        let result = determine_keyword_sources(&final_kw, &stat_kw);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Rust");
        assert!(matches!(result[0].source, KeywordSource::Statistical));
        assert_eq!(result[0].confidence, 0.8);
    }

    #[test]
    fn test_determine_keyword_sources_ai() {
        use crate::keywords::types::KeywordSource;
        let final_kw = vec!["Rust".to_string(), "Python".to_string()];
        let stat_kw = vec!["Rust".to_string()];
        let result = determine_keyword_sources(&final_kw, &stat_kw);
        assert_eq!(result[1].name, "Python");
        assert!(matches!(result[1].source, KeywordSource::Ai));
        assert_eq!(result[1].confidence, 1.0);
    }

    #[test]
    fn test_determine_keyword_sources_case_insensitive() {
        use crate::keywords::types::KeywordSource;
        let final_kw = vec!["RUST".to_string()];
        let stat_kw = vec!["rust".to_string()];
        let result = determine_keyword_sources(&final_kw, &stat_kw);
        assert_eq!(result[0].name, "RUST"); // Preserves original case
        assert!(matches!(result[0].source, KeywordSource::Statistical));
    }

    #[test]
    fn test_determine_keyword_sources_empty() {
        let final_kw: Vec<String> = vec![];
        let stat_kw: Vec<String> = vec![];
        let result = determine_keyword_sources(&final_kw, &stat_kw);
        assert!(result.is_empty());
    }

    #[test]
    fn test_determine_keyword_sources_all_statistical() {
        use crate::keywords::types::KeywordSource;
        let final_kw = vec!["Alpha".to_string(), "Beta".to_string()];
        let stat_kw = vec!["Alpha".to_string(), "Beta".to_string()];
        let result = determine_keyword_sources(&final_kw, &stat_kw);
        assert!(result
            .iter()
            .all(|k| matches!(k.source, KeywordSource::Statistical)));
    }

    #[test]
    fn test_determine_keyword_sources_none_statistical() {
        use crate::keywords::types::KeywordSource;
        let final_kw = vec!["Alpha".to_string(), "Beta".to_string()];
        let stat_kw: Vec<String> = vec![];
        let result = determine_keyword_sources(&final_kw, &stat_kw);
        assert!(result.iter().all(|k| matches!(k.source, KeywordSource::Ai)));
    }

    // ========================================
    // DETERMINE_CATEGORY_SOURCES TESTS
    // ========================================

    #[test]
    fn test_determine_category_sources_statistical() {
        let final_cats = vec!["Politik".to_string(), "Wirtschaft".to_string()];
        let stat_cats = vec![("Politik".to_string(), 0.85)];
        let result = determine_category_sources(&final_cats, &stat_cats);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Politik");
        assert_eq!(result[0].source, "statistical");
        assert_eq!(result[0].confidence, 0.85);
    }

    #[test]
    fn test_determine_category_sources_ai() {
        let final_cats = vec!["Wirtschaft".to_string()];
        let stat_cats: Vec<(String, f64)> = vec![];
        let result = determine_category_sources(&final_cats, &stat_cats);
        assert_eq!(result[0].name, "Wirtschaft");
        assert_eq!(result[0].source, "ai");
        assert_eq!(result[0].confidence, 1.0);
    }

    #[test]
    fn test_determine_category_sources_case_insensitive() {
        let final_cats = vec!["POLITIK".to_string()];
        let stat_cats = vec![("politik".to_string(), 0.9)];
        let result = determine_category_sources(&final_cats, &stat_cats);
        assert_eq!(result[0].name, "POLITIK"); // Preserves original case
        assert_eq!(result[0].source, "statistical");
        assert_eq!(result[0].confidence, 0.9);
    }

    #[test]
    fn test_determine_category_sources_mixed() {
        let final_cats = vec![
            "Politik".to_string(),
            "Wirtschaft".to_string(),
            "Technik".to_string(),
        ];
        let stat_cats = vec![("Politik".to_string(), 0.8), ("Technik".to_string(), 0.6)];
        let result = determine_category_sources(&final_cats, &stat_cats);
        assert_eq!(result[0].source, "statistical"); // Politik
        assert_eq!(result[1].source, "ai"); // Wirtschaft
        assert_eq!(result[2].source, "statistical"); // Technik
    }

    #[test]
    fn test_determine_category_sources_empty() {
        let final_cats: Vec<String> = vec![];
        let stat_cats: Vec<(String, f64)> = vec![];
        let result = determine_category_sources(&final_cats, &stat_cats);
        assert!(result.is_empty());
    }

    // ========================================
    // COMPUTE_CONTENT_HASH TESTS
    // ========================================

    #[test]
    fn test_compute_content_hash_deterministic() {
        let hash1 = compute_content_hash("Title", "Content body here");
        let hash2 = compute_content_hash("Title", "Content body here");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_content_hash_different_inputs_different_hashes() {
        let hash1 = compute_content_hash("Title A", "Content A");
        let hash2 = compute_content_hash("Title B", "Content B");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_content_hash_different_title_different_hash() {
        let hash1 = compute_content_hash("Title A", "Same content");
        let hash2 = compute_content_hash("Title B", "Same content");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_content_hash_different_content_different_hash() {
        let hash1 = compute_content_hash("Same title", "Content A");
        let hash2 = compute_content_hash("Same title", "Content B");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_content_hash_short_content_not_affected_by_truncation() {
        let short = "Short content";
        let hash1 = compute_content_hash("Title", short);
        // Same content should produce same hash regardless of truncation
        let hash2 = compute_content_hash("Title", short);
        assert_eq!(hash1, hash2);
        // Verify the hash is a valid hex string
        assert!(hash1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_compute_content_hash_long_content_truncated() {
        // Create content longer than 6000 chars
        let long_content: String = "A".repeat(10000);
        let truncated_content: String = "A".repeat(6000);
        let hash_long = compute_content_hash("Title", &long_content);
        let hash_truncated = compute_content_hash("Title", &truncated_content);
        // Both should produce the same hash since truncation happens at 6000 chars
        assert_eq!(hash_long, hash_truncated);
    }

    #[test]
    fn test_compute_content_hash_extra_chars_beyond_6000_ignored() {
        let base: String = "X".repeat(6000);
        let extended = format!("{}extra content here", base);
        let hash_base = compute_content_hash("Title", &base);
        let hash_extended = compute_content_hash("Title", &extended);
        assert_eq!(hash_base, hash_extended);
    }

    #[test]
    fn test_compute_content_hash_returns_hex_string() {
        let hash = compute_content_hash("Test", "Content");
        // SHA-256 hex string is 64 chars
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ========================================
    // GET_MODEL_PRICING TESTS
    // ========================================

    #[test]
    fn test_get_model_pricing_known_models() {
        assert_eq!(get_model_pricing("gpt-5-nano"), (0.05, 0.40));
        assert_eq!(get_model_pricing("gpt-5-mini"), (0.25, 2.00));
        assert_eq!(get_model_pricing("gpt-5"), (1.25, 10.00));
        assert_eq!(get_model_pricing("gpt-4.1-nano"), (0.10, 0.40));
        assert_eq!(get_model_pricing("gpt-4.1-mini"), (0.40, 1.60));
        assert_eq!(get_model_pricing("gpt-4.1"), (2.00, 8.00));
        assert_eq!(get_model_pricing("gpt-4o-mini"), (0.15, 0.60));
        assert_eq!(get_model_pricing("gpt-4o"), (2.50, 10.00));
    }

    #[test]
    fn test_get_model_pricing_with_suffix() {
        // Model names with version suffixes should still match via starts_with
        assert_eq!(get_model_pricing("gpt-5-nano-2025-01-01"), (0.05, 0.40));
        assert_eq!(get_model_pricing("gpt-4o-mini-latest"), (0.15, 0.60));
    }

    #[test]
    fn test_get_model_pricing_unknown_model_returns_default() {
        let (input, output) = get_model_pricing("unknown-model");
        assert_eq!(input, 0.50);
        assert_eq!(output, 2.00);
    }

    #[test]
    fn test_get_model_pricing_ollama_returns_default() {
        // Ollama models should get conservative defaults
        let (input, output) = get_model_pricing("ministral-3:latest");
        assert_eq!(input, 0.50);
        assert_eq!(output, 2.00);
    }

    // ========================================
    // TRUNCATE_STR_HELPER TESTS
    // ========================================

    #[test]
    fn test_truncate_str_helper_short_string() {
        assert_eq!(truncate_str_helper("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_str_helper_exact_length() {
        assert_eq!(truncate_str_helper("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_str_helper_truncates() {
        assert_eq!(truncate_str_helper("hello world", 5), "hello");
    }

    #[test]
    fn test_truncate_str_helper_respects_utf8_boundaries() {
        // 'ä' is 2 bytes in UTF-8, so truncating at byte 1 should give empty
        let s = "ä";
        assert_eq!(s.len(), 2);
        let result = truncate_str_helper(s, 1);
        // Should not panic, should back up to char boundary
        assert!(result.is_empty() || result.len() <= 1);
    }

    #[test]
    fn test_truncate_str_helper_multibyte_chars() {
        // "Müller" - 'ü' is 2 bytes
        let s = "Müller";
        let result = truncate_str_helper(s, 3);
        // Should truncate at a valid char boundary
        assert!(result.len() <= 3);
        assert!(result.is_char_boundary(result.len()));
    }

    // ========================================
    // LOG_GENERATION_COST TESTS
    // ========================================

    /// Helper: create an in-memory SQLite DB with the ai_cost_log table
    fn create_test_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE ai_cost_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                estimated_cost_usd REAL NOT NULL DEFAULT 0.0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            "#,
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_log_generation_cost_with_tokens() {
        let conn = create_test_db();
        let usage = TokenUsage {
            input_tokens: Some(1000),
            output_tokens: Some(500),
        };
        log_generation_cost(&conn, "openai_compatible", "gpt-5-nano", &usage);

        // Verify a row was inserted
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ai_cost_log", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);

        // Verify the values
        let (provider, model, input_tok, output_tok, cost): (String, String, i64, i64, f64) = conn
            .query_row(
                "SELECT provider, model, input_tokens, output_tokens, estimated_cost_usd FROM ai_cost_log",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .unwrap();
        assert_eq!(provider, "openai_compatible");
        assert_eq!(model, "gpt-5-nano");
        assert_eq!(input_tok, 1000);
        assert_eq!(output_tok, 500);
        // gpt-5-nano: $0.05/1M input + $0.40/1M output
        // cost = 1000 * 0.05 / 1_000_000 + 500 * 0.40 / 1_000_000
        //      = 0.00005 + 0.0002 = 0.00025
        assert!((cost - 0.00025).abs() < 1e-10);
    }

    #[test]
    fn test_log_generation_cost_no_tokens_no_log() {
        let conn = create_test_db();
        // Ollama-style: no token counts
        let usage = TokenUsage {
            input_tokens: None,
            output_tokens: None,
        };
        log_generation_cost(&conn, "ollama", "ministral-3:latest", &usage);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ai_cost_log", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0); // Nothing logged
    }

    #[test]
    fn test_log_generation_cost_partial_tokens_no_log() {
        let conn = create_test_db();
        // Only input tokens available, output missing
        let usage = TokenUsage {
            input_tokens: Some(1000),
            output_tokens: None,
        };
        log_generation_cost(&conn, "openai_compatible", "gpt-5-nano", &usage);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ai_cost_log", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0); // Nothing logged - both must be present
    }

    #[test]
    fn test_log_generation_cost_zero_tokens() {
        let conn = create_test_db();
        let usage = TokenUsage {
            input_tokens: Some(0),
            output_tokens: Some(0),
        };
        log_generation_cost(&conn, "openai_compatible", "gpt-5-nano", &usage);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ai_cost_log", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1); // Should still log (both tokens present)

        let cost: f64 = conn
            .query_row("SELECT estimated_cost_usd FROM ai_cost_log", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(cost, 0.0);
    }
}
