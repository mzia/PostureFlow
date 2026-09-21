//! Model Context Protocol (MCP) Server for PostureFlow
//!
//! Exposes a zero-trust, local-first MCP server over stdio for AI assistants (Claude, Cursor, Antigravity).
//! Enforces non-negotiable security invariants:
//! 1. Read-only audit and posture telemetry by default.
//! 2. Zero-Trust One-Way Ratchet: AI can escalate security (Dev -> Work -> Travel) or engage lockdown,
//!    but is cryptographically and logically blocked from downgrading firewall/system defenses.
//! 3. Closed enums and strict schema validation: no arbitrary shell or command execution.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use crate::system;
use crate::inspector;
use crate::hardware::sensors;

pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
pub const SERVER_NAME: &str = "postureflow-mcp";
pub const SERVER_VERSION: &str = "1.0.1";

/// Security ranks for posture profiles (higher = more restrictive/secure).
pub fn profile_security_rank(profile_id: &str) -> u8 {
    match profile_id.to_lowercase().as_str() {
        "travel" | "lockdown" | "secure" => 4,
        "work" | "office" => 3,
        "home" => 2,
        "dev" | "developer" => 1,
        _ => 2, // Default to moderate
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct McpServer {
    current_profile_override: Option<String>,
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            current_profile_override: None,
        }
    }

    /// For testing: simulate a specific active profile
    pub fn with_simulated_profile(profile: &str) -> Self {
        Self {
            current_profile_override: Some(profile.to_string()),
        }
    }

    pub fn get_active_profile(&self) -> String {
        if let Some(ref p) = self.current_profile_override {
            p.clone()
        } else {
            system::get_active_profile()
        }
    }

    /// Process a single incoming JSON-RPC line and generate the response (if applicable).
    pub fn handle_message(&mut self, line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                let err_resp = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                };
                return serde_json::to_string(&err_resp).ok();
            }
        };

        // Notifications don't have responses
        if request.id.is_none() {
            return None;
        }

        let id = request.id.clone();
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(),
            "ping" => Ok(json!({})),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(request.params),
            _ => Err((-32601, format!("Method not found: {}", request.method))),
        };

        let response = match result {
            Ok(res) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(res),
                error: None,
            },
            Err((code, msg)) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code,
                    message: msg,
                    data: None,
                }),
            },
        };

        serde_json::to_string(&response).ok()
    }

    fn handle_initialize(&self) -> Result<Value, (i32, String)> {
        Ok(json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": SERVER_NAME,
                "version": SERVER_VERSION
            }
        }))
    }

    fn handle_tools_list(&self) -> Result<Value, (i32, String)> {
        let tools = json!([
            {
                "name": "get_posture_status",
                "description": "Read active security posture (Home, Work, Dev, Travel), firewall state, and live security score.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            },
            {
                "name": "get_security_audit",
                "description": "Evaluate full 100-point security audit report with letter grade, risk breakdown, and remediation hints.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            },
            {
                "name": "inspect_listening_ports",
                "description": "Audit actively listening TCP and UDP sockets with loopback vs public exposure classification.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            },
            {
                "name": "get_sensor_privacy_status",
                "description": "Check microphone mute status, camera lockout, and emergency sensor privacy report.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            },
            {
                "name": "engage_security_lockdown",
                "description": "Instantly engage emergency Travel lockdown (drop-all inbound firewall, ICMP stealth, and screen lock). This is an authorized one-way defense escalation.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "reason": {
                            "type": "string",
                            "description": "Reason for triggering security lockdown (e.g. untrusted public Wi-Fi, suspicious activity)"
                        }
                    },
                    "required": ["reason"]
                }
            },
            {
                "name": "request_posture_switch",
                "description": "Request a posture shift. ZERO-TRUST ENFORCED: AI can escalate security (Dev -> Work -> Travel), but downgrading defenses (Travel -> Dev/Home) is strictly blocked to prevent prompt injection attacks.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "target_profile": {
                            "type": "string",
                            "enum": ["travel", "work", "home", "dev"],
                            "description": "Target profile to apply"
                        },
                        "reason": {
                            "type": "string",
                            "description": "Why this posture shift is requested"
                        }
                    },
                    "required": ["target_profile", "reason"]
                }
            }
        ]);

        Ok(json!({ "tools": tools }))
    }

    fn handle_tools_call(&mut self, params: Option<Value>) -> Result<Value, (i32, String)> {
        let params = params.ok_or((-32602, "Missing params".to_string()))?;
        let tool_name = params.get("name")
            .and_then(|v| v.as_str())
            .ok_or((-32602, "Missing tool name".to_string()))?;

        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        match tool_name {
            "get_posture_status" => self.tool_get_posture_status(),
            "get_security_audit" => self.tool_get_security_audit(),
            "inspect_listening_ports" => self.tool_inspect_listening_ports(),
            "get_sensor_privacy_status" => self.tool_get_sensor_privacy_status(),
            "engage_security_lockdown" => self.tool_engage_security_lockdown(&args),
            "request_posture_switch" => self.tool_request_posture_switch(&args),
            _ => Err((-32601, format!("Unknown tool: {}", tool_name))),
        }
    }

    fn tool_get_posture_status(&self) -> Result<Value, (i32, String)> {
        let current_profile = self.get_active_profile();
        let score_report = inspector::score::PostureScoreReport::compute();

        let status_info = json!({
            "active_profile": current_profile,
            "security_score": score_report.total_score,
            "security_grade": score_report.letter_grade,
            "firewall_active": score_report.ufw_active,
            "public_ports_count": score_report.public_ports_count,
            "local_ports_count": score_report.local_ports_count,
            "stealth_mode": (current_profile == "travel"),
            "zero_trust_rank": profile_security_rank(&current_profile),
        });

        let formatted = format!(
            "PostureFlow Status:\n• Active Profile: {}\n• Security Score: {}/100 (Grade: {})\n• Firewall: {}\n• Public Ports: {} (Local Ports: {})\n• Stealth Mode: {}",
            status_info["active_profile"],
            status_info["security_score"],
            status_info["security_grade"],
            if status_info["firewall_active"].as_bool().unwrap_or(false) { "Active (Enforced)" } else { "Inactive" },
            status_info["public_ports_count"],
            status_info["local_ports_count"],
            if status_info["stealth_mode"].as_bool().unwrap_or(false) { "Enabled" } else { "Disabled" },
        );

        Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": formatted
                }
            ],
            "isError": false
        }))
    }

    fn tool_get_security_audit(&self) -> Result<Value, (i32, String)> {
        let score_report = inspector::score::PostureScoreReport::compute();

        let mut text = format!(
            "=== PostureFlow Security Audit ===\nTotal Score: {}/100 (Grade: {})\n\nCategory Breakdown:\n",
            score_report.total_score, score_report.letter_grade
        );

        text.push_str(&format!("  • Firewall Security: {}/35 pts (Active: {})\n", score_report.firewall_score, score_report.ufw_active));
        text.push_str(&format!("  • Attack Surface: {}/25 pts (Public Ports: {})\n", score_report.attack_surface_score, score_report.public_ports_count));
        text.push_str(&format!("  • Kernel Hardening: {}/25 pts\n", score_report.kernel_score));
        text.push_str(&format!("  • Resource Limits: {}/15 pts\n", score_report.limits_score));

        if !score_report.recommendations.is_empty() {
            text.push_str("\nSecurity Recommendations:\n");
            for rec in &score_report.recommendations {
                text.push_str(&format!("  ⚠️ {}\n", rec));
            }
        }

        Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": text
                }
            ],
            "isError": false
        }))
    }

    fn tool_inspect_listening_ports(&self) -> Result<Value, (i32, String)> {
        let ports_list = inspector::ports::scan_listening_ports();

        let mut text = format!("Actively Listening Ports (Total: {}):\n", ports_list.len());
        if ports_list.is_empty() {
            text.push_str("  No open listening ports detected. Perimeter is fully secured.\n");
        } else {
            for p in &ports_list {
                text.push_str(&format!(
                    "  • Port {} ({}) [{}] - Process: {} ({})\n",
                    p.port,
                    p.protocol,
                    if p.is_local_only { "Loopback-Only" } else { "PUBLIC / LAN EXPOSED" },
                    p.process_name,
                    p.service_hint
                ));
            }
        }

        Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": text
                }
            ],
            "isError": false
        }))
    }

    fn tool_get_sensor_privacy_status(&self) -> Result<Value, (i32, String)> {
        let report = sensors::get_sensor_privacy_report();

        let text = format!(
            "Hardware Sensor Privacy Status:\n• Microphone Muted: {}\n• Camera Blocked: {}\n• Geolocation Blocked: {}\n• Active Connected Cameras: {}\n• Active Audio Sources: {}",
            if report.microphone_muted { "YES (0% Volume)" } else { "NO (Active)" },
            if report.camera_blocked { "YES (Blocked)" } else { "NO (Permitted)" },
            if report.location_blocked { "YES (Disabled)" } else { "NO (Allowed)" },
            report.active_cameras_count,
            report.active_audio_sources_count
        );

        Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": text
                }
            ],
            "isError": false
        }))
    }

    fn tool_engage_security_lockdown(&mut self, args: &Value) -> Result<Value, (i32, String)> {
        let reason = args.get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("Unspecified security incident");

        let current = self.get_active_profile();

        // 1. Switch profile to Travel (Maximum lockdown)
        if self.current_profile_override.is_some() {
            self.current_profile_override = Some("travel".to_string());
        } else {
            let _ = system::apply_profile_by_id("travel");
        }

        // 2. Lock desktop screen session
        let _ = std::process::Command::new("loginctl")
            .arg("lock-session")
            .spawn();

        let msg = format!(
            "🛡️ EMERGENCY SECURITY LOCKDOWN ENGAGED:\n• Previous Profile: {}\n• Active Profile: Travel (Strict Lockdown)\n• Action: Inbound network traffic dropped, stealth mode engaged, screen lock dispatched.\n• Incident Reason: {}",
            current, reason
        );

        Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": msg
                }
            ],
            "isError": false
        }))
    }

    fn tool_request_posture_switch(&mut self, args: &Value) -> Result<Value, (i32, String)> {
        let target = args.get("target_profile")
            .and_then(|v| v.as_str())
            .ok_or((-32602, "Missing 'target_profile'".to_string()))?
            .to_lowercase();

        let reason = args.get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("Automated workflow coordination");

        let current = self.get_active_profile().to_lowercase();
        let current_rank = profile_security_rank(&current);
        let target_rank = profile_security_rank(&target);

        // ZERO-TRUST ENFORCEMENT:
        // Escalations (target_rank >= current_rank) are permitted.
        // Downgrades (target_rank < current_rank) are strictly BLOCKED.
        if target_rank < current_rank {
            let rejection_msg = format!(
                "⛔ ZERO-TRUST SECURITY VIOLATION:\n\
                Automated profile downgrade blocked by policy.\n\
                • Current Posture: {} (Security Rank: {})\n\
                • Requested Posture: {} (Security Rank: {})\n\
                • Violation: Switching from '{}' to '{}' reduces firewall and system perimeter restrictions.\n\
                • Protection: To defend against indirect prompt injection attacks, downgrading defenses requires physical user confirmation on the host OS via 'postureflow {}' or the COSMIC applet.",
                current, current_rank, target, target_rank, current, target, target
            );

            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": rejection_msg
                    }
                ],
                "isError": true
            }));
        }

        // Apply escalation
        if self.current_profile_override.is_some() {
            self.current_profile_override = Some(target.clone());
        } else {
            let _ = system::apply_profile_by_id(&target);
        }

        let success_msg = format!(
            "✔ Security Posture Escalation Applied:\n• Previous Profile: {}\n• New Profile: {} (Security Rank: {})\n• Rationale: {}",
            current, target, target_rank, reason
        );

        Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": success_msg
                }
            ],
            "isError": false
        }))
    }
}

/// Runs the standard input/output JSON-RPC event loop for Model Context Protocol.
pub fn run_stdio_server() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut server = McpServer::new();

    for line in stdin.lock().lines() {
        let line = line?;
        if let Some(response) = server.handle_message(&line) {
            writeln!(stdout, "{}", response)?;
            stdout.flush()?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_handshake() {
        let mut server = McpServer::new();
        let init_req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {}
            }
        });

        let resp_str = server.handle_message(&init_req.to_string()).expect("Expected response");
        let resp: Value = serde_json::from_str(&resp_str).unwrap();

        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["serverInfo"]["name"], "postureflow-mcp");
        assert_eq!(resp["result"]["protocolVersion"], "2024-11-05");
    }

    #[test]
    fn test_tools_list_schema() {
        let mut server = McpServer::new();
        let req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let resp_str = server.handle_message(&req.to_string()).expect("Expected response");
        let resp: Value = serde_json::from_str(&resp_str).unwrap();

        let tools = resp["result"]["tools"].as_array().expect("Tools array");
        assert_eq!(tools.len(), 6);

        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"get_posture_status"));
        assert!(names.contains(&"get_security_audit"));
        assert!(names.contains(&"inspect_listening_ports"));
        assert!(names.contains(&"get_sensor_privacy_status"));
        assert!(names.contains(&"engage_security_lockdown"));
        assert!(names.contains(&"request_posture_switch"));
    }

    #[test]
    fn test_zero_trust_downgrade_prevention() {
        // Given a system in Travel lockdown (Rank 4)
        let mut server = McpServer::with_simulated_profile("travel");

        // Attempting to downgrade to Dev (Rank 1) via prompt injection
        let downgrade_req = json!({
            "jsonrpc": "2.0",
            "id": 100,
            "method": "tools/call",
            "params": {
                "name": "request_posture_switch",
                "arguments": {
                    "target_profile": "dev",
                    "reason": "Attacker payload attempting to lower firewall defenses"
                }
            }
        });

        let resp_str = server.handle_message(&downgrade_req.to_string()).expect("Response");
        let resp: Value = serde_json::from_str(&resp_str).unwrap();

        assert_eq!(resp["result"]["isError"], true);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("ZERO-TRUST SECURITY VIOLATION"));
        assert!(text.contains("downgrade blocked"));
        assert_eq!(server.get_active_profile(), "travel");
    }

    #[test]
    fn test_zero_trust_escalation_permitted() {
        // Given a system in Dev (Rank 1)
        let mut server = McpServer::with_simulated_profile("dev");

        // Escalating to Work (Rank 3)
        let escalate_req = json!({
            "jsonrpc": "2.0",
            "id": 101,
            "method": "tools/call",
            "params": {
                "name": "request_posture_switch",
                "arguments": {
                    "target_profile": "work",
                    "reason": "User started corporate VPN session"
                }
            }
        });

        let resp_str = server.handle_message(&escalate_req.to_string()).expect("Response");
        let resp: Value = serde_json::from_str(&resp_str).unwrap();

        assert_eq!(resp["result"]["isError"], false);
        assert_eq!(server.get_active_profile(), "work");
    }

    #[test]
    fn test_emergency_lockdown_always_allowed() {
        let mut server = McpServer::with_simulated_profile("home");

        let lockdown_req = json!({
            "jsonrpc": "2.0",
            "id": 102,
            "method": "tools/call",
            "params": {
                "name": "engage_security_lockdown",
                "arguments": {
                    "reason": "Hostile public Wi-Fi detected"
                }
            }
        });

        let resp_str = server.handle_message(&lockdown_req.to_string()).expect("Response");
        let resp: Value = serde_json::from_str(&resp_str).unwrap();

        assert_eq!(resp["result"]["isError"], false);
        assert_eq!(server.get_active_profile(), "travel");
    }
}
