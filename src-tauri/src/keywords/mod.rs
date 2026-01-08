use keyword_extraction::rake::{Rake, RakeParams};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use whatlang::{detect, Lang};
use yake_rust::{get_n_best, Config, StopWords};

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

static STOPWORDS_DE: Lazy<HashSet<String>> = Lazy::new(|| {
    include_str!("stopwords_de.txt")
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|s| s.to_lowercase())
        .collect()
});

static STOPWORDS_EN: Lazy<HashSet<String>> = Lazy::new(|| {
    include_str!("stopwords_en.txt")
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|s| s.to_lowercase())
        .collect()
});

static NEWS_STOPWORDS: Lazy<HashSet<String>> = Lazy::new(|| {
    [
        "bericht",
        "sagt",
        "laut",
        "unterdessen",
        "heute",
        "gestern",
        "video",
        "update",
        "interview",
        "kommentar",
        "mehr",
        "neue",
        "ersten",
        "lesen",
        "artikel",
        "news",
        "uhr",
        "foto",
        "quelle",
        "dpa",
        "afp",
        "reuters",
        "report",
        "says",
        "according",
        "today",
        "yesterday",
        "comment",
        "read",
        "article",
        "source",
        "photo",
        "breaking",
        "exclusive",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
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

    fn get_stopwords(lang: Language) -> &'static HashSet<String> {
        match lang {
            Language::German => &STOPWORDS_DE,
            Language::English => &STOPWORDS_EN,
        }
    }

    pub fn extract(&self, title: &str, content: &str) -> Vec<ExtractedKeyword> {
        let full_text = format!("{}\n\n{}", title, content);
        let lang = Self::detect_language(&full_text);
        let stopwords = Self::get_stopwords(lang);

        let mut candidates: HashMap<String, ExtractedKeyword> = HashMap::new();

        for kw in self.extract_yake(&full_text, lang) {
            let key = kw.text.to_lowercase();
            candidates.entry(key).or_insert(kw);
        }

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

        let title_lower = title.to_lowercase();
        for candidate in candidates.values_mut() {
            if title_lower.contains(&candidate.text.to_lowercase()) {
                candidate.score += 0.25;
            }
        }

        let mut filtered: Vec<ExtractedKeyword> = candidates
            .into_values()
            .filter(|kw| self.is_valid_keyword(&kw.text, stopwords))
            .collect();

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
                && !STOPWORDS_DE.contains(&phrase.to_lowercase())
                && !STOPWORDS_EN.contains(&phrase.to_lowercase())
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
        if NEWS_STOPWORDS.contains(&lower) {
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

pub fn normalize_keyword(keyword: &str) -> Option<String> {
    let trimmed = keyword.trim();

    if trimmed.len() < 2 || trimmed.len() > 50 {
        return None;
    }

    if trimmed
        .chars()
        .all(|c| c.is_numeric() || c.is_whitespace() || c == '.' || c == ',')
    {
        return None;
    }

    let dominated_by_special = {
        let special_count = trimmed
            .chars()
            .filter(|c| !c.is_alphanumeric() && !c.is_whitespace())
            .count();
        special_count > trimmed.len() / 3
    };
    if dominated_by_special {
        return None;
    }

    if KNOWN_ACRONYMS.contains(trimmed) {
        return Some(trimmed.to_string());
    }

    let words: Vec<&str> = trimmed.split_whitespace().collect();
    let is_proper_noun = words.len() <= 3
        && words
            .iter()
            .all(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && w.len() >= 2);

    if is_proper_noun {
        return Some(trimmed.to_string());
    }

    Some(trimmed.to_lowercase())
}

pub fn normalize_and_dedupe_keywords(keywords: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    for kw in keywords {
        if let Some(normalized) = normalize_keyword(kw) {
            let key = normalized.to_lowercase();
            if seen.insert(key) {
                result.push(normalized);
            }
        }
    }

    result
}
