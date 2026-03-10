//! Integration test for OpenAI-compatible API provider.
//!
//! Reads 5 real articles from the database, sends them to the OpenAI API,
//! and validates the responses.
//!
//! Run with: cargo test --manifest-path src-tauri/Cargo.toml --test openai_integration_test -- --ignored --nocapture

use fuckuprss_lib::ai_provider::{create_provider, ProviderConfig, ProviderType};
use rusqlite::Connection;
use serde::Deserialize;

// ============================================================
// Test article struct
// ============================================================

struct TestArticle {
    id: i64,
    title: String,
    content: String,
}

// ============================================================
// Response struct (mirrors the private RawDiscordianAnalysisWithRejections)
// ============================================================

#[derive(Deserialize, Debug)]
struct DiscordianResponse {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    categories: Vec<serde_json::Value>,
    #[serde(default)]
    keywords: Vec<serde_json::Value>,
    #[serde(default)]
    rejected_keywords: Vec<serde_json::Value>,
    #[serde(default)]
    rejected_categories: Vec<serde_json::Value>,
    #[serde(default)]
    political_bias: f64,
    #[serde(default)]
    sachlichkeit: f64,
}

// ============================================================
// DB helpers
// ============================================================

fn load_settings(conn: &Connection) -> (String, String, String) {
    let get = |key: &str, default: &str| -> String {
        conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
            row.get::<_, String>(0)
        })
        .unwrap_or_else(|_| default.to_string())
    };

    let base_url = get("openai_base_url", "https://api.openai.com");
    let api_key = get("openai_api_key", "");
    let model = get("openai_model", "gpt-5-nano");
    (base_url, api_key, model)
}

fn load_articles(conn: &Connection, limit: usize) -> Vec<TestArticle> {
    let mut stmt = conn
        .prepare(
            r#"SELECT id, title, content_full FROM fnords
               WHERE content_full IS NOT NULL AND LENGTH(content_full) >= 200
               ORDER BY published_at DESC
               LIMIT ?1"#,
        )
        .expect("Failed to prepare article query");

    stmt.query_map([limit], |row| {
        Ok(TestArticle {
            id: row.get(0)?,
            title: row.get(1)?,
            content: row.get(2)?,
        })
    })
    .expect("Failed to query articles")
    .filter_map(|r| r.ok())
    .collect()
}

fn build_discordian_prompt(title: &str, content: &str) -> String {
    let truncated: String = content.chars().take(6000).collect();
    format!(
        r#"Analyze this article. Statistical pre-analysis already computed keywords and categories.

PRE-COMPUTED: keywords=none, categories=none

YOUR TASKS:
1. Write summary (2-3 factual sentences in German)
2. Assess political_bias: -2=strong left, -1=left, 0=neutral, 1=right, 2=strong right
3. Assess sachlichkeit: 0=emotional/sensational, 2=mixed, 4=objective/factual
4. Review keywords: keep good ones, add max 2 important missing ones
5. Categories: only provide if pre-computed ones are clearly wrong (empty [] is fine)

Return ONLY valid JSON:
{{
  "political_bias": <-2 to 2>,
  "sachlichkeit": <0 to 4>,
  "summary": "<summary in German>",
  "keywords": ["kw1", "kw2", "..."],
  "categories": [],
  "rejected_keywords": [],
  "rejected_categories": []
}}

Title: {title}
Content: {truncated}"#
    )
}

fn build_summary_prompt(content: &str) -> String {
    let truncated: String = content.chars().take(8000).collect();
    format!(
        "Fasse den folgenden Nachrichtenartikel in 2-3 Saetzen auf Deutsch zusammen. \
         Antworte NUR mit der Zusammenfassung, ohne Einleitung oder Erklaerung.\n\n{}",
        truncated
    )
}

// ============================================================
// Tests
// ============================================================

#[tokio::test]
#[ignore] // Requires real OpenAI API key and network access
async fn test_openai_discordian_analysis_5_articles() {
    let db_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/fuckup.db");
    assert!(db_path.exists(), "Database not found at {:?}", db_path);

    let conn = Connection::open(&db_path).expect("Failed to open database");
    let (base_url, api_key, model) = load_settings(&conn);

    assert!(!api_key.is_empty(), "OpenAI API key not configured in DB");
    println!("\n============================================================");
    println!("OpenAI Integration Test");
    println!("  Provider: {}", base_url);
    println!("  Model:    {}", model);
    println!("============================================================\n");

    let config = ProviderConfig {
        provider_type: ProviderType::OpenAiCompatible,
        ollama_url: String::new(),
        ollama_model: String::new(),
        ollama_num_ctx: 4096,
        ollama_concurrency: 1,
        openai_base_url: base_url.clone(),
        openai_api_key: api_key.clone(),
        openai_model: model.clone(),
        openai_temperature: None,
    };
    let provider = create_provider(&config);

    // Test 1: Provider availability
    println!("[TEST] Checking provider availability...");
    let available = provider.is_available().await;
    assert!(
        available,
        "OpenAI provider is not available. Check API key and URL."
    );
    println!("[PASS] Provider is available\n");

    // Test 2: Load 5 articles
    let articles = load_articles(&conn, 5);
    assert!(
        articles.len() >= 3,
        "Not enough articles in DB (found {}), need at least 3",
        articles.len()
    );
    println!("[INFO] Loaded {} articles for testing\n", articles.len());

    let mut success_count = 0;
    let mut total_input_tokens = 0u32;
    let mut total_output_tokens = 0u32;

    // Test 3: Discordian Analysis (JSON mode) for each article
    for (i, article) in articles.iter().enumerate() {
        println!(
            "[TEST {}/{}] Discordian Analysis: \"{}\" (ID: {})",
            i + 1,
            articles.len(),
            &article.title[..article.title.len().min(60)],
            article.id
        );

        let prompt = build_discordian_prompt(&article.title, &article.content);
        let start = std::time::Instant::now();

        let schema = fuckuprss_lib::ollama::discordian_schema();
        match provider.generate_text(&model, &prompt, Some(schema)).await {
            Ok(result) => {
                let elapsed = start.elapsed();
                println!(
                    "  Response in {:.2}s ({} chars)",
                    elapsed.as_secs_f64(),
                    result.text.len()
                );

                // Check token counts
                match (result.input_tokens, result.output_tokens) {
                    (Some(input), Some(output)) => {
                        println!("  Tokens: {} input, {} output", input, output);
                        total_input_tokens += input;
                        total_output_tokens += output;
                    }
                    _ => {
                        println!("  WARNING: No token counts returned!");
                    }
                }

                // Parse JSON response
                match serde_json::from_str::<DiscordianResponse>(&result.text) {
                    Ok(analysis) => {
                        // Validate fields
                        let summary_ok =
                            !analysis.summary.is_empty() && analysis.summary.len() > 20;
                        let bias_ok =
                            analysis.political_bias >= -2.0 && analysis.political_bias <= 2.0;
                        let sach_ok = analysis.sachlichkeit >= 0.0 && analysis.sachlichkeit <= 4.0;
                        let keywords_ok = !analysis.keywords.is_empty();

                        println!(
                            "  Summary:  {} chars {}",
                            analysis.summary.len(),
                            if summary_ok { "OK" } else { "FAIL" }
                        );
                        println!(
                            "  Bias:     {} {}",
                            analysis.political_bias,
                            if bias_ok { "OK" } else { "FAIL" }
                        );
                        println!(
                            "  Sachlich: {} {}",
                            analysis.sachlichkeit,
                            if sach_ok { "OK" } else { "FAIL" }
                        );
                        println!(
                            "  Keywords: {} {}",
                            analysis.keywords.len(),
                            if keywords_ok { "OK" } else { "FAIL" }
                        );
                        println!("  Categories: {}", analysis.categories.len());

                        if summary_ok && bias_ok && sach_ok && keywords_ok {
                            println!("  [PASS]");
                            success_count += 1;
                        } else {
                            println!("  [FAIL] Some fields invalid");
                        }
                    }
                    Err(e) => {
                        println!("  [FAIL] JSON parse error: {}", e);
                        println!(
                            "  Raw response: {}",
                            &result.text[..result.text.len().min(300)]
                        );
                    }
                }
            }
            Err(e) => {
                println!("  [FAIL] API error: {}", e);
            }
        }
        println!();
    }

    // Test 4: Summary mode (plain text, no JSON)
    println!("[TEST] Summary mode (plain text)...");
    let summary_prompt = build_summary_prompt(&articles[0].content);
    let start = std::time::Instant::now();

    match provider.generate_text(&model, &summary_prompt, None).await {
        Ok(result) => {
            let elapsed = start.elapsed();
            println!(
                "  Response in {:.2}s ({} chars)",
                elapsed.as_secs_f64(),
                result.text.len()
            );
            assert!(
                result.text.len() > 20,
                "Summary too short: {}",
                result.text.len()
            );

            if let (Some(input), Some(output)) = (result.input_tokens, result.output_tokens) {
                println!("  Tokens: {} input, {} output", input, output);
                total_input_tokens += input;
                total_output_tokens += output;
            }
            println!("  Summary: {}", &result.text[..result.text.len().min(200)]);
            println!("  [PASS]\n");
        }
        Err(e) => {
            println!("  [FAIL] API error: {}\n", e);
            panic!("Summary generation failed");
        }
    }

    // Final report
    println!("============================================================");
    println!("RESULTS");
    println!("============================================================");
    println!(
        "  Discordian Analysis: {}/{} passed",
        success_count,
        articles.len()
    );
    println!("  Summary Mode:        PASS");
    println!(
        "  Total tokens:        {} input, {} output",
        total_input_tokens, total_output_tokens
    );

    if total_input_tokens > 0 {
        // gpt-5-nano pricing: $0.05/1M input, $0.40/1M output
        let cost_input = (total_input_tokens as f64 / 1_000_000.0) * 0.05;
        let cost_output = (total_output_tokens as f64 / 1_000_000.0) * 0.40;
        println!(
            "  Estimated cost:      ${:.6} (input: ${:.6}, output: ${:.6})",
            cost_input + cost_output,
            cost_input,
            cost_output
        );
    }

    println!(
        "  Token tracking:      {}",
        if total_input_tokens > 0 {
            "WORKING"
        } else {
            "NOT WORKING"
        }
    );
    println!();

    assert!(
        success_count >= 3,
        "Too many failures: only {}/{} articles parsed successfully",
        success_count,
        articles.len()
    );
}
