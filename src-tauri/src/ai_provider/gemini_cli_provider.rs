//! Gemini CLI-based AI text provider
//!
//! Invokes the `gemini` CLI binary for text generation.
//! This provider has NO embedding support — embeddings always use Ollama.
//!
//! Binary search order:
//! 1. "gemini" (PATH lookup)
//! 2. /opt/homebrew/bin/gemini
//! 3. /usr/local/bin/gemini
//! 4. $HOME/.local/bin/gemini
//! 5. $HOME/.npm-global/bin/gemini

use async_trait::async_trait;
use log::{debug, info, warn};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

use super::{AiProviderError, AiTextProvider, GenerationResult};

/// Default timeout for CLI execution (seconds)
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Cached binary path (resolved once, reused)
static BINARY_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Gemini CLI text generation provider
pub struct GeminiCliProvider {
    timeout_secs: u64,
}

impl GeminiCliProvider {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            timeout_secs: if timeout_secs > 0 {
                timeout_secs
            } else {
                DEFAULT_TIMEOUT_SECS
            },
        }
    }

    /// Find the gemini binary by checking known paths.
    ///
    /// Tauri GUI apps on macOS/Linux do NOT inherit the shell's $PATH,
    /// so we check fixed paths first, then fall back to PATH lookup.
    fn find_binary() -> Option<PathBuf> {
        BINARY_PATH
            .get_or_init(|| {
                let home = dirs::home_dir().unwrap_or_default();

                let candidates = [
                    PathBuf::from("/opt/homebrew/bin/gemini"),
                    PathBuf::from("/usr/local/bin/gemini"),
                    home.join(".local/bin/gemini"),
                    home.join(".npm-global/bin/gemini"),
                ];

                // Check fixed paths first
                for path in &candidates {
                    if path.exists() {
                        info!("[Gemini CLI] Found binary at: {}", path.display());
                        return Some(path.clone());
                    }
                }

                // Fall back to PATH lookup via `which`
                match which_binary("gemini") {
                    Some(path) => {
                        info!("[Gemini CLI] Found binary via PATH: {}", path.display());
                        Some(path)
                    }
                    None => {
                        warn!("[Gemini CLI] Binary not found in any known location");
                        None
                    }
                }
            })
            .clone()
    }

    /// Build the prompt with optional JSON schema instruction.
    ///
    /// Since the Gemini CLI has no --json-schema flag, we embed
    /// the schema requirement directly into the prompt text.
    fn build_prompt(prompt: &str, json_schema: Option<&serde_json::Value>) -> String {
        match json_schema {
            Some(schema) => {
                let schema_str =
                    serde_json::to_string_pretty(schema).unwrap_or_else(|_| schema.to_string());
                format!(
                    "IMPORTANT: You MUST respond with valid JSON that conforms to this exact schema:\n\
                     ```json\n{}\n```\n\n\
                     Do not include any text outside the JSON object.\n\n{}",
                    schema_str, prompt
                )
            }
            None => prompt.to_string(),
        }
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
impl AiTextProvider for GeminiCliProvider {
    async fn generate_text(
        &self,
        _model: &str,
        prompt: &str,
        json_schema: Option<serde_json::Value>,
    ) -> Result<GenerationResult, AiProviderError> {
        let binary = Self::find_binary().ok_or_else(|| {
            AiProviderError::NotAvailable(
                "Gemini CLI binary not found. Install: npm i -g @google/gemini-cli".to_string(),
            )
        })?;

        let full_prompt = Self::build_prompt(prompt, json_schema.as_ref());

        debug!(
            "[Gemini CLI] Executing: {} -p (prompt: {} chars, json_schema: {})",
            binary.display(),
            full_prompt.len(),
            json_schema.is_some()
        );

        let start = std::time::Instant::now();

        // Spawn the gemini CLI process
        let mut child = tokio::process::Command::new(&binary)
            .args(["-p", "--output-format", "json"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                AiProviderError::NotAvailable(format!(
                    "Failed to start Gemini CLI at {}: {}",
                    binary.display(),
                    e
                ))
            })?;

        // Send prompt via stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(full_prompt.as_bytes()).await.map_err(|e| {
                AiProviderError::GenerationFailed(format!("Failed to write prompt to stdin: {}", e))
            })?;
            // Close stdin to signal EOF
            drop(stdin);
        }

        // Wait for completion with timeout
        let timeout = Duration::from_secs(self.timeout_secs);
        let output = tokio::time::timeout(timeout, child.wait_with_output())
            .await
            .map_err(|_| {
                AiProviderError::GenerationFailed(format!(
                    "Gemini CLI timed out after {}s",
                    self.timeout_secs
                ))
            })?
            .map_err(|e| {
                AiProviderError::GenerationFailed(format!("Gemini CLI process error: {}", e))
            })?;

        let duration = start.elapsed();

        // Log stderr (progress bars, warnings) but don't treat as error
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            debug!("[Gemini CLI] stderr: {}", stderr.trim());
        }

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            let stderr_msg = stderr.trim();
            return Err(AiProviderError::GenerationFailed(format!(
                "Gemini CLI exited with code {}: {}",
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
                "Gemini CLI returned empty output".to_string(),
            ));
        }

        // The --output-format json flag wraps the response in a JSON envelope.
        // Try to extract the actual response text from the envelope.
        let text = parse_gemini_json_output(&stdout)?;

        info!(
            "[Gemini CLI] Completed in {:.2}s (response: {} chars)",
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
        "Gemini CLI"
    }

    fn suggested_concurrency(&self) -> usize {
        1 // CLI is sequential
    }
}

/// Parse the Gemini CLI JSON output envelope.
///
/// The `--output-format json` flag produces output like:
/// ```json
/// [{"type":"response","response":"...actual text..."}]
/// ```
///
/// We extract the response text from the envelope.
/// If parsing fails, fall back to using raw stdout.
fn parse_gemini_json_output(stdout: &str) -> Result<String, AiProviderError> {
    let trimmed = stdout.trim();

    // Try to parse as JSON array (Gemini CLI output format)
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(trimmed) {
        // Look for the response entry
        for entry in &arr {
            if entry.get("type").and_then(|t| t.as_str()) == Some("response") {
                if let Some(response) = entry.get("response").and_then(|r| r.as_str()) {
                    return Ok(response.to_string());
                }
            }
        }

        // If no "response" type found, try to get text from first entry
        if let Some(first) = arr.first() {
            if let Some(text) = first.get("text").and_then(|t| t.as_str()) {
                return Ok(text.to_string());
            }
            if let Some(content) = first.get("content").and_then(|c| c.as_str()) {
                return Ok(content.to_string());
            }
        }
    }

    // Try to parse as single JSON object
    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(response) = obj.get("response").and_then(|r| r.as_str()) {
            return Ok(response.to_string());
        }
        if let Some(text) = obj.get("text").and_then(|t| t.as_str()) {
            return Ok(text.to_string());
        }
        if let Some(content) = obj.get("content").and_then(|c| c.as_str()) {
            return Ok(content.to_string());
        }
    }

    // Fall back to raw stdout (might be plain text or already the desired JSON)
    warn!(
        "[Gemini CLI] Could not parse JSON envelope, using raw output ({} chars)",
        trimmed.len()
    );
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_name() {
        let provider = GeminiCliProvider::new(120);
        assert_eq!(provider.provider_name(), "Gemini CLI");
    }

    #[test]
    fn test_suggested_concurrency() {
        let provider = GeminiCliProvider::new(120);
        assert_eq!(provider.suggested_concurrency(), 1);
    }

    #[test]
    fn test_default_timeout() {
        let provider = GeminiCliProvider::new(0);
        assert_eq!(provider.timeout_secs, DEFAULT_TIMEOUT_SECS);
    }

    #[test]
    fn test_custom_timeout() {
        let provider = GeminiCliProvider::new(300);
        assert_eq!(provider.timeout_secs, 300);
    }

    #[test]
    fn test_build_prompt_without_schema() {
        let prompt = "Analyze this article";
        let result = GeminiCliProvider::build_prompt(prompt, None);
        assert_eq!(result, "Analyze this article");
    }

    #[test]
    fn test_build_prompt_with_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "summary": { "type": "string" }
            }
        });
        let prompt = "Analyze this article";
        let result = GeminiCliProvider::build_prompt(prompt, Some(&schema));
        assert!(result.contains("IMPORTANT: You MUST respond with valid JSON"));
        assert!(result.contains("\"summary\""));
        assert!(result.contains("Analyze this article"));
    }

    #[test]
    fn test_parse_gemini_json_output_array_format() {
        let output = r#"[{"type":"response","response":"Hello world"}]"#;
        let result = parse_gemini_json_output(output).unwrap();
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_parse_gemini_json_output_object_format() {
        let output = r#"{"response":"Hello world"}"#;
        let result = parse_gemini_json_output(output).unwrap();
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_parse_gemini_json_output_text_field() {
        let output = r#"{"text":"Hello world"}"#;
        let result = parse_gemini_json_output(output).unwrap();
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_parse_gemini_json_output_raw_fallback() {
        let output = "Just plain text response";
        let result = parse_gemini_json_output(output).unwrap();
        assert_eq!(result, "Just plain text response");
    }

    #[test]
    fn test_parse_gemini_json_output_trimming() {
        let output = "  \n  Hello world  \n  ";
        let result = parse_gemini_json_output(output).unwrap();
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_parse_gemini_json_output_json_response_in_raw() {
        // When the response itself is JSON (e.g. structured output)
        let output = r#"{"summary": "Test article", "category": 1}"#;
        let result = parse_gemini_json_output(output).unwrap();
        // Falls back to raw since no "response"/"text"/"content" key
        assert!(result.contains("summary"));
        assert!(result.contains("Test article"));
    }
}
