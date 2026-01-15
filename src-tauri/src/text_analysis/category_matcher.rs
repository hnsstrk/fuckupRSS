//! Category matching via word frequency analysis
//!
//! Matches article text against predefined category word lists to suggest categories.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use unicode_segmentation::UnicodeSegmentation;

use super::stopwords::STOPWORDS;
use super::bias::BiasWeights;

/// A term with base and learned weights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedTerm {
    pub term: String,
    pub base_weight: f64,
}

/// Category score result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryScore {
    /// Sephiroth ID (subcategory level)
    pub sephiroth_id: i64,
    /// Category name
    pub name: String,
    /// Combined score
    pub score: f64,
    /// Normalized score (0.0 - 1.0)
    pub confidence: f64,
    /// Terms that matched
    pub matching_terms: Vec<String>,
}

/// Category word lists for statistical matching
/// Maps subcategory ID to weighted terms
static CATEGORY_TERMS: LazyLock<HashMap<i64, (&'static str, Vec<WeightedTerm>)>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // 101: Technik (parent: 1 - Wissen & Technologie)
    m.insert(101, ("Technik", vec![
        wt("software", 1.5), wt("hardware", 1.5), wt("app", 1.3), wt("digital", 1.2),
        wt("computer", 1.3), wt("smartphone", 1.3), wt("internet", 1.2), wt("online", 1.0),
        wt("cloud", 1.2), wt("server", 1.2), wt("daten", 1.1), wt("algorithmus", 1.3),
        wt("programmierung", 1.4), wt("entwickler", 1.2), wt("künstliche", 1.3),
        wt("intelligenz", 1.2), wt("maschinelles", 1.3), wt("lernen", 0.8),
        wt("blockchain", 1.4), wt("kryptowährung", 1.3), wt("bitcoin", 1.3),
        wt("cybersecurity", 1.4), wt("hacker", 1.2), wt("virus", 1.0),
        wt("startup", 1.1), wt("innovation", 1.0), wt("technologie", 1.2),
    ]));

    // 102: Wissenschaft (parent: 1 - Wissen & Technologie)
    m.insert(102, ("Wissenschaft", vec![
        wt("forschung", 1.5), wt("wissenschaft", 1.5), wt("studie", 1.3), wt("experiment", 1.3),
        wt("labor", 1.2), wt("universität", 1.2), wt("professor", 1.1), wt("doktor", 0.8),
        wt("physik", 1.4), wt("chemie", 1.4), wt("biologie", 1.4), wt("mathematik", 1.3),
        wt("astronaut", 1.3), wt("weltraum", 1.3), wt("mars", 1.2), wt("mond", 1.1),
        wt("entdeckung", 1.2), wt("theorie", 1.1), wt("hypothese", 1.2),
        wt("nobelpreis", 1.4), wt("genom", 1.3), wt("evolution", 1.2),
    ]));

    // 201: Politik (parent: 2 - Politik & Gesellschaft)
    m.insert(201, ("Politik", vec![
        wt("regierung", 1.5), wt("parlament", 1.5), wt("bundestag", 1.5), wt("bundesrat", 1.4),
        wt("minister", 1.3), wt("kanzler", 1.4), wt("präsident", 1.3), wt("opposition", 1.3),
        wt("koalition", 1.4), wt("partei", 1.3), wt("wahl", 1.3), wt("abstimmung", 1.2),
        wt("gesetz", 1.2), wt("reform", 1.1), wt("politik", 1.4), wt("politisch", 1.2),
        wt("demokratie", 1.3), wt("demokratisch", 1.2), wt("abgeordnete", 1.3),
        wt("fraktion", 1.3), wt("cdu", 1.2), wt("spd", 1.2), wt("grüne", 1.1),
        wt("fdp", 1.2), wt("afd", 1.2), wt("linke", 1.0),
    ]));

    // 202: Gesellschaft (parent: 2 - Politik & Gesellschaft)
    m.insert(202, ("Gesellschaft", vec![
        wt("gesellschaft", 1.4), wt("sozial", 1.2), wt("migration", 1.3), wt("flüchtling", 1.3),
        wt("integration", 1.2), wt("bildung", 1.2), wt("schule", 1.1), wt("universität", 1.0),
        wt("gleichstellung", 1.3), wt("diskriminierung", 1.3), wt("rassismus", 1.3),
        wt("protest", 1.2), wt("demonstration", 1.3), wt("aktivist", 1.2),
        wt("religion", 1.2), wt("kirche", 1.1), wt("islam", 1.2), wt("christentum", 1.1),
        wt("familie", 1.0), wt("jugend", 1.0), wt("senioren", 1.0),
    ]));

    // 203: Recht (parent: 2 - Politik & Gesellschaft)
    m.insert(203, ("Recht", vec![
        wt("gericht", 1.5), wt("richter", 1.4), wt("urteil", 1.4), wt("anklage", 1.3),
        wt("staatsanwalt", 1.4), wt("verteidiger", 1.2), wt("anwalt", 1.2),
        wt("gesetz", 1.2), wt("paragraph", 1.3), wt("verfassung", 1.4),
        wt("grundgesetz", 1.4), wt("rechtsprechung", 1.4), wt("justiz", 1.4),
        wt("straftat", 1.3), wt("verbrechen", 1.2), wt("klage", 1.2),
        wt("prozess", 1.1), wt("berufung", 1.2), wt("revision", 1.2),
        wt("datenschutz", 1.2), wt("urheberrecht", 1.3), wt("patent", 1.2),
    ]));

    // 301: Wirtschaft (parent: 3 - Wirtschaft)
    m.insert(301, ("Wirtschaft", vec![
        wt("wirtschaft", 1.4), wt("unternehmen", 1.3), wt("firma", 1.2), wt("konzern", 1.3),
        wt("aktie", 1.4), wt("börse", 1.4), wt("dax", 1.4), wt("kurs", 1.2),
        wt("investition", 1.3), wt("investor", 1.3), wt("anleger", 1.3),
        wt("umsatz", 1.3), wt("gewinn", 1.2), wt("verlust", 1.1), wt("bilanz", 1.3),
        wt("handel", 1.2), wt("export", 1.3), wt("import", 1.3), wt("zoll", 1.2),
        wt("inflation", 1.4), wt("rezession", 1.4), wt("wachstum", 1.2),
        wt("arbeitsmarkt", 1.2), wt("arbeitslosigkeit", 1.3), wt("konjunktur", 1.4),
    ]));

    // 302: Energie (parent: 3 - Wirtschaft)
    m.insert(302, ("Energie", vec![
        wt("energie", 1.5), wt("strom", 1.3), wt("gas", 1.2), wt("öl", 1.2),
        wt("erneuerbar", 1.4), wt("solar", 1.4), wt("windkraft", 1.4), wt("windrad", 1.3),
        wt("atomkraft", 1.4), wt("kernkraft", 1.4), wt("kraftwerk", 1.3),
        wt("energiewende", 1.5), wt("kohle", 1.3), wt("kohleausstieg", 1.4),
        wt("photovoltaik", 1.4), wt("wasserstoff", 1.4), wt("batterie", 1.2),
        wt("netzausbau", 1.3), wt("strompreis", 1.3), wt("energiekosten", 1.3),
    ]));

    // 401: Umwelt (parent: 4 - Umwelt & Gesundheit)
    m.insert(401, ("Umwelt", vec![
        wt("umwelt", 1.5), wt("klima", 1.5), wt("klimawandel", 1.5), wt("erderwärmung", 1.5),
        wt("naturschutz", 1.4), wt("artenschutz", 1.4), wt("biodiversität", 1.4),
        wt("nachhaltigkeit", 1.3), wt("nachhaltig", 1.2), wt("ökologie", 1.3),
        wt("emission", 1.4), wt("treibhausgas", 1.4), wt("kohlendioxid", 1.3),
        wt("verschmutzung", 1.3), wt("plastik", 1.2), wt("müll", 1.1), wt("recycling", 1.2),
        wt("wald", 1.1), wt("regenwald", 1.3), wt("meer", 1.0), wt("ozean", 1.1),
    ]));

    // 402: Gesundheit (parent: 4 - Umwelt & Gesundheit)
    m.insert(402, ("Gesundheit", vec![
        wt("gesundheit", 1.5), wt("krankheit", 1.3), wt("patient", 1.3), wt("arzt", 1.2),
        wt("krankenhaus", 1.3), wt("klinik", 1.3), wt("medizin", 1.4), wt("medizinisch", 1.2),
        wt("impfung", 1.4), wt("impfstoff", 1.4), wt("corona", 1.3), wt("covid", 1.3),
        wt("pandemie", 1.4), wt("epidemie", 1.3), wt("virus", 1.2), wt("infektion", 1.2),
        wt("therapie", 1.3), wt("behandlung", 1.2), wt("medikament", 1.3),
        wt("pflege", 1.2), wt("pflegekraft", 1.3), wt("krankenkasse", 1.2),
    ]));

    // 501: Sicherheit (parent: 5 - Sicherheit)
    m.insert(501, ("Sicherheit", vec![
        wt("sicherheit", 1.4), wt("polizei", 1.4), wt("kriminalität", 1.4), wt("verbrechen", 1.3),
        wt("terrorismus", 1.5), wt("terrorist", 1.4), wt("anschlag", 1.4),
        wt("geheimdienst", 1.4), wt("spionage", 1.4), wt("überwachung", 1.3),
        wt("cyberkriminalität", 1.5), wt("cyberangriff", 1.5), wt("hacker", 1.3),
        wt("gefängnis", 1.2), wt("festnahme", 1.3), wt("verhaftung", 1.3),
        wt("droge", 1.2), wt("drogenhandel", 1.3), wt("organisiert", 1.0),
    ]));

    // 502: Verteidigung (parent: 5 - Sicherheit)
    m.insert(502, ("Verteidigung", vec![
        wt("verteidigung", 1.4), wt("militär", 1.5), wt("armee", 1.4), wt("bundeswehr", 1.5),
        wt("soldat", 1.4), wt("truppen", 1.3), wt("einsatz", 1.1), wt("manöver", 1.3),
        wt("krieg", 1.4), wt("konflikt", 1.2), wt("waffe", 1.3), wt("rüstung", 1.4),
        wt("nato", 1.5), wt("bündnis", 1.2), wt("verteidigungsminister", 1.4),
        wt("panzer", 1.3), wt("kampfjet", 1.3), wt("marine", 1.3), wt("luftwaffe", 1.3),
    ]));

    // 601: Kultur (parent: 6 - Kultur & Leben)
    m.insert(601, ("Kultur", vec![
        wt("kultur", 1.4), wt("kunst", 1.4), wt("museum", 1.3), wt("galerie", 1.3),
        wt("theater", 1.3), wt("oper", 1.3), wt("konzert", 1.2), wt("festival", 1.2),
        wt("film", 1.2), wt("kino", 1.2), wt("regisseur", 1.3), wt("schauspieler", 1.2),
        wt("musik", 1.2), wt("musiker", 1.2), wt("album", 1.1), wt("band", 0.8),
        wt("literatur", 1.3), wt("buch", 1.0), wt("autor", 1.2), wt("roman", 1.2),
        wt("ausstellung", 1.3), wt("künstler", 1.3), wt("gemälde", 1.3),
    ]));

    // 602: Sport (parent: 6 - Kultur & Leben)
    m.insert(602, ("Sport", vec![
        wt("sport", 1.4), wt("sportler", 1.3), wt("athlet", 1.3), wt("olympia", 1.4),
        wt("fußball", 1.5), wt("bundesliga", 1.5), wt("meisterschaft", 1.3),
        wt("torwart", 1.3), wt("stürmer", 1.2), wt("trainer", 1.2), wt("verein", 1.1),
        wt("tennis", 1.3), wt("basketball", 1.3), wt("handball", 1.3), wt("hockey", 1.3),
        wt("formel", 1.2), wt("rennen", 1.1), wt("marathon", 1.3), wt("triathlon", 1.3),
        wt("weltmeister", 1.4), wt("europameister", 1.4), wt("medaille", 1.3),
        wt("rekord", 1.2), wt("sieg", 1.1), wt("niederlage", 1.1), wt("spiel", 1.0),
    ]));

    m
});

fn wt(term: &'static str, base_weight: f64) -> WeightedTerm {
    WeightedTerm {
        term: term.to_string(),
        base_weight,
    }
}

/// Category matcher using word frequency analysis
pub struct CategoryMatcher {
    /// Minimum score threshold for a category to be considered
    pub min_score: f64,
    /// Maximum number of categories to return
    pub max_categories: usize,
    /// Minimum confidence threshold
    pub min_confidence: f64,
}

impl Default for CategoryMatcher {
    fn default() -> Self {
        Self {
            min_score: 1.0,
            max_categories: 3,
            min_confidence: 0.2,
        }
    }
}

impl CategoryMatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure minimum score threshold
    #[allow(dead_code)]
    pub fn with_min_score(mut self, score: f64) -> Self {
        self.min_score = score;
        self
    }

    /// Configure maximum categories to return
    pub fn with_max_categories(mut self, max: usize) -> Self {
        self.max_categories = max;
        self
    }

    /// Tokenize and normalize text for matching
    fn tokenize(&self, text: &str) -> Vec<String> {
        text.unicode_words()
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() >= 3 && !STOPWORDS.contains(w.as_str()))
            .collect()
    }

    /// Score categories based on word frequency analysis
    pub fn score_categories(&self, text: &str, bias: Option<&BiasWeights>) -> Vec<CategoryScore> {
        let tokens = self.tokenize(text);
        if tokens.is_empty() {
            return Vec::new();
        }

        // Build frequency map
        let mut freq: HashMap<&str, u32> = HashMap::new();
        for token in &tokens {
            *freq.entry(token.as_str()).or_insert(0) += 1;
        }

        let mut scores: Vec<CategoryScore> = Vec::new();
        let mut max_score: f64 = 0.0;

        // Score each category
        for (sephiroth_id, (name, terms)) in CATEGORY_TERMS.iter() {
            let mut score = 0.0;
            let mut matching_terms = Vec::new();

            for term in terms {
                if let Some(&count) = freq.get(term.term.as_str()) {
                    let mut weight = term.base_weight;

                    // Apply learned bias if available
                    if let Some(bias) = bias {
                        weight *= bias.get_category_term_weight(*sephiroth_id, &term.term);
                    }

                    score += count as f64 * weight;
                    matching_terms.push(term.term.clone());
                }
            }

            if score > 0.0 {
                if score > max_score {
                    max_score = score;
                }
                scores.push(CategoryScore {
                    sephiroth_id: *sephiroth_id,
                    name: name.to_string(),
                    score,
                    confidence: 0.0, // Will be normalized later
                    matching_terms,
                });
            }
        }

        // Normalize confidence scores
        if max_score > 0.0 {
            for cat in &mut scores {
                cat.confidence = cat.score / max_score;
            }
        }

        // Filter by minimum score and confidence
        scores.retain(|c| c.score >= self.min_score && c.confidence >= self.min_confidence);

        // Sort by score descending
        scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Return top N categories
        scores.truncate(self.max_categories);
        scores
    }

    /// Get all available category IDs
    #[allow(dead_code)]
    pub fn get_category_ids() -> Vec<i64> {
        CATEGORY_TERMS.keys().copied().collect()
    }

    /// Get category name by ID
    #[allow(dead_code)]
    pub fn get_category_name(id: i64) -> Option<&'static str> {
        CATEGORY_TERMS.get(&id).map(|(name, _)| *name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_categories_politics() {
        let matcher = CategoryMatcher::new();
        let text = "Die Regierung plant neue Gesetze. Der Bundestag debattiert über die Reform. \
                    Die Opposition kritisiert den Koalitionsvertrag. Der Minister verteidigt die Politik.";

        let scores = matcher.score_categories(text, None);

        assert!(!scores.is_empty());
        // Politik (201) should be the top category
        assert_eq!(scores[0].sephiroth_id, 201);
        assert!(scores[0].matching_terms.contains(&"regierung".to_string()));
    }

    #[test]
    fn test_score_categories_tech() {
        let matcher = CategoryMatcher::new();
        let text = "Das neue Smartphone bietet innovative Software-Features. \
                    Die App nutzt künstliche Intelligenz und maschinelles Lernen. \
                    Der Algorithmus analysiert die Daten in der Cloud.";

        let scores = matcher.score_categories(text, None);

        assert!(!scores.is_empty());
        // Technik (101) should be the top category
        assert_eq!(scores[0].sephiroth_id, 101);
    }

    #[test]
    fn test_score_categories_sport() {
        let matcher = CategoryMatcher::new();
        let text = "Der Fußball-Bundesliga-Meister feiert den Sieg. \
                    Der Trainer lobt den Torwart und die Stürmer. \
                    Der Verein plant weitere Verstärkungen für die neue Saison.";

        let scores = matcher.score_categories(text, None);

        assert!(!scores.is_empty());
        // Sport (602) should be the top category
        assert_eq!(scores[0].sephiroth_id, 602);
    }

    #[test]
    fn test_multiple_categories() {
        let matcher = CategoryMatcher::new().with_max_categories(5);
        let text = "Die Regierung investiert in erneuerbare Energien. \
                    Das Wirtschaftsministerium fördert Solaranlagen und Windkraft. \
                    Die Energiewende ist ein politisches Ziel.";

        let scores = matcher.score_categories(text, None);

        // Should detect multiple categories
        assert!(scores.len() >= 2);
        // Should include Politik and/or Energie
        let category_ids: Vec<i64> = scores.iter().map(|s| s.sephiroth_id).collect();
        assert!(category_ids.contains(&201) || category_ids.contains(&302));
    }

    #[test]
    fn test_empty_text() {
        let matcher = CategoryMatcher::new();
        let scores = matcher.score_categories("", None);
        assert!(scores.is_empty());
    }

    #[test]
    fn test_no_matching_terms() {
        let matcher = CategoryMatcher::new();
        let text = "abc xyz qwerty asdf";
        let scores = matcher.score_categories(text, None);
        assert!(scores.is_empty());
    }
}
