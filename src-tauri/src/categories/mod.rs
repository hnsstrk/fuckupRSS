use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(test)]
mod tests;

pub const SEPHIROTH_CATEGORIES: &[&str] = &[
    "Technik",
    "Politik",
    "Wirtschaft",
    "Wissenschaft",
    "Kultur",
    "Sport",
    "Gesellschaft",
    "Umwelt",
    "Sicherheit",
    "Gesundheit",
    "Verteidigung",
    "Energie",
    "Recht",
];

pub const CATEGORY_CLASSIFICATION_MODEL: &str = "qwen2.5:1.5b";

const CATEGORY_PROMPT_DE: &str = r#"Du bist ein Nachrichtenklassifikator. Ordne den folgenden Artikel den passenden Kategorien zu.

VERFÜGBARE KATEGORIEN:
- Technik: IT, Software, Hardware, Internet, KI, Digitalisierung
- Politik: Innenpolitik, Außenpolitik, Parteien, Wahlen, Gesetze
- Wirtschaft: Unternehmen, Börse, Finanzen, Handel, Arbeitsmarkt
- Wissenschaft: Forschung, Studien, Entdeckungen, Raumfahrt
- Kultur: Kunst, Musik, Film, Theater, Literatur, Medien
- Sport: Fußball, Tennis, Olympia, Motorsport, alle Sportarten
- Gesellschaft: Soziales, Familie, Bildung, Migration, Demografie
- Umwelt: Klima, Naturschutz, Nachhaltigkeit, Wetter
- Sicherheit: Kriminalität, Polizei, Terrorismus, Cybersecurity
- Gesundheit: Medizin, Krankheiten, Gesundheitswesen, Pharma
- Verteidigung: Militär, Bundeswehr, NATO, Konflikte, Krieg
- Energie: Strom, Gas, Erneuerbare, Atomkraft, Energiewende
- Recht: Justiz, Gerichte, Urteile, Rechtsstreit, Datenschutz

REGELN:
- Wähle 1-5 passende Kategorien
- Nur Kategorien aus der obigen Liste
- Sortiere nach Relevanz (wichtigste zuerst)
- Antworte NUR mit den Kategorienamen, kommagetrennt

ARTIKEL:
Titel: {title}
Inhalt: {content}

KATEGORIEN:"#;

const CATEGORY_PROMPT_EN: &str = r#"You are a news classifier. Assign the following article to matching categories.

AVAILABLE CATEGORIES:
- Technik: IT, software, hardware, internet, AI, digitalization
- Politik: domestic policy, foreign policy, parties, elections, laws
- Wirtschaft: companies, stock market, finance, trade, labor market
- Wissenschaft: research, studies, discoveries, space
- Kultur: art, music, film, theater, literature, media
- Sport: football, tennis, Olympics, motorsport, all sports
- Gesellschaft: social issues, family, education, migration, demographics
- Umwelt: climate, nature conservation, sustainability, weather
- Sicherheit: crime, police, terrorism, cybersecurity
- Gesundheit: medicine, diseases, healthcare, pharma
- Verteidigung: military, defense, NATO, conflicts, war
- Energie: electricity, gas, renewables, nuclear, energy transition
- Recht: justice, courts, verdicts, legal disputes, data protection

RULES:
- Choose 1-5 matching categories
- Only categories from the list above
- Sort by relevance (most important first)
- Answer ONLY with category names, comma-separated

ARTICLE:
Title: {title}
Content: {content}

CATEGORIES:"#;

#[derive(Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: GenerateOptions,
}

#[derive(Serialize)]
struct GenerateOptions {
    num_ctx: u32,
    temperature: f32,
    top_p: f32,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

pub struct CategoryClassifier {
    base_url: String,
    model: String,
}

impl Default for CategoryClassifier {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl CategoryClassifier {
    pub fn new(base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: model.unwrap_or_else(|| CATEGORY_CLASSIFICATION_MODEL.to_string()),
        }
    }

    fn client(&self) -> reqwest_new::Client {
        reqwest_new::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client")
    }

    pub async fn classify(
        &self,
        title: &str,
        content: &str,
        locale: &str,
    ) -> Result<Vec<String>, String> {
        let prompt_template = match locale {
            "en" => CATEGORY_PROMPT_EN,
            _ => CATEGORY_PROMPT_DE,
        };

        let truncated_content: String = content.chars().take(2000).collect();
        let prompt = prompt_template
            .replace("{title}", title)
            .replace("{content}", &truncated_content);

        let response = self.generate(&prompt).await?;
        let categories = self.parse_categories(&response);

        Ok(categories)
    }

    async fn generate(&self, prompt: &str) -> Result<String, String> {
        let url = format!("{}/api/generate", self.base_url);
        let client = self.client();

        let request = GenerateRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options: GenerateOptions {
                num_ctx: 4096,
                temperature: 0.1,
                top_p: 0.9,
            },
        };

        let resp = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }

        let result: GenerateResponse = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        Ok(result.response)
    }

    fn parse_categories(&self, response: &str) -> Vec<String> {
        let cleaned = response
            .trim()
            .trim_start_matches("KATEGORIEN:")
            .trim_start_matches("CATEGORIES:")
            .trim();

        let mut seen = std::collections::HashSet::new();
        let categories: Vec<String> = cleaned
            .split([',', '\n', ';'])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .filter_map(|s| {
                let normalized = self.normalize_category(&s);
                if SEPHIROTH_CATEGORIES.contains(&normalized.as_str())
                    && seen.insert(normalized.clone())
                {
                    Some(normalized)
                } else {
                    None
                }
            })
            .take(5)
            .collect();

        categories
    }

    fn normalize_category(&self, input: &str) -> String {
        let lower = input.trim().to_lowercase();

        for cat in SEPHIROTH_CATEGORIES {
            if cat.to_lowercase() == lower {
                return cat.to_string();
            }
        }

        let english_to_german = [
            ("technology", "Technik"),
            ("tech", "Technik"),
            ("politics", "Politik"),
            ("political", "Politik"),
            ("economy", "Wirtschaft"),
            ("business", "Wirtschaft"),
            ("finance", "Wirtschaft"),
            ("science", "Wissenschaft"),
            ("research", "Wissenschaft"),
            ("culture", "Kultur"),
            ("arts", "Kultur"),
            ("entertainment", "Kultur"),
            ("sports", "Sport"),
            ("society", "Gesellschaft"),
            ("social", "Gesellschaft"),
            ("environment", "Umwelt"),
            ("climate", "Umwelt"),
            ("security", "Sicherheit"),
            ("crime", "Sicherheit"),
            ("health", "Gesundheit"),
            ("medicine", "Gesundheit"),
            ("defense", "Verteidigung"),
            ("defence", "Verteidigung"),
            ("military", "Verteidigung"),
            ("energy", "Energie"),
            ("law", "Recht"),
            ("legal", "Recht"),
            ("justice", "Recht"),
        ];

        for (english, german) in english_to_german {
            if lower == english {
                return german.to_string();
            }
        }

        input.trim().to_string()
    }
}

#[allow(dead_code)] // Reserved for Phase 3 KI-Features
pub async fn classify_article(
    title: &str,
    content: &str,
    locale: &str,
) -> Result<Vec<String>, String> {
    let classifier = CategoryClassifier::default();
    classifier.classify(title, content, locale).await
}

pub fn classify_by_keywords(keywords: &[String]) -> Vec<String> {
    let keyword_category_map: &[(&[&str], &str)] = &[
        (
            &[
                "software",
                "app",
                "ki",
                "ai",
                "computer",
                "internet",
                "digital",
                "algorithmus",
                "programmierung",
                "cloud",
                "server",
                "daten",
                "cyber",
                "online",
                "smartphone",
                "chip",
                "prozessor",
            ],
            "Technik",
        ),
        (
            &[
                "regierung",
                "minister",
                "kanzler",
                "präsident",
                "partei",
                "wahl",
                "gesetz",
                "parlament",
                "bundestag",
                "koalition",
                "opposition",
                "abgeordnete",
                "politiker",
                "diplomat",
            ],
            "Politik",
        ),
        (
            &[
                "unternehmen",
                "konzern",
                "aktie",
                "börse",
                "umsatz",
                "gewinn",
                "inflation",
                "zinsen",
                "bank",
                "investition",
                "export",
                "import",
                "handel",
                "arbeitsmarkt",
                "insolvenz",
            ],
            "Wirtschaft",
        ),
        (
            &[
                "forscher",
                "studie",
                "wissenschaftler",
                "universität",
                "forschung",
                "entdeckung",
                "experiment",
                "labor",
                "nasa",
                "esa",
                "satellit",
                "mars",
                "mond",
            ],
            "Wissenschaft",
        ),
        (
            &[
                "film",
                "musik",
                "kunst",
                "theater",
                "museum",
                "künstler",
                "konzert",
                "festival",
                "autor",
                "buch",
                "ausstellung",
                "kultur",
                "literatur",
                "kino",
            ],
            "Kultur",
        ),
        (
            &[
                "fußball",
                "bundesliga",
                "champions",
                "olympia",
                "tennis",
                "formel",
                "rennen",
                "meister",
                "trainer",
                "spieler",
                "tor",
                "sieg",
                "niederlage",
                "turnier",
                "mannschaft",
            ],
            "Sport",
        ),
        (
            &[
                "familie",
                "kinder",
                "schule",
                "bildung",
                "migration",
                "flüchtling",
                "integration",
                "rente",
                "sozial",
                "armut",
                "demografe",
                "wohnung",
                "miete",
            ],
            "Gesellschaft",
        ),
        (
            &[
                "klima",
                "umwelt",
                "co2",
                "emission",
                "naturschutz",
                "artenschutz",
                "nachhaltigkeit",
                "wetter",
                "sturm",
                "überschwemmung",
                "dürre",
                "temperatur",
                "erderwärmung",
            ],
            "Umwelt",
        ),
        (
            &[
                "polizei",
                "kriminalität",
                "verbrechen",
                "mord",
                "raub",
                "betrug",
                "terror",
                "anschlag",
                "festnahme",
                "ermittlung",
                "verdächtig",
                "hacker",
                "cyberangriff",
            ],
            "Sicherheit",
        ),
        (
            &[
                "krankenhaus",
                "arzt",
                "patient",
                "krankheit",
                "impfung",
                "medikament",
                "therapie",
                "virus",
                "pandemie",
                "pflege",
                "krankenkasse",
                "operation",
                "diagnose",
            ],
            "Gesundheit",
        ),
        (
            &[
                "bundeswehr",
                "soldat",
                "militär",
                "nato",
                "krieg",
                "konflikt",
                "waffe",
                "panzer",
                "rakete",
                "verteidigung",
                "armee",
                "truppen",
                "angriff",
                "invasion",
            ],
            "Verteidigung",
        ),
        (
            &[
                "strom",
                "gas",
                "öl",
                "kohle",
                "atomkraft",
                "kernkraft",
                "windkraft",
                "solar",
                "photovoltaik",
                "energiewende",
                "blackout",
                "netz",
                "kraftwerk",
            ],
            "Energie",
        ),
        (
            &[
                "gericht",
                "richter",
                "urteil",
                "klage",
                "anwalt",
                "prozess",
                "strafe",
                "verurteilung",
                "berufung",
                "datenschutz",
                "verfassungsgericht",
                "staatsanwalt",
            ],
            "Recht",
        ),
    ];

    let mut category_scores: std::collections::HashMap<&str, i32> =
        std::collections::HashMap::new();

    for keyword in keywords {
        let kw_lower = keyword.to_lowercase();
        for (kw_list, category) in keyword_category_map {
            for kw in *kw_list {
                if kw_lower.contains(kw) || kw.contains(&kw_lower) {
                    *category_scores.entry(category).or_insert(0) += 1;
                }
            }
        }
    }

    let mut scored: Vec<(&&str, &i32)> = category_scores.iter().collect();
    scored.sort_by(|a, b| b.1.cmp(a.1));

    scored
        .into_iter()
        .filter(|(_, &score)| score > 0)
        .take(5)
        .map(|(cat, _)| cat.to_string())
        .collect()
}
