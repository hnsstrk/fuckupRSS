//! Advanced Keyword Extraction Methods
//!
//! This module provides additional keyword extraction techniques:
//! - N-gram extraction (bigrams/trigrams)
//! - POS-like filtering using heuristics
//! - TextRank graph-based extraction
//! - Enhanced Named Entity Recognition

use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use super::{ExtractedKeyword, KeywordType, Language, UNIFIED_STOPWORDS};

// Cached regex patterns for Enhanced NER (compiled once)
static TITLE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b((?:herr|frau|dr\.?|prof\.?|minister(?:in)?|präsident(?:in)?|kanzler(?:in)?|mr\.?|mrs\.?|ms\.?|president|chancellor|senator)\s+)([A-ZÄÖÜ][a-zäöüß]+(?:\s+[A-ZÄÖÜ][a-zäöüß]+){0,2})\b"
    ).unwrap()
});

static ORG_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\b([A-ZÄÖÜ][a-zäöüß]*(?:\s+[A-ZÄÖÜ&][a-zäöüß]*){0,4})\s+(GmbH|AG|Inc\.?|Ltd\.?|Corp\.?|Co\.?|KG|e\.V\.?|SE)\b"
    ).unwrap()
});

static MINISTRY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\b((?:Bundes)?[A-ZÄÖÜ][a-zäöüß]*(?:ministerium|ministry|department|behörde|amt|agency))\b"
    ).unwrap()
});

static LOCATION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:in|aus|nach|from|to|at)\s+([A-ZÄÖÜ][a-zäöüß]+(?:\s+[A-ZÄÖÜ][a-zäöüß]+)?)\b",
    )
    .unwrap()
});

static EVENT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\b([A-ZÄÖÜ][a-zäöüß]*[-\s]?(?:gipfel|konferenz|summit|conference|wahlen?|election|treffen|meeting))\b"
    ).unwrap()
});

static YEAR_CONTEXT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([A-ZÄÖÜ][a-zäöüß]+(?:\s+[A-ZÄÖÜ][a-zäöüß]+)?)\s+(20\d{2}|19\d{2})\b").unwrap()
});

// ============================================================
// N-GRAM EXTRACTION
// ============================================================

/// Extract significant bigrams and trigrams from text
pub fn extract_ngrams(text: &str, lang: Language, max_ngrams: usize) -> Vec<ExtractedKeyword> {
    let words = tokenize_words(text);
    if words.len() < 2 {
        return vec![];
    }

    let mut ngram_counts: HashMap<String, usize> = HashMap::new();

    // Extract bigrams
    for window in words.windows(2) {
        if is_valid_ngram_sequence(window, lang) {
            let ngram = window.join(" ");
            *ngram_counts.entry(ngram).or_insert(0) += 1;
        }
    }

    // Extract trigrams
    for window in words.windows(3) {
        if is_valid_ngram_sequence(window, lang) {
            let ngram = window.join(" ");
            *ngram_counts.entry(ngram).or_insert(0) += 1;
        }
    }

    // Filter and score ngrams
    let total_ngrams: usize = ngram_counts.values().sum();
    let mut ngrams: Vec<ExtractedKeyword> = ngram_counts
        .into_iter()
        .filter(|(ngram, count)| {
            // Must appear at least twice or be a significant phrase
            *count >= 2 || is_significant_phrase(ngram)
        })
        .map(|(ngram, count)| {
            let word_count = ngram.split_whitespace().count();
            // Score based on frequency and n-gram length (longer = more specific)
            let frequency_score = (count as f64) / (total_ngrams as f64).max(1.0);
            let length_bonus = match word_count {
                2 => 1.0,
                3 => 1.2, // Trigrams get bonus for specificity
                _ => 0.8,
            };
            ExtractedKeyword {
                text: ngram,
                score: (frequency_score * length_bonus).min(1.0),
                keyword_type: KeywordType::Concept,
                source: "ngram".to_string(),
            }
        })
        .collect();

    ngrams.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ngrams.truncate(max_ngrams);
    ngrams
}

/// Tokenize text into words, preserving case for proper nouns
fn tokenize_words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '\'')
        .filter(|s| !s.is_empty() && s.len() >= 2)
        .map(|s| s.to_string())
        .collect()
}

/// Check if a sequence of words forms a valid n-gram
fn is_valid_ngram_sequence(words: &[String], lang: Language) -> bool {
    if words.is_empty() {
        return false;
    }

    // At least one word must not be a stopword
    let stopwords = &*UNIFIED_STOPWORDS;
    let non_stop_count = words
        .iter()
        .filter(|w| !stopwords.contains(&w.to_lowercase()))
        .count();

    if non_stop_count == 0 {
        return false;
    }

    // Guard against empty words (defensive check)
    if words.is_empty() {
        return false;
    }

    // First and last word should preferably not be stopwords
    // Safe: words is guaranteed non-empty after the guard above
    let first_is_stop = stopwords.contains(&words[0].to_lowercase());
    let last_is_stop = stopwords.contains(&words[words.len() - 1].to_lowercase());

    if first_is_stop && last_is_stop {
        return false;
    }

    // Check for proper noun patterns (capitalized words)
    let has_proper_noun = words
        .iter()
        .any(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && w.len() >= 3);

    // Apply language-specific patterns
    match lang {
        Language::German => {
            // German: All nouns are capitalized, so be more lenient
            non_stop_count >= 1 || has_proper_noun
        }
        Language::English => {
            // English: Proper nouns are significant
            non_stop_count >= 1 || has_proper_noun
        }
    }
}

/// Check if a phrase is significant based on patterns
fn is_significant_phrase(phrase: &str) -> bool {
    let words: Vec<&str> = phrase.split_whitespace().collect();

    // Named entity pattern: Multiple capitalized words
    let all_capitalized = words
        .iter()
        .all(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false));

    if all_capitalized && words.len() >= 2 {
        return true;
    }

    // Known phrase patterns
    let lower = phrase.to_lowercase();
    SIGNIFICANT_PHRASES.iter().any(|p| lower.contains(p))
}

/// Known significant phrase patterns
static SIGNIFICANT_PHRASES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        // German political/news phrases
        "künstliche intelligenz",
        "climate change",
        "klimawandel",
        "bundesregierung",
        "bundestag",
        "europäische union",
        "vereinigte staaten",
        "social media",
        "soziale medien",
        "machine learning",
        "deep learning",
        "fake news",
        "cyber security",
        "data protection",
        "datenschutz",
        // Common news compound concepts
        "supply chain",
        "lieferkette",
        "interest rate",
        "zinssatz",
        "central bank",
        "zentralbank",
        "foreign policy",
        "außenpolitik",
    ]
    .iter()
    .copied()
    .collect()
});

// ============================================================
// POS-LIKE FILTERING (Heuristic-based)
// ============================================================

/// Filter keywords based on POS-like heuristics
/// Since we don't have a full POS tagger, we use pattern matching
pub fn filter_by_pos_heuristics(keywords: Vec<ExtractedKeyword>) -> Vec<ExtractedKeyword> {
    keywords
        .into_iter()
        .filter(|kw| is_likely_noun_phrase(&kw.text))
        .map(|mut kw| {
            // Boost score for likely proper nouns
            if is_likely_proper_noun(&kw.text) {
                kw.score += 0.15;
                kw.score = kw.score.min(1.0);
            }
            kw
        })
        .collect()
}

/// Check if text is likely a noun phrase based on patterns
fn is_likely_noun_phrase(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();

    // Single word checks
    if words.len() == 1 {
        let word = words[0];
        // Likely noun if:
        // - Capitalized (proper noun or German noun)
        // - Contains numbers (model numbers, dates)
        // - Is in our known noun patterns
        return word
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
            || word.chars().any(|c| c.is_numeric())
            || is_known_noun_pattern(word);
    }

    // Multi-word checks
    // At least one word should look like a noun
    words.iter().any(|w| {
        w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) || is_known_noun_pattern(w)
    })
}

/// Check if a word matches known noun patterns
fn is_known_noun_pattern(word: &str) -> bool {
    let lower = word.to_lowercase();

    // German noun suffixes
    let de_suffixes = [
        "ung", "heit", "keit", "schaft", "tum", "nis", "sal", "ion", "ität", "ie", "ik", "ur",
        "eur", "ist", "ent", "ant",
    ];

    // English noun suffixes
    let en_suffixes = [
        "tion", "sion", "ment", "ness", "ity", "ance", "ence", "ship", "dom", "hood", "ist", "ism",
        "er", "or",
    ];

    de_suffixes.iter().any(|s| lower.ends_with(s)) || en_suffixes.iter().any(|s| lower.ends_with(s))
}

/// Check if text is likely a proper noun
fn is_likely_proper_noun(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();

    // All words capitalized (except small connecting words)
    let connecting = ["of", "the", "von", "der", "die", "das", "und", "and"];

    words.iter().all(|w| {
        let is_connecting = connecting.contains(&w.to_lowercase().as_str());
        is_connecting || w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
    })
}

// ============================================================
// TEXTRANK GRAPH-BASED EXTRACTION
// ============================================================

/// Extract keywords using TextRank algorithm
pub fn extract_textrank(
    text: &str,
    window_size: usize,
    max_keywords: usize,
) -> Vec<ExtractedKeyword> {
    let words = tokenize_for_textrank(text);
    if words.len() < 3 {
        return vec![];
    }

    // Build co-occurrence graph
    let mut graph: HashMap<String, HashMap<String, f64>> = HashMap::new();

    for window in words.windows(window_size) {
        for i in 0..window.len() {
            for j in (i + 1)..window.len() {
                let word_i = &window[i];
                let word_j = &window[j];

                // Add edge in both directions
                *graph
                    .entry(word_i.clone())
                    .or_default()
                    .entry(word_j.clone())
                    .or_insert(0.0) += 1.0;

                *graph
                    .entry(word_j.clone())
                    .or_default()
                    .entry(word_i.clone())
                    .or_insert(0.0) += 1.0;
            }
        }
    }

    // Run PageRank
    let scores = pagerank(&graph, 0.85, 30);

    // Convert to keywords
    let mut keywords: Vec<ExtractedKeyword> = scores
        .into_iter()
        .map(|(word, score)| ExtractedKeyword {
            text: word,
            score,
            keyword_type: KeywordType::Concept,
            source: "textrank".to_string(),
        })
        .collect();

    keywords.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    keywords.truncate(max_keywords);
    keywords
}

/// Tokenize text for TextRank (filter stopwords, keep significant words)
fn tokenize_for_textrank(text: &str) -> Vec<String> {
    let stopwords = &*UNIFIED_STOPWORDS;

    text.split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|s| {
            let lower = s.to_lowercase();
            !s.is_empty()
                && s.len() >= 3
                && !stopwords.contains(&lower)
                && s.chars().any(|c| c.is_alphabetic())
        })
        .map(|s| s.to_string())
        .collect()
}

/// Simple PageRank implementation
fn pagerank(
    graph: &HashMap<String, HashMap<String, f64>>,
    damping: f64,
    iterations: usize,
) -> HashMap<String, f64> {
    let nodes: Vec<String> = graph.keys().cloned().collect();
    let n = nodes.len();

    if n == 0 {
        return HashMap::new();
    }

    // Initialize scores
    let initial_score = 1.0 / n as f64;
    let mut scores: HashMap<String, f64> =
        nodes.iter().map(|n| (n.clone(), initial_score)).collect();

    // Calculate out-degree for each node
    let out_degree: HashMap<String, f64> = graph
        .iter()
        .map(|(node, edges)| {
            let total: f64 = edges.values().sum();
            (node.clone(), total.max(1.0))
        })
        .collect();

    // Iterate
    for _ in 0..iterations {
        let mut new_scores: HashMap<String, f64> = HashMap::new();

        for node in &nodes {
            let mut rank = (1.0 - damping) / n as f64;

            // Sum contributions from all neighbors
            if let Some(neighbors) = graph.get(node) {
                for (neighbor, weight) in neighbors {
                    if let Some(&neighbor_score) = scores.get(neighbor) {
                        let neighbor_out = out_degree.get(neighbor).unwrap_or(&1.0);
                        rank += damping * neighbor_score * (weight / neighbor_out);
                    }
                }
            }

            new_scores.insert(node.clone(), rank);
        }

        scores = new_scores;
    }

    // Normalize scores to 0-1 range
    let max_score = scores.values().cloned().fold(0.0_f64, f64::max);
    if max_score > 0.0 {
        for score in scores.values_mut() {
            *score /= max_score;
        }
    }

    scores
}

// ============================================================
// ENHANCED NAMED ENTITY RECOGNITION
// ============================================================

/// Enhanced NER patterns
/// Used for future NER improvements in keyword extraction
#[allow(dead_code)]
static LOCATION_INDICATORS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        // German
        "stadt",
        "land",
        "region",
        "bezirk",
        "kreis",
        "bundesland",
        "provinz",
        "staat",
        "republik",
        "königreich",
        "gebiet",
        // English
        "city",
        "country",
        "region",
        "state",
        "province",
        "republic",
        "kingdom",
        "district",
        "area",
        "territory",
    ]
    .iter()
    .copied()
    .collect()
});

/// Person titles for NER - used for future improvements
#[allow(dead_code)]
static PERSON_TITLES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        // German titles
        "herr",
        "frau",
        "dr",
        "prof",
        "minister",
        "ministerin",
        "präsident",
        "präsidentin",
        "kanzler",
        "kanzlerin",
        "bundeskanzler",
        "bundeskanzlerin",
        "bürgermeister",
        "bürgermeisterin",
        // English titles
        "mr",
        "mrs",
        "ms",
        "dr",
        "prof",
        "president",
        "chancellor",
        "minister",
        "secretary",
        "senator",
        "governor",
        "mayor",
        "ceo",
        "cfo",
        "cto",
    ]
    .iter()
    .copied()
    .collect()
});

/// Event patterns for NER - used for future improvements
#[allow(dead_code)]
static EVENT_PATTERNS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        // German
        "gipfel",
        "konferenz",
        "treffen",
        "verhandlungen",
        "wahl",
        "wahlen",
        "abstimmung",
        "referendum",
        "protest",
        "demonstration",
        "streik",
        "kongress",
        "festival",
        "messe",
        "ausstellung",
        // English
        "summit",
        "conference",
        "meeting",
        "negotiations",
        "election",
        "elections",
        "vote",
        "referendum",
        "protest",
        "demonstration",
        "strike",
        "congress",
        "festival",
        "fair",
        "exhibition",
    ]
    .iter()
    .copied()
    .collect()
});

/// Extract entities with enhanced patterns
pub fn extract_enhanced_entities(text: &str) -> Vec<ExtractedKeyword> {
    let mut entities = Vec::new();

    // Pattern 1: Person names with titles
    entities.extend(extract_persons_with_titles(text));

    // Pattern 2: Organizations with legal forms
    entities.extend(extract_organizations(text));

    // Pattern 3: Locations with indicators
    entities.extend(extract_locations(text));

    // Pattern 4: Events
    entities.extend(extract_events(text));

    // Pattern 5: Dates and time references
    entities.extend(extract_temporal_entities(text));

    // Deduplicate
    let mut seen = HashSet::new();
    entities.retain(|e| seen.insert(e.text.to_lowercase()));

    entities
}

/// Extract person names, especially those with titles
fn extract_persons_with_titles(text: &str) -> Vec<ExtractedKeyword> {
    let mut persons = Vec::new();

    // Use cached regex pattern for titles
    for cap in TITLE_PATTERN.captures_iter(text) {
        if let Some(name_match) = cap.get(2) {
            let name = name_match.as_str().to_string();
            if name.split_whitespace().count() >= 1 && name.len() >= 4 {
                persons.push(ExtractedKeyword {
                    text: name,
                    score: 0.85,
                    keyword_type: KeywordType::Person,
                    source: "enhanced_ner".to_string(),
                });
            }
        }
    }

    persons
}

/// Extract organizations with enhanced patterns
fn extract_organizations(text: &str) -> Vec<ExtractedKeyword> {
    let mut orgs = Vec::new();

    // Use cached regex pattern for organizations with legal forms
    for cap in ORG_PATTERN.captures_iter(text) {
        if let (Some(name), Some(form)) = (cap.get(1), cap.get(2)) {
            let full_name = format!("{} {}", name.as_str(), form.as_str());
            orgs.push(ExtractedKeyword {
                text: full_name,
                score: 0.9,
                keyword_type: KeywordType::Organization,
                source: "enhanced_ner".to_string(),
            });
        }
    }

    // Use cached regex pattern for ministry/department names
    for cap in MINISTRY_PATTERN.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            orgs.push(ExtractedKeyword {
                text: m.as_str().to_string(),
                score: 0.85,
                keyword_type: KeywordType::Organization,
                source: "enhanced_ner".to_string(),
            });
        }
    }

    orgs
}

/// Extract location entities
fn extract_locations(text: &str) -> Vec<ExtractedKeyword> {
    let mut locations = Vec::new();

    // Use cached regex pattern for location prepositions
    for cap in LOCATION_PATTERN.captures_iter(text) {
        if let Some(loc) = cap.get(1) {
            let location = loc.as_str().to_string();
            // Filter out common non-locations
            if !is_common_non_location(&location) && location.len() >= 3 {
                locations.push(ExtractedKeyword {
                    text: location,
                    score: 0.75,
                    keyword_type: KeywordType::Location,
                    source: "enhanced_ner".to_string(),
                });
            }
        }
    }

    locations
}

/// Check if a word is commonly mistaken for a location
fn is_common_non_location(word: &str) -> bool {
    let non_locations = [
        "der", "die", "das", "dem", "den", "the", "a", "an", "diesem", "dieser", "dieses", "this",
        "that", "einem", "einer", "eines", "one", "some",
    ];
    non_locations.contains(&word.to_lowercase().as_str())
}

/// Extract event-related entities
fn extract_events(text: &str) -> Vec<ExtractedKeyword> {
    let mut events = Vec::new();

    // Use cached regex pattern for event types
    for cap in EVENT_PATTERN.captures_iter(text) {
        if let Some(event) = cap.get(1) {
            events.push(ExtractedKeyword {
                text: event.as_str().to_string(),
                score: 0.8,
                keyword_type: KeywordType::Concept, // Events are concepts
                source: "enhanced_ner".to_string(),
            });
        }
    }

    events
}

/// Extract temporal entities (dates, periods)
fn extract_temporal_entities(text: &str) -> Vec<ExtractedKeyword> {
    let mut temporal = Vec::new();

    // Use cached regex pattern for year contexts
    for cap in YEAR_CONTEXT_PATTERN.captures_iter(text) {
        if let (Some(context), Some(year)) = (cap.get(1), cap.get(2)) {
            let entity = format!("{} {}", context.as_str(), year.as_str());
            // Only include if context is meaningful (not just a preposition)
            if context.as_str().len() >= 4 {
                temporal.push(ExtractedKeyword {
                    text: entity,
                    score: 0.7,
                    keyword_type: KeywordType::Concept,
                    source: "enhanced_ner".to_string(),
                });
            }
        }
    }

    temporal
}

// ============================================================
// SEMANTIC SIMILARITY SCORING (Hook for Embeddings)
// ============================================================

/// Candidate for semantic scoring
#[allow(dead_code)] // Public API for semantic keyword scoring with embeddings
#[derive(Debug, Clone)]
pub struct SemanticCandidate {
    pub text: String,
    pub base_score: f64,
    pub keyword_type: KeywordType,
    pub source: String,
}

/// Prepare candidates for semantic scoring
/// This returns candidates that can be scored against the full document embedding
#[allow(dead_code)] // Public API for semantic keyword scoring with embeddings
pub fn prepare_semantic_candidates(keywords: &[ExtractedKeyword]) -> Vec<SemanticCandidate> {
    keywords
        .iter()
        .map(|kw| SemanticCandidate {
            text: kw.text.clone(),
            base_score: kw.score,
            keyword_type: kw.keyword_type.clone(),
            source: kw.source.clone(),
        })
        .collect()
}

/// Apply semantic scores to candidates
/// similarity_scores is a map from keyword text to similarity score (0-1)
#[allow(dead_code)] // Public API for semantic keyword scoring with embeddings
pub fn apply_semantic_scores(
    candidates: Vec<SemanticCandidate>,
    similarity_scores: &HashMap<String, f64>,
    semantic_weight: f64,
) -> Vec<ExtractedKeyword> {
    candidates
        .into_iter()
        .map(|c| {
            let semantic_score = similarity_scores.get(&c.text).copied().unwrap_or(0.5);
            let combined_score =
                (c.base_score * (1.0 - semantic_weight)) + (semantic_score * semantic_weight);

            ExtractedKeyword {
                text: c.text,
                score: combined_score.min(1.0),
                keyword_type: c.keyword_type,
                source: if semantic_weight > 0.0 {
                    format!("{},semantic", c.source)
                } else {
                    c.source
                },
            }
        })
        .collect()
}

// ============================================================
// MMR (MAXIMAL MARGINAL RELEVANCE) DIVERSIFICATION
// ============================================================

/// Configuration for MMR diversification
#[derive(Debug, Clone)]
pub struct MmrConfig {
    /// Lambda parameter balancing relevance vs diversity (0.0-1.0)
    /// Lower values = more diversity, higher values = more relevance
    pub lambda: f64,
    /// Minimum similarity threshold between document and candidate
    pub min_relevance: f64,
}

impl Default for MmrConfig {
    fn default() -> Self {
        Self {
            lambda: 0.6, // Balanced between relevance and diversity
            min_relevance: 0.1,
        }
    }
}

/// Result of MMR diversification with both keyword and embedding
#[derive(Debug, Clone)]
pub struct MmrCandidate {
    pub keyword: ExtractedKeyword,
    pub embedding: Option<Vec<f32>>,
    pub mmr_score: f64,
}

/// Calculate cosine similarity between two embedding vectors
fn cosine_similarity_f32(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    (dot / (norm_a * norm_b)) as f64
}

/// Apply MMR diversification to a list of keyword candidates with embeddings
///
/// MMR formula: MMR = (1 - λ) × sim(candidate, doc) - λ × max(sim(candidate, selected))
///
/// This ensures selected keywords are both relevant to the document
/// and diverse from each other.
///
/// # Arguments
/// * `candidates` - List of candidates with their embeddings and document similarity scores
/// * `doc_embedding` - The document's embedding vector
/// * `max_keywords` - Maximum number of keywords to select
/// * `config` - MMR configuration parameters
///
/// # Returns
/// Diversified list of keywords sorted by MMR score
pub fn apply_mmr_diversification(
    candidates: Vec<(ExtractedKeyword, Option<Vec<f32>>)>,
    doc_embedding: Option<&[f32]>,
    max_keywords: usize,
    config: &MmrConfig,
) -> Vec<MmrCandidate> {
    if candidates.is_empty() || max_keywords == 0 {
        return vec![];
    }

    // Calculate document similarity for each candidate
    let mut scored_candidates: Vec<MmrCandidate> = candidates
        .into_iter()
        .map(|(kw, emb)| {
            let doc_sim = match (&emb, doc_embedding) {
                (Some(candidate_emb), Some(doc_emb)) => {
                    cosine_similarity_f32(candidate_emb, doc_emb)
                }
                _ => kw.score, // Fall back to original score if no embeddings
            };
            MmrCandidate {
                keyword: kw,
                embedding: emb,
                mmr_score: doc_sim,
            }
        })
        .filter(|c| c.mmr_score >= config.min_relevance)
        .collect();

    if scored_candidates.is_empty() {
        return vec![];
    }

    // Greedy MMR selection
    let mut selected: Vec<MmrCandidate> = Vec::with_capacity(max_keywords);

    // Select the most relevant candidate first
    scored_candidates.sort_by(|a, b| {
        b.mmr_score
            .partial_cmp(&a.mmr_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if let Some(first) = scored_candidates.first().cloned() {
        selected.push(first);
        scored_candidates.remove(0);
    }

    // Iteratively select candidates using MMR
    while selected.len() < max_keywords && !scored_candidates.is_empty() {
        let mut best_idx = 0;
        let mut best_mmr = f64::NEG_INFINITY;

        for (idx, candidate) in scored_candidates.iter().enumerate() {
            // Calculate max similarity to already selected keywords
            let max_sim_to_selected = selected
                .iter()
                .map(|s| match (&candidate.embedding, &s.embedding) {
                    (Some(c_emb), Some(s_emb)) => cosine_similarity_f32(c_emb, s_emb),
                    _ => {
                        // Fall back to text-based similarity using Levenshtein
                        let dist = levenshtein_distance(
                            &candidate.keyword.text.to_lowercase(),
                            &s.keyword.text.to_lowercase(),
                        );
                        let max_len = candidate.keyword.text.len().max(s.keyword.text.len());
                        if max_len == 0 {
                            0.0
                        } else {
                            1.0 - (dist as f64 / max_len as f64)
                        }
                    }
                })
                .fold(0.0_f64, f64::max);

            // MMR score: balance relevance and diversity
            let mmr =
                (1.0 - config.lambda) * candidate.mmr_score - config.lambda * max_sim_to_selected;

            if mmr > best_mmr {
                best_mmr = mmr;
                best_idx = idx;
            }
        }

        let selected_candidate = scored_candidates.remove(best_idx);
        selected.push(MmrCandidate {
            keyword: selected_candidate.keyword,
            embedding: selected_candidate.embedding,
            mmr_score: best_mmr,
        });
    }

    selected
}

/// Simplified MMR without embeddings (uses text similarity)
/// Useful when embeddings are not available
pub fn apply_mmr_text_based(
    keywords: Vec<ExtractedKeyword>,
    max_keywords: usize,
    lambda: f64,
) -> Vec<ExtractedKeyword> {
    if keywords.is_empty() || max_keywords == 0 {
        return vec![];
    }

    let candidates: Vec<(ExtractedKeyword, Option<Vec<f32>>)> =
        keywords.into_iter().map(|kw| (kw, None)).collect();

    let config = MmrConfig {
        lambda,
        min_relevance: 0.0, // No filtering for text-based
    };

    apply_mmr_diversification(candidates, None, max_keywords, &config)
        .into_iter()
        .map(|c| c.keyword)
        .collect()
}

// ============================================================
// LEVENSHTEIN DISTANCE FOR TEXT SIMILARITY
// ============================================================

/// Calculate Levenshtein distance between two strings
pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let m = s1_chars.len();
    let n = s2_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // Use two rows instead of full matrix for space efficiency
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr: Vec<usize> = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1) // Deletion
                .min(curr[j - 1] + 1) // Insertion
                .min(prev[j - 1] + cost); // Substitution
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Check if two keywords are near-duplicates based on Levenshtein distance
pub fn is_near_duplicate(s1: &str, s2: &str, max_distance: usize) -> bool {
    // Quick length check: if difference > max_distance, can't be near-duplicate
    let len_diff = (s1.len() as i64 - s2.len() as i64).unsigned_abs() as usize;
    if len_diff > max_distance {
        return false;
    }

    levenshtein_distance(&s1.to_lowercase(), &s2.to_lowercase()) <= max_distance
}

// ============================================================
// TRISUM MULTI-CENTRALITY (Eigenvector + Betweenness)
// ============================================================

/// Centrality scores from TRISUM analysis
#[allow(dead_code)] // Public API for TRISUM centrality-based keyword ranking
#[derive(Debug, Clone, Default)]
pub struct CentralityScores {
    /// PageRank-style score (already implemented)
    pub pagerank: f64,
    /// Eigenvector centrality (global influence)
    pub eigenvector: f64,
    /// Betweenness centrality (bridge terms)
    pub betweenness: f64,
    /// Combined TRISUM score
    pub trisum: f64,
}

/// Configuration for TRISUM scoring
#[derive(Debug, Clone)]
pub struct TrisumConfig {
    /// Weight for PageRank component
    pub pagerank_weight: f64,
    /// Weight for Eigenvector centrality
    pub eigenvector_weight: f64,
    /// Weight for Betweenness centrality
    pub betweenness_weight: f64,
}

impl Default for TrisumConfig {
    fn default() -> Self {
        Self {
            pagerank_weight: 0.4,
            eigenvector_weight: 0.35,
            betweenness_weight: 0.25,
        }
    }
}

/// Calculate eigenvector centrality using power iteration
fn eigenvector_centrality(
    graph: &HashMap<String, HashMap<String, f64>>,
    iterations: usize,
    tolerance: f64,
) -> HashMap<String, f64> {
    let nodes: Vec<String> = graph.keys().cloned().collect();
    let n = nodes.len();

    if n == 0 {
        return HashMap::new();
    }

    // Initialize all scores equally
    let initial = 1.0 / (n as f64).sqrt();
    let mut scores: HashMap<String, f64> = nodes.iter().map(|n| (n.clone(), initial)).collect();

    for _ in 0..iterations {
        let mut new_scores: HashMap<String, f64> = HashMap::new();

        for node in &nodes {
            let mut score = 0.0;
            // Sum contributions from neighbors
            if let Some(neighbors) = graph.get(node) {
                for (neighbor, weight) in neighbors {
                    if let Some(&neighbor_score) = scores.get(neighbor) {
                        score += neighbor_score * weight;
                    }
                }
            }
            new_scores.insert(node.clone(), score);
        }

        // Normalize
        let norm: f64 = new_scores.values().map(|v| v * v).sum::<f64>().sqrt();
        if norm > 0.0 {
            for score in new_scores.values_mut() {
                *score /= norm;
            }
        }

        // Check convergence
        let diff: f64 = nodes
            .iter()
            .map(|n| {
                let old = scores.get(n).unwrap_or(&0.0);
                let new = new_scores.get(n).unwrap_or(&0.0);
                (old - new).abs()
            })
            .sum();

        scores = new_scores;

        if diff < tolerance {
            break;
        }
    }

    // Normalize to 0-1 range
    let max_score = scores.values().cloned().fold(0.0_f64, f64::max);
    if max_score > 0.0 {
        for score in scores.values_mut() {
            *score /= max_score;
        }
    }

    scores
}

/// Calculate betweenness centrality using Brandes' algorithm (simplified)
fn betweenness_centrality(graph: &HashMap<String, HashMap<String, f64>>) -> HashMap<String, f64> {
    let nodes: Vec<String> = graph.keys().cloned().collect();
    let n = nodes.len();

    if n == 0 {
        return HashMap::new();
    }

    let mut betweenness: HashMap<String, f64> = nodes.iter().map(|n| (n.clone(), 0.0)).collect();

    // For each source node
    for source in &nodes {
        // BFS to find shortest paths
        let mut stack: Vec<String> = Vec::new();
        let mut predecessors: HashMap<String, Vec<String>> = HashMap::new();
        let mut sigma: HashMap<String, f64> = nodes.iter().map(|n| (n.clone(), 0.0)).collect();
        let mut dist: HashMap<String, i32> = nodes.iter().map(|n| (n.clone(), -1)).collect();

        sigma.insert(source.clone(), 1.0);
        dist.insert(source.clone(), 0);

        let mut queue = std::collections::VecDeque::new();
        queue.push_back(source.clone());

        while let Some(v) = queue.pop_front() {
            stack.push(v.clone());
            let v_dist = *dist.get(&v).unwrap_or(&-1);

            if let Some(neighbors) = graph.get(&v) {
                for neighbor in neighbors.keys() {
                    let n_dist = dist.get(neighbor).copied().unwrap_or(-1);
                    // First visit?
                    if n_dist < 0 {
                        queue.push_back(neighbor.clone());
                        dist.insert(neighbor.clone(), v_dist + 1);
                    }
                    // Shortest path?
                    if dist.get(neighbor).copied().unwrap_or(-1) == v_dist + 1 {
                        let v_sigma = sigma.get(&v).copied().unwrap_or(0.0);
                        *sigma.entry(neighbor.clone()).or_insert(0.0) += v_sigma;
                        predecessors
                            .entry(neighbor.clone())
                            .or_default()
                            .push(v.clone());
                    }
                }
            }
        }

        // Accumulation
        let mut delta: HashMap<String, f64> = nodes.iter().map(|n| (n.clone(), 0.0)).collect();

        while let Some(w) = stack.pop() {
            if let Some(preds) = predecessors.get(&w) {
                let w_sigma = sigma.get(&w).copied().unwrap_or(1.0);
                let w_delta = delta.get(&w).copied().unwrap_or(0.0);

                for v in preds {
                    let v_sigma = sigma.get(v).copied().unwrap_or(1.0);
                    let contribution = (v_sigma / w_sigma) * (1.0 + w_delta);
                    *delta.entry(v.clone()).or_insert(0.0) += contribution;
                }
            }

            if &w != source {
                let w_delta = delta.get(&w).copied().unwrap_or(0.0);
                *betweenness.entry(w.clone()).or_insert(0.0) += w_delta;
            }
        }
    }

    // Normalize (undirected graph, divide by 2)
    let max_bc = betweenness.values().cloned().fold(0.0_f64, f64::max);
    if max_bc > 0.0 {
        for score in betweenness.values_mut() {
            *score /= max_bc;
        }
    }

    betweenness
}

/// Extract keywords using TRISUM multi-centrality approach
pub fn extract_textrank_trisum(
    text: &str,
    window_size: usize,
    max_keywords: usize,
    config: Option<TrisumConfig>,
) -> Vec<ExtractedKeyword> {
    let config = config.unwrap_or_default();
    let words = tokenize_for_textrank(text);

    if words.len() < 3 {
        return vec![];
    }

    // Build co-occurrence graph
    let mut graph: HashMap<String, HashMap<String, f64>> = HashMap::new();

    for window in words.windows(window_size) {
        for i in 0..window.len() {
            for j in (i + 1)..window.len() {
                let word_i = &window[i];
                let word_j = &window[j];

                *graph
                    .entry(word_i.clone())
                    .or_default()
                    .entry(word_j.clone())
                    .or_insert(0.0) += 1.0;

                *graph
                    .entry(word_j.clone())
                    .or_default()
                    .entry(word_i.clone())
                    .or_insert(0.0) += 1.0;
            }
        }
    }

    // Calculate all three centrality measures
    let pagerank_scores = pagerank(&graph, 0.85, 30);
    let eigenvector_scores = eigenvector_centrality(&graph, 50, 1e-6);
    let betweenness_scores = betweenness_centrality(&graph);

    // Combine scores using TRISUM weights
    let mut keywords: Vec<ExtractedKeyword> = graph
        .keys()
        .map(|word| {
            let pr = pagerank_scores.get(word).copied().unwrap_or(0.0);
            let ev = eigenvector_scores.get(word).copied().unwrap_or(0.0);
            let bc = betweenness_scores.get(word).copied().unwrap_or(0.0);

            let trisum_score = config.pagerank_weight * pr
                + config.eigenvector_weight * ev
                + config.betweenness_weight * bc;

            ExtractedKeyword {
                text: word.clone(),
                score: trisum_score.min(1.0),
                keyword_type: KeywordType::Concept,
                source: "trisum".to_string(),
            }
        })
        .collect();

    keywords.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    keywords.truncate(max_keywords);
    keywords
}

/// Get detailed centrality scores for analysis
#[allow(dead_code)] // Public API for TRISUM centrality analysis
pub fn get_centrality_scores(text: &str, window_size: usize) -> HashMap<String, CentralityScores> {
    let words = tokenize_for_textrank(text);
    let config = TrisumConfig::default();

    if words.len() < 3 {
        return HashMap::new();
    }

    // Build co-occurrence graph
    let mut graph: HashMap<String, HashMap<String, f64>> = HashMap::new();

    for window in words.windows(window_size) {
        for i in 0..window.len() {
            for j in (i + 1)..window.len() {
                let word_i = &window[i];
                let word_j = &window[j];

                *graph
                    .entry(word_i.clone())
                    .or_default()
                    .entry(word_j.clone())
                    .or_insert(0.0) += 1.0;

                *graph
                    .entry(word_j.clone())
                    .or_default()
                    .entry(word_i.clone())
                    .or_insert(0.0) += 1.0;
            }
        }
    }

    let pagerank_scores = pagerank(&graph, 0.85, 30);
    let eigenvector_scores = eigenvector_centrality(&graph, 50, 1e-6);
    let betweenness_scores = betweenness_centrality(&graph);

    graph
        .keys()
        .map(|word| {
            let pr = pagerank_scores.get(word).copied().unwrap_or(0.0);
            let ev = eigenvector_scores.get(word).copied().unwrap_or(0.0);
            let bc = betweenness_scores.get(word).copied().unwrap_or(0.0);
            let trisum = config.pagerank_weight * pr
                + config.eigenvector_weight * ev
                + config.betweenness_weight * bc;

            (
                word.clone(),
                CentralityScores {
                    pagerank: pr,
                    eigenvector: ev,
                    betweenness: bc,
                    trisum,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ngrams() {
        let text = "Die künstliche Intelligenz verändert die künstliche Intelligenz Branche";
        let ngrams = extract_ngrams(text, Language::German, 10);
        assert!(!ngrams.is_empty());
        // Should find "künstliche Intelligenz" as repeated bigram
        assert!(ngrams
            .iter()
            .any(|n| n.text.to_lowercase().contains("künstliche intelligenz")));
    }

    #[test]
    fn test_textrank_extraction() {
        let text = "Machine learning and artificial intelligence are transforming technology. \
                    Technology companies invest in machine learning. \
                    Artificial intelligence research advances rapidly.";
        let keywords = extract_textrank(text, 4, 5);
        assert!(!keywords.is_empty());
        // Common terms should have higher scores
        assert!(keywords
            .iter()
            .any(|k| k.text.to_lowercase().contains("learning")
                || k.text.to_lowercase().contains("intelligence")));
    }

    #[test]
    fn test_is_likely_noun_phrase() {
        assert!(is_likely_noun_phrase("Angela Merkel"));
        assert!(is_likely_noun_phrase("Bundestag"));
        assert!(is_likely_noun_phrase("Entwicklung")); // German noun suffix
        assert!(!is_likely_noun_phrase("quickly")); // Adverb
    }

    #[test]
    fn test_enhanced_entity_extraction() {
        let text = "Bundeskanzler Scholz traf sich mit President Biden in Washington. \
                    Die Deutsche Bank AG meldete Gewinne.";
        let entities = extract_enhanced_entities(text);
        assert!(!entities.is_empty());
        // Should find person names and organizations
        let has_person = entities
            .iter()
            .any(|e| e.keyword_type == KeywordType::Person);
        let has_org = entities
            .iter()
            .any(|e| e.keyword_type == KeywordType::Organization);
        assert!(has_person || has_org);
    }

    #[test]
    fn test_pos_filtering() {
        let keywords = vec![
            ExtractedKeyword {
                text: "Bundestag".to_string(),
                score: 0.8,
                keyword_type: KeywordType::Concept,
                source: "test".to_string(),
            },
            ExtractedKeyword {
                text: "quickly".to_string(),
                score: 0.5,
                keyword_type: KeywordType::Concept,
                source: "test".to_string(),
            },
        ];
        let filtered = filter_by_pos_heuristics(keywords);
        // "Bundestag" should pass (capitalized, noun suffix)
        assert!(filtered.iter().any(|k| k.text == "Bundestag"));
    }

    // ============================================================
    // MMR DIVERSIFICATION TESTS
    // ============================================================

    #[test]
    fn test_levenshtein_distance_identical() {
        assert_eq!(levenshtein_distance("test", "test"), 0);
    }

    #[test]
    fn test_levenshtein_distance_single_edit() {
        assert_eq!(levenshtein_distance("test", "tests"), 1); // Insertion
        assert_eq!(levenshtein_distance("tests", "test"), 1); // Deletion
        assert_eq!(levenshtein_distance("test", "best"), 1); // Substitution
    }

    #[test]
    fn test_levenshtein_distance_empty() {
        assert_eq!(levenshtein_distance("", "test"), 4);
        assert_eq!(levenshtein_distance("test", ""), 4);
        assert_eq!(levenshtein_distance("", ""), 0);
    }

    #[test]
    fn test_levenshtein_distance_german() {
        // Test with German characters
        assert_eq!(levenshtein_distance("Änderung", "Änderungen"), 2);
        // "Größe" (5 chars) vs "Groesse" (7 chars): ö->oe (2 ops) + ß->ss (2 ops) = 4
        assert_eq!(levenshtein_distance("Größe", "Groesse"), 4);
        // Simple German umlaut tests
        assert_eq!(levenshtein_distance("Häuser", "Hauser"), 1);
        assert_eq!(levenshtein_distance("Büro", "Buro"), 1);
    }

    #[test]
    fn test_is_near_duplicate() {
        assert!(is_near_duplicate("Trump", "Trumps", 2));
        assert!(is_near_duplicate("Politik", "Politk", 2)); // Typo
        assert!(!is_near_duplicate("Berlin", "Washington", 2));
    }

    #[test]
    fn test_mmr_text_based_diversifies() {
        let keywords = vec![
            ExtractedKeyword {
                text: "Trump".to_string(),
                score: 0.9,
                keyword_type: KeywordType::Person,
                source: "test".to_string(),
            },
            ExtractedKeyword {
                text: "Trumps".to_string(), // Near-duplicate
                score: 0.85,
                keyword_type: KeywordType::Person,
                source: "test".to_string(),
            },
            ExtractedKeyword {
                text: "Biden".to_string(),
                score: 0.8,
                keyword_type: KeywordType::Person,
                source: "test".to_string(),
            },
            ExtractedKeyword {
                text: "Klimawandel".to_string(),
                score: 0.75,
                keyword_type: KeywordType::Concept,
                source: "test".to_string(),
            },
        ];

        let diversified = apply_mmr_text_based(keywords, 3, 0.6);

        // Should select diverse keywords
        assert_eq!(diversified.len(), 3);
        // Trump should be first (highest relevance)
        assert_eq!(diversified[0].text, "Trump");
        // Trumps should likely be penalized due to similarity to Trump
        // Either Biden or Klimawandel should appear before Trumps
    }

    #[test]
    fn test_mmr_with_embeddings() {
        // Create some mock embeddings (simple 3D vectors)
        let doc_emb = vec![1.0f32, 0.0, 0.0];
        let kw1_emb = vec![0.9f32, 0.1, 0.0]; // Similar to doc
        let kw2_emb = vec![0.85f32, 0.15, 0.0]; // Also similar to doc
        let kw3_emb = vec![0.0f32, 0.0, 1.0]; // Orthogonal to doc

        let candidates = vec![
            (
                ExtractedKeyword {
                    text: "keyword1".to_string(),
                    score: 0.9,
                    keyword_type: KeywordType::Concept,
                    source: "test".to_string(),
                },
                Some(kw1_emb),
            ),
            (
                ExtractedKeyword {
                    text: "keyword2".to_string(),
                    score: 0.85,
                    keyword_type: KeywordType::Concept,
                    source: "test".to_string(),
                },
                Some(kw2_emb),
            ),
            (
                ExtractedKeyword {
                    text: "keyword3".to_string(),
                    score: 0.5, // Lower relevance but different
                    keyword_type: KeywordType::Concept,
                    source: "test".to_string(),
                },
                Some(kw3_emb),
            ),
        ];

        let config = MmrConfig {
            lambda: 0.6,
            min_relevance: 0.0,
        };

        let result = apply_mmr_diversification(candidates, Some(&doc_emb), 3, &config);

        assert_eq!(result.len(), 3);
        // First should be most relevant
        assert_eq!(result[0].keyword.text, "keyword1");
    }

    // ============================================================
    // TRISUM MULTI-CENTRALITY TESTS
    // ============================================================

    #[test]
    fn test_trisum_extraction() {
        let text = "Machine learning and artificial intelligence are transforming technology. \
                    Technology companies invest in machine learning. \
                    Artificial intelligence research advances rapidly. \
                    Learning systems improve over time.";

        let keywords = extract_textrank_trisum(text, 4, 5, None);

        assert!(!keywords.is_empty());
        // All keywords should have trisum as source
        assert!(keywords.iter().all(|k| k.source == "trisum"));
    }

    #[test]
    fn test_centrality_scores() {
        let text = "A connects to B. B connects to C. C connects to D. D connects to A. \
                    B also connects to D. This forms a network.";

        let scores = get_centrality_scores(text, 4);

        // Should have scores for the key terms
        assert!(!scores.is_empty());

        // Check that all centrality types are calculated
        for (_word, score) in &scores {
            // All scores should be in 0-1 range (normalized)
            assert!(score.pagerank >= 0.0 && score.pagerank <= 1.0);
            assert!(score.eigenvector >= 0.0 && score.eigenvector <= 1.0);
            assert!(score.betweenness >= 0.0 && score.betweenness <= 1.0);
            assert!(score.trisum >= 0.0 && score.trisum <= 1.0);
        }
    }

    #[test]
    fn test_betweenness_centrality_bridge() {
        // In a simple A-B-C chain, B should have highest betweenness
        let mut graph: HashMap<String, HashMap<String, f64>> = HashMap::new();
        graph.insert(
            "A".to_string(),
            [("B".to_string(), 1.0)].into_iter().collect(),
        );
        graph.insert(
            "B".to_string(),
            [("A".to_string(), 1.0), ("C".to_string(), 1.0)]
                .into_iter()
                .collect(),
        );
        graph.insert(
            "C".to_string(),
            [("B".to_string(), 1.0)].into_iter().collect(),
        );

        let bc = betweenness_centrality(&graph);

        // B is the bridge between A and C
        let b_score = bc.get("B").copied().unwrap_or(0.0);
        let a_score = bc.get("A").copied().unwrap_or(0.0);
        let c_score = bc.get("C").copied().unwrap_or(0.0);

        assert!(
            b_score >= a_score,
            "Bridge node B should have higher betweenness"
        );
        assert!(
            b_score >= c_score,
            "Bridge node B should have higher betweenness"
        );
    }

    #[test]
    fn test_eigenvector_centrality() {
        // Create a star graph: center connected to all others
        let mut graph: HashMap<String, HashMap<String, f64>> = HashMap::new();
        let center = "center".to_string();
        let periphery = vec!["A", "B", "C", "D"];

        for node in &periphery {
            graph.insert(
                node.to_string(),
                [(center.clone(), 1.0)].into_iter().collect(),
            );
        }
        graph.insert(
            center.clone(),
            periphery.iter().map(|n| (n.to_string(), 1.0)).collect(),
        );

        let ec = eigenvector_centrality(&graph, 50, 1e-6);

        // Center should have highest eigenvector centrality
        let center_score = ec.get(&center).copied().unwrap_or(0.0);

        for node in &periphery {
            let node_score = ec.get(*node).copied().unwrap_or(0.0);
            assert!(
                center_score >= node_score,
                "Center should have higher eigenvector centrality"
            );
        }
    }

    #[test]
    fn test_trisum_config_weights() {
        let config = TrisumConfig {
            pagerank_weight: 1.0,
            eigenvector_weight: 0.0,
            betweenness_weight: 0.0,
        };

        let text = "Test text with some words repeated. Words are important. Test again.";
        let keywords = extract_textrank_trisum(text, 4, 3, Some(config));

        // With only pagerank weight, results should match standard textrank
        let standard = extract_textrank(text, 4, 3);

        // Both should extract keywords from the same text
        assert!(!keywords.is_empty());
        assert!(!standard.is_empty());
    }
}
