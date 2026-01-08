use super::*;

#[test]
fn test_sephiroth_categories_count() {
    assert_eq!(SEPHIROTH_CATEGORIES.len(), 13);
}

#[test]
fn test_sephiroth_categories_content() {
    assert!(SEPHIROTH_CATEGORIES.contains(&"Politik"));
    assert!(SEPHIROTH_CATEGORIES.contains(&"Wirtschaft"));
    assert!(SEPHIROTH_CATEGORIES.contains(&"Technik"));
    assert!(SEPHIROTH_CATEGORIES.contains(&"Sport"));
}

#[test]
fn test_normalize_category_german() {
    let classifier = CategoryClassifier::default();

    assert_eq!(classifier.normalize_category("Politik"), "Politik");
    assert_eq!(classifier.normalize_category("politik"), "Politik");
    assert_eq!(classifier.normalize_category("POLITIK"), "Politik");
    assert_eq!(classifier.normalize_category("  Politik  "), "Politik");
}

#[test]
fn test_normalize_category_english_to_german() {
    let classifier = CategoryClassifier::default();

    assert_eq!(classifier.normalize_category("politics"), "Politik");
    assert_eq!(classifier.normalize_category("economy"), "Wirtschaft");
    assert_eq!(classifier.normalize_category("technology"), "Technik");
    assert_eq!(classifier.normalize_category("sports"), "Sport");
    assert_eq!(classifier.normalize_category("health"), "Gesundheit");
    assert_eq!(classifier.normalize_category("environment"), "Umwelt");
}

#[test]
fn test_parse_categories_simple() {
    let classifier = CategoryClassifier::default();

    let response = "Politik, Wirtschaft, Technik";
    let categories = classifier.parse_categories(response);

    assert_eq!(categories.len(), 3);
    assert!(categories.contains(&"Politik".to_string()));
    assert!(categories.contains(&"Wirtschaft".to_string()));
    assert!(categories.contains(&"Technik".to_string()));
}

#[test]
fn test_parse_categories_with_newlines() {
    let classifier = CategoryClassifier::default();

    let response = "Politik\nWirtschaft\nTechnik";
    let categories = classifier.parse_categories(response);

    assert_eq!(categories.len(), 3);
}

#[test]
fn test_parse_categories_filters_invalid() {
    let classifier = CategoryClassifier::default();

    let response = "Politik, InvalidCategory, Wirtschaft, NotACategory";
    let categories = classifier.parse_categories(response);

    assert_eq!(categories.len(), 2);
    assert!(categories.contains(&"Politik".to_string()));
    assert!(categories.contains(&"Wirtschaft".to_string()));
}

#[test]
fn test_parse_categories_max_five() {
    let classifier = CategoryClassifier::default();

    let response = "Politik, Wirtschaft, Technik, Sport, Kultur, Gesundheit, Umwelt, Recht";
    let categories = classifier.parse_categories(response);

    assert!(
        categories.len() <= 5,
        "Should limit to 5 categories, got {}",
        categories.len()
    );
}

#[test]
fn test_parse_categories_dedup() {
    let classifier = CategoryClassifier::default();

    let response = "Politik, Politik, Wirtschaft, Politik";
    let categories = classifier.parse_categories(response);

    let politik_count = categories.iter().filter(|c| *c == "Politik").count();
    assert_eq!(politik_count, 1, "Should deduplicate categories");
}

#[test]
fn test_classify_by_keywords_politics() {
    let keywords = vec![
        "Bundeskanzler".to_string(),
        "Regierung".to_string(),
        "Gesetz".to_string(),
        "Bundestag".to_string(),
    ];

    let categories = classify_by_keywords(&keywords);

    assert!(!categories.is_empty());
    assert!(
        categories.contains(&"Politik".to_string()),
        "Should detect Politik from keywords"
    );
}

#[test]
fn test_classify_by_keywords_tech() {
    let keywords = vec![
        "Software".to_string(),
        "KI".to_string(),
        "Computer".to_string(),
        "Algorithmus".to_string(),
    ];

    let categories = classify_by_keywords(&keywords);

    assert!(!categories.is_empty());
    assert!(
        categories.contains(&"Technik".to_string()),
        "Should detect Technik from keywords"
    );
}

#[test]
fn test_classify_by_keywords_mixed() {
    let keywords = vec![
        "Bundeswehr".to_string(),
        "NATO".to_string(),
        "Rakete".to_string(),
        "Bundeskanzler".to_string(),
        "Regierung".to_string(),
    ];

    let categories = classify_by_keywords(&keywords);

    assert!(categories.len() >= 2, "Should detect multiple categories");
    assert!(
        categories.contains(&"Verteidigung".to_string())
            || categories.contains(&"Politik".to_string())
    );
}

#[test]
fn test_classify_by_keywords_empty() {
    let keywords: Vec<String> = vec![];
    let categories = classify_by_keywords(&keywords);

    assert!(categories.is_empty());
}

#[test]
fn test_classify_by_keywords_no_match() {
    let keywords = vec!["xyz123".to_string(), "unbekannt".to_string()];

    let categories = classify_by_keywords(&keywords);
    assert!(categories.is_empty());
}

#[test]
fn test_classify_by_keywords_max_five() {
    let keywords = vec![
        "Bundeskanzler".to_string(),
        "Software".to_string(),
        "Bundeswehr".to_string(),
        "Krankenhaus".to_string(),
        "Klima".to_string(),
        "Gericht".to_string(),
        "Aktie".to_string(),
    ];

    let categories = classify_by_keywords(&keywords);

    assert!(categories.len() <= 5, "Should limit to 5 categories");
}
