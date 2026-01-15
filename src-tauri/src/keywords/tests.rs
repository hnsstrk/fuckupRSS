use super::*;

#[test]
fn test_language_detection_german() {
    let text =
        "Die Bundesregierung hat heute neue Maßnahmen zur Bekämpfung der Inflation angekündigt.";
    assert_eq!(KeywordExtractor::detect_language(text), Language::German);
}

#[test]
fn test_language_detection_english() {
    let text = "The government announced new measures to combat inflation today.";
    assert_eq!(KeywordExtractor::detect_language(text), Language::English);
}

#[test]
fn test_extract_keywords_german() {
    let title = "Bundeskanzler Scholz trifft NATO-Generalsekretär in Berlin";
    let content = "Bundeskanzler Olaf Scholz hat sich heute mit dem NATO-Generalsekretär in Berlin getroffen. Bei dem Gespräch ging es um die Sicherheitslage in Europa und die weitere Unterstützung der Ukraine.";

    let keywords = extract_keywords(title, content, 7);

    assert!(!keywords.is_empty());
    assert!(keywords.len() <= 7);

    let keywords_lower: Vec<String> = keywords.iter().map(|k| k.to_lowercase()).collect();
    let has_relevant = keywords_lower.iter().any(|k| {
        k.contains("scholz")
            || k.contains("nato")
            || k.contains("berlin")
            || k.contains("ukraine")
            || k.contains("bundeskanzler")
    });
    assert!(
        has_relevant,
        "Should extract relevant keywords, got: {:?}",
        keywords
    );
}

#[test]
fn test_extract_keywords_english() {
    let title = "President Biden meets with NATO leaders in Brussels";
    let content = "President Joe Biden held meetings with NATO leaders in Brussels today. The discussions focused on European security and continued support for Ukraine.";

    let keywords = extract_keywords(title, content, 7);

    assert!(!keywords.is_empty());
    assert!(keywords.len() <= 7);
}

#[test]
fn test_keyword_extraction_filters_stopwords() {
    let title = "Test Article";
    let content = "Der die das ein eine einer und oder aber doch sondern weil wenn als wie wo.";

    let extractor = KeywordExtractor::new(10);
    let keywords = extractor.extract(title, content);

    for kw in &keywords {
        let lower = kw.text.to_lowercase();
        assert!(
            !["der", "die", "das", "und", "oder"].contains(&lower.as_str()),
            "Stopword '{}' should be filtered",
            kw.text
        );
    }
}

#[test]
fn test_news_stopwords_filtered() {
    let title = "Bericht: Neue Entwicklungen";
    let content =
        "Laut einem Bericht gibt es heute neue Entwicklungen. Das Video zeigt die Situation.";

    let extractor = KeywordExtractor::new(10);
    let keywords = extractor.extract(title, content);

    for kw in &keywords {
        let lower = kw.text.to_lowercase();
        assert!(
            !["bericht", "video", "heute"].contains(&lower.as_str()),
            "News stopword '{}' should be filtered",
            kw.text
        );
    }
}

#[test]
fn test_entity_extraction() {
    let text =
        "Angela Merkel und Emmanuel Macron trafen sich in Paris. Die EU und NATO waren Themen.";

    let extractor = KeywordExtractor::new(10);
    let entities = extractor.extract_entities(text);

    let entity_names: Vec<&str> = entities.iter().map(|e| e.text.as_str()).collect();

    assert!(
        entity_names
            .iter()
            .any(|e| e.contains("Merkel") || e.contains("Macron") || e.contains("Paris")),
        "Should extract named entities, got: {:?}",
        entity_names
    );
}

#[test]
fn test_acronym_extraction() {
    let text = "Die NATO und die EU haben gemeinsame Ziele. Die SPD unterstützt die Initiative.";

    let extractor = KeywordExtractor::new(10);
    let entities = extractor.extract_entities(text);

    let has_acronym = entities.iter().any(|e| {
        matches!(e.keyword_type, KeywordType::Acronym)
            && ["NATO", "EU", "SPD"].contains(&e.text.as_str())
    });

    assert!(has_acronym, "Should extract acronyms like NATO, EU, SPD");
}

#[test]
fn test_diversity_ensures_mix() {
    let title = "Technologie und Politik";
    let content = "Microsoft und Google entwickeln neue KI-Systeme. Bundeskanzler Scholz diskutiert Regulierung. Die EU plant neue Gesetze für künstliche Intelligenz in Europa.";

    let extractor = KeywordExtractor::new(7);
    let keywords = extractor.extract(title, content);

    let has_entity = keywords.iter().any(|k| {
        matches!(
            k.keyword_type,
            KeywordType::Person | KeywordType::Organization | KeywordType::Acronym
        )
    });
    let has_concept = keywords
        .iter()
        .any(|k| k.keyword_type == KeywordType::Concept);

    assert!(
        has_entity || has_concept,
        "Should have diverse keyword types"
    );
}

#[test]
fn test_title_boost() {
    let title = "Klimawandel bedroht Küstenstädte";
    let content =
        "Wissenschaftler warnen vor steigendem Meeresspiegel. Viele Städte müssen sich anpassen.";

    let extractor = KeywordExtractor::new(5);
    let keywords = extractor.extract(title, content);

    let klimawandel_kw = keywords
        .iter()
        .find(|k| k.text.to_lowercase().contains("klimawandel"));

    if let Some(kw) = klimawandel_kw {
        assert!(kw.score > 0.5, "Title keywords should have boosted score");
    }
}

#[test]
fn test_max_keywords_respected() {
    let title = "Viele Themen in einem Artikel";
    let content = "Politik Wirtschaft Technik Sport Kultur Wissenschaft Umwelt Gesundheit Energie Recht Sicherheit Verteidigung Gesellschaft. Angela Merkel, Olaf Scholz, Joe Biden, Emmanuel Macron diskutierten in Berlin, Paris, Washington und Brüssel über NATO, EU, UN und WTO.";

    let extractor = KeywordExtractor::new(5);
    let keywords = extractor.extract(title, content);

    assert!(
        keywords.len() <= 5,
        "Should respect max_keywords limit, got {}",
        keywords.len()
    );
}

#[test]
fn test_find_canonical_keyword_countries() {
    // Test country adjective → country name normalization
    assert_eq!(find_canonical_keyword("russian"), Some("Russland"));
    assert_eq!(find_canonical_keyword("russia"), Some("Russland"));
    assert_eq!(find_canonical_keyword("moscow"), Some("Russland"));
    assert_eq!(find_canonical_keyword("kreml"), Some("Russland"));

    assert_eq!(find_canonical_keyword("ukrainian"), Some("Ukraine"));
    assert_eq!(find_canonical_keyword("kyiv"), Some("Ukraine"));
    assert_eq!(find_canonical_keyword("kiev"), Some("Ukraine"));

    assert_eq!(find_canonical_keyword("european"), Some("Europäische Union"));
    assert_eq!(find_canonical_keyword("eu"), Some("Europäische Union"));
    assert_eq!(find_canonical_keyword("brüssel"), Some("Europäische Union"));

    assert_eq!(find_canonical_keyword("british"), Some("Großbritannien"));
    assert_eq!(find_canonical_keyword("scotland"), Some("Großbritannien"));
    assert_eq!(find_canonical_keyword("london"), Some("Großbritannien"));

    assert_eq!(find_canonical_keyword("french"), Some("Frankreich"));
    assert_eq!(find_canonical_keyword("paris"), Some("Frankreich"));

    assert_eq!(find_canonical_keyword("iranian"), Some("Iran"));
    assert_eq!(find_canonical_keyword("teheran"), Some("Iran"));

    assert_eq!(find_canonical_keyword("american"), Some("Vereinigte Staaten"));
    assert_eq!(find_canonical_keyword("usa"), Some("Vereinigte Staaten"));
    assert_eq!(find_canonical_keyword("washington"), Some("Vereinigte Staaten"));
}

#[test]
fn test_find_canonical_keyword_topics() {
    // Test topic normalization
    assert_eq!(find_canonical_keyword("military"), Some("Sicherheit"));
    assert_eq!(find_canonical_keyword("security"), Some("Sicherheit"));
    assert_eq!(find_canonical_keyword("defense"), Some("Sicherheit"));

    assert_eq!(find_canonical_keyword("economy"), Some("Wirtschaft"));
    assert_eq!(find_canonical_keyword("economic"), Some("Wirtschaft"));

    assert_eq!(find_canonical_keyword("migration"), Some("Migration"));
    assert_eq!(find_canonical_keyword("refugees"), Some("Migration"));
    assert_eq!(find_canonical_keyword("asylum"), Some("Migration"));

    assert_eq!(find_canonical_keyword("climate change"), Some("Klimawandel"));
    assert_eq!(find_canonical_keyword("global warming"), Some("Klimawandel"));
}

#[test]
fn test_find_canonical_keyword_persons() {
    // Test prominent persons normalization (full names as canonical)
    assert_eq!(find_canonical_keyword("trump"), Some("Donald Trump"));
    assert_eq!(find_canonical_keyword("präsident trump"), Some("Donald Trump"));
    assert_eq!(find_canonical_keyword("biden"), Some("Joe Biden"));
    assert_eq!(find_canonical_keyword("putin"), Some("Wladimir Putin"));
    assert_eq!(find_canonical_keyword("scholz"), Some("Olaf Scholz"));
    assert_eq!(find_canonical_keyword("bundeskanzler scholz"), Some("Olaf Scholz"));
}

#[test]
fn test_find_canonical_keyword_declensions() {
    // Test German declension forms
    assert_eq!(find_canonical_keyword("vereinigten staaten"), Some("Vereinigte Staaten"));
    assert_eq!(find_canonical_keyword("irans"), Some("Iran"));
    assert_eq!(find_canonical_keyword("der iran"), Some("Iran"));
}

#[test]
fn test_find_canonical_keyword_no_match() {
    // Test that non-matching keywords return None
    assert_eq!(find_canonical_keyword("bitcoin"), None);
    assert_eq!(find_canonical_keyword("fußball"), None);
    assert_eq!(find_canonical_keyword("apple"), None);
}
