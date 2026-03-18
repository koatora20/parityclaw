//! GuavaBridge — guava-anti MCP → OpenCrabs Tool System bridge
//!
//! Spawns the guava-anti binary as a subprocess and proxies tool calls
//! through its CLI interface. Each guava-anti MCP tool maps to an
//! OpenCrabs `Tool` implementation.

use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;

/// Result of a guava-anti CLI invocation
#[derive(Debug, Serialize, Deserialize)]
pub struct GuavaResult {
    pub ok: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// Configuration for the guava-anti bridge
#[derive(Debug, Clone)]
pub struct GuavaBridgeConfig {
    /// Path to the guava-anti binary
    pub binary_path: String,
    /// Workspace root
    pub workspace: String,
    /// Default timeout in seconds
    pub timeout_secs: u64,
}

impl Default for GuavaBridgeConfig {
    fn default() -> Self {
        Self {
            binary_path: "/usr/local/bin/guava-anti".to_string(),
            workspace: dirs::home_dir()
                .map(|h| h.join(".openclaw/workspace").to_string_lossy().to_string())
                .unwrap_or_default(),
            timeout_secs: 30,
        }
    }
}

/// Execute a guava-anti CLI command and return structured output
pub async fn exec_guava(
    config: &GuavaBridgeConfig,
    args: &[&str],
) -> Result<GuavaResult, String> {
    let mut cmd = Command::new(&config.binary_path);
    cmd.args(args)
        .arg("--json")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .env("GUAVA_WORKSPACE", &config.workspace);

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn guava-anti: {}", e))?;

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(config.timeout_secs),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| format!("guava-anti command timed out after {}s", config.timeout_secs))?
    .map_err(|e| format!("guava-anti process error: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        // Try to parse as JSON, fall back to raw output
        match serde_json::from_str::<serde_json::Value>(&stdout) {
            Ok(json) => Ok(GuavaResult {
                ok: true,
                output: Some(json.to_string()),
                error: None,
            }),
            Err(_) => Ok(GuavaResult {
                ok: true,
                output: Some(stdout),
                error: None,
            }),
        }
    } else {
        Ok(GuavaResult {
            ok: false,
            output: if stdout.is_empty() { None } else { Some(stdout) },
            error: Some(if stderr.is_empty() {
                format!("Process exited with code {:?}", output.status.code())
            } else {
                stderr
            }),
        })
    }
}

/// Tool definitions for each guava-anti MCP tool
/// These map directly to OpenCrabs Tool trait implementations.
///
/// # Tool Mapping
///
/// | guava-anti CLI          | OpenCrabs Tool Name | Capability         |
/// |-------------------------|--------------------|--------------------|
/// | `memory store/search`   | `gpi_memory`       | ReadFiles          |
/// | `guard "text"`          | `gpi_guard`        | ReadFiles          |
/// | `search "query"`        | `gpi_search`       | ReadFiles          |
/// | `session load/shutdown` | `gpi_session`      | ReadFiles          |
/// | `metacognition run`     | `gpi_meta`         | ReadFiles          |
/// | `wallet sign/verify`    | `gpi_wallet`       | ReadFiles          |
/// | `x post/timeline`       | `gpi_x`            | NetworkAccess      |
/// | `shell "cmd"`           | `gpi_shell`        | ExecuteShell       |
/// | `governance`            | `gpi_governance`   | ReadFiles          |
/// | `skills`                | `gpi_skills`       | ReadFiles          |

pub mod tools {
    use super::*;

    /// Schema for the gpi_memory tool
    pub fn memory_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["store", "write", "search", "recall"],
                    "description": "Memory action to perform"
                },
                "content": {
                    "type": "string",
                    "description": "Content to store/write (for store/write actions)"
                },
                "query": {
                    "type": "string",
                    "description": "Search query (for search/recall actions)"
                },
                "tags": {
                    "type": "string",
                    "description": "Comma-separated tags"
                },
                "layer": {
                    "type": "string",
                    "enum": ["L1", "L4", "L6"],
                    "description": "Memory layer (for write action)"
                }
            },
            "required": ["action"]
        })
    }

    /// Schema for the gpi_guard tool
    pub fn guard_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text to scan for security threats"
                },
                "soul_lock": {
                    "type": "boolean",
                    "description": "Enable SOUL lock defense patterns"
                }
            },
            "required": ["text"]
        })
    }

    /// Schema for the gpi_search tool
    pub fn search_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query for knowledge base"
                },
                "mode": {
                    "type": "string",
                    "enum": ["search", "timeline", "history"],
                    "description": "Search mode"
                }
            },
            "required": ["query"]
        })
    }

    /// Schema for the gpi_meta tool
    pub fn metacognition_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["run", "growth", "report"],
                    "description": "Metacognition action"
                },
                "project": {
                    "type": "string",
                    "description": "Project name"
                },
                "task": {
                    "type": "string",
                    "description": "Task label for run mode"
                }
            },
            "required": ["action"]
        })
    }

    /// Execute a memory operation via guava-anti CLI
    pub async fn exec_memory(
        config: &GuavaBridgeConfig,
        action: &str,
        content: Option<&str>,
        query: Option<&str>,
        tags: Option<&str>,
        layer: Option<&str>,
    ) -> Result<GuavaResult, String> {
        let mut args: Vec<&str> = vec!["memory", action];

        if let Some(c) = content {
            args.push(c);
        }
        if let Some(q) = query {
            args.push(q);
        }
        if let Some(t) = tags {
            args.push("--tags");
            args.push(t);
        }
        if let Some(l) = layer {
            args.push("--layer");
            args.push(l);
        }

        exec_guava(config, &args).await
    }

    /// Execute a guard scan via guava-anti CLI
    pub async fn exec_guard(
        config: &GuavaBridgeConfig,
        text: &str,
        soul_lock: bool,
    ) -> Result<GuavaResult, String> {
        let mut args = vec!["guard", text];
        if soul_lock {
            args.push("--soul-lock");
        }
        exec_guava(config, &args).await
    }

    /// Execute a search via guava-anti CLI
    pub async fn exec_search(
        config: &GuavaBridgeConfig,
        query: &str,
        mode: Option<&str>,
    ) -> Result<GuavaResult, String> {
        let mut args = vec!["search", query];
        if let Some(m) = mode {
            args.push("--mode");
            args.push(m);
        }
        exec_guava(config, &args).await
    }

    /// Execute metacognition via guava-anti CLI
    pub async fn exec_metacognition(
        config: &GuavaBridgeConfig,
        action: &str,
        project: Option<&str>,
        task: Option<&str>,
    ) -> Result<GuavaResult, String> {
        let mut args = vec!["metacognition", action];
        if let Some(p) = project {
            args.push("--project");
            args.push(p);
        }
        if let Some(t) = task {
            args.push("--task");
            args.push(t);
        }
        exec_guava(config, &args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GuavaBridgeConfig::default();
        assert_eq!(config.binary_path, "/usr/local/bin/guava-anti");
        assert_eq!(config.timeout_secs, 30);
        assert!(config.workspace.contains(".openclaw/workspace"));
    }

    #[test]
    fn test_memory_schema_valid() {
        let schema = tools::memory_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["action"]["enum"]
            .as_array()
            .unwrap()
            .len() == 4);
    }

    #[test]
    fn test_guard_schema_valid() {
        let schema = tools::guard_schema();
        assert_eq!(schema["required"][0], "text");
    }

    #[test]
    fn test_metacognition_schema_valid() {
        let schema = tools::metacognition_schema();
        let actions = schema["properties"]["action"]["enum"]
            .as_array()
            .unwrap();
        assert_eq!(actions.len(), 3);
        assert!(actions.contains(&serde_json::json!("run")));
        assert!(actions.contains(&serde_json::json!("growth")));
        assert!(actions.contains(&serde_json::json!("report")));
    }

    #[tokio::test]
    async fn test_exec_guava_binary_exists() {
        let config = GuavaBridgeConfig::default();
        // Verify the binary exists
        let exists = std::path::Path::new(&config.binary_path).exists();
        assert!(exists, "guava-anti binary must exist at {}", config.binary_path);
    }

    #[tokio::test]
    #[ignore] // Integration test: requires guava-anti not serving MCP
    async fn test_exec_guard_scan() {
        let config = GuavaBridgeConfig::default();
        let result = tools::exec_guard(&config, "hello world", false).await;
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.ok, "Guard scan should succeed on benign text");
    }

    #[tokio::test]
    #[ignore] // Integration test: requires guava-anti not serving MCP
    async fn test_exec_search() {
        let config = GuavaBridgeConfig::default();
        let result = tools::exec_search(&config, "test query", None).await;
        assert!(result.is_ok());
    }
}
