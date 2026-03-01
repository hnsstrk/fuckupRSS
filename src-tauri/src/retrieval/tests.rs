//! Retrieval module tests

use super::*;

#[test]
fn test_is_truncated_short_content() {
    // Content less than 500 chars should be considered truncated
    let short = "This is a short article.";
    assert!(
        HagbardRetrieval::is_truncated(short),
        "Short content should be considered truncated"
    );
}

#[test]
fn test_is_truncated_long_content() {
    // Content longer than 500 chars without truncation markers
    let long = "A".repeat(600);
    assert!(
        !HagbardRetrieval::is_truncated(&long),
        "Long content without markers should not be truncated"
    );
}

#[test]
fn test_is_truncated_ellipsis_pattern() {
    let content = "A".repeat(600) + "...";
    assert!(
        HagbardRetrieval::is_truncated(&content),
        "Content ending with '...' should be truncated"
    );
}

#[test]
fn test_is_truncated_unicode_ellipsis() {
    let content = "A".repeat(600) + "…";
    assert!(
        HagbardRetrieval::is_truncated(&content),
        "Content ending with '…' should be truncated"
    );
}

#[test]
fn test_is_truncated_read_more() {
    let content = "A".repeat(600) + " read more";
    assert!(
        HagbardRetrieval::is_truncated(&content),
        "Content ending with 'read more' should be truncated"
    );
}

#[test]
fn test_is_truncated_weiterlesen() {
    let content = "A".repeat(600) + " weiterlesen";
    assert!(
        HagbardRetrieval::is_truncated(&content),
        "Content ending with 'weiterlesen' should be truncated"
    );
}

#[test]
fn test_is_truncated_continue_reading() {
    let content = "A".repeat(600) + " continue reading";
    assert!(
        HagbardRetrieval::is_truncated(&content),
        "Content ending with 'continue reading' should be truncated"
    );
}

#[test]
fn test_is_truncated_brackets_ellipsis() {
    let content = "A".repeat(600) + " [...]";
    assert!(
        HagbardRetrieval::is_truncated(&content),
        "Content ending with '[...]' should be truncated"
    );
}

#[test]
fn test_is_truncated_mehr_lesen() {
    let content = "A".repeat(600) + " mehr lesen";
    assert!(
        HagbardRetrieval::is_truncated(&content),
        "Content ending with 'mehr lesen' should be truncated"
    );
}

#[test]
fn test_is_truncated_read_full() {
    let content = "A".repeat(600) + " read the full";
    assert!(
        HagbardRetrieval::is_truncated(&content),
        "Content ending with 'read the full' should be truncated"
    );
}

#[test]
fn test_is_truncated_case_insensitive() {
    let content = "A".repeat(600) + " READ MORE";
    assert!(
        HagbardRetrieval::is_truncated(&content),
        "Pattern matching should be case insensitive"
    );
}

#[test]
fn test_is_truncated_pattern_in_html() {
    let content = "A".repeat(600) + " weiterlesen</a>";
    assert!(
        HagbardRetrieval::is_truncated(&content),
        "Should detect pattern in HTML context"
    );
}

#[test]
fn test_is_truncated_normal_ending() {
    let content = "A".repeat(600) + ". This is the end of the article.";
    assert!(
        !HagbardRetrieval::is_truncated(&content),
        "Normal article ending should not be truncated"
    );
}

#[test]
fn test_is_truncated_whitespace_handling() {
    let content = "A".repeat(600) + "   ";
    assert!(
        !HagbardRetrieval::is_truncated(&content),
        "Trailing whitespace should be trimmed"
    );
}

#[test]
fn test_is_truncated_exactly_500_chars() {
    let content = "A".repeat(500);
    assert!(
        !HagbardRetrieval::is_truncated(&content),
        "Exactly 500 chars should not be truncated"
    );
}

#[test]
fn test_is_truncated_499_chars() {
    let content = "A".repeat(499);
    assert!(
        HagbardRetrieval::is_truncated(&content),
        "499 chars should be truncated"
    );
}

#[test]
fn test_hagbard_retrieval_creation() {
    let retrieval = HagbardRetrieval::new().expect("Failed to create HagbardRetrieval");
    // Just verify it can be created without error
    drop(retrieval);
}

#[test]
fn test_hagbard_retrieval_default() {
    let retrieval = HagbardRetrieval::default();
    // Just verify default() works
    drop(retrieval);
}

#[test]
fn test_extracted_article_struct() {
    let article = ExtractedArticle {
        title: Some("Test Title".to_string()),
        content: "<p>HTML content</p>".to_string(),
        text_content: "Plain text content".to_string(),
    };

    assert_eq!(article.title.as_deref(), Some("Test Title"));
    assert!(article.content.contains("<p>"));
    assert!(!article.text_content.contains("<p>"));
}

#[test]
fn test_extracted_article_no_title() {
    let article = ExtractedArticle {
        title: None,
        content: "Content".to_string(),
        text_content: "Text".to_string(),
    };

    assert!(article.title.is_none());
}

#[test]
fn test_retrieval_error_display() {
    let error = RetrievalError::Extraction("test error".to_string());
    let display = format!("{}", error);
    assert!(display.contains("test error"));
}

#[test]
fn test_url_parse_error() {
    let result = Url::parse("not a valid url");
    assert!(result.is_err());
}
