//! Advanced Keyword Extraction Methods
//!
//! This module provides additional keyword extraction techniques:
//! - N-gram extraction (bigrams/trigrams)
//! - POS-like filtering using heuristics
//! - TextRank graph-based extraction
//! - Enhanced Named Entity Recognition

use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

use super::{ExtractedKeyword, KeywordType, Language, UNIFIED_STOPWORDS};

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
        if is_valid_ngram_sequence(&window, lang) {
            let ngram = window.join(" ");
            *ngram_counts.entry(ngram).or_insert(0) += 1;
        }
    }

    // Extract trigrams
    for window in words.windows(3) {
        if is_valid_ngram_sequence(&window, lang) {
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

    ngrams.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
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

    // First and last word should preferably not be stopwords
    let first_is_stop = stopwords.contains(&words.first().unwrap().to_lowercase());
    let last_is_stop = stopwords.contains(&words.last().unwrap().to_lowercase());

    if first_is_stop && last_is_stop {
        return false;
    }

    // Check for proper noun patterns (capitalized words)
    let has_proper_noun = words.iter().any(|w| {
        w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && w.len() >= 3
    });

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
    let all_capitalized = words.iter().all(|w| {
        w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
    });

    if all_capitalized && words.len() >= 2 {
        return true;
    }

    // Known phrase patterns
    let lower = phrase.to_lowercase();
    SIGNIFICANT_PHRASES.iter().any(|p| lower.contains(p))
}

/// Known significant phrase patterns
static SIGNIFICANT_PHRASES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // German political/news phrases
        "künstliche intelligenz", "climate change", "klimawandel",
        "bundesregierung", "bundestag", "europäische union", "vereinigte staaten",
        "social media", "soziale medien", "machine learning", "deep learning",
        "fake news", "cyber security", "data protection", "datenschutz",
        // Common news compound concepts
        "supply chain", "lieferkette", "interest rate", "zinssatz",
        "central bank", "zentralbank", "foreign policy", "außenpolitik",
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
        return word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            || word.chars().any(|c| c.is_numeric())
            || is_known_noun_pattern(word);
    }

    // Multi-word checks
    // At least one word should look like a noun
    words.iter().any(|w| {
        w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            || is_known_noun_pattern(w)
    })
}

/// Check if a word matches known noun patterns
fn is_known_noun_pattern(word: &str) -> bool {
    let lower = word.to_lowercase();

    // German noun suffixes
    let de_suffixes = [
        "ung", "heit", "keit", "schaft", "tum", "nis", "sal", "ion",
        "ität", "ie", "ik", "ur", "eur", "ist", "ent", "ant",
    ];

    // English noun suffixes
    let en_suffixes = [
        "tion", "sion", "ment", "ness", "ity", "ance", "ence",
        "ship", "dom", "hood", "ist", "ism", "er", "or",
    ];

    de_suffixes.iter().any(|s| lower.ends_with(s))
        || en_suffixes.iter().any(|s| lower.ends_with(s))
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
pub fn extract_textrank(text: &str, window_size: usize, max_keywords: usize) -> Vec<ExtractedKeyword> {
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

    keywords.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
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
    let mut scores: HashMap<String, f64> = nodes.iter().map(|n| (n.clone(), initial_score)).collect();

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
static LOCATION_INDICATORS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // German
        "stadt", "land", "region", "bezirk", "kreis", "bundesland",
        "provinz", "staat", "republik", "königreich", "gebiet",
        // English
        "city", "country", "region", "state", "province", "republic",
        "kingdom", "district", "area", "territory",
    ]
    .iter()
    .copied()
    .collect()
});

static PERSON_TITLES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // German titles
        "herr", "frau", "dr", "prof", "minister", "ministerin",
        "präsident", "präsidentin", "kanzler", "kanzlerin",
        "bundeskanzler", "bundeskanzlerin", "bürgermeister", "bürgermeisterin",
        // English titles
        "mr", "mrs", "ms", "dr", "prof", "president", "chancellor",
        "minister", "secretary", "senator", "governor", "mayor",
        "ceo", "cfo", "cto",
    ]
    .iter()
    .copied()
    .collect()
});

static EVENT_PATTERNS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // German
        "gipfel", "konferenz", "treffen", "verhandlungen", "wahl", "wahlen",
        "abstimmung", "referendum", "protest", "demonstration", "streik",
        "kongress", "festival", "messe", "ausstellung",
        // English
        "summit", "conference", "meeting", "negotiations", "election", "elections",
        "vote", "referendum", "protest", "demonstration", "strike",
        "congress", "festival", "fair", "exhibition",
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

    // Pattern: Title + Capitalized Name(s)
    let title_pattern = regex::Regex::new(
        r"(?i)\b((?:herr|frau|dr\.?|prof\.?|minister(?:in)?|präsident(?:in)?|kanzler(?:in)?|mr\.?|mrs\.?|ms\.?|president|chancellor|senator)\s+)([A-ZÄÖÜ][a-zäöüß]+(?:\s+[A-ZÄÖÜ][a-zäöüß]+){0,2})\b"
    ).unwrap();

    for cap in title_pattern.captures_iter(text) {
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

    // Pattern: Capitalized words + legal form
    let org_pattern = regex::Regex::new(
        r"\b([A-ZÄÖÜ][a-zäöüß]*(?:\s+[A-ZÄÖÜ&][a-zäöüß]*){0,4})\s+(GmbH|AG|Inc\.?|Ltd\.?|Corp\.?|Co\.?|KG|e\.V\.?|SE)\b"
    ).unwrap();

    for cap in org_pattern.captures_iter(text) {
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

    // Pattern: Ministry/Department names
    let ministry_pattern = regex::Regex::new(
        r"\b((?:Bundes)?[A-ZÄÖÜ][a-zäöüß]*(?:ministerium|ministry|department|behörde|amt|agency))\b"
    ).unwrap();

    for cap in ministry_pattern.captures_iter(text) {
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

    // Pattern: "in/aus/nach [Location]"
    let loc_pattern = regex::Regex::new(
        r"(?i)\b(?:in|aus|nach|from|to|at)\s+([A-ZÄÖÜ][a-zäöüß]+(?:\s+[A-ZÄÖÜ][a-zäöüß]+)?)\b"
    ).unwrap();

    for cap in loc_pattern.captures_iter(text) {
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
        "der", "die", "das", "dem", "den", "the", "a", "an",
        "diesem", "dieser", "dieses", "this", "that",
        "einem", "einer", "eines", "one", "some",
    ];
    non_locations.contains(&word.to_lowercase().as_str())
}

/// Extract event-related entities
fn extract_events(text: &str) -> Vec<ExtractedKeyword> {
    let mut events = Vec::new();

    // Pattern: Event type + location/year
    let event_pattern = regex::Regex::new(
        r"\b([A-ZÄÖÜ][a-zäöüß]*[-\s]?(?:gipfel|konferenz|summit|conference|wahlen?|election|treffen|meeting))\b"
    ).unwrap();

    for cap in event_pattern.captures_iter(text) {
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

    // Year patterns
    let year_context_pattern = regex::Regex::new(
        r"\b([A-ZÄÖÜ][a-zäöüß]+(?:\s+[A-ZÄÖÜ][a-zäöüß]+)?)\s+(20\d{2}|19\d{2})\b"
    ).unwrap();

    for cap in year_context_pattern.captures_iter(text) {
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
#[derive(Debug, Clone)]
pub struct SemanticCandidate {
    pub text: String,
    pub base_score: f64,
    pub keyword_type: KeywordType,
    pub source: String,
}

/// Prepare candidates for semantic scoring
/// This returns candidates that can be scored against the full document embedding
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
pub fn apply_semantic_scores(
    candidates: Vec<SemanticCandidate>,
    similarity_scores: &HashMap<String, f64>,
    semantic_weight: f64,
) -> Vec<ExtractedKeyword> {
    candidates
        .into_iter()
        .map(|c| {
            let semantic_score = similarity_scores.get(&c.text).copied().unwrap_or(0.5);
            let combined_score = (c.base_score * (1.0 - semantic_weight))
                + (semantic_score * semantic_weight);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ngrams() {
        let text = "Die künstliche Intelligenz verändert die künstliche Intelligenz Branche";
        let ngrams = extract_ngrams(text, Language::German, 10);
        assert!(!ngrams.is_empty());
        // Should find "künstliche Intelligenz" as repeated bigram
        assert!(ngrams.iter().any(|n| n.text.to_lowercase().contains("künstliche intelligenz")));
    }

    #[test]
    fn test_textrank_extraction() {
        let text = "Machine learning and artificial intelligence are transforming technology. \
                    Technology companies invest in machine learning. \
                    Artificial intelligence research advances rapidly.";
        let keywords = extract_textrank(text, 4, 5);
        assert!(!keywords.is_empty());
        // Common terms should have higher scores
        assert!(keywords.iter().any(|k| k.text.to_lowercase().contains("learning")
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
        let has_person = entities.iter().any(|e| e.keyword_type == KeywordType::Person);
        let has_org = entities.iter().any(|e| e.keyword_type == KeywordType::Organization);
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
}
