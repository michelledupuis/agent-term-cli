use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Duration;

#[derive(Debug)]
pub struct McpClient {
    url: String,
    token: String,
    agent: ureq::Agent,
    id: Mutex<u64>,
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    id: Option<serde_json::Value>,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ScreenMetadata {
    pub rows: usize,
    pub cols: usize,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub is_alternate_buffer: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ScreenOutput {
    pub output: String,
    pub mode: String,
    pub metadata: Option<ScreenMetadata>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct StartSessionResult {
    pub session_id: String,
    pub cols: u16,
    pub rows: u16,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub shell: String,
    pub cols: u16,
    pub rows: u16,
    pub idle_seconds: u64,
    pub exited: bool,
    pub exit_code: Option<i32>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ListSessionsResult {
    pub sessions: Vec<SessionInfo>,
    pub count: usize,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct SearchMatch {
    pub row: usize,
    pub col: usize,
    pub text: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub results: Vec<SearchMatch>,
    pub count: usize,
}

impl McpClient {
    pub fn new(url: &str, token: &str) -> Self {
        let agent = ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(30)))
                .build(),
        );
        // Normalize URL: ensure it ends with /mcp so all requests target the
        // correct MCP endpoint.  The health_check derives the /health path
        // from this, so both paths must be consistent.
        let url = {
            let trimmed = url.trim_end_matches('/');
            if trimmed.ends_with("/mcp") {
                trimmed.to_string()
            } else {
                format!("{}/mcp", trimmed)
            }
        };
        Self {
            url,
            token: token.to_string(),
            agent,
            id: Mutex::new(1),
        }
    }

    fn next_id(&self) -> u64 {
        let mut id = self.id.lock().unwrap();
        let val = *id;
        *id += 1;
        val
    }

    fn post(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let id = self.next_id();
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        let response: JsonRpcResponse = self
            .agent
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.token))
            .send_json(&request)?
            .body_mut()
            .read_json()?;

        if let Some(err) = response.error {
            return Err(anyhow!("MCP error {}: {}", err.code, err.message));
        }

        response
            .result
            .ok_or_else(|| anyhow!("MCP response missing 'result'"))
    }

    pub fn health_check(&self) -> bool {
        let base_url = if self.url.ends_with("/mcp") {
            self.url.trim_end_matches("/mcp").to_string()
        } else {
            self.url.trim_end_matches('/').to_string()
        };
        let health_url = format!("{}/health", base_url);
        self.agent
            .get(&health_url)
            .call()
            .map(|r| r.status() == 200)
            .unwrap_or(false)
    }

    pub fn initialize(&self) -> Result<()> {
        self.post(
            "initialize",
            serde_json::json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "agent-term-cli", "version": "0.1.0"}
            }),
        )?;
        Ok(())
    }

    pub fn start_session(
        &self,
        shell: &str,
        cols: u16,
        rows: u16,
    ) -> Result<StartSessionResult> {
        let result = self.post(
            "tools/call",
            serde_json::json!({
                "name": "start_shell_session",
                "arguments": {
                    "shell": shell,
                    "cols": cols,
                    "rows": rows
                }
            }),
        )?;

        let text = result["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid start_session response"))?;
        let parsed: StartSessionResult = serde_json::from_str(text)?;
        Ok(parsed)
    }

    pub fn send_input(&self, session_id: &str, input: &str) -> Result<()> {
        self.post(
            "tools/call",
            serde_json::json!({
                "name": "send_shell_input",
                "arguments": {
                    "sessionId": session_id,
                    "input": input
                }
            }),
        )?;
        Ok(())
    }

    pub fn read_screen(&self, session_id: &str) -> Result<ScreenOutput> {
        let result = self.post(
            "tools/call",
            serde_json::json!({
                "name": "read_shell_output",
                "arguments": {
                    "sessionId": session_id,
                    "mode": "screen",
                    "waitForIdle": 50
                }
            }),
        )?;

        let text = result["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid read_screen response"))?;
        let parsed: ScreenOutput = serde_json::from_str(text)?;
        Ok(parsed)
    }

    #[allow(dead_code)]
    pub fn read_streaming(&self, session_id: &str) -> Result<ScreenOutput> {
        let result = self.post(
            "tools/call",
            serde_json::json!({
                "name": "read_shell_output",
                "arguments": {
                    "sessionId": session_id,
                    "mode": "streaming"
                }
            }),
        )?;

        let text = result["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid read_streaming response"))?;
        let parsed: ScreenOutput = serde_json::from_str(text)?;
        Ok(parsed)
    }

    #[allow(dead_code)]
    pub fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<()> {
        self.post(
            "tools/call",
            serde_json::json!({
                "name": "resize_shell",
                "arguments": {
                    "sessionId": session_id,
                    "cols": cols,
                    "rows": rows
                }
            }),
        )?;
        Ok(())
    }

    pub fn end_session(&self, session_id: &str) -> Result<()> {
        self.post(
            "tools/call",
            serde_json::json!({
                "name": "end_shell_session",
                "arguments": {
                    "sessionId": session_id
                }
            }),
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn list_sessions(&self) -> Result<ListSessionsResult> {
        let result = self.post(
            "tools/call",
            serde_json::json!({
                "name": "list_sessions",
                "arguments": {}
            }),
        )?;

        let text = result["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid list_sessions response"))?;
        let parsed: ListSessionsResult = serde_json::from_str(text)?;
        Ok(parsed)
    }

    #[allow(dead_code)]
    pub fn get_cursor(&self, session_id: &str) -> Result<(usize, usize, String)> {
        let result = self.post(
            "tools/call",
            serde_json::json!({
                "name": "get_screen_cursor",
                "arguments": {
                    "sessionId": session_id
                }
            }),
        )?;

        let text = result["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid get_cursor response"))?;
        let v: serde_json::Value = serde_json::from_str(text)?;
        let x = v["cursor"]["x"].as_u64().unwrap_or(0) as usize;
        let y = v["cursor"]["y"].as_u64().unwrap_or(0) as usize;
        let line = v["currentLine"].as_str().unwrap_or("").to_string();
        Ok((x, y, line))
    }

    #[allow(dead_code)]
    pub fn search_screen(
        &self,
        session_id: &str,
        pattern: &str,
        is_regex: bool,
    ) -> Result<SearchResult> {
        let result = self.post(
            "tools/call",
            serde_json::json!({
                "name": "search_screen",
                "arguments": {
                    "sessionId": session_id,
                    "pattern": pattern,
                    "regex": is_regex
                }
            }),
        )?;

        let text = result["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid search_screen response"))?;
        let parsed: SearchResult = serde_json::from_str(text)?;
        Ok(parsed)
    }
}
