use crate::AppState;
use opml::{Head, Outline, OPML};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct OpmlImportResult {
    pub total_feeds: usize,
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpmlFeedPreview {
    pub url: String,
    pub title: Option<String>,
    pub category: Option<String>,
    pub already_exists: bool,
}

/// Parse OPML content and return a preview of feeds to import
#[tauri::command]
pub fn parse_opml_preview(
    state: State<AppState>,
    content: String,
) -> Result<Vec<OpmlFeedPreview>, String> {
    let opml = OPML::from_str(&content).map_err(|e| format!("OPML parse error: {}", e))?;

    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get existing feed URLs for duplicate detection
    let existing_urls: Vec<String> = {
        let mut stmt = db
            .conn()
            .prepare("SELECT url FROM pentacles")
            .map_err(|e| e.to_string())?;

        let urls: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        urls
    };

    let mut feeds = Vec::new();
    collect_feeds_from_outlines(&opml.body.outlines, None, &existing_urls, &mut feeds);

    Ok(feeds)
}

/// Import feeds from OPML content
#[tauri::command]
pub fn import_opml(
    state: State<AppState>,
    content: String,
    skip_existing: bool,
) -> Result<OpmlImportResult, String> {
    let opml = OPML::from_str(&content).map_err(|e| format!("OPML parse error: {}", e))?;

    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get existing feed URLs for duplicate detection
    let existing_urls: Vec<String> = {
        let mut stmt = db
            .conn()
            .prepare("SELECT url FROM pentacles")
            .map_err(|e| e.to_string())?;

        let urls: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        urls
    };

    let mut feeds = Vec::new();
    collect_feeds_from_outlines(&opml.body.outlines, None, &existing_urls, &mut feeds);

    let total_feeds = feeds.len();
    let mut imported = 0;
    let mut skipped = 0;
    let mut errors = Vec::new();

    for feed in feeds {
        if feed.already_exists
            && skip_existing {
                skipped += 1;
                continue;
            }

        // Insert feed into database
        match db.conn().execute(
            "INSERT OR IGNORE INTO pentacles (url, title) VALUES (?1, ?2)",
            (&feed.url, &feed.title),
        ) {
            Ok(rows) => {
                if rows > 0 {
                    imported += 1;
                } else {
                    skipped += 1;
                }
            }
            Err(e) => {
                errors.push(format!("{}: {}", feed.url, e));
            }
        }
    }

    Ok(OpmlImportResult {
        total_feeds,
        imported,
        skipped,
        errors,
    })
}

/// Export all feeds to OPML format
#[tauri::command]
pub fn export_opml(state: State<AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get all feeds from database
    let feeds: Vec<(String, Option<String>, Option<String>)> = {
        let mut stmt = db
            .conn()
            .prepare("SELECT url, title, site_url FROM pentacles ORDER BY title")
            .map_err(|e| e.to_string())?;

        let result: Vec<(String, Option<String>, Option<String>)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        result
    };

    // Build OPML structure
    let mut opml = OPML {
        version: "2.0".to_string(),
        head: Some(Head {
            title: Some("fuckupRSS Feeds".to_string()),
            ..Head::default()
        }),
        ..OPML::default()
    };

    // Create outline for each feed
    for (url, title, site_url) in feeds {
        let display_title = title.clone().unwrap_or_else(|| url.clone());
        let outline = Outline {
            text: display_title.clone(),
            title,
            r#type: Some("rss".to_string()),
            xml_url: Some(url),
            html_url: site_url,
            ..Outline::default()
        };

        opml.body.outlines.push(outline);
    }

    // Serialize to XML string
    opml.to_string().map_err(|e| format!("OPML serialization error: {}", e))
}

/// Recursively collect feeds from OPML outlines
fn collect_feeds_from_outlines(
    outlines: &[Outline],
    category: Option<&str>,
    existing_urls: &[String],
    feeds: &mut Vec<OpmlFeedPreview>,
) {
    for outline in outlines {
        // Check if this is a feed (has xmlUrl) or a category (has children)
        if let Some(xml_url) = &outline.xml_url {
            // This is a feed
            let url = xml_url.trim().to_string();
            if !url.is_empty() {
                // Get title from title or text field
                let title = if let Some(ref t) = outline.title {
                    if !t.is_empty() {
                        Some(t.clone())
                    } else {
                        None
                    }
                } else if !outline.text.is_empty() {
                    Some(outline.text.clone())
                } else {
                    None
                };

                let already_exists = existing_urls.iter().any(|u| u == &url);

                feeds.push(OpmlFeedPreview {
                    url,
                    title,
                    category: category.map(|s| s.to_string()),
                    already_exists,
                });
            }
        }

        // Recursively process children (for nested categories)
        if !outline.outlines.is_empty() {
            let cat_name = outline
                .title
                .as_deref()
                .unwrap_or_else(|| {
                    if outline.text.is_empty() {
                        category.unwrap_or("")
                    } else {
                        &outline.text
                    }
                });
            let cat_name = if cat_name.is_empty() { None } else { Some(cat_name) };
            collect_feeds_from_outlines(&outline.outlines, cat_name, existing_urls, feeds);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_OPML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head>
    <title>My Feeds</title>
  </head>
  <body>
    <outline text="Tech" title="Tech">
      <outline type="rss" text="Ars Technica" title="Ars Technica"
               xmlUrl="https://feeds.arstechnica.com/arstechnica/index"
               htmlUrl="https://arstechnica.com"/>
      <outline type="rss" text="Hacker News"
               xmlUrl="https://news.ycombinator.com/rss"/>
    </outline>
    <outline type="rss" text="BBC News" title="BBC News"
             xmlUrl="https://feeds.bbci.co.uk/news/rss.xml"
             htmlUrl="https://www.bbc.com/news"/>
  </body>
</opml>"#;

    #[test]
    fn test_parse_opml() {
        let opml = OPML::from_str(SAMPLE_OPML).unwrap();
        assert_eq!(opml.body.outlines.len(), 2);
    }

    #[test]
    fn test_collect_feeds() {
        let opml = OPML::from_str(SAMPLE_OPML).unwrap();
        let existing: Vec<String> = vec![];
        let mut feeds = Vec::new();

        collect_feeds_from_outlines(&opml.body.outlines, None, &existing, &mut feeds);

        assert_eq!(feeds.len(), 3);

        // Check Tech category feeds
        let ars = feeds.iter().find(|f| f.url.contains("arstechnica")).unwrap();
        assert_eq!(ars.title, Some("Ars Technica".to_string()));
        assert_eq!(ars.category, Some("Tech".to_string()));

        let hn = feeds.iter().find(|f| f.url.contains("ycombinator")).unwrap();
        assert_eq!(hn.title, Some("Hacker News".to_string()));
        assert_eq!(hn.category, Some("Tech".to_string()));

        // Check uncategorized feed
        let bbc = feeds.iter().find(|f| f.url.contains("bbc")).unwrap();
        assert_eq!(bbc.title, Some("BBC News".to_string()));
        assert_eq!(bbc.category, None);
    }

    #[test]
    fn test_detect_existing_feeds() {
        let opml = OPML::from_str(SAMPLE_OPML).unwrap();
        let existing = vec!["https://feeds.bbci.co.uk/news/rss.xml".to_string()];
        let mut feeds = Vec::new();

        collect_feeds_from_outlines(&opml.body.outlines, None, &existing, &mut feeds);

        let bbc = feeds.iter().find(|f| f.url.contains("bbc")).unwrap();
        assert!(bbc.already_exists);

        let ars = feeds.iter().find(|f| f.url.contains("arstechnica")).unwrap();
        assert!(!ars.already_exists);
    }

    #[test]
    fn test_empty_opml() {
        // OPML with only category outline (no feeds)
        let empty_opml = r#"<?xml version="1.0"?>
<opml version="2.0">
  <head><title>Empty</title></head>
  <body>
    <outline text="Empty Category"/>
  </body>
</opml>"#;

        let opml = OPML::from_str(empty_opml).unwrap();
        let mut feeds = Vec::new();
        collect_feeds_from_outlines(&opml.body.outlines, None, &[], &mut feeds);

        // No feeds, just an empty category
        assert!(feeds.is_empty());
    }

    #[test]
    fn test_nested_categories() {
        let nested_opml = r#"<?xml version="1.0"?>
<opml version="2.0">
  <head><title>Nested</title></head>
  <body>
    <outline text="News">
      <outline text="German">
        <outline type="rss" text="Tagesschau" xmlUrl="https://tagesschau.de/rss"/>
      </outline>
    </outline>
  </body>
</opml>"#;

        let opml = OPML::from_str(nested_opml).unwrap();
        let mut feeds = Vec::new();
        collect_feeds_from_outlines(&opml.body.outlines, None, &[], &mut feeds);

        assert_eq!(feeds.len(), 1);
        // Innermost category wins
        assert_eq!(feeds[0].category, Some("German".to_string()));
    }

    #[test]
    fn test_generate_opml() {
        // Test that we can generate valid OPML
        let mut opml = OPML::default();
        opml.version = "2.0".to_string();
        opml.head = Some(Head {
            title: Some("Test Feeds".to_string()),
            ..Head::default()
        });

        let mut outline = Outline::default();
        outline.text = "Test Feed".to_string();
        outline.title = Some("Test Feed".to_string());
        outline.r#type = Some("rss".to_string());
        outline.xml_url = Some("https://example.com/feed.xml".to_string());
        outline.html_url = Some("https://example.com".to_string());
        opml.body.outlines.push(outline);

        let xml = opml.to_string().unwrap();
        assert!(xml.contains("Test Feed"));
        assert!(xml.contains("https://example.com/feed.xml"));
        assert!(xml.contains("version=\"2.0\""));
    }

    #[test]
    fn test_roundtrip_opml() {
        // Generate OPML
        let mut opml = OPML::default();
        opml.version = "2.0".to_string();
        opml.head = Some(Head {
            title: Some("Roundtrip Test".to_string()),
            ..Head::default()
        });

        let mut outline = Outline::default();
        outline.text = "Ars Technica".to_string();
        outline.title = Some("Ars Technica".to_string());
        outline.r#type = Some("rss".to_string());
        outline.xml_url = Some("https://feeds.arstechnica.com/arstechnica/index".to_string());
        outline.html_url = Some("https://arstechnica.com".to_string());
        opml.body.outlines.push(outline);

        let xml = opml.to_string().unwrap();

        // Parse it back
        let parsed = OPML::from_str(&xml).unwrap();
        assert_eq!(parsed.body.outlines.len(), 1);
        assert_eq!(
            parsed.body.outlines[0].xml_url,
            Some("https://feeds.arstechnica.com/arstechnica/index".to_string())
        );
        assert_eq!(
            parsed.body.outlines[0].title,
            Some("Ars Technica".to_string())
        );
    }
}
