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
            "1".to_string(),
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
        1 // CLI is sequential
    }
}

/// Parse the Claude Code CLI JSON output.
///
/// With `--output-format json`, Claude CLI produces output like:
/// ```json
/// {"type":"result","subtype":"success","cost_usd":0.003,"is_error":false,
///  "duration_ms":1234,"duration_api_ms":1000,"num_turns":1,
///  "result":"...actual text...","session_id":"..."}
/// ```
///
/// We extract the "result" field from the JSON envelope.
fn parse_claude_json_output(stdout: &str) -> Result<String, AiProviderError> {
    let trimmed = stdout.trim();

    // Try to parse as JSON object (expected format)
    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(trimmed) {
        // Check for error response
        if obj.get("is_error").and_then(|v| v.as_bool()) == Some(true) {
            let error_msg = obj
                .get("result")
                .and_then(|r| r.as_str())
                .unwrap_or("Unknown error from Claude CLI");
            return Err(AiProviderError::GenerationFailed(format!(
                "Claude CLI error: {}",
                error_msg
            )));
        }

        // Extract the result text
        if let Some(result) = obj.get("result").and_then(|r| r.as_str()) {
            return Ok(result.to_string());
        }

        // Try alternative field names
        if let Some(text) = obj.get("text").and_then(|t| t.as_str()) {
            return Ok(text.to_string());
        }
        if let Some(content) = obj.get("content").and_then(|c| c.as_str()) {
            return Ok(content.to_string());
        }
    }

    // Try to parse as JSON array (in case of multi-turn output)
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(trimmed) {
        // Look for the result entry
        for entry in &arr {
            if entry.get("type").and_then(|t| t.as_str()) == Some("result") {
                if let Some(result) = entry.get("result").and_then(|r| r.as_str()) {
                    return Ok(result.to_string());
                }
            }
        }

        // Fall back to last entry's text
        if let Some(last) = arr.last() {
            if let Some(result) = last.get("result").and_then(|r| r.as_str()) {
                return Ok(result.to_string());
            }
        }
    }

    // Fall back to raw stdout
    warn!(
        "[Claude CLI] Could not parse JSON envelope, using raw output ({} chars)",
        trimmed.len()
    );
    Ok(trimmed.to_string())
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
        assert_eq!(provider.suggested_concurrency(), 1);
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
        let provider =
            ClaudeCodeCliProvider::new("claude-sonnet-4-20250514", 5.0, 120);
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
    fn test_parse_claude_json_output_raw_fallback() {
        let output = "Just plain text";
        let result = parse_claude_json_output(output).unwrap();
        assert_eq!(result, "Just plain text");
    }

    #[test]
    fn test_parse_claude_json_output_trimming() {
        let output =
            "  \n  {\"type\":\"result\",\"is_error\":false,\"result\":\"Hello\"}  \n  ";
        let result = parse_claude_json_output(output).unwrap();
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_parse_claude_json_output_array_format() {
        let output = r#"[{"type":"result","is_error":false,"result":"From array"}]"#;
        let result = parse_claude_json_output(output).unwrap();
        assert_eq!(result, "From array");
    }
}
