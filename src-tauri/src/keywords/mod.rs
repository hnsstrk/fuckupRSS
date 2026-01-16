use crate::text_analysis::STOPWORDS;
use keyword_extraction::rake::{Rake, RakeParams};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use whatlang::{detect, Lang};
use yake_rust::{get_n_best, Config, StopWords};

pub mod advanced;
pub mod clustering;
pub mod config;
pub mod types;

pub use advanced::{
    extract_ngrams, extract_textrank, extract_enhanced_entities,
    filter_by_pos_heuristics, prepare_semantic_candidates, apply_semantic_scores,
    SemanticCandidate,
    // MMR Diversification
    apply_mmr_diversification, apply_mmr_text_based, MmrConfig, MmrCandidate,
    // Levenshtein distance utilities
    levenshtein_distance, is_near_duplicate,
    // TRISUM Multi-Centrality
    extract_textrank_trisum, get_centrality_scores, TrisumConfig, CentralityScores,
};
pub use clustering::{
    cluster_articles, get_representatives, transfer_keywords_to_cluster, calculate_savings,
    ArticleCluster, ArticleForClustering, ClusterConfig, ClusteringResult,
};
pub use config::{KeywordConfig, defaults as keyword_defaults};
pub use types::{
    ArticleKeywordRef, ExtractedKeywordCandidate, KeywordSource, KeywordWithMetadata,
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq)]
pub enum KeywordType {
    Concept,
    Person,
    Organization,
    Location,
    Acronym,
}

#[derive(Debug, Clone)]
pub struct ExtractedKeyword {
    pub text: String,
    pub score: f64,
    pub keyword_type: KeywordType,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    German,
    English,
}

/// Unified stopwords from central text_analysis module (converted to owned Strings for RAKE/YAKE)
static UNIFIED_STOPWORDS: Lazy<HashSet<String>> = Lazy::new(|| {
    STOPWORDS.iter().map(|s| s.to_string()).collect()
});

static ORG_SUFFIXES: Lazy<Vec<&str>> = Lazy::new(|| {
    vec![
        "gmbh",
        "ag",
        "inc",
        "ltd",
        "corp",
        "co",
        "kg",
        "e.v.",
        "ev",
        "ministerium",
        "ministry",
        "university",
        "universität",
        "institut",
        "institute",
        "stiftung",
        "foundation",
        "verband",
        "association",
        "partei",
        "party",
        "fraktion",
        "gruppe",
        "group",
        "bank",
        "bundesamt",
    ]
});

static KNOWN_ACRONYMS: Lazy<HashSet<&str>> = Lazy::new(|| {
    [
        "USA", "EU", "UN", "NATO", "SPD", "CDU", "CSU", "AFD", "FDP", "EZB", "EKD", "BMW", "VW",
        "BASF", "SAP", "DFB", "FIFA", "UEFA", "IOC", "WHO", "WTO", "FBI", "CIA", "NSA", "BND",
        "BKA", "LKA", "BBC", "CNN", "ARD", "ZDF",
    ]
    .iter()
    .copied()
    .collect()
});

pub struct KeywordExtractor {
    max_keywords: usize,
}

impl Default for KeywordExtractor {
    fn default() -> Self {
        Self::new(10)
    }
}

impl KeywordExtractor {
    pub fn new(max_keywords: usize) -> Self {
        Self { max_keywords }
    }

    pub fn detect_language(text: &str) -> Language {
        match detect(text) {
            Some(info) if info.lang() == Lang::Deu => Language::German,
            Some(info) if info.lang() == Lang::Eng => Language::English,
            _ => {
                let has_umlauts = text
                    .chars()
                    .any(|c| matches!(c, 'ä' | 'ö' | 'ü' | 'ß' | 'Ä' | 'Ö' | 'Ü'));
                if has_umlauts {
                    Language::German
                } else {
                    Language::English
                }
            }
        }
    }

    fn get_stopwords(_lang: Language) -> &'static HashSet<String> {
        // Use unified stopwords for all languages (includes DE, EN, HTML, and news terms)
        &UNIFIED_STOPWORDS
    }

    pub fn extract(&self, title: &str, content: &str) -> Vec<ExtractedKeyword> {
        let full_text = format!("{}\n\n{}", title, content);
        let lang = Self::detect_language(&full_text);
        let stopwords = Self::get_stopwords(lang);

        let mut candidates: HashMap<String, ExtractedKeyword> = HashMap::new();

        // === Traditional Methods ===

        // 1. YAKE extraction
        for kw in self.extract_yake(&full_text, lang) {
            let key = kw.text.to_lowercase();
            candidates.entry(key).or_insert(kw);
        }

        // 2. RAKE extraction
        for kw in self.extract_rake(&full_text, stopwords) {
            let key = kw.text.to_lowercase();
            candidates
                .entry(key)
                .and_modify(|existing| {
                    existing.score = (existing.score + kw.score) / 2.0;
                    if !existing.source.contains(&kw.source) {
                        existing.source = format!("{},{}", existing.source, kw.source);
                    }
                })
                .or_insert(kw);
        }

        // 3. Basic entity extraction
        for kw in self.extract_entities(&full_text) {
            let key = kw.text.to_lowercase();
            candidates
                .entry(key)
                .and_modify(|existing| {
                    existing.score += 0.3;
                    existing.keyword_type = kw.keyword_type.clone();
                })
                .or_insert(kw);
        }

        // === Advanced Methods ===

        // 4. N-gram extraction (bigrams/trigrams)
        for kw in advanced::extract_ngrams(&full_text, lang, self.max_keywords * 2) {
            let key = kw.text.to_lowercase();
            candidates
                .entry(key)
                .and_modify(|existing| {
                    // N-grams that match existing keywords boost the score
                    existing.score = (existing.score + kw.score * 0.5).min(1.0);
                    if !existing.source.contains("ngram") {
                        existing.source = format!("{},ngram", existing.source);
                    }
                })
                .or_insert(kw);
        }

        // 5. TextRank graph-based extraction
        for kw in advanced::extract_textrank(&full_text, 4, self.max_keywords * 2) {
            let key = kw.text.to_lowercase();
            candidates
                .entry(key)
                .and_modify(|existing| {
                    // TextRank confirmation boosts score
                    existing.score = (existing.score + kw.score * 0.3).min(1.0);
                    if !existing.source.contains("textrank") {
                        existing.source = format!("{},textrank", existing.source);
                    }
                })
                .or_insert(kw);
        }

        // 6. Enhanced Named Entity Recognition
        for kw in advanced::extract_enhanced_entities(&full_text) {
            let key = kw.text.to_lowercase();
            candidates
                .entry(key)
                .and_modify(|existing| {
                    // Enhanced NER boosts score and updates type
                    existing.score = (existing.score + 0.2).min(1.0);
                    if existing.keyword_type == KeywordType::Concept {
                        existing.keyword_type = kw.keyword_type.clone();
                    }
                    if !existing.source.contains("enhanced_ner") {
                        existing.source = format!("{},enhanced_ner", existing.source);
                    }
                })
                .or_insert(kw);
        }

        // === Boosting & Filtering ===

        // Title boost
        let title_lower = title.to_lowercase();
        for candidate in candidates.values_mut() {
            if title_lower.contains(&candidate.text.to_lowercase()) {
                candidate.score += 0.25;
                candidate.score = candidate.score.min(1.0);
            }
        }

        // 7. Apply POS-like filtering to boost noun phrases
        let mut filtered: Vec<ExtractedKeyword> = candidates
            .into_values()
            .filter(|kw| self.is_valid_keyword(&kw.text, stopwords))
            .collect();

        // Apply POS heuristics to boost likely nouns
        filtered = advanced::filter_by_pos_heuristics(filtered);

        // Sort and diversify
        filtered.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.ensure_diversity(filtered)
    }

    fn extract_yake(&self, text: &str, lang: Language) -> Vec<ExtractedKeyword> {
        let config = Config {
            ngrams: 3,
            ..Config::default()
        };

        let stopwords = match lang {
            Language::German => StopWords::predefined("de").unwrap_or_default(),
            Language::English => StopWords::predefined("en").unwrap_or_default(),
        };

        let results = get_n_best(self.max_keywords * 2, text, &stopwords, &config);

        results
            .into_iter()
            .map(|item| ExtractedKeyword {
                text: item.keyword,
                score: 1.0 - item.score.min(1.0),
                keyword_type: KeywordType::Concept,
                source: "yake".to_string(),
            })
            .collect()
    }

    fn extract_rake(&self, text: &str, stopwords: &HashSet<String>) -> Vec<ExtractedKeyword> {
        let stopwords_vec: Vec<String> = stopwords.iter().cloned().collect();
        let rake = Rake::new(RakeParams::WithDefaults(text, &stopwords_vec));
        let keywords = rake.get_ranked_keyword_scores(self.max_keywords * 2);

        keywords
            .into_iter()
            .map(|(keyword, score)| {
                let normalized_score = (score as f64 / 10.0).min(1.0);
                ExtractedKeyword {
                    text: keyword,
                    score: normalized_score,
                    keyword_type: KeywordType::Concept,
                    source: "rake".to_string(),
                }
            })
            .collect()
    }

    fn extract_entities(&self, text: &str) -> Vec<ExtractedKeyword> {
        let mut entities = Vec::new();
        let cap_pattern =
            regex::Regex::new(r"\b([A-ZÄÖÜ][a-zäöüß]+(?:\s+[A-ZÄÖÜ][a-zäöüß]+){0,3})\b").unwrap();

        for cap in cap_pattern.captures_iter(text) {
            let phrase = cap.get(1).unwrap().as_str().to_string();
            if phrase.len() >= 3
                && !UNIFIED_STOPWORDS.contains(&phrase.to_lowercase())
            {
                let kw_type = self.classify_entity(&phrase);
                if kw_type != KeywordType::Concept {
                    entities.push(ExtractedKeyword {
                        text: phrase,
                        score: 0.7,
                        keyword_type: kw_type,
                        source: "entity".to_string(),
                    });
                }
            }
        }

        let acronym_pattern = regex::Regex::new(r"\b([A-Z]{2,6})\b").unwrap();
        for cap in acronym_pattern.captures_iter(text) {
            let acronym = cap.get(1).unwrap().as_str();
            if KNOWN_ACRONYMS.contains(acronym) {
                entities.push(ExtractedKeyword {
                    text: acronym.to_string(),
                    score: 0.85,
                    keyword_type: KeywordType::Acronym,
                    source: "entity".to_string(),
                });
            }
        }

        entities
    }

    fn classify_entity(&self, text: &str) -> KeywordType {
        let lower = text.to_lowercase();

        for suffix in ORG_SUFFIXES.iter() {
            if lower.ends_with(suffix) || lower.contains(&format!(" {}", suffix)) {
                return KeywordType::Organization;
            }
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        if (words.len() == 2 || words.len() == 3)
            && words.iter().all(|w| {
                w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                    && w.len() >= 2
                    && w.len() <= 15
            })
        {
            return KeywordType::Person;
        }

        KeywordType::Concept
    }

    fn is_valid_keyword(&self, text: &str, stopwords: &HashSet<String>) -> bool {
        let lower = text.to_lowercase();

        if text.len() < 2 || text.len() > 50 {
            return false;
        }
        // Check against unified stopwords (includes news, HTML, DE, EN)
        if stopwords.contains(&lower) {
            return false;
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        if words.iter().all(|w| stopwords.contains(&w.to_lowercase())) {
            return false;
        }
        if text
            .chars()
            .all(|c| c.is_numeric() || c.is_whitespace() || c == '.' || c == ',')
        {
            return false;
        }

        true
    }

    fn ensure_diversity(&self, mut keywords: Vec<ExtractedKeyword>) -> Vec<ExtractedKeyword> {
        let mut result = Vec::new();
        let mut has_entity = false;
        let mut has_concept = false;

        for kw in keywords.iter() {
            if result.len() >= self.max_keywords {
                break;
            }

            let is_entity = matches!(
                kw.keyword_type,
                KeywordType::Person
                    | KeywordType::Organization
                    | KeywordType::Location
                    | KeywordType::Acronym
            );

            if !has_entity && is_entity {
                result.push(kw.clone());
                has_entity = true;
            } else if !has_concept && kw.keyword_type == KeywordType::Concept {
                result.push(kw.clone());
                has_concept = true;
            }
        }

        for kw in keywords.drain(..) {
            if result.len() >= self.max_keywords {
                break;
            }
            if !result
                .iter()
                .any(|r| r.text.to_lowercase() == kw.text.to_lowercase())
            {
                result.push(kw);
            }
        }

        result
    }
}

pub fn extract_keywords(title: &str, content: &str, max_keywords: usize) -> Vec<String> {
    let extractor = KeywordExtractor::new(max_keywords);
    extractor
        .extract(title, content)
        .into_iter()
        .map(|kw| kw.text)
        .collect()
}

/// Extract keywords with full metadata including source and type
pub fn extract_keywords_with_metadata(
    title: &str,
    content: &str,
    max_keywords: usize,
) -> Vec<ExtractedKeyword> {
    let extractor = KeywordExtractor::new(max_keywords);
    extractor.extract(title, content)
}

/// Result of semantic keyword extraction
#[derive(Debug, Clone)]
pub struct SemanticKeywordResult {
    pub text: String,
    pub score: f64,
    pub keyword_type: KeywordType,
    pub sources: Vec<String>,
    pub semantic_score: Option<f64>,
}

impl From<ExtractedKeyword> for SemanticKeywordResult {
    fn from(kw: ExtractedKeyword) -> Self {
        Self {
            text: kw.text,
            score: kw.score,
            keyword_type: kw.keyword_type,
            sources: kw.source.split(',').map(|s| s.to_string()).collect(),
            semantic_score: None,
        }
    }
}

/// Extract keywords with optional semantic scoring
///
/// If `semantic_scores` is provided, it should be a HashMap from keyword text
/// to semantic similarity scores (0.0-1.0). The `semantic_weight` parameter
/// controls how much the semantic score contributes to the final score.
///
/// This is useful for KeyBERT-style extraction where you:
/// 1. Extract candidate keywords with this function
/// 2. Generate embeddings for each candidate and the full document
/// 3. Calculate cosine similarity between each candidate and the document
/// 4. Call this function again with the semantic scores
pub fn extract_keywords_with_semantic_scoring(
    title: &str,
    content: &str,
    max_keywords: usize,
    semantic_scores: Option<&HashMap<String, f64>>,
    semantic_weight: f64,
) -> Vec<SemanticKeywordResult> {
    let keywords = extract_keywords_with_metadata(title, content, max_keywords * 2);

    let results: Vec<SemanticKeywordResult> = if let Some(scores) = semantic_scores {
        let candidates = advanced::prepare_semantic_candidates(&keywords);
        let scored = advanced::apply_semantic_scores(candidates, scores, semantic_weight);

        scored
            .into_iter()
            .map(|kw| {
                let semantic_score = scores.get(&kw.text).copied();
                SemanticKeywordResult {
                    text: kw.text,
                    score: kw.score,
                    keyword_type: kw.keyword_type,
                    sources: kw.source.split(',').map(|s| s.to_string()).collect(),
                    semantic_score,
                }
            })
            .collect()
    } else {
        keywords.into_iter().map(|kw| kw.into()).collect()
    };

    // Sort and truncate
    let mut results = results;
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(max_keywords);
    results
}

static GARBAGE_PATTERNS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "doctype",
        "html",
        "http",
        "https",
        "www",
        "com",
        "org",
        "de",
        "net",
        "the",
        "and",
        "for",
        "with",
        "from",
        "that",
        "this",
        "have",
        "has",
        "been",
        "were",
        "was",
        "are",
        "will",
        "would",
        "could",
        "should",
        "their",
        "they",
        "them",
        "there",
        "then",
        "than",
        "when",
        "what",
        "which",
        "where",
        "who",
        "how",
        "why",
        "all",
        "also",
        "only",
        "just",
        "more",
        "most",
        "some",
        "many",
        "much",
        "very",
        "well",
        "even",
        "still",
        "back",
        "over",
        "such",
        "into",
        "year",
        "years",
        "time",
        "first",
        "last",
        "new",
        "now",
        "way",
        "may",
        "day",
        "get",
        "make",
        "like",
        "know",
        "take",
        "come",
        "could",
        "good",
        "see",
        "after",
        "other",
        "being",
        "made",
        "can",
        "been",
        "about",
        "out",
        "up",
        "down",
        "off",
        "says",
        "said",
        "according",
        "reuters",
        "dpa",
        "afp",
        "der",
        "die",
        "das",
        "ein",
        "eine",
        "einer",
        "eines",
        "einem",
        "einen",
        "und",
        "oder",
        "aber",
        "doch",
        "noch",
        "schon",
        "auch",
        "nur",
        "sehr",
        "mehr",
        "viel",
        "viele",
        "alle",
        "jede",
        "jeder",
        "jedes",
        "keine",
        "nicht",
        "kein",
        "wird",
        "werden",
        "wurde",
        "wurden",
        "ist",
        "sind",
        "war",
        "waren",
        "hat",
        "haben",
        "hatte",
        "hatten",
        "kann",
        "können",
        "soll",
        "sollen",
        "muss",
        "müssen",
        "will",
        "wollen",
        "nach",
        "vor",
        "bei",
        "mit",
        "von",
        "aus",
        "für",
        "über",
        "unter",
        "zwischen",
        "durch",
        "gegen",
        "ohne",
        "bis",
        "seit",
        "während",
        "wegen",
        "sowie",
        "dabei",
        "dazu",
        "daher",
        "deshalb",
        "jedoch",
        "dennoch",
        "bereits",
        "immer",
        "wieder",
        "weiter",
        "weitere",
        "anderen",
        "andere",
        "ersten",
        "erste",
        "neuen",
        "neue",
        "neuer",
        "großen",
        "große",
        "eigenen",
        "eigene",
        "letzten",
        "letzte",
        "deutschen",
        "deutsche",
        "rund",
        "etwa",
        "fast",
        "knapp",
        "insgesamt",
        "zurück",
        "vorfall",
        "vertrag",
        "festnahme",
        "verleihung",
        "angriffe",
        "ermittlungen",
        "nachfolgerin",
        "gefangene",
        "nationen",
        "viertel",
        "behörden",
        "bericht",
        "heute",
        "gestern",
        "morgen",
        "uhr",
        "jahr",
        "jahre",
    ]
    .iter()
    .copied()
    .collect()
});

static VALID_SINGLE_WORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "nato",
        "eu",
        "un",
        "usa",
        "cdu",
        "csu",
        "spd",
        "fdp",
        "afd",
        "dax",
        "iran",
        "irak",
        "gaza",
        "israel",
        "china",
        "japan",
        "trump",
        "biden",
        "putin",
        "scholz",
        "merz",
        "habeck",
        "lindner",
        "tesla",
        "apple",
        "google",
        "amazon",
        "microsoft",
        "meta",
        "bitcoin",
        "corona",
        "covid",
        "klima",
        "ukraine",
        "russland",
    ]
    .iter()
    .copied()
    .collect()
});

pub fn normalize_keyword(keyword: &str) -> Option<String> {
    let trimmed = keyword.trim();
    let lower = trimmed.to_lowercase();

    if trimmed.len() < 3 || trimmed.len() > 50 {
        return None;
    }

    if GARBAGE_PATTERNS.contains(lower.as_str()) {
        return None;
    }

    if trimmed
        .chars()
        .any(|c| c == '<' || c == '>' || c == '{' || c == '}')
    {
        return None;
    }

    if trimmed
        .chars()
        .all(|c| c.is_numeric() || c.is_whitespace() || ".,-/:".contains(c))
    {
        return None;
    }

    let special_count = trimmed
        .chars()
        .filter(|c| !c.is_alphanumeric() && !c.is_whitespace() && *c != '-')
        .count();
    if special_count > trimmed.len() / 4 {
        return None;
    }

    let alpha_count = trimmed.chars().filter(|c| c.is_alphabetic()).count();
    if alpha_count < trimmed.len() / 2 {
        return None;
    }

    if KNOWN_ACRONYMS.contains(trimmed) {
        return Some(trimmed.to_string());
    }

    let words: Vec<&str> = trimmed.split_whitespace().collect();

    if words.len() == 1 {
        if lower.len() < 4 && !VALID_SINGLE_WORDS.contains(lower.as_str()) {
            return None;
        }

        if !VALID_SINGLE_WORDS.contains(lower.as_str())
            && !trimmed
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
        {
            return None;
        }
    }

    if words.len() > 4 {
        return None;
    }

    let is_proper_noun = words.len() >= 1
        && words.len() <= 3
        && words
            .iter()
            .all(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && w.len() >= 2);

    if is_proper_noun {
        return Some(trimmed.to_string());
    }

    if words.len() == 1 && VALID_SINGLE_WORDS.contains(lower.as_str()) {
        return Some(trimmed.to_string());
    }

    let has_proper_noun = words
        .iter()
        .any(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && w.len() >= 3);

    if has_proper_noun {
        return Some(trimmed.to_string());
    }

    None
}

pub fn normalize_and_dedupe_keywords(keywords: &[String]) -> Vec<String> {
    normalize_and_dedupe_keywords_with_levenshtein(keywords, 2)
}

/// Normalize and deduplicate keywords using both exact matching and Levenshtein distance
///
/// This function:
/// 1. Normalizes each keyword (filters garbage, validates format)
/// 2. Removes exact duplicates (case-insensitive)
/// 3. Removes near-duplicates based on Levenshtein distance
///
/// # Arguments
/// * `keywords` - List of keywords to process
/// * `max_distance` - Maximum Levenshtein distance for near-duplicate detection
///                    (recommended: 2 for short keywords, 3 for longer ones)
///
/// # Returns
/// Deduplicated list of keywords, keeping the first occurrence
pub fn normalize_and_dedupe_keywords_with_levenshtein(
    keywords: &[String],
    max_distance: usize,
) -> Vec<String> {
    let mut result = Vec::new();

    for kw in keywords {
        if let Some(normalized) = normalize_keyword(kw) {
            // Check if this keyword is a near-duplicate of any existing result
            let is_duplicate = result.iter().any(|existing: &String| {
                let existing_lower = existing.to_lowercase();
                let normalized_lower = normalized.to_lowercase();

                // Exact match
                if existing_lower == normalized_lower {
                    return true;
                }

                // Near-duplicate check using Levenshtein distance
                advanced::is_near_duplicate(&existing_lower, &normalized_lower, max_distance)
            });

            if !is_duplicate {
                result.push(normalized);
            }
        }
    }

    result
}

/// Normalize and deduplicate keywords, keeping the one with higher score
///
/// Similar to `normalize_and_dedupe_keywords_with_levenshtein`, but allows
/// specifying scores for each keyword to keep the higher-scored one.
///
/// # Arguments
/// * `keywords` - List of (keyword, score) tuples
/// * `max_distance` - Maximum Levenshtein distance for near-duplicate detection
///
/// # Returns
/// Deduplicated list of (keyword, score) tuples
pub fn normalize_and_dedupe_keywords_with_scores(
    keywords: &[(String, f64)],
    max_distance: usize,
) -> Vec<(String, f64)> {
    let mut result: Vec<(String, f64)> = Vec::new();

    for (kw, score) in keywords {
        if let Some(normalized) = normalize_keyword(kw) {
            let normalized_lower = normalized.to_lowercase();

            // Find if there's a near-duplicate in results
            let duplicate_idx = result.iter().position(|(existing, _)| {
                let existing_lower = existing.to_lowercase();

                // Exact match
                if existing_lower == normalized_lower {
                    return true;
                }

                // Near-duplicate check
                advanced::is_near_duplicate(&existing_lower, &normalized_lower, max_distance)
            });

            match duplicate_idx {
                Some(idx) => {
                    // Keep the one with higher score
                    if *score > result[idx].1 {
                        result[idx] = (normalized, *score);
                    }
                }
                None => {
                    result.push((normalized, *score));
                }
            }
        }
    }

    result
}

static SYNONYM_GROUPS: Lazy<Vec<(&'static str, Vec<&'static str>)>> = Lazy::new(|| {
    vec![
        // === Technologie ===
        (
            "Künstliche Intelligenz",
            vec![
                "ki", "ai", "artificial intelligence", "maschinelles lernen",
                "machine learning", "ml", "deep learning", "neuronale netze",
            ],
        ),
        // === Internationale Organisationen ===
        (
            "Europäische Union",
            vec![
                "eu", "european union", "europa", "european", "europäisch",
                "europäischen", "europäischer", "brüssel", "brussels",
            ],
        ),
        (
            "NATO",
            vec![
                "north atlantic treaty", "atlantisches bündnis", "nato-",
                "transatlantisch", "transatlantic",
            ],
        ),
        (
            "Vereinte Nationen",
            vec!["un", "uno", "united nations", "un-"],
        ),
        // === Länder mit Adjektiven ===
        (
            "Vereinigte Staaten",
            vec![
                "usa", "us", "united states", "amerika", "america",
                "american", "amerikanisch", "amerikanischen", "amerikanischer",
                "washington", "white house", "weißes haus",
                // German declension forms
                "vereinigten staaten", "vereinigter staaten", "staaten",
            ],
        ),
        (
            "Deutschland",
            vec![
                "germany", "brd", "bundesrepublik", "german", "deutsch",
                "deutschen", "deutscher", "berlin", "bundestag", "bundesregierung",
            ],
        ),
        (
            "Russland",
            vec![
                "russia", "russian", "russisch", "russischen", "russischer",
                "moskau", "moscow", "kreml", "kremlin", "russische föderation",
            ],
        ),
        (
            "Ukraine",
            vec![
                "ukrainian", "ukrainisch", "ukrainischen", "ukrainischer",
                "kiew", "kyiv", "kiev", "donbass", "donezk", "luhansk",
            ],
        ),
        (
            "China",
            vec![
                "chinese", "chinesisch", "chinesischen", "chinesischer",
                "peking", "beijing", "volksrepublik", "prc",
            ],
        ),
        (
            "Großbritannien",
            vec![
                "uk", "united kingdom", "british", "britisch", "britischen",
                "england", "english", "englisch", "london", "westminster",
                "scotland", "scottish", "schottland", "schottisch",
                "wales", "welsh", "walisisch",
            ],
        ),
        (
            "Frankreich",
            vec![
                "france", "french", "französisch", "französischen",
                "paris", "élysée", "elysee",
            ],
        ),
        (
            "Iran",
            vec![
                "iranian", "iranisch", "iranische", "iranischen", "iranischer",
                "teheran", "tehran", "persisch", "persian",
                // Common variations
                "irans", "der iran", "im iran", "iran-krise", "iran-konflikt",
            ],
        ),
        (
            "Israel",
            vec![
                "israeli", "israelisch", "israelischen", "israelischer",
                "jerusalem", "tel aviv", "netanjahu", "netanyahu",
            ],
        ),
        (
            "Türkei",
            vec![
                "turkey", "turkish", "türkisch", "türkischen",
                "ankara", "istanbul", "erdogan",
            ],
        ),
        (
            "Japan",
            vec![
                "japanese", "japanisch", "japanischen", "tokio", "tokyo",
            ],
        ),
        (
            "Indien",
            vec![
                "india", "indian", "indisch", "indischen", "neu-delhi", "new delhi",
            ],
        ),
        (
            "Brasilien",
            vec![
                "brazil", "brazilian", "brasilianisch", "brasilianischen",
                "brasília", "brasilia",
            ],
        ),
        (
            "Spanien",
            vec![
                "spain", "spanish", "spanisch", "spanischen", "madrid",
            ],
        ),
        (
            "Italien",
            vec![
                "italy", "italian", "italienisch", "italienischen", "rom", "rome",
            ],
        ),
        (
            "Polen",
            vec![
                "poland", "polish", "polnisch", "polnischen", "warschau", "warsaw",
            ],
        ),
        (
            "Niederlande",
            vec![
                "netherlands", "dutch", "niederländisch", "holland",
                "amsterdam", "den haag", "the hague",
            ],
        ),
        (
            "Österreich",
            vec![
                "austria", "austrian", "österreichisch", "österreichischen", "wien", "vienna",
            ],
        ),
        (
            "Schweiz",
            vec![
                "switzerland", "swiss", "schweizerisch", "schweizer",
                "bern", "zürich", "zurich", "genf", "geneva",
            ],
        ),
        (
            "Dänemark",
            vec![
                "denmark", "danish", "dänisch", "dänische", "dänischen",
                "kopenhagen", "copenhagen",
            ],
        ),
        (
            "Griechenland",
            vec![
                "greece", "greek", "griechisch", "griechische", "griechischen",
                "athen", "athens",
            ],
        ),
        (
            "Ungarn",
            vec![
                "hungary", "hungarian", "ungarisch", "ungarische", "ungarischen",
                "budapest", "orban", "orbán",
            ],
        ),
        (
            "Schweden",
            vec![
                "sweden", "swedish", "schwedisch", "schwedische", "schwedischen",
                "stockholm",
            ],
        ),
        (
            "Norwegen",
            vec![
                "norway", "norwegian", "norwegisch", "norwegische", "norwegischen",
                "oslo",
            ],
        ),
        (
            "Finnland",
            vec![
                "finland", "finnish", "finnisch", "finnische", "finnischen",
                "helsinki",
            ],
        ),
        (
            "Belgien",
            vec![
                "belgium", "belgian", "belgisch", "belgische", "belgischen",
            ],
        ),
        (
            "Portugal",
            vec![
                "portuguese", "portugiesisch", "portugiesische", "portugiesischen",
                "lissabon", "lisbon",
            ],
        ),
        (
            "Tschechien",
            vec![
                "czech", "tschechisch", "tschechische", "tschechischen",
                "prag", "prague",
            ],
        ),
        (
            "Rumänien",
            vec![
                "romania", "romanian", "rumänisch", "rumänische", "rumänischen",
                "bukarest", "bucharest",
            ],
        ),
        (
            "Serbien",
            vec![
                "serbia", "serbian", "serbisch", "serbische", "serbischen",
                "belgrad", "belgrade",
            ],
        ),
        (
            "Kroatien",
            vec![
                "croatia", "croatian", "kroatisch", "kroatische", "kroatischen",
                "zagreb",
            ],
        ),
        // === Themen ===
        (
            "Klimawandel",
            vec![
                "klimakrise", "climate change", "global warming", "erderwärmung",
                "klimaschutz", "climate protection", "co2", "treibhausgas",
            ],
        ),
        (
            "COVID-19",
            vec!["corona", "coronavirus", "covid", "pandemie", "pandemic", "sars-cov-2"],
        ),
        (
            "Migration",
            vec![
                "migration", "flüchtlinge", "refugees", "asyl", "asylum",
                "einwanderung", "immigration", "migranten", "migrants",
            ],
        ),
        (
            "Wirtschaft",
            vec![
                "economy", "economic", "ökonomie", "ökonomisch", "wirtschaftlich",
                "wirtschaftlichen", "konjunktur", "rezession", "recession",
            ],
        ),
        (
            "Sicherheit",
            vec![
                "security", "sicherheitspolitik", "verteidigung", "defense", "defence",
                "militär", "military", "streitkräfte", "armed forces",
            ],
        ),
        // === Deutsche Konzepte mit Deklinationsformen ===
        (
            "Minderjährige",
            vec!["minderjährige", "minderjährigen", "minderjähriger"],
        ),
        (
            "Flüchtlinge",
            vec!["flüchtling", "flüchtlingen", "geflüchtete", "geflüchteten"],
        ),
        (
            "Jugendliche",
            vec!["jugendliche", "jugendlichen", "jugendlicher"],
        ),
        (
            "Proteste",
            vec!["protest", "proteste", "protesten", "protests", "demonstration", "demonstrationen"],
        ),
        (
            "Sanktionen",
            vec!["sanktion", "sanktionen", "sanctions"],
        ),
        (
            "Wahlen",
            vec!["wahl", "wahlen", "election", "elections"],
        ),
        (
            "Regierung",
            vec!["regierung", "regierungen", "government", "governments"],
        ),
        (
            "Opposition",
            vec!["opposition", "oppositionelle", "oppositionellen"],
        ),
        (
            "Soldaten",
            vec!["soldat", "soldaten", "soldier", "soldiers", "troops"],
        ),
        (
            "Demonstranten",
            vec!["demonstrant", "demonstranten", "protestierende", "protestierenden"],
        ),
        // === Prominente Personen (vollständige Namen) ===
        (
            "Donald Trump",
            vec!["trump", "trumps", "donald j. trump", "ex-präsident trump", "präsident trump"],
        ),
        (
            "Joe Biden",
            vec!["biden", "bidens", "präsident biden", "us-präsident biden"],
        ),
        (
            "Wladimir Putin",
            vec!["putin", "putins", "präsident putin", "russlands präsident"],
        ),
        (
            "Olaf Scholz",
            vec!["scholz", "bundeskanzler scholz", "kanzler scholz"],
        ),
    ]
});

pub fn find_canonical_keyword(keyword: &str) -> Option<&'static str> {
    let lower = keyword.to_lowercase();

    for (canonical, synonyms) in SYNONYM_GROUPS.iter() {
        if canonical.to_lowercase() == lower {
            return Some(canonical);
        }
        for syn in synonyms {
            if *syn == lower {
                return Some(canonical);
            }
        }
    }

    None
}

#[allow(dead_code)] // Reserved for Phase 3 Immanentize Network features
pub fn get_all_synonyms(keyword: &str) -> Vec<&'static str> {
    let lower = keyword.to_lowercase();

    for (canonical, synonyms) in SYNONYM_GROUPS.iter() {
        if canonical.to_lowercase() == lower || synonyms.iter().any(|s| *s == lower) {
            let mut result: Vec<&str> = synonyms.iter().copied().collect();
            result.push(canonical);
            return result;
        }
    }

    vec![]
}

// ============================================================
// DYNAMIC SYNONYMS FROM DATABASE
// ============================================================

use rusqlite::Connection;
use std::sync::RwLock;

/// Cached dynamic synonyms loaded from the database
/// Maps variant name (lowercase) → canonical name
static DYNAMIC_SYNONYMS: Lazy<RwLock<HashMap<String, String>>> = Lazy::new(|| {
    RwLock::new(HashMap::new())
});

/// Load synonyms from database (canonical_id relationships)
/// Call this on startup and after merging keywords
pub fn load_dynamic_synonyms(conn: &Connection) -> Result<usize, String> {
    // Load all keyword pairs where canonical_id is set
    // This means: variant (name) → canonical (canonical's name)
    let mut stmt = conn
        .prepare(
            r#"SELECT v.name, c.name
               FROM immanentize v
               JOIN immanentize c ON v.canonical_id = c.id
               WHERE v.canonical_id IS NOT NULL"#,
        )
        .map_err(|e| e.to_string())?;

    let pairs: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let count = pairs.len();

    // Update the cache
    let mut cache = DYNAMIC_SYNONYMS
        .write()
        .map_err(|e| format!("Lock error: {}", e))?;

    cache.clear();
    for (variant, canonical) in pairs {
        cache.insert(variant.to_lowercase(), canonical);
    }

    Ok(count)
}

/// Find canonical keyword using both static SYNONYM_GROUPS and dynamic DB synonyms
/// Prefers dynamic synonyms (more recent/specific) over static ones
pub fn find_canonical_keyword_with_db(keyword: &str) -> Option<String> {
    let lower = keyword.to_lowercase();

    // First check dynamic synonyms from database (higher priority)
    if let Ok(cache) = DYNAMIC_SYNONYMS.read() {
        if let Some(canonical) = cache.get(&lower) {
            return Some(canonical.clone());
        }
    }

    // Fall back to static SYNONYM_GROUPS
    find_canonical_keyword(keyword).map(|s| s.to_string())
}

/// Get all known synonyms for a keyword (static + dynamic)
pub fn get_all_synonyms_with_db(keyword: &str) -> Vec<String> {
    let lower = keyword.to_lowercase();
    let mut result: Vec<String> = Vec::new();

    // Get static synonyms
    let static_synonyms = get_all_synonyms(keyword);
    for s in static_synonyms {
        result.push(s.to_string());
    }

    // Get dynamic synonyms from cache
    if let Ok(cache) = DYNAMIC_SYNONYMS.read() {
        // Find all variants that map to the same canonical
        let canonical = cache.get(&lower).cloned();
        let check_canonical = canonical.as_ref().map(|s| s.to_lowercase()).unwrap_or(lower.clone());

        for (variant, canon) in cache.iter() {
            if canon.to_lowercase() == check_canonical && !result.contains(&variant.to_string()) {
                result.push(variant.clone());
            }
        }
    }

    result
}

/// Clear the dynamic synonyms cache
pub fn clear_dynamic_synonyms_cache() {
    if let Ok(mut cache) = DYNAMIC_SYNONYMS.write() {
        cache.clear();
    }
}

// ============================================================
// COMPOUND KEYWORD SPLITTING
// ============================================================

/// Words that should not be split from compounds (particles, prepositions)
static COMPOUND_IGNORE_PARTS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "und", "oder", "für", "mit", "von", "zu", "bei", "auf", "in", "an",
        "and", "or", "for", "with", "from", "to", "at", "on", "in",
        "der", "die", "das", "den", "dem", "des",
        "the", "a", "an",
        "-", "",
    ]
    .iter()
    .copied()
    .collect()
});

/// Keywords that are valid as single parts after splitting
static VALID_COMPOUND_PARTS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // Politicians (last names)
        "trump", "biden", "putin", "scholz", "merkel", "macron", "erdogan",
        "merz", "habeck", "lindner", "weidel", "söder", "laschet",
        // Parties and organizations
        "cdu", "csu", "spd", "fdp", "afd", "grüne", "linke",
        "nato", "eu", "un", "usa", "bnd", "cia", "fbi",
        // Countries
        "ukraine", "russland", "china", "iran", "israel", "gaza",
        // Common topic words
        "klima", "energie", "migration", "wirtschaft", "politik",
        "krieg", "krise", "deal", "streit", "konflikt", "reform",
        "zölle", "sanktionen", "abkommen", "gipfel", "verhandlungen",
    ]
    .iter()
    .copied()
    .collect()
});

/// Split compound keyword into components
/// e.g., "Trump-Zölle" → ["Trump", "Zölle"]
/// e.g., "CDU-CSU-Fraktion" → ["CDU", "CSU", "Fraktion"]
pub fn split_compound_keyword(keyword: &str) -> Vec<String> {
    // Only split if contains hyphen
    if !keyword.contains('-') {
        return vec![keyword.to_string()];
    }

    let parts: Vec<&str> = keyword.split('-').collect();

    // Don't split single hyphen or too many parts
    if parts.len() < 2 || parts.len() > 4 {
        return vec![keyword.to_string()];
    }

    // Filter out ignored parts and validate remaining ones
    let valid_parts: Vec<String> = parts
        .iter()
        .map(|p| p.trim())
        .filter(|p| {
            let lower = p.to_lowercase();
            // Must be at least 2 chars and not an ignored particle
            p.len() >= 2 && !COMPOUND_IGNORE_PARTS.contains(lower.as_str())
        })
        .map(|p| {
            // Capitalize first letter for proper nouns, keep acronyms uppercase
            if p.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
                p.to_string() // Acronym, keep as is
            } else {
                // Capitalize first letter
                let mut chars = p.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            }
        })
        .collect();

    // If only one valid part remains, include original compound
    if valid_parts.len() <= 1 {
        return vec![keyword.to_string()];
    }

    // Check if at least one part is meaningful (known keyword or capitalized)
    let has_meaningful_part = valid_parts.iter().any(|p| {
        let lower = p.to_lowercase();
        VALID_COMPOUND_PARTS.contains(lower.as_str())
            || p.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
    });

    if !has_meaningful_part {
        return vec![keyword.to_string()];
    }

    // Return both the original compound AND the split parts
    let mut result = vec![keyword.to_string()];
    for part in valid_parts {
        if !result.iter().any(|r| r.to_lowercase() == part.to_lowercase()) {
            result.push(part);
        }
    }

    result
}

/// Expand a list of keywords by splitting compounds
/// Returns original keywords plus their split components
pub fn expand_compound_keywords(keywords: &[String]) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut seen = HashSet::new();

    for kw in keywords {
        for part in split_compound_keyword(kw) {
            let lower = part.to_lowercase();
            if seen.insert(lower) {
                result.push(part);
            }
        }
    }

    result
}
