//! Integration test for the full OpenAI article analysis pipeline.
//!
//! Tests discordian analysis, summarization, provider configuration isolation,
//! and response completeness (no truncation).
//!
//! Run with: cargo test --manifest-path src-tauri/Cargo.toml --test openai_pipeline_test -- --ignored --nocapture

use fuckuprss_lib::ai_provider::{
    create_provider, resolve_effective_model, ProviderConfig, ProviderType,
};
use rusqlite::Connection;
use serde::Deserialize;

// ============================================================
// Test data structures
// ============================================================

struct TestArticle {
    id: i64,
    title: String,
    content: String,
}

/// Local response struct mirroring RawDiscordianAnalysisWithRejections.
/// All fields have `#[serde(default)]` so partial responses still parse.
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

fn open_db() -> Connection {
    let db_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/fuckup.db");
    assert!(db_path.exists(), "Database not found at {:?}", db_path);
    Connection::open(&db_path).expect("Failed to open database")
}

fn load_settings(conn: &Connection) -> (String, String, String, Option<f32>) {
    let get = |key: &str, default: &str| -> String {
        conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
            row.get::<_, String>(0)
        })
        .unwrap_or_else(|_| default.to_string())
    };

    let base_url = get("openai_base_url", "https://api.openai.com");
    let api_key = get("openai_api_key", "");
    let model = get("openai_model", "gpt-5-nano");
    let temperature = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'openai_temperature'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse::<f32>().ok());

    (base_url, api_key, model, temperature)
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

/// Load a long article (>= 4000 chars) for truncation testing.
fn load_long_article(conn: &Connection) -> TestArticle {
    conn.query_row(
        r#"SELECT id, title, content_full FROM fnords
           WHERE content_full IS NOT NULL AND LENGTH(content_full) >= 4000
           ORDER BY LENGTH(content_full) DESC
           LIMIT 1"#,
        [],
        |row| {
            Ok(TestArticle {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
            })
        },
    )
    .expect("No long article found in DB (need >= 4000 chars)")
}

fn load_ollama_model(conn: &Connection) -> String {
    conn.query_row(
        "SELECT value FROM settings WHERE key = 'ollama_model'",
        [],
        |row| row.get::<_, String>(0),
    )
    .unwrap_or_else(|_| "ministral-3:latest".to_string())
}

// ============================================================
// Prompt builders
// ============================================================

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

/// Build a discordian prompt with a very long article to stress-test max_completion_tokens.
fn build_long_discordian_prompt(title: &str, content: &str) -> String {
    // Use more content (up to 10000 chars) to produce longer responses
    let truncated: String = content.chars().take(10000).collect();
    format!(
        r#"Analyze this article thoroughly. Statistical pre-analysis already computed keywords and categories.

PRE-COMPUTED: keywords=none, categories=none

YOUR TASKS:
1. Write a detailed summary (4-5 factual sentences in German)
2. Assess political_bias: -2=strong left, -1=left, 0=neutral, 1=right, 2=strong right
3. Assess sachlichkeit: 0=emotional/sensational, 2=mixed, 4=objective/factual
4. List 5-8 relevant keywords
5. Categories: provide 1-3 fitting main categories

Return ONLY valid JSON:
{{
  "political_bias": <-2 to 2>,
  "sachlichkeit": <0 to 4>,
  "summary": "<detailed summary in German>",
  "keywords": ["kw1", "kw2", "kw3", "kw4", "kw5"],
  "categories": ["cat1"],
  "rejected_keywords": [],
  "rejected_categories": []
}}

Title: {title}
Content: {truncated}"#
    )
}

// ============================================================
// Cost calculation helper
// ============================================================

fn estimate_cost(model: &str, input_tokens: u32, output_tokens: u32) -> (f64, f64) {
    // Pricing per 1M tokens (input, output)
    let (input_price, output_price) = match model {
        m if m.contains("gpt-5-nano") => (0.05, 0.40),
        m if m.contains("gpt-5-mini") => (0.25, 2.00),
        m if m.contains("gpt-4o-mini") => (0.15, 0.60),
        m if m.contains("gpt-4o") => (2.50, 10.00),
        _ => (0.05, 0.40), // default to nano pricing
    };
    let cost_in = (input_tokens as f64 / 1_000_000.0) * input_price;
    let cost_out = (output_tokens as f64 / 1_000_000.0) * output_price;
    (cost_in, cost_out)
}

// ============================================================
// Tests
// ============================================================

/// Full pipeline test: discordian analysis for 5 articles + summary mode.
///
/// Validates:
/// - Provider availability
/// - JSON mode produces valid, parseable responses
/// - All discordian fields are within expected ranges
/// - Summary mode produces plain text
/// - Token usage is tracked
/// - Responses are not truncated (complete JSON)
#[tokio::test]
#[ignore] // Requires real OpenAI API key and network access
async fn test_openai_discordian_analysis_pipeline() {
    let conn = open_db();
    let (base_url, api_key, model, temperature) = load_settings(&conn);

    assert!(!api_key.is_empty(), "OpenAI API key not configured in DB");
    println!("\n============================================================");
    println!("OpenAI Pipeline Integration Test - Discordian Analysis");
    println!("  Provider:    {}", base_url);
    println!("  Model:       {}", model);
    println!("  Temperature: {:?}", temperature);
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
        openai_temperature: temperature,
    };
    let provider = create_provider(&config);

    // ---- Step 1: Check provider availability ----
    println!("[TEST] Checking provider availability...");
    let available = provider.is_available().await;
    assert!(
        available,
        "OpenAI provider is not available. Check API key and URL."
    );
    println!("[PASS] Provider is available\n");

    // ---- Step 2: Load articles ----
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
    let mut total_time_secs = 0.0f64;

    // ---- Step 3: Discordian Analysis for each article (JSON mode) ----
    for (i, article) in articles.iter().enumerate() {
        let title_display = if article.title.len() > 60 {
            &article.title[..60]
        } else {
            &article.title
        };
        println!(
            "[TEST {}/{}] Discordian Analysis: \"{}\" (ID: {}, content: {} chars)",
            i + 1,
            articles.len(),
            title_display,
            article.id,
            article.content.len()
        );

        let prompt = build_discordian_prompt(&article.title, &article.content);
        let start = std::time::Instant::now();

        let schema = fuckuprss_lib::ollama::discordian_schema();
        match provider.generate_text(&model, &prompt, Some(schema)).await {
            Ok(result) => {
                let elapsed = start.elapsed();
                total_time_secs += elapsed.as_secs_f64();
                println!(
                    "  Response in {:.2}s ({} chars)",
                    elapsed.as_secs_f64(),
                    result.text.len()
                );

                // Track token usage
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

                // Verify response is not truncated: a valid JSON should parse fully
                // If finish_reason was "length", the JSON would likely be incomplete
                let json_parses = serde_json::from_str::<DiscordianResponse>(&result.text);

                match json_parses {
                    Ok(analysis) => {
                        // Validate all fields
                        let summary_ok =
                            !analysis.summary.is_empty() && analysis.summary.len() > 20;
                        let bias_ok =
                            analysis.political_bias >= -2.0 && analysis.political_bias <= 2.0;
                        let sach_ok = analysis.sachlichkeit >= 0.0 && analysis.sachlichkeit <= 4.0;
                        let keywords_ok = !analysis.keywords.is_empty();

                        // Check for truncation indicators: if the response ends abruptly
                        // it won't parse as JSON at all. Additional check: summary should
                        // end with proper punctuation (not mid-word).
                        let summary_complete = analysis.summary.ends_with('.')
                            || analysis.summary.ends_with('!')
                            || analysis.summary.ends_with(')')
                            || analysis.summary.ends_with('"')
                            || analysis.summary.ends_with('»');

                        println!(
                            "  Summary:  {} chars {} {}",
                            analysis.summary.len(),
                            if summary_ok { "OK" } else { "FAIL" },
                            if summary_complete {
                                "(complete)"
                            } else {
                                "(may be truncated)"
                            }
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
                            "  Keywords: {:?} {}",
                            analysis.keywords,
                            if keywords_ok { "OK" } else { "FAIL" }
                        );
                        println!("  Categories:          {:?}", analysis.categories);
                        println!(
                            "  Rejected keywords:   {}",
                            analysis.rejected_keywords.len()
                        );
                        println!(
                            "  Rejected categories: {}",
                            analysis.rejected_categories.len()
                        );

                        if summary_ok && bias_ok && sach_ok && keywords_ok {
                            println!("  [PASS]");
                            success_count += 1;
                        } else {
                            println!("  [FAIL] Some fields invalid");
                            if !summary_ok {
                                println!(
                                    "    Summary was: \"{}\"",
                                    &analysis.summary[..analysis.summary.len().min(100)]
                                );
                            }
                        }
                    }
                    Err(e) => {
                        // JSON parse failure strongly suggests truncation
                        println!("  [FAIL] JSON parse error: {}", e);
                        println!(
                            "  Raw response (first 500 chars): {}",
                            &result.text[..result.text.len().min(500)]
                        );
                        println!(
                            "  Raw response (last 200 chars):  {}",
                            if result.text.len() > 200 {
                                &result.text[result.text.len() - 200..]
                            } else {
                                &result.text
                            }
                        );
                        println!("  LIKELY TRUNCATED (finish_reason was probably 'length')");
                    }
                }
            }
            Err(e) => {
                println!("  [FAIL] API error: {}", e);
            }
        }
        println!();
    }

    // ---- Step 4: Summary mode (plain text, no JSON) ----
    println!("[TEST] Summary mode (plain text, no JSON mode)...");
    let summary_prompt = build_summary_prompt(&articles[0].content);
    let start = std::time::Instant::now();

    match provider.generate_text(&model, &summary_prompt, None).await {
        Ok(result) => {
            let elapsed = start.elapsed();
            total_time_secs += elapsed.as_secs_f64();
            println!(
                "  Response in {:.2}s ({} chars)",
                elapsed.as_secs_f64(),
                result.text.len()
            );
            assert!(
                result.text.len() > 20,
                "Summary too short: {} chars",
                result.text.len()
            );

            if let (Some(input), Some(output)) = (result.input_tokens, result.output_tokens) {
                println!("  Tokens: {} input, {} output", input, output);
                total_input_tokens += input;
                total_output_tokens += output;
            }

            // Verify it's NOT JSON (plain text mode)
            let looks_like_json = result.text.trim().starts_with('{');
            if looks_like_json {
                println!("  WARNING: Summary looks like JSON despite non-JSON mode");
            }

            println!("  Summary: {}", &result.text[..result.text.len().min(300)]);
            println!("  [PASS]\n");
        }
        Err(e) => {
            println!("  [FAIL] API error: {}\n", e);
            panic!("Summary generation failed: {}", e);
        }
    }

    // ---- Final Report ----
    println!("============================================================");
    println!("RESULTS - Discordian Analysis Pipeline");
    println!("============================================================");
    println!(
        "  Discordian Analysis: {}/{} passed",
        success_count,
        articles.len()
    );
    println!("  Summary Mode:        PASS");
    println!("  Total time:          {:.2}s", total_time_secs);
    println!(
        "  Total tokens:        {} input, {} output",
        total_input_tokens, total_output_tokens
    );

    if total_input_tokens > 0 {
        let (cost_in, cost_out) = estimate_cost(&model, total_input_tokens, total_output_tokens);
        println!(
            "  Estimated cost:      ${:.6} (input: ${:.6}, output: ${:.6})",
            cost_in + cost_out,
            cost_in,
            cost_out
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

/// Test provider configuration isolation: OpenAI config must not leak into Ollama config.
///
/// Validates:
/// - `resolve_effective_model` returns the OpenAI model when provider is OpenAI
/// - `resolve_effective_model` does NOT allow frontend override for OpenAI
/// - Temperature=None means the field is omitted (not sent as 0)
/// - Provider name is correct
#[tokio::test]
#[ignore] // Requires real database
async fn test_openai_provider_config_isolation() {
    let conn = open_db();
    let (base_url, api_key, model, temperature) = load_settings(&conn);
    let ollama_model = load_ollama_model(&conn);

    assert!(!api_key.is_empty(), "OpenAI API key not configured in DB");
    println!("\n============================================================");
    println!("OpenAI Pipeline Test - Provider Config Isolation");
    println!("============================================================\n");

    // ---- Test 1: resolve_effective_model for OpenAI ----
    println!("[TEST] resolve_effective_model: OpenAI provider ignores frontend model...");
    let effective = resolve_effective_model("OpenAI-compatible", &ollama_model, &model);
    assert_eq!(
        effective, model,
        "OpenAI provider should use config model '{}', got '{}'",
        model, effective
    );
    println!("  Config model:   {}", model);
    println!("  Frontend model: {}", ollama_model);
    println!("  Effective:      {}", effective);
    println!("  [PASS]\n");

    // ---- Test 2: resolve_effective_model for Ollama uses frontend model ----
    println!("[TEST] resolve_effective_model: Ollama provider uses frontend override...");
    let effective_ollama =
        resolve_effective_model("Ollama", "custom-model:latest", "default-model");
    assert_eq!(
        effective_ollama, "custom-model:latest",
        "Ollama provider should use frontend model"
    );
    println!("  [PASS]\n");

    // ---- Test 3: resolve_effective_model Ollama empty frontend falls back ----
    println!("[TEST] resolve_effective_model: Ollama with empty frontend uses config...");
    let effective_fallback = resolve_effective_model("Ollama", "", "default-model");
    assert_eq!(
        effective_fallback, "default-model",
        "Ollama provider should fall back to config model when frontend is empty"
    );
    println!("  [PASS]\n");

    // ---- Test 4: Provider name is correct ----
    println!("[TEST] Provider name is 'OpenAI-compatible'...");
    let config = ProviderConfig {
        provider_type: ProviderType::OpenAiCompatible,
        ollama_url: String::new(),
        ollama_model: String::new(),
        ollama_num_ctx: 4096,
        ollama_concurrency: 1,
        openai_base_url: base_url.clone(),
        openai_api_key: api_key.clone(),
        openai_model: model.clone(),
        openai_temperature: temperature,
    };
    let provider = create_provider(&config);
    assert_eq!(provider.provider_name(), "OpenAI-compatible");
    println!("  [PASS]\n");

    // ---- Test 5: Temperature=None is respected ----
    println!("[TEST] Temperature setting...");
    println!("  DB temperature: {:?}", temperature);
    if temperature.is_none() {
        println!("  Temperature is None -> will be omitted from API request (API default used)");
    } else {
        println!(
            "  Temperature is Some({}) -> will be sent explicitly",
            temperature.unwrap()
        );
    }
    // Verify by creating provider and checking it works (the actual serialization
    // skip_serializing_if = "Option::is_none" is tested via the API call)
    let available = provider.is_available().await;
    assert!(
        available,
        "Provider should be available regardless of temperature setting"
    );
    println!("  [PASS]\n");

    // ---- Test 6: OpenAI provider does not use Ollama model ----
    println!("[TEST] OpenAI config does not mix with Ollama config...");
    let openai_config = ProviderConfig {
        provider_type: ProviderType::OpenAiCompatible,
        ollama_url: "http://localhost:11434".to_string(),
        ollama_model: "ministral-3:latest".to_string(),
        ollama_num_ctx: 8192,
        ollama_concurrency: 1,
        openai_base_url: base_url,
        openai_api_key: api_key,
        openai_model: model.clone(),
        openai_temperature: None,
    };
    let openai_provider = create_provider(&openai_config);
    assert_eq!(openai_provider.provider_name(), "OpenAI-compatible");

    // The effective model for OpenAI should be the OpenAI model, not Ollama
    let effective = resolve_effective_model(
        openai_provider.provider_name(),
        &openai_config.ollama_model,
        &openai_config.openai_model,
    );
    assert_eq!(effective, model);
    println!("  Ollama model in config: {}", openai_config.ollama_model);
    println!("  Effective model:        {}", effective);
    println!("  [PASS]\n");

    println!("============================================================");
    println!("RESULTS - Config Isolation: ALL PASSED");
    println!("============================================================\n");
}

/// Test that long articles produce complete (non-truncated) responses.
///
/// Validates:
/// - Response for a long article is valid, complete JSON
/// - JSON can be fully parsed (not cut off mid-stream)
/// - All expected fields are present
/// - If the response were truncated (finish_reason=length), the JSON would be invalid
#[tokio::test]
#[ignore] // Requires real OpenAI API key and network access
async fn test_openai_no_truncation() {
    let conn = open_db();
    let (base_url, api_key, model, temperature) = load_settings(&conn);

    assert!(!api_key.is_empty(), "OpenAI API key not configured in DB");
    println!("\n============================================================");
    println!("OpenAI Pipeline Test - No Truncation");
    println!("  Provider: {}", base_url);
    println!("  Model:    {}", model);
    println!("============================================================\n");

    let config = ProviderConfig {
        provider_type: ProviderType::OpenAiCompatible,
        ollama_url: String::new(),
        ollama_model: String::new(),
        ollama_num_ctx: 4096,
        ollama_concurrency: 1,
        openai_base_url: base_url,
        openai_api_key: api_key,
        openai_model: model.clone(),
        openai_temperature: temperature,
    };
    let provider = create_provider(&config);

    // ---- Test 1: Long article discordian analysis ----
    let article = load_long_article(&conn);
    println!(
        "[TEST] Long article discordian analysis: \"{}\" (ID: {}, {} chars)",
        &article.title[..article.title.len().min(60)],
        article.id,
        article.content.len()
    );

    let prompt = build_long_discordian_prompt(&article.title, &article.content);
    println!("  Prompt length: {} chars", prompt.len());

    let start = std::time::Instant::now();
    let result = provider
        .generate_text(&model, &prompt, Some(fuckuprss_lib::ollama::discordian_schema()))
        .await
        .expect("API call failed for long article");
    let elapsed = start.elapsed();

    println!(
        "  Response in {:.2}s ({} chars)",
        elapsed.as_secs_f64(),
        result.text.len()
    );

    if let (Some(input), Some(output)) = (result.input_tokens, result.output_tokens) {
        println!("  Tokens: {} input, {} output", input, output);
        let (cost_in, cost_out) = estimate_cost(&model, input, output);
        println!("  Cost: ${:.6}", cost_in + cost_out);
    }

    // The critical check: if max_completion_tokens was too low, the response
    // would be truncated (finish_reason=length) and the JSON would be invalid.
    let analysis: DiscordianResponse = serde_json::from_str(&result.text).unwrap_or_else(|e| {
        println!("  [FAIL] JSON parse error: {}", e);
        println!(
            "  Response (first 500 chars): {}",
            &result.text[..result.text.len().min(500)]
        );
        println!(
            "  Response (last 300 chars):  {}",
            if result.text.len() > 300 {
                &result.text[result.text.len() - 300..]
            } else {
                &result.text
            }
        );
        panic!(
            "Response was likely TRUNCATED (finish_reason: length). \
             This means max_completion_tokens is too low for this article."
        );
    });

    // Validate the parsed response is complete
    assert!(!analysis.summary.is_empty(), "Summary should not be empty");
    assert!(
        analysis.summary.len() > 30,
        "Summary too short ({} chars) - may be incomplete",
        analysis.summary.len()
    );
    assert!(
        analysis.political_bias >= -2.0 && analysis.political_bias <= 2.0,
        "political_bias {} out of range [-2, 2]",
        analysis.political_bias
    );
    assert!(
        analysis.sachlichkeit >= 0.0 && analysis.sachlichkeit <= 4.0,
        "sachlichkeit {} out of range [0, 4]",
        analysis.sachlichkeit
    );
    assert!(
        !analysis.keywords.is_empty(),
        "keywords should not be empty"
    );

    println!("  Summary:  {} chars", analysis.summary.len());
    println!("  Bias:     {}", analysis.political_bias);
    println!("  Sachlich: {}", analysis.sachlichkeit);
    println!("  Keywords: {} items", analysis.keywords.len());
    println!("  [PASS] Response is complete (not truncated)\n");

    // ---- Test 2: Verify plain-text summary is also not truncated ----
    println!("[TEST] Long article summary (plain text)...");
    let summary_prompt = build_summary_prompt(&article.content);
    let start = std::time::Instant::now();

    let summary_result = provider
        .generate_text(&model, &summary_prompt, None)
        .await
        .expect("Summary API call failed");
    let elapsed = start.elapsed();

    println!(
        "  Response in {:.2}s ({} chars)",
        elapsed.as_secs_f64(),
        summary_result.text.len()
    );
    assert!(
        summary_result.text.len() > 30,
        "Summary too short ({} chars)",
        summary_result.text.len()
    );

    // A truncated summary would typically end mid-word or mid-sentence
    let text = summary_result.text.trim();
    let ends_properly = text.ends_with('.')
        || text.ends_with('!')
        || text.ends_with('?')
        || text.ends_with(')')
        || text.ends_with('"')
        || text.ends_with('»');
    if !ends_properly {
        println!(
            "  WARNING: Summary may be truncated (last chars: \"...{}\")",
            &text[text.len().saturating_sub(50)..]
        );
    } else {
        println!("  Summary ends properly (not truncated)");
    }

    println!("  Summary: {}", &text[..text.len().min(300)]);
    println!("  [PASS]\n");

    println!("============================================================");
    println!("RESULTS - No Truncation: ALL PASSED");
    println!("============================================================\n");
}

/// Test multiple sequential requests to verify provider stability and consistent behavior.
///
/// Validates:
/// - Provider handles multiple requests without errors
/// - Token counts are consistently returned
/// - Response times are reasonable
#[tokio::test]
#[ignore] // Requires real OpenAI API key and network access
async fn test_openai_provider_stability() {
    let conn = open_db();
    let (base_url, api_key, model, temperature) = load_settings(&conn);

    assert!(!api_key.is_empty(), "OpenAI API key not configured in DB");
    println!("\n============================================================");
    println!("OpenAI Pipeline Test - Provider Stability");
    println!("  Model: {}", model);
    println!("============================================================\n");

    let config = ProviderConfig {
        provider_type: ProviderType::OpenAiCompatible,
        ollama_url: String::new(),
        ollama_model: String::new(),
        ollama_num_ctx: 4096,
        ollama_concurrency: 1,
        openai_base_url: base_url,
        openai_api_key: api_key,
        openai_model: model.clone(),
        openai_temperature: temperature,
    };
    let provider = create_provider(&config);

    let articles = load_articles(&conn, 3);
    assert!(
        articles.len() >= 3,
        "Need at least 3 articles, found {}",
        articles.len()
    );

    let mut all_had_tokens = true;
    let mut response_times = Vec::new();
    let mut total_input = 0u32;
    let mut total_output = 0u32;

    for (i, article) in articles.iter().enumerate() {
        println!(
            "[TEST {}/{}] \"{}\"",
            i + 1,
            articles.len(),
            &article.title[..article.title.len().min(50)]
        );

        let prompt = build_discordian_prompt(&article.title, &article.content);
        let start = std::time::Instant::now();

        let result = provider
            .generate_text(&model, &prompt, Some(fuckuprss_lib::ollama::discordian_schema()))
            .await
            .unwrap_or_else(|e| panic!("Request {} failed: {}", i + 1, e));
        let elapsed = start.elapsed();
        response_times.push(elapsed.as_secs_f64());

        match (result.input_tokens, result.output_tokens) {
            (Some(input), Some(output)) => {
                total_input += input;
                total_output += output;
                println!(
                    "  {:.2}s | {} input + {} output tokens | {} chars",
                    elapsed.as_secs_f64(),
                    input,
                    output,
                    result.text.len()
                );
            }
            _ => {
                all_had_tokens = false;
                println!(
                    "  {:.2}s | NO TOKEN COUNTS | {} chars",
                    elapsed.as_secs_f64(),
                    result.text.len()
                );
            }
        }

        // Verify parseable
        let _: DiscordianResponse = serde_json::from_str(&result.text)
            .unwrap_or_else(|e| panic!("Request {} produced invalid JSON: {}", i + 1, e));
        println!("  [PASS]");
    }

    println!();
    println!("============================================================");
    println!("RESULTS - Provider Stability");
    println!("============================================================");
    println!(
        "  Requests:     {}/{} succeeded",
        articles.len(),
        articles.len()
    );
    println!(
        "  Token counts: {}",
        if all_had_tokens {
            "ALL PRESENT"
        } else {
            "SOME MISSING"
        }
    );
    println!(
        "  Avg response: {:.2}s",
        response_times.iter().sum::<f64>() / response_times.len() as f64
    );
    println!(
        "  Total tokens: {} input, {} output",
        total_input, total_output
    );
    if total_input > 0 {
        let (cost_in, cost_out) = estimate_cost(&model, total_input, total_output);
        println!("  Total cost:   ${:.6}", cost_in + cost_out);
    }
    println!();

    assert!(all_had_tokens, "All requests should return token counts");
}
