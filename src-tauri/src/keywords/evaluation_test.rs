//! Evaluation test for comparing OLD vs NEW keyword extraction methods
//!
//! This module compares:
//! - OLD method: No MMR, No TRISUM, Levenshtein distance = 0
//! - NEW method: MMR enabled, TRISUM enabled, Levenshtein distance = 2
//!
//! Run with: cargo test --package fuckup-rss --lib -- keywords::evaluation_test --nocapture

use super::*;
use super::config::KeywordConfig;
use super::advanced::{levenshtein_distance, is_near_duplicate};
use std::collections::{HashMap, HashSet};

/// Sample article for testing
#[derive(Debug, Clone)]
struct TestArticle {
    id: i64,
    title: String,
    content: String,
}

/// Results of keyword extraction
#[derive(Debug, Clone)]
struct ExtractionResult {
    article_id: i64,
    title: String,
    old_keywords: Vec<String>,
    new_keywords: Vec<String>,
    old_near_duplicates: usize,
    new_near_duplicates: usize,
}

/// Statistics across all articles
#[derive(Debug, Default)]
struct EvaluationStats {
    total_articles: usize,
    old_avg_keywords: f64,
    new_avg_keywords: f64,
    old_total_near_duplicates: usize,
    new_total_near_duplicates: usize,
    old_diversity_score: f64,
    new_diversity_score: f64,
    keyword_frequency: HashMap<String, usize>,
}

/// Configuration for OLD method (baseline)
fn old_method_config() -> KeywordConfig {
    KeywordConfig::standard()
        .with_mmr(false)
        .with_trisum(false)
        .with_levenshtein_distance(0)
        .with_max_keywords(15)
}

/// Configuration for NEW method (improved)
fn new_method_config() -> KeywordConfig {
    KeywordConfig::standard()
        .with_mmr(true)
        .with_mmr_lambda(0.5)
        .with_trisum(true)
        .with_levenshtein_distance(2)
        .with_max_keywords(15)
}

/// Count near-duplicates in a keyword list using Levenshtein distance
fn count_near_duplicates(keywords: &[String], max_distance: usize) -> usize {
    let mut count = 0;
    for i in 0..keywords.len() {
        for j in (i + 1)..keywords.len() {
            if is_near_duplicate(&keywords[i], &keywords[j], max_distance) {
                count += 1;
            }
        }
    }
    count
}

/// Calculate diversity score (0-1) based on average pairwise Levenshtein distance
fn calculate_diversity_score(keywords: &[String]) -> f64 {
    if keywords.len() < 2 {
        return 1.0;
    }

    let mut total_distance = 0;
    let mut pairs = 0;

    for i in 0..keywords.len() {
        for j in (i + 1)..keywords.len() {
            let dist = levenshtein_distance(&keywords[i].to_lowercase(), &keywords[j].to_lowercase());
            total_distance += dist;
            pairs += 1;
        }
    }

    if pairs == 0 {
        return 1.0;
    }

    // Average distance, normalized by average keyword length
    let avg_len: f64 = keywords.iter().map(|k| k.len()).sum::<usize>() as f64 / keywords.len() as f64;
    let avg_distance = total_distance as f64 / pairs as f64;

    // Normalize: max distance = 2 * avg_len (very different), min = 0 (identical)
    (avg_distance / (avg_len * 2.0)).min(1.0)
}

/// Extract keywords using a specific configuration
fn extract_with_config(title: &str, content: &str, config: KeywordConfig) -> Vec<String> {
    let extractor = KeywordExtractor::with_config(config);
    extractor.extract(title, content)
        .into_iter()
        .map(|kw| kw.text)
        .collect()
}

/// Sample test articles (from real fuckupRSS database)
fn get_sample_articles() -> Vec<TestArticle> {
    vec![
        TestArticle {
            id: 1,
            title: "Massenproteste im Iran reißen nicht ab".to_string(),
            content: r#"Die Proteste im Iran dauern weiter an. Seit Wochen gehen Menschen auf die Straßen,
            um gegen das Regime zu demonstrieren. Die iranische Regierung reagiert mit Gewalt.
            Sicherheitskräfte setzen Tränengas und Schlagstöcke ein. Mehrere Demonstranten wurden verhaftet.
            Die internationale Gemeinschaft kritisiert das Vorgehen scharf. Die EU fordert ein Ende der Gewalt.
            Die USA haben neue Sanktionen angekündigt. Die Proteste begannen nach dem Tod einer jungen Frau
            in Polizeigewahrsam. Sie soll wegen eines nicht korrekt sitzenden Kopftuchs festgenommen worden sein.
            Die Demonstranten fordern Freiheit und ein Ende der islamischen Republik."#.to_string(),
        },
        TestArticle {
            id: 2,
            title: "Deutschland und Indien vertiefen ihre Zusammenarbeit".to_string(),
            content: r#"Berlin und Neu-Delhi haben ein neues Abkommen zur wirtschaftlichen Zusammenarbeit unterzeichnet.
            Bundeskanzler Scholz empfing Premierminister Modi im Kanzleramt. Beide Seiten einigten sich auf
            verstärkte Kooperation in den Bereichen Technologie, Klimaschutz und Verteidigung.
            Deutschland will Investitionen in Indien ausbauen. Indische IT-Unternehmen sollen leichteren
            Zugang zum deutschen Markt bekommen. Die strategische Partnerschaft soll gegen den wachsenden
            Einfluss Chinas gestärkt werden. Modi betonte die historisch guten Beziehungen beider Länder.
            Scholz hob die Bedeutung demokratischer Werte hervor."#.to_string(),
        },
        TestArticle {
            id: 3,
            title: "Klimawandel bedroht Artenvielfalt in Europa".to_string(),
            content: r#"Eine neue Studie warnt vor dramatischen Auswirkungen des Klimawandels auf die europäische
            Tierwelt. Forscher der Universität München haben Daten aus 30 Jahren ausgewertet. Die Ergebnisse
            sind alarmierend: Viele Tierarten wandern nordwärts, weil es ihnen zu warm wird. Andere sterben aus.
            Besonders betroffen sind Insekten, Vögel und Amphibien. Die Wissenschaftler fordern dringend
            mehr Naturschutzgebiete. Auch der CO2-Ausstoß muss drastisch reduziert werden. Die EU hat
            bereits Maßnahmen angekündigt, aber Kritiker sagen, das reiche nicht aus. Der Klimawandel
            schreitet schneller voran als gedacht."#.to_string(),
        },
        TestArticle {
            id: 4,
            title: "Künstliche Intelligenz revolutioniert Medizin".to_string(),
            content: r#"KI-Systeme erzielen bei der Diagnose von Krankheiten immer bessere Ergebnisse.
            Ein neues System der Charité Berlin kann Hautkrebs mit 95% Genauigkeit erkennen.
            Die Künstliche Intelligenz wurde mit Millionen von Bildern trainiert. Machine Learning
            ermöglicht ständige Verbesserung. Ärzte betonen jedoch, dass KI nur unterstützen kann.
            Die finale Diagnose muss beim Menschen bleiben. Ethische Fragen werden diskutiert.
            Wer haftet bei Fehldiagnosen? Datenschutz ist ein weiteres wichtiges Thema.
            Patientendaten müssen geschützt werden. Die Technologie birgt großes Potenzial."#.to_string(),
        },
        TestArticle {
            id: 5,
            title: "Bundestagswahl: CDU liegt in Umfragen vorne".to_string(),
            content: r#"Die neuesten Umfragen sehen die CDU/CSU deutlich vor der SPD. Friedrich Merz
            konnte seinen Vorsprung ausbauen. Die Union kommt auf 32 Prozent, die SPD erreicht nur 18 Prozent.
            Die Grünen liegen bei 14 Prozent. Die AfD erreicht 21 Prozent und ist damit zweitstärkste Kraft.
            Die FDP kämpft mit der Fünf-Prozent-Hürde. Die Linke würde nicht mehr in den Bundestag einziehen.
            Wahlkampfthemen sind Migration, Wirtschaft und Klimaschutz. Kanzler Scholz steht unter Druck.
            Die Ampelkoalition hat an Zustimmung verloren. Neuwahlen werden diskutiert."#.to_string(),
        },
        TestArticle {
            id: 6,
            title: "Tesla kündigt neue Fabrik in Brandenburg an".to_string(),
            content: r#"Der Elektroautobauer Tesla will seine Produktion in Deutschland ausweiten.
            Elon Musk kündigte eine Erweiterung der Gigafactory Berlin-Brandenburg an. 10.000 neue
            Arbeitsplätze sollen entstehen. Die Investition beträgt mehrere Milliarden Euro.
            Tesla will auch Batteriezellen vor Ort produzieren. Umweltschützer kritisieren den
            hohen Wasserverbrauch der Fabrik. Die Landesregierung begrüßt die Investition.
            Wirtschaftsminister Habeck sprach von einem wichtigen Signal für den Standort Deutschland.
            Die Elektromobilität gewinnt an Bedeutung. Tesla konkurriert mit deutschen Herstellern."#.to_string(),
        },
        TestArticle {
            id: 7,
            title: "Neue Corona-Variante beunruhigt Experten".to_string(),
            content: r#"Eine neue Variante des Coronavirus bereitet Wissenschaftlern Sorgen.
            Die Mutation wurde erstmals in Südafrika nachgewiesen. Sie soll noch ansteckender sein.
            Die WHO beobachtet die Entwicklung genau. Impfstoffe könnten weniger wirksam sein.
            Die Bundesregierung erwägt neue Einreisebeschränkungen. Experten raten zur Vorsicht.
            Eine vierte Impfung wird diskutiert. Die Krankenhäuser bereiten sich auf steigende
            Fallzahlen vor. Die Pandemie ist noch nicht überwunden. Long Covid bleibt ein Problem.
            Die wirtschaftlichen Auswirkungen sind enorm."#.to_string(),
        },
        TestArticle {
            id: 8,
            title: "Ukraine-Krieg: Neue Waffenlieferungen geplant".to_string(),
            content: r#"Die NATO-Staaten planen weitere Waffenlieferungen an die Ukraine.
            Deutschland will Leopard-2-Panzer bereitstellen. Die USA liefern Patriot-Raketen.
            Präsident Selenskyj fordert mehr Unterstützung. Russland warnt vor einer Eskalation.
            Putin drohte mit Vergeltung. Die Kämpfe im Donbass dauern an. Tausende Zivilisten
            sind auf der Flucht. Die humanitäre Lage ist dramatisch. Friedensverhandlungen
            scheinen in weiter Ferne. Die EU verhängt neue Sanktionen gegen Russland.
            Die Energiepreise steigen weiter. Der Winter wird hart."#.to_string(),
        },
        TestArticle {
            id: 9,
            title: "Fußball-Bundesliga: Bayern München gewinnt Spitzenspiel".to_string(),
            content: r#"Der FC Bayern München hat das Spitzenspiel gegen Borussia Dortmund gewonnen.
            Mit 3:1 setzten sich die Münchner durch. Harry Kane erzielte zwei Tore. Jamal Musiala
            spielte überragend. Der BVB konnte nicht überzeugen. Trainer Edin Terzic steht unter Druck.
            Bayern führt die Tabelle souverän an. Dortmund rutschte auf Platz drei ab.
            Bayer Leverkusen ist neuer Verfolger. Die Meisterschaft scheint entschieden.
            Im DFB-Pokal trifft Bayern auf Union Berlin. Die Champions League ruft."#.to_string(),
        },
        TestArticle {
            id: 10,
            title: "Inflation in Deutschland auf Rekordniveau".to_string(),
            content: r#"Die Inflationsrate in Deutschland erreicht den höchsten Stand seit Jahrzehnten.
            Die Preise stiegen um 7,3 Prozent im Vergleich zum Vorjahr. Besonders Lebensmittel und
            Energie werden teurer. Die EZB reagiert mit Zinserhöhungen. Die Bundesbank warnt vor
            anhaltender Inflation. Verbraucher müssen sparen. Der Mindestlohn wurde erhöht.
            Gewerkschaften fordern Lohnerhöhungen. Die Wirtschaft wächst langsamer als erwartet.
            Eine Rezession droht. Die Regierung plant Entlastungspakete. Gas- und Strompreisbremse
            sollen helfen. Die Energiewende wird beschleunigt."#.to_string(),
        },
    ]
}

/// Run the evaluation and print results
fn run_evaluation(articles: &[TestArticle]) -> EvaluationStats {
    let old_config = old_method_config();
    let new_config = new_method_config();

    let mut results: Vec<ExtractionResult> = Vec::new();
    let mut keyword_frequency: HashMap<String, usize> = HashMap::new();

    for article in articles {
        let old_keywords = extract_with_config(&article.title, &article.content, old_config.clone());
        let new_keywords = extract_with_config(&article.title, &article.content, new_config.clone());

        // Count near-duplicates in each result (using distance 2)
        let old_near_dups = count_near_duplicates(&old_keywords, 2);
        let new_near_dups = count_near_duplicates(&new_keywords, 2);

        // Track keyword frequency
        for kw in &new_keywords {
            *keyword_frequency.entry(kw.to_lowercase()).or_insert(0) += 1;
        }

        results.push(ExtractionResult {
            article_id: article.id,
            title: article.title.clone(),
            old_keywords,
            new_keywords,
            old_near_duplicates: old_near_dups,
            new_near_duplicates: new_near_dups,
        });
    }

    // Calculate statistics
    let total = results.len();
    let old_avg_kw: f64 = results.iter().map(|r| r.old_keywords.len()).sum::<usize>() as f64 / total as f64;
    let new_avg_kw: f64 = results.iter().map(|r| r.new_keywords.len()).sum::<usize>() as f64 / total as f64;
    let old_total_dups: usize = results.iter().map(|r| r.old_near_duplicates).sum();
    let new_total_dups: usize = results.iter().map(|r| r.new_near_duplicates).sum();

    let old_diversity: f64 = results.iter()
        .map(|r| calculate_diversity_score(&r.old_keywords))
        .sum::<f64>() / total as f64;
    let new_diversity: f64 = results.iter()
        .map(|r| calculate_diversity_score(&r.new_keywords))
        .sum::<f64>() / total as f64;

    EvaluationStats {
        total_articles: total,
        old_avg_keywords: old_avg_kw,
        new_avg_keywords: new_avg_kw,
        old_total_near_duplicates: old_total_dups,
        new_total_near_duplicates: new_total_dups,
        old_diversity_score: old_diversity,
        new_diversity_score: new_diversity,
        keyword_frequency,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_extraction_comparison() {
        println!("\n======================================================");
        println!("KEYWORD EXTRACTION EVALUATION: OLD vs NEW METHOD");
        println!("======================================================\n");

        let articles = get_sample_articles();
        let old_config = old_method_config();
        let new_config = new_method_config();

        println!("Configuration Comparison:");
        println!("----------------------------");
        println!("OLD Method: MMR={}, TRISUM={}, Levenshtein={}",
            old_config.use_mmr, old_config.use_trisum, old_config.levenshtein_max_distance);
        println!("NEW Method: MMR={}, TRISUM={}, Levenshtein={}",
            new_config.use_mmr, new_config.use_trisum, new_config.levenshtein_max_distance);
        println!();

        // Detailed analysis for each article
        for article in &articles {
            println!("------------------------------------------------------");
            println!("Article #{}: {}", article.id, article.title);
            println!("------------------------------------------------------");

            let old_keywords = extract_with_config(&article.title, &article.content, old_config.clone());
            let new_keywords = extract_with_config(&article.title, &article.content, new_config.clone());

            let old_near_dups = count_near_duplicates(&old_keywords, 2);
            let new_near_dups = count_near_duplicates(&new_keywords, 2);

            let old_diversity = calculate_diversity_score(&old_keywords);
            let new_diversity = calculate_diversity_score(&new_keywords);

            println!("\nOLD Method ({} keywords, {} near-duplicates, diversity: {:.2}):",
                old_keywords.len(), old_near_dups, old_diversity);
            for (i, kw) in old_keywords.iter().enumerate() {
                print!("  {}. {}", i + 1, kw);
                // Check if it's a near-duplicate of another keyword
                for (j, other) in old_keywords.iter().enumerate() {
                    if i != j && is_near_duplicate(kw, other, 2) {
                        print!(" [NEAR-DUP of: {}]", other);
                        break;
                    }
                }
                println!();
            }

            println!("\nNEW Method ({} keywords, {} near-duplicates, diversity: {:.2}):",
                new_keywords.len(), new_near_dups, new_diversity);
            for (i, kw) in new_keywords.iter().enumerate() {
                print!("  {}. {}", i + 1, kw);
                // Check if it's a near-duplicate
                for (j, other) in new_keywords.iter().enumerate() {
                    if i != j && is_near_duplicate(kw, other, 2) {
                        print!(" [NEAR-DUP of: {}]", other);
                        break;
                    }
                }
                println!();
            }

            // Quality assessment
            let improvement = if new_near_dups < old_near_dups {
                format!("IMPROVED (removed {} near-duplicates)", old_near_dups - new_near_dups)
            } else if new_near_dups == old_near_dups && new_near_dups == 0 {
                "EQUAL (no near-duplicates)".to_string()
            } else {
                format!("NO CHANGE ({} near-duplicates)", new_near_dups)
            };

            println!("\n=> Result: {}", improvement);
            println!();
        }

        // Run full evaluation
        let stats = run_evaluation(&articles);

        println!("\n======================================================");
        println!("SUMMARY STATISTICS");
        println!("======================================================\n");

        println!("Total articles analyzed: {}", stats.total_articles);
        println!();
        println!("Average keywords per article:");
        println!("  OLD: {:.1}", stats.old_avg_keywords);
        println!("  NEW: {:.1}", stats.new_avg_keywords);
        println!();
        println!("Total near-duplicates detected (distance <= 2):");
        println!("  OLD: {}", stats.old_total_near_duplicates);
        println!("  NEW: {}", stats.new_total_near_duplicates);
        let dup_reduction = if stats.old_total_near_duplicates > 0 {
            100.0 * (1.0 - stats.new_total_near_duplicates as f64 / stats.old_total_near_duplicates as f64)
        } else {
            0.0
        };
        println!("  Reduction: {:.1}%", dup_reduction);
        println!();
        println!("Average diversity score (0-1, higher = more diverse):");
        println!("  OLD: {:.3}", stats.old_diversity_score);
        println!("  NEW: {:.3}", stats.new_diversity_score);
        let diversity_improvement = (stats.new_diversity_score - stats.old_diversity_score) / stats.old_diversity_score * 100.0;
        println!("  Improvement: {:+.1}%", diversity_improvement);
        println!();

        // Most frequent keywords
        println!("Top 20 most frequent keywords (NEW method):");
        let mut freq_vec: Vec<_> = stats.keyword_frequency.iter().collect();
        freq_vec.sort_by(|a, b| b.1.cmp(a.1));
        for (i, (kw, count)) in freq_vec.iter().take(20).enumerate() {
            println!("  {}. {} ({})", i + 1, kw, count);
        }

        println!("\n======================================================");
        println!("QUALITY ASSESSMENT");
        println!("======================================================\n");

        // Criteria assessment
        let relevance_score = 7; // Would need manual assessment
        let diversity_score = if stats.new_diversity_score > 0.6 { 9 } else if stats.new_diversity_score > 0.4 { 7 } else { 5 };
        let quality_score = if stats.new_total_near_duplicates == 0 { 9 } else if stats.new_total_near_duplicates < 5 { 7 } else { 5 };
        let coverage_score = 8; // Would need manual assessment

        println!("Criteria (1-10 scale):");
        println!("  - Relevance: {} (estimated - requires manual review)", relevance_score);
        println!("  - Diversity: {} (based on diversity score)", diversity_score);
        println!("  - Quality (no garbage/stopwords): {} (based on near-duplicates)", quality_score);
        println!("  - Topic coverage: {} (estimated)", coverage_score);
        println!();

        let overall = (relevance_score + diversity_score + quality_score + coverage_score) as f64 / 4.0;
        println!("Overall Quality Score: {:.1}/10", overall);
        println!();

        // Assertions for the test
        assert!(stats.total_articles >= 10, "Should have at least 10 sample articles");
        assert!(stats.new_diversity_score >= stats.old_diversity_score * 0.9,
            "NEW method diversity should be at least 90% of OLD method");
    }

    #[test]
    fn test_levenshtein_near_duplicate_detection() {
        // Test that our near-duplicate detection works correctly
        let keywords_with_dups = vec![
            "Trump".to_string(),
            "Trumps".to_string(),  // Near-duplicate
            "Biden".to_string(),
            "Ukraine".to_string(),
            "Ukrainer".to_string(), // Near-duplicate (distance 2)
        ];

        let near_dups = count_near_duplicates(&keywords_with_dups, 2);
        assert!(near_dups >= 1, "Should detect at least 1 near-duplicate pair");
        println!("Near-duplicates in test set: {}", near_dups);
    }

    #[test]
    fn test_diversity_score_calculation() {
        // Highly diverse set
        let diverse = vec![
            "Klimawandel".to_string(),
            "Bundesregierung".to_string(),
            "Technologie".to_string(),
        ];
        let diverse_score = calculate_diversity_score(&diverse);

        // Similar keywords
        let similar = vec![
            "Politik".to_string(),
            "Politiker".to_string(),
            "politisch".to_string(),
        ];
        let similar_score = calculate_diversity_score(&similar);

        println!("Diverse set score: {:.3}", diverse_score);
        println!("Similar set score: {:.3}", similar_score);

        assert!(diverse_score > similar_score, "Diverse keywords should have higher diversity score");
    }
}
