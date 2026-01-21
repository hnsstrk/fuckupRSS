# Import/Export

fuckupRSS unterstützt den Import und Export von Feed-Abonnements sowie Artikeln in verschiedenen Formaten.

## OPML-Import

Import bestehender Feed-Abonnements aus anderen RSS-Readern im OPML-Format:

```rust
async fn import_opml(path: &Path) -> Result<Vec<Pentacle>> {
    let content = fs::read_to_string(path)?;
    let document = opml::parse(&content)?;

    let mut pentacles = Vec::new();

    for outline in document.body.outlines {
        if let Some(xml_url) = outline.xml_url {
            pentacles.push(Pentacle {
                url: xml_url,
                title: outline.title.or(outline.text),
                ..Default::default()
            });
        }
    }

    Ok(pentacles)
}
```

**Unterstützte Reader:**
- Feedly, Inoreader, NewsBlur
- Thunderbird, Outlook
- Andere OPML 2.0 kompatible Reader

## OPML-Export

Export aller abonnierten Feeds im OPML 2.0 Format:

```rust
fn export_opml(pentacles: &[Pentacle]) -> String {
    // OPML 2.0 Format generieren
}
```

## Artikel-Export

| Format | Zweck | Inhalt |
|--------|-------|--------|
| **Markdown** | Einzelner Artikel | Titel, Content, Metadaten |
| **HTML** | Mit Formatierung | Vollständiger Artikel |
| **JSON** | Mit allen Metadaten | Artikel + KI-Analyse |
| **PDF** | Zum Archivieren | Formatierter Artikel |

## Relevante Quelldateien

- `src-tauri/src/commands/import_export.rs` - Import/Export Tauri Commands (geplant)
- `src/lib/components/SettingsView.svelte` - UI für Import/Export
