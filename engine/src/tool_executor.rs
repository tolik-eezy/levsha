//! Tool executor — runs system commands via tokio::process::Command.
//!
//! Captures stdout, stderr, and exit code. Configurable timeout per command.
//! Full root access (no sandboxing in MVP).

use regex::Regex;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Result of a tool execution.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

impl ToolResult {
    /// Format the result for sending back to the LLM as tool_result content.
    pub fn format_for_api(&self) -> String {
        if self.timed_out {
            return "Error: Command timed out.".to_string();
        }

        let mut output = String::new();
        if !self.stdout.is_empty() {
            output.push_str(&self.stdout);
        }
        if !self.stderr.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str("[stderr] ");
            output.push_str(&self.stderr);
        }
        if output.is_empty() {
            if self.exit_code == 0 {
                output.push_str("(no output)");
            } else {
                output.push_str(&format!("Command failed with exit code {}", self.exit_code));
            }
        }
        output
    }

    /// Whether the command succeeded (exit code 0, no timeout).
    pub fn success(&self) -> bool {
        !self.timed_out && self.exit_code == 0
    }
}

/// Executes shell commands for tools.
pub struct ToolExecutor {
    pub timeout: Duration,
}

impl ToolExecutor {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Execute a shell command, capturing stdout, stderr, and exit code.
    pub async fn execute(&self, command: &str) -> ToolResult {
        self.execute_with_timeout(command, self.timeout).await
    }

    /// Execute with a specific timeout override.
    pub async fn execute_with_timeout(&self, command: &str, timeout: Duration) -> ToolResult {
        debug!("Executing command: {}", command);

        let child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        let child = match child {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to spawn command: {}", e);
                return ToolResult {
                    stdout: String::new(),
                    stderr: format!("Failed to spawn command: {}", e),
                    exit_code: -1,
                    timed_out: false,
                };
            }
        };

        match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => {
                let exit_code = output.status.code().unwrap_or(-1);
                debug!("Command exited with code {}", exit_code);
                ToolResult {
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    exit_code,
                    timed_out: false,
                }
            }
            Ok(Err(e)) => {
                warn!("Command execution failed: {}", e);
                ToolResult {
                    stdout: String::new(),
                    stderr: format!("Execution failed: {}", e),
                    exit_code: -1,
                    timed_out: false,
                }
            }
            Err(_) => {
                warn!("Command timed out after {:?}", timeout);
                ToolResult {
                    stdout: String::new(),
                    stderr: format!("Command timed out after {} seconds", timeout.as_secs()),
                    exit_code: -1,
                    timed_out: true,
                }
            }
        }
    }

    /// Execute a shell command with streaming stderr output.
    /// Each line of stderr is sent through `line_tx`.
    pub async fn execute_streaming(
        &self,
        command: &str,
        timeout: Duration,
        line_tx: mpsc::Sender<String>,
    ) -> ToolResult {
        debug!("Executing command (streaming): {}", command);

        let mut child = match Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to spawn command: {}", e);
                return ToolResult {
                    stdout: String::new(),
                    stderr: format!("Failed to spawn command: {}", e),
                    exit_code: -1,
                    timed_out: false,
                };
            }
        };

        let stderr = child.stderr.take();

        // Wrap the entire read+wait in a timeout so that a hanging process
        // is correctly detected. Reading stderr to EOF blocks until the child
        // exits, so putting the timeout only around `child.wait()` would be a
        // no-op — the process would have already exited by then.
        let streaming_future = async {
            let mut all_output = String::new();

            if let Some(stderr) = stderr {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    all_output.push_str(&line);
                    all_output.push('\n');
                    line_tx.send(line).await.ok();
                }
            }

            let status = child.wait().await;
            (all_output, status)
        };

        match tokio::time::timeout(timeout, streaming_future).await {
            Ok((all_output, Ok(status))) => {
                let exit_code = status.code().unwrap_or(-1);
                debug!("Streaming command exited with code {}", exit_code);
                ToolResult {
                    stdout: String::new(),
                    stderr: all_output,
                    exit_code,
                    timed_out: false,
                }
            }
            Ok((all_output, Err(e))) => {
                warn!("Streaming command execution failed: {}", e);
                ToolResult {
                    stdout: String::new(),
                    stderr: format!("{}\nExecution failed: {}", all_output, e),
                    exit_code: -1,
                    timed_out: false,
                }
            }
            Err(_) => {
                warn!("Streaming command timed out after {:?}", timeout);
                ToolResult {
                    stdout: String::new(),
                    stderr: format!(
                        "Command timed out after {} seconds",
                        timeout.as_secs()
                    ),
                    exit_code: -1,
                    timed_out: true,
                }
            }
        }
    }
}

/// Render a command template by substituting {{param}} placeholders.
///
/// Supports:
/// - `{{param}}` — direct substitution
/// - `{{param | join(' ')}}` — join array values with a separator
/// - `{{param | quote}}` — shell-quote the value
pub fn render_template(template: &str, params: &serde_json::Value) -> String {
    let re = Regex::new(r"\{\{(\w+)(?:\s*\|\s*(\w+)(?:\(([^)]*)\))?)?\}\}").unwrap();

    re.replace_all(template, |caps: &regex::Captures| {
        let param_name = &caps[1];
        let filter = caps.get(2).map(|m| m.as_str());
        let filter_arg = caps.get(3).map(|m| m.as_str().trim_matches('\'').trim_matches('"'));

        let value = match params.get(param_name) {
            Some(v) => v,
            None => return String::new(),
        };

        match filter {
            Some("join") => {
                let separator = filter_arg.unwrap_or(" ");
                if let Some(arr) = value.as_array() {
                    arr.iter()
                        .map(|v| match v.as_str() {
                            Some(s) => s.to_string(),
                            None => v.to_string(),
                        })
                        .collect::<Vec<_>>()
                        .join(separator)
                } else {
                    value_to_string(value)
                }
            }
            Some("quote") => {
                let s = value_to_string(value);
                shell_quote(&s)
            }
            _ => value_to_string(value),
        }
    })
    .into_owned()
}

/// Convert a JSON value to its string representation for command templates.
fn value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .map(value_to_string)
            .collect::<Vec<_>>()
            .join(" "),
        _ => value.to_string(),
    }
}

/// Simple shell quoting — wraps in single quotes and escapes existing single quotes.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

// Re-export DestructiveGuard from risk_classifier for backwards compatibility.
pub use crate::risk_classifier::DestructiveGuard;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn render_template_simple_substitution() {
        let template = "echo {{message}}";
        let params = json!({"message": "hello world"});
        assert_eq!(render_template(template, &params), "echo hello world");
    }

    #[test]
    fn render_template_multiple_params() {
        let template = "cp {{src}} {{dst}}";
        let params = json!({"src": "/tmp/a", "dst": "/tmp/b"});
        assert_eq!(render_template(template, &params), "cp /tmp/a /tmp/b");
    }

    #[test]
    fn render_template_join_filter() {
        let template = "dnf install {{packages | join(' ')}}";
        let params = json!({"packages": ["vim", "curl", "git"]});
        assert_eq!(
            render_template(template, &params),
            "dnf install vim curl git"
        );
    }

    #[test]
    fn render_template_quote_filter() {
        let template = "echo {{msg | quote()}}";
        let params = json!({"msg": "hello world"});
        assert_eq!(
            render_template(template, &params),
            "echo 'hello world'"
        );
    }

    #[test]
    fn render_template_quote_escapes_single_quotes() {
        let template = "echo {{msg | quote()}}";
        let params = json!({"msg": "it's a test"});
        assert_eq!(
            render_template(template, &params),
            "echo 'it'\\''s a test'"
        );
    }

    #[test]
    fn render_template_missing_param_produces_empty() {
        let template = "echo {{missing}}";
        let params = json!({"other": "value"});
        assert_eq!(render_template(template, &params), "echo ");
    }

    #[test]
    fn render_template_numeric_value() {
        let template = "sleep {{seconds}}";
        let params = json!({"seconds": 5});
        assert_eq!(render_template(template, &params), "sleep 5");
    }

    #[test]
    fn render_template_boolean_value() {
        let template = "echo {{flag}}";
        let params = json!({"flag": true});
        assert_eq!(render_template(template, &params), "echo true");
    }

    #[test]
    fn render_template_array_without_join_uses_space() {
        let template = "echo {{items}}";
        let params = json!({"items": ["a", "b", "c"]});
        assert_eq!(render_template(template, &params), "echo a b c");
    }

    #[test]
    fn render_template_no_placeholders() {
        let template = "ls -la /tmp";
        let params = json!({});
        assert_eq!(render_template(template, &params), "ls -la /tmp");
    }

    #[test]
    fn render_template_quote_filter_without_parens() {
        let template = "tree -L {{depth}} {{path | quote}}";
        let params = json!({"depth": 3, "path": "/home/levsha"});
        assert_eq!(
            render_template(template, &params),
            "tree -L 3 '/home/levsha'"
        );
    }
}
