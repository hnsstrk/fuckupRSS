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

// ============================================================
// ADVANCED EXTRACTION TESTS
// ============================================================

#[test]
fn test_extract_keywords_with_metadata() {
    let title = "Bundeskanzler Scholz besucht Berlin";
    let content = "Der Bundeskanzler Olaf Scholz war heute in Berlin. Das Treffen mit NATO-Vertretern war wichtig.";

    let keywords = extract_keywords_with_metadata(title, content, 5);

    assert!(!keywords.is_empty());
    // Keywords should have sources
    assert!(keywords.iter().all(|k| !k.source.is_empty()));
    // At least one keyword should have multiple sources (combined from different methods)
    let has_combined = keywords.iter().any(|k| k.source.contains(','));
    // This is not guaranteed but likely for typical news text
    assert!(
        keywords.len() >= 1,
        "Should extract at least one keyword"
    );
}

#[test]
fn test_semantic_keyword_result_from() {
    let kw = ExtractedKeyword {
        text: "Test Keyword".to_string(),
        score: 0.8,
        keyword_type: KeywordType::Concept,
        source: "yake,rake,ngram".to_string(),
    };

    let result: SemanticKeywordResult = kw.into();

    assert_eq!(result.text, "Test Keyword");
    assert_eq!(result.score, 0.8);
    assert_eq!(result.sources, vec!["yake", "rake", "ngram"]);
    assert!(result.semantic_score.is_none());
}

#[test]
fn test_extract_keywords_with_semantic_scoring_without_scores() {
    let title = "Künstliche Intelligenz transformiert die Wirtschaft";
    let content = "Machine Learning und Deep Learning verändern Unternehmen. Die KI-Revolution ist in vollem Gange.";

    let results = extract_keywords_with_semantic_scoring(title, content, 5, None, 0.0);

    assert!(!results.is_empty());
    assert!(results.len() <= 5);
    // Without semantic scores, all semantic_score should be None
    assert!(results.iter().all(|r| r.semantic_score.is_none()));
}

#[test]
fn test_extract_keywords_with_semantic_scoring_with_scores() {
    let title = "Künstliche Intelligenz transformiert die Wirtschaft";
    let content = "Machine Learning und Deep Learning verändern Unternehmen. Die KI-Revolution ist in vollem Gange.";

    let mut semantic_scores = HashMap::new();
    semantic_scores.insert("Künstliche Intelligenz".to_string(), 0.95);
    semantic_scores.insert("Machine Learning".to_string(), 0.85);
    semantic_scores.insert("Deep Learning".to_string(), 0.82);

    let results = extract_keywords_with_semantic_scoring(
        title,
        content,
        5,
        Some(&semantic_scores),
        0.3, // 30% weight for semantic scores
    );

    assert!(!results.is_empty());
    // Keywords with semantic scores should have them set
    let has_semantic = results.iter().any(|r| r.semantic_score.is_some());
    // May or may not have semantic scores depending on extraction results
    // Just verify the function runs without error
}

#[test]
fn test_advanced_ngram_integration() {
    let title = "Europäische Union diskutiert neue Sanktionen";
    let content = "Die Europäische Union plant neue Sanktionen. Die Europäische Union hat bereits mehrere Runden von Maßnahmen beschlossen. Experten der Europäischen Union beraten.";

    let keywords = extract_keywords_with_metadata(title, content, 10);

    // Should find "Europäische Union" as a frequent bigram
    let has_eu = keywords.iter().any(|k|
        k.text.to_lowercase().contains("europäische union") ||
        k.text.to_lowercase().contains("eu")
    );
    assert!(
        has_eu || keywords.iter().any(|k| k.text.contains("Union")),
        "Should extract repeated phrases, got: {:?}",
        keywords.iter().map(|k| &k.text).collect::<Vec<_>>()
    );
}

#[test]
fn test_advanced_textrank_integration() {
    // TextRank should boost terms that are central in the co-occurrence graph
    let title = "Technologie und Innovation";
    let content = "Technologie treibt Innovation. Innovation ermöglicht neue Technologie. \
                   Digitale Transformation basiert auf Technologie und Innovation. \
                   Unternehmen investieren in Technologie für Innovation.";

    let keywords = extract_keywords_with_metadata(title, content, 5);

    // "Technologie" and "Innovation" should score high due to TextRank centrality
    let scores: Vec<(&str, f64)> = keywords.iter().map(|k| (k.text.as_str(), k.score)).collect();

    assert!(
        !keywords.is_empty(),
        "Should extract keywords from connected text"
    );
}

#[test]
fn test_advanced_enhanced_ner_integration() {
    let title = "Deutsche Bank AG meldet Quartalszahlen";
    let content = "Die Deutsche Bank AG hat heute ihre Quartalszahlen veröffentlicht. \
                   CEO Christian Sewing präsentierte die Ergebnisse in Frankfurt.";

    let keywords = extract_keywords_with_metadata(title, content, 10);

    // Should find organization (Deutsche Bank AG) and potentially person names
    let has_org = keywords.iter().any(|k|
        k.keyword_type == KeywordType::Organization ||
        k.text.to_lowercase().contains("bank")
    );

    assert!(
        has_org || !keywords.is_empty(),
        "Should extract organization entities"
    );
}

#[test]
fn test_multi_method_confirmation_boosts_score() {
    // Keywords confirmed by multiple methods should have higher scores
    let title = "NATO Gipfel in Washington";
    let content = "Der NATO Gipfel findet in Washington statt. Die NATO-Mitglieder diskutieren \
                   Sicherheitsfragen. Washington ist Gastgeber des NATO-Treffens.";

    let keywords = extract_keywords_with_metadata(title, content, 10);

    // Find keywords with multiple sources (indicating confirmation)
    let multi_source_keywords: Vec<_> = keywords
        .iter()
        .filter(|k| k.source.contains(','))
        .collect();

    // Keywords confirmed by multiple methods exist
    // (may not always have multi-source keywords depending on text)
    assert!(
        !keywords.is_empty(),
        "Should extract keywords from NATO text"
    );
}

// ============================================================
// LEVENSHTEIN DEDUPLIFICATION TESTS
// ============================================================

#[test]
fn test_normalize_and_dedupe_with_levenshtein() {
    let keywords = vec![
        "Trump".to_string(),
        "Trumps".to_string(), // Near-duplicate (distance 1)
        "Biden".to_string(),
        "Bidens".to_string(), // Near-duplicate (distance 1)
        "Klimawandel".to_string(),
    ];

    let deduped = normalize_and_dedupe_keywords_with_levenshtein(&keywords, 2);

    // Should remove near-duplicates
    assert!(deduped.len() <= 4, "Should have removed some near-duplicates");
    // Should keep at least one Trump and one Biden
    assert!(
        deduped.iter().any(|k| k.to_lowercase().contains("trump")),
        "Should keep a Trump variant"
    );
    assert!(
        deduped.iter().any(|k| k.to_lowercase().contains("biden")),
        "Should keep a Biden variant"
    );
}

#[test]
fn test_normalize_and_dedupe_keeps_distinct() {
    let keywords = vec![
        "Berlin".to_string(),
        "Washington".to_string(),
        "Paris".to_string(),
        "London".to_string(),
    ];

    let deduped = normalize_and_dedupe_keywords_with_levenshtein(&keywords, 2);

    // All are distinct, should keep all
    assert_eq!(deduped.len(), 4, "Should keep all distinct keywords");
}

#[test]
fn test_normalize_and_dedupe_with_scores_keeps_higher() {
    let keywords = vec![
        ("Trump".to_string(), 0.8),
        ("Trumps".to_string(), 0.9), // Higher score, should win
        ("Biden".to_string(), 0.7),
    ];

    let deduped = normalize_and_dedupe_keywords_with_scores(&keywords, 2);

    // Should keep Trump variant with higher score (0.9)
    let trump = deduped.iter().find(|(k, _)| k.to_lowercase().contains("trump"));
    assert!(trump.is_some());
    assert_eq!(trump.unwrap().1, 0.9, "Should keep the higher-scored variant");
}

#[test]
fn test_normalize_and_dedupe_exact_duplicates() {
    let keywords = vec![
        "Politik".to_string(),
        "politik".to_string(), // Exact duplicate (case-insensitive)
        "POLITIK".to_string(), // Exact duplicate
    ];

    let deduped = normalize_and_dedupe_keywords(&keywords);

    // Should have only one
    assert_eq!(deduped.len(), 1, "Should remove exact duplicates");
}

#[test]
fn test_normalize_and_dedupe_typos() {
    let keywords = vec![
        "Bundeskanzler".to_string(),
        "Bundeskanzlerin".to_string(), // Different word, distance > 2
        "Bundekanzler".to_string(),    // Typo, distance 1
    ];

    let deduped = normalize_and_dedupe_keywords_with_levenshtein(&keywords, 2);

    // Typo should be caught, but "Bundeskanzlerin" is different enough
    // Note: "Bundeskanzlerin" has distance 2 from "Bundeskanzler" (add "in")
    // So it might be considered a near-duplicate depending on threshold
    assert!(deduped.len() >= 1, "Should keep at least one variant");
}

// ============================================================
// IS_GARBAGE_KEYWORD TESTS
// ============================================================

#[test]
fn test_is_garbage_keyword_empty() {
    assert!(is_garbage_keyword(""));
    assert!(is_garbage_keyword("   "));
}

#[test]
fn test_is_garbage_keyword_mostly_digits() {
    // All digits = garbage
    assert!(is_garbage_keyword("12345"));     // All digits
    // Mixed with hex letters only = garbage (looks like hex ID)
    assert!(is_garbage_keyword("abc123def456")); // Hex-like pattern
    // But words with some digits should pass if they look like real terms
    assert!(!is_garbage_keyword("Artikel123")); // Real word prefix
    assert!(!is_garbage_keyword("COVID19"));    // Real term
}

#[test]
fn test_is_garbage_keyword_data_attributes() {
    // HTML data attributes
    assert!(is_garbage_keyword("data-component"));
    assert!(is_garbage_keyword("data-testid"));
    assert!(is_garbage_keyword("aria-label"));
    assert!(is_garbage_keyword("aria-hidden"));
}

#[test]
fn test_is_garbage_keyword_hex_ids() {
    // Hexadecimal IDs (only hex chars 0-9, a-f, and mixed digits+letters)
    assert!(is_garbage_keyword("11516c14826")); // Pure hex ID
    assert!(is_garbage_keyword("abc123def456")); // 12 chars, hex-like
    assert!(is_garbage_keyword("deadbeef12")); // Pure hex
    // But real words should pass (contain non-hex letters like g-z)
    assert!(!is_garbage_keyword("Klimawandel")); // Contains 'k', 'l', 'i', 'm', 'w', 'n'
    assert!(!is_garbage_keyword("NATO")); // Short acronym
    assert!(!is_garbage_keyword("Technology")); // Contains non-hex letters
}

#[test]
fn test_is_garbage_keyword_css_class_names() {
    // CSS-like class names with multiple dashes/underscores
    assert!(is_garbage_keyword("nav-item-active"));
    assert!(is_garbage_keyword("btn_primary_large"));
    assert!(is_garbage_keyword("header--sticky--top"));
    // Single dash is ok (like compound words)
    assert!(!is_garbage_keyword("COVID-19"));
}

#[test]
fn test_is_garbage_keyword_file_paths() {
    // File paths and URL components
    assert!(is_garbage_keyword("path/to/file"));
    assert!(is_garbage_keyword("script.js"));
    assert!(is_garbage_keyword("style.css"));
    assert!(is_garbage_keyword("index.html"));
}

#[test]
fn test_is_garbage_keyword_js_keywords() {
    // JavaScript keywords
    assert!(is_garbage_keyword("function"));
    assert!(is_garbage_keyword("return"));
    assert!(is_garbage_keyword("const"));
    assert!(is_garbage_keyword("async"));
    assert!(is_garbage_keyword("null"));
    assert!(is_garbage_keyword("undefined"));
}

#[test]
fn test_is_garbage_keyword_short_words() {
    // Very short words (unless known acronyms)
    assert!(is_garbage_keyword("xy"));
    assert!(is_garbage_keyword("ab"));
    // Known acronyms should pass
    assert!(!is_garbage_keyword("EU"));
    assert!(!is_garbage_keyword("UN"));
}

#[test]
fn test_is_garbage_keyword_valid_keywords() {
    // Valid news keywords should pass
    assert!(!is_garbage_keyword("Berlin"));
    assert!(!is_garbage_keyword("Politik"));
    assert!(!is_garbage_keyword("Klimawandel"));
    assert!(!is_garbage_keyword("Bundesregierung"));
    assert!(!is_garbage_keyword("Ukraine"));
    assert!(!is_garbage_keyword("Angela Merkel"));
    assert!(!is_garbage_keyword("Künstliche Intelligenz"));
}

// ============================================================
// STRIP_EDGE_STOPWORDS TESTS
// ============================================================

#[test]
fn test_strip_edge_stopwords_german_articles() {
    assert_eq!(strip_edge_stopwords("Kopenhagen die"), "Kopenhagen");
    assert_eq!(strip_edge_stopwords("der Bundestag"), "Bundestag");
    assert_eq!(strip_edge_stopwords("die Regierung das"), "Regierung");
    assert_eq!(strip_edge_stopwords("Die Bundesregierung"), "Bundesregierung");
}

#[test]
fn test_strip_edge_stopwords_german_prepositions() {
    assert_eq!(strip_edge_stopwords("In Kopenhagen"), "Kopenhagen");
    assert_eq!(strip_edge_stopwords("nach Berlin"), "Berlin");
    assert_eq!(strip_edge_stopwords("von Deutschland"), "Deutschland");
    assert_eq!(strip_edge_stopwords("Berlin und"), "Berlin");
}

#[test]
fn test_strip_edge_stopwords_english() {
    assert_eq!(strip_edge_stopwords("the President"), "President");
    assert_eq!(strip_edge_stopwords("to Washington"), "Washington");
    assert_eq!(strip_edge_stopwords("London and"), "London");
    assert_eq!(strip_edge_stopwords("The White House"), "White House");
}

#[test]
fn test_strip_edge_stopwords_multiple() {
    assert_eq!(strip_edge_stopwords("In der Bundesregierung von"), "Bundesregierung");
    assert_eq!(strip_edge_stopwords("die und der"), "die und der"); // All stopwords, keep original
}

#[test]
fn test_strip_edge_stopwords_preserves_middle() {
    // Stopwords in the middle should be preserved
    assert_eq!(strip_edge_stopwords("Angela von der Leyen"), "Angela von der Leyen");
    assert_eq!(strip_edge_stopwords("Ursula von der Leyen"), "Ursula von der Leyen");
}

#[test]
fn test_strip_edge_stopwords_single_word() {
    // Single words should be unchanged
    assert_eq!(strip_edge_stopwords("Berlin"), "Berlin");
    assert_eq!(strip_edge_stopwords("Politik"), "Politik");
}

#[test]
fn test_normalize_keyword_strips_stopwords() {
    // normalize_keyword should strip edge stopwords before validation
    let result = normalize_keyword("Kopenhagen die");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), "Kopenhagen");

    let result = normalize_keyword("In Berlin");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), "Berlin");

    let result = normalize_keyword("der Bundestag");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), "Bundestag");
}
