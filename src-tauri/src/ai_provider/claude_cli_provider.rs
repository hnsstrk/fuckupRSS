//! Claude Code CLI-based AI text provider
//!
//! Invokes the `claude` CLI binary for text generation.
//! This provider has NO embedding support — embeddings always use Ollama.
//!
//! Binary search order:
//! 1. "claude" (PATH lookup)
//! 2. /usr/local/bin/claude
//! 3. /opt/homebrew/bin/claude
//! 4. $HOME/.claude/local/claude
//! 5. $HOME/.local/bin/claude
//! 6. $HOME/.npm-global/bin/claude

use async_trait::async_trait;
use log::{debug, info, warn};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use super::{AiProviderError, AiTextProvider, GenerationResult};

/// Default timeout for CLI execution (seconds)
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Cached binary path (resolved once, reused)
static BINARY_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Claude Code CLI text generation provider
pub struct ClaudeCodeCliProvider {
    model: String,
    max_budget_usd: f64,
    timeout_secs: u64,
}

impl ClaudeCodeCliProvider {
    pub fn new(model: &str, max_budget_usd: f64, timeout_secs: u64) -> Self {
        Self {
            model: model.to_string(),
            max_budget_usd,
            timeout_secs: if timeout_secs > 0 {
                timeout_secs
            } else {
                DEFAULT_TIMEOUT_SECS
            },
        }
    }

    /// Find the claude binary by checking known paths.
    ///
    /// Tauri GUI apps on macOS/Linux do NOT inherit the shell's $PATH,
    /// so we check fixed paths first, then fall back to PATH lookup.
    fn find_binary() -> Option<PathBuf> {
        BINARY_PATH
            .get_or_init(|| {
                let home = dirs::home_dir().unwrap_or_default();

                let candidates = [
                    PathBuf::from("/usr/local/bin/claude"),
                    PathBuf::from("/opt/homebrew/bin/claude"),
                    home.join(".claude/local/claude"),
                    home.join(".local/bin/claude"),
                    home.join(".npm-global/bin/claude"),
                ];

                // Check fixed paths first
                for path in &candidates {
                    if path.exists() {
                        info!("[Claude CLI] Found binary at: {}", path.display());
                        return Some(path.clone());
                    }
                }

                // Fall back to PATH lookup via `which`
                match which_binary("claude") {
                    Some(path) => {
                        info!("[Claude CLI] Found binary via PATH: {}", path.display());
                        Some(path)
                    }
                    None => {
                        warn!("[Claude CLI] Binary not found in any known location");
                        None
                    }
                }
            })
            .clone()
    }
}

/// Look up a binary name in PATH using `which` command (synchronous).
fn which_binary(name: &str) -> Option<PathBuf> {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    Some(PathBuf::from(path_str))
                } else {
                    None
                }
            } else {
                None
            }
        })
}

#[async_trait]
impl AiTextProvider for ClaudeCodeCliProvider {
    async fn generate_text(
        &self,
        _model: &str,
        prompt: &str,
        json_schema: Option<serde_json::Value>,
    ) -> Result<GenerationResult, AiProviderError> {
        let binary = Self::find_binary().ok_or_else(|| {
            AiProviderError::NotAvailable(
                "Claude Code CLI binary not found. Install: npm i -g @anthropic-ai/claude-code"
                    .to_string(),
            )
        })?;

        // Build command arguments
        let mut args: Vec<String> = vec![
            "-p".to_string(),
            "--output-format".to_string(),
            "json".to_string(),
            "--max-turns".to_string(),
            "3".to_string(),
        ];

        // Add model if configured
        if !self.model.is_empty() {
            args.push("--model".to_string());
            args.push(self.model.clone());
        }

        // Add budget limit if configured
        if self.max_budget_usd > 0.0 {
            args.push("--max-budget-usd".to_string());
            args.push(format!("{:.2}", self.max_budget_usd));
        }

        // Add JSON schema if provided
        if let Some(ref schema) = json_schema {
            let schema_str = serde_json::to_string(schema).unwrap_or_else(|_| schema.to_string());
            args.push("--json-schema".to_string());
            args.push(schema_str);
        }

        // Add prompt as the last argument (not via stdin)
        args.push(prompt.to_string());

        debug!(
            "[Claude CLI] Executing: {} (args: {}, prompt: {} chars, json_schema: {})",
            binary.display(),
            args.len(),
            prompt.len(),
            json_schema.is_some()
        );

        let start = std::time::Instant::now();

        // Spawn the claude CLI process
        let child = tokio::process::Command::new(&binary)
            .args(&args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                AiProviderError::NotAvailable(format!(
                    "Failed to start Claude CLI at {}: {}",
                    binary.display(),
                    e
                ))
            })?;

        // Wait for completion with timeout
        let timeout = Duration::from_secs(self.timeout_secs);
        let output = tokio::time::timeout(timeout, child.wait_with_output())
            .await
            .map_err(|_| {
                AiProviderError::GenerationFailed(format!(
                    "Claude CLI timed out after {}s",
                    self.timeout_secs
                ))
            })?
            .map_err(|e| {
                AiProviderError::GenerationFailed(format!("Claude CLI process error: {}", e))
            })?;

        let duration = start.elapsed();

        // Log stderr (progress bars, warnings) but don't treat as error
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            debug!("[Claude CLI] stderr: {}", stderr.trim());
        }

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            let stderr_msg = stderr.trim();
            return Err(AiProviderError::GenerationFailed(format!(
                "Claude CLI exited with code {}: {}",
                exit_code,
                if stderr_msg.is_empty() {
                    "no error message"
                } else {
                    stderr_msg
                }
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        if stdout.trim().is_empty() {
            return Err(AiProviderError::GenerationFailed(
                "Claude CLI returned empty output".to_string(),
            ));
        }

        // Parse the JSON output from Claude CLI
        let text = parse_claude_json_output(&stdout)?;

        info!(
            "[Claude CLI] Completed in {:.2}s (response: {} chars)",
            duration.as_secs_f64(),
            text.len()
        );

        Ok(GenerationResult {
            text,
            // CLI doesn't provide token counts
            input_tokens: None,
            output_tokens: None,
        })
    }

    async fn is_available(&self) -> bool {
        Self::find_binary().is_some()
    }

    fn provider_name(&self) -> &str {
        "Claude Code CLI"
    }

    fn suggested_concurrency(&self) -> usize {
        2 // 2-3 parallel claude -p processes are safe with Max Plan
    }
}

/// Strip markdown codeblock wrappers (```json ... ```) from a string.
///
/// Claude CLI with --output-format json wraps the result field in markdown:
/// ```json\n{"summary":"..."}\n```
fn strip_markdown_codeblock(s: &str) -> String {
    let trimmed = s.trim();
    // Check for ```json or ``` prefix
    if let Some(rest) = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
    {
        // Strip the closing ```
        if let Some(content) = rest.strip_suffix("```") {
            return content.trim().to_string();
        }
        return rest.trim().to_string();
    }
    trimmed.to_string()
}

/// Parse the Claude Code CLI JSON output.
///
/// With `--output-format json`, Claude CLI returns a JSON envelope:
///
/// **With `--json-schema`** (structured output):
/// - `structured_output`: JSON object with the schema-conforming result
/// - `result`: empty string `""`
///
/// **Without `--json-schema`** (free-form):
/// - `result`: string (possibly wrapped in markdown code fences)
/// - no `structured_output` field
///
/// **Error cases:**
/// - `is_error: true` — explicit error
/// - `subtype: "error_max_turns"` — ran out of turns before completing
fn parse_claude_json_output(stdout: &str) -> Result<String, AiProviderError> {
    let trimmed = stdout.trim();

    // Try to parse as JSON
    let parsed: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(val) => val,
        Err(_) => {
            // Not valid JSON — use as raw text if it doesn't look like broken JSON
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                warn!(
                    "[Claude CLI] Output looks like JSON but failed to parse ({} chars)",
                    trimmed.len()
                );
                return Err(AiProviderError::GenerationFailed(format!(
                    "Claude CLI returned unparseable JSON output ({} chars)",
                    trimmed.len()
                )));
            }
            warn!(
                "[Claude CLI] Non-JSON output, using raw text ({} chars)",
                trimmed.len()
            );
            return Ok(trimmed.to_string());
        }
    };

    // Collect candidate objects to extract from:
    // - Single object → [obj]
    // - Array → prefer entries with type="result", else all entries
    let candidates: Vec<&serde_json::Value> = if let Some(arr) = parsed.as_array() {
        let results: Vec<&serde_json::Value> = arr
            .iter()
            .filter(|e| e.get("type").and_then(|t| t.as_str()) == Some("result"))
            .collect();
        if results.is_empty() {
            arr.iter().collect()
        } else {
            results
        }
    } else if parsed.is_object() {
        vec![&parsed]
    } else {
        // Parsed as a primitive JSON value (string, number, bool, null)
        return Ok(trimmed.to_string());
    };

    for obj in &candidates {
        // Check for explicit error response
        if obj.get("is_error").and_then(|v| v.as_bool()) == Some(true) {
            let error_msg = obj
                .get("result")
                .and_then(|r| r.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("Unknown error from Claude CLI");
            return Err(AiProviderError::GenerationFailed(format!(
                "Claude CLI error: {}",
                error_msg
            )));
        }

        // Check for error subtypes (e.g. error_max_turns)
        if let Some(subtype) = obj.get("subtype").and_then(|s| s.as_str()) {
            if subtype.starts_with("error_") {
                // Even with is_error=false, error subtypes mean no usable output
                // But check for structured_output first (might exist despite error)
                if let Some(structured) = obj.get("structured_output") {
                    if !structured.is_null() {
                        debug!(
                            "[Claude CLI] Got structured_output despite subtype '{}'",
                            subtype
                        );
                        return Ok(serde_json::to_string(structured)
                            .unwrap_or_else(|_| structured.to_string()));
                    }
                }
                return Err(AiProviderError::GenerationFailed(format!(
                    "Claude CLI failed with subtype '{}' — no result produced",
                    subtype
                )));
            }
        }

        // Prefer structured_output (when --json-schema was used)
        if let Some(structured) = obj.get("structured_output") {
            if !structured.is_null() {
                debug!(
                    "[Claude CLI] Extracted structured_output ({} bytes)",
                    serde_json::to_string(structured)
                        .map(|s| s.len())
                        .unwrap_or(0)
                );
                return Ok(
                    serde_json::to_string(structured).unwrap_or_else(|_| structured.to_string())
                );
            }
        }

        // Extract the result text (strip markdown codeblocks if present)
        if let Some(result) = obj.get("result").and_then(|r| r.as_str()) {
            if !result.is_empty() {
                return Ok(strip_markdown_codeblock(result));
            }
        }

        // Try alternative field names
        if let Some(text) = obj.get("text").and_then(|t| t.as_str()) {
            return Ok(text.to_string());
        }
        if let Some(content) = obj.get("content").and_then(|c| c.as_str()) {
            return Ok(content.to_string());
        }
    }

    // Parsed as JSON but no extractable content from any candidate
    Err(AiProviderError::GenerationFailed(
        "Claude CLI returned JSON envelope but no result or structured_output".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_name() {
        let provider = ClaudeCodeCliProvider::new("", 0.0, 120);
        assert_eq!(provider.provider_name(), "Claude Code CLI");
    }

    #[test]
    fn test_suggested_concurrency() {
        let provider = ClaudeCodeCliProvider::new("", 0.0, 120);
        assert_eq!(provider.suggested_concurrency(), 2);
    }

    #[test]
    fn test_default_timeout() {
        let provider = ClaudeCodeCliProvider::new("", 0.0, 0);
        assert_eq!(provider.timeout_secs, DEFAULT_TIMEOUT_SECS);
    }

    #[test]
    fn test_custom_timeout() {
        let provider = ClaudeCodeCliProvider::new("", 0.0, 300);
        assert_eq!(provider.timeout_secs, 300);
    }

    #[test]
    fn test_model_and_budget() {
        let provider = ClaudeCodeCliProvider::new("claude-sonnet-4-20250514", 5.0, 120);
        assert_eq!(provider.model, "claude-sonnet-4-20250514");
        assert_eq!(provider.max_budget_usd, 5.0);
    }

    #[test]
    fn test_parse_claude_json_output_success() {
        let output = r#"{"type":"result","subtype":"success","cost_usd":0.003,"is_error":false,"duration_ms":1234,"num_turns":1,"result":"Hello world","session_id":"abc123"}"#;
        let result = parse_claude_json_output(output).unwrap();
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_parse_claude_json_output_error() {
        let output = r#"{"type":"result","subtype":"error","is_error":true,"result":"Rate limit exceeded","session_id":"abc123"}"#;
        let result = parse_claude_json_output(output);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Rate limit exceeded"));
    }

    #[test]
    fn test_parse_claude_json_output_json_result() {
        // When the result itself is JSON (structured output via --json-schema)
        let output = r#"{"type":"result","subtype":"success","is_error":false,"result":"{\"summary\": \"Test\", \"category\": 1}","session_id":"abc123"}"#;
        let result = parse_claude_json_output(output).unwrap();
        assert!(result.contains("summary"));
        assert!(result.contains("Test"));
    }

    #[test]
    fn test_parse_claude_json_output_markdown_codeblock() {
        // Claude CLI wraps result in markdown codeblocks
        let output = r#"{"type":"result","subtype":"success","is_error":false,"result":"```json\n{\"summary\": \"Test article\", \"political_bias\": 0}\n```","session_id":"abc123"}"#;
        let result = parse_claude_json_output(output).unwrap();
        assert!(result.contains("summary"));
        assert!(result.contains("Test article"));
        assert!(!result.contains("```"));
    }

    #[test]
    fn test_strip_markdown_codeblock_json() {
        let input = "```json\n{\"key\": \"value\"}\n```";
        assert_eq!(strip_markdown_codeblock(input), "{\"key\": \"value\"}");
    }

    #[test]
    fn test_strip_markdown_codeblock_plain() {
        let input = "```\nplain text\n```";
        assert_eq!(strip_markdown_codeblock(input), "plain text");
    }

    #[test]
    fn test_strip_markdown_codeblock_no_wrapper() {
        let input = "{\"already\": \"json\"}";
        assert_eq!(strip_markdown_codeblock(input), "{\"already\": \"json\"}");
    }

    #[test]
    fn test_parse_claude_json_output_raw_fallback() {
        let output = "Just plain text";
        let result = parse_claude_json_output(output).unwrap();
        assert_eq!(result, "Just plain text");
    }

    #[test]
    fn test_parse_claude_json_output_trimming() {
        let output = "  \n  {\"type\":\"result\",\"is_error\":false,\"result\":\"Hello\"}  \n  ";
        let result = parse_claude_json_output(output).unwrap();
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_parse_claude_json_output_array_format() {
        let output = r#"[{"type":"result","is_error":false,"result":"From array"}]"#;
        let result = parse_claude_json_output(output).unwrap();
        assert_eq!(result, "From array");
    }

    #[test]
    fn test_parse_structured_output_real_format() {
        // Real Claude CLI output with --json-schema: structured_output is a JSON object,
        // result is empty string
        let output = r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":30296,"num_turns":2,"result":"","session_id":"e49fc7b7","structured_output":{"summary":"Test summary","political_bias":0}}"#;
        let result = parse_claude_json_output(output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["summary"], "Test summary");
        assert_eq!(parsed["political_bias"], 0);
    }

    #[test]
    fn test_parse_error_max_turns() {
        // Real Claude CLI output when --max-turns is too low: no result, no structured_output
        let output = r#"{"type":"result","subtype":"error_max_turns","duration_ms":17669,"is_error":false,"num_turns":2,"stop_reason":"tool_use","session_id":"c63dc7d9"}"#;
        let result = parse_claude_json_output(output);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("error_max_turns"));
    }

    #[test]
    fn test_parse_result_with_markdown_freeform() {
        // Real Claude CLI output without --json-schema: result contains markdown
        let output = r#"{"type":"result","subtype":"success","is_error":false,"num_turns":1,"result":"```json\n{\"summary\": \"test\", \"political_bias\": 0}\n```","session_id":"30604e25"}"#;
        let result = parse_claude_json_output(output).unwrap();
        assert!(!result.contains("```"));
        assert!(result.contains("summary"));
    }

    #[test]
    fn test_parse_empty_result_no_structured_output_is_error() {
        // Edge case: success but empty result and no structured_output
        let output = r#"{"type":"result","subtype":"success","is_error":false,"result":"","session_id":"abc"}"#;
        let result = parse_claude_json_output(output);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_structured_output_in_array() {
        // Array format with structured_output
        let output = r#"[{"type":"result","is_error":false,"result":"","structured_output":{"key":"value"}}]"#;
        let result = parse_claude_json_output(output).unwrap();
        assert!(result.contains("key"));
        assert!(result.contains("value"));
    }
}
