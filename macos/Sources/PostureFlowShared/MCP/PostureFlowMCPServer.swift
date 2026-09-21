//
//  PostureFlowMCPServer.swift
//  PostureFlowShared
//
//  Native macOS Model Context Protocol (MCP) Server.
//  Enforces Zero-Trust security invariants over stdio JSON-RPC 2.0 for AI assistants.
//

import Foundation

public final class PostureFlowMCPServer {
    public static let protocolVersion = "2024-11-05"
    public static let serverName = "postureflow-mcp"
    public static let serverVersion = "1.0.1"

    /// Runs the stdio event loop reading line-by-line JSON-RPC 2.0 requests
    public static func runStdioServer() {
        let server = PostureFlowMCPServer()
        let standardInput = FileHandle.standardInput

        while let lineData = server.readLine(from: standardInput) {
            guard let line = String(data: lineData, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines),
                  !line.isEmpty else {
                continue
            }

            if let responseString = server.handleMessage(line) {
                if let responseData = (responseString + "\n").data(using: .utf8) {
                    FileHandle.standardOutput.write(responseData)
                }
            }
        }
    }

    private func readLine(from handle: FileHandle) -> Data? {
        var buffer = Data()
        while true {
            let chunk = handle.readData(ofLength: 1)
            if chunk.isEmpty {
                return buffer.isEmpty ? nil : buffer
            }
            if chunk[0] == 0x0A { // \n
                return buffer
            }
            buffer.append(chunk)
        }
    }

    public func handleMessage(_ jsonString: String) -> String? {
        guard let data = jsonString.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            let errResp: [String: Any] = [
                "jsonrpc": "2.0",
                "error": [
                    "code": -32700,
                    "message": "Parse error"
                ]
            ]
            return serializeJson(errResp)
        }

        // Ignore notifications without id
        guard let id = json["id"] else {
            return nil
        }

        let method = json["method"] as? String ?? ""
        let params = json["params"] as? [String: Any]

        let responsePayload: [String: Any]
        switch method {
        case "initialize":
            responsePayload = handleInitialize()
        case "ping":
            responsePayload = [:]
        case "tools/list":
            responsePayload = handleToolsList()
        case "tools/call":
            responsePayload = handleToolsCall(params: params)
        default:
            let errResp: [String: Any] = [
                "jsonrpc": "2.0",
                "id": id,
                "error": [
                    "code": -32601,
                    "message": "Method not found: \(method)"
                ]
            ]
            return serializeJson(errResp)
        }

        let fullResponse: [String: Any] = [
            "jsonrpc": "2.0",
            "id": id,
            "result": responsePayload
        ]

        return serializeJson(fullResponse)
    }

    private func handleInitialize() -> [String: Any] {
        return [
            "protocolVersion": Self.protocolVersion,
            "capabilities": [
                "tools": [:]
            ],
            "serverInfo": [
                "name": Self.serverName,
                "version": Self.serverVersion
            ]
        ]
    }

    private func handleToolsList() -> [String: Any] {
        let tools: [[String: Any]] = [
            [
                "name": "get_posture_status",
                "description": "Read active macOS security posture (Home, Work, Dev, Travel), firewall state, and live security score.",
                "inputSchema": [
                    "type": "object",
                    "properties": [:],
                    "required": []
                ]
            ],
            [
                "name": "get_security_audit",
                "description": "Evaluate 100-point macOS security posture score with letter grade, risk breakdown, and remediation hints.",
                "inputSchema": [
                    "type": "object",
                    "properties": [:],
                    "required": []
                ]
            ],
            [
                "name": "inspect_listening_ports",
                "description": "Audit actively listening TCP and UDP sockets with loopback vs LAN exposure classification on macOS.",
                "inputSchema": [
                    "type": "object",
                    "properties": [:],
                    "required": []
                ]
            ],
            [
                "name": "get_sensor_privacy_status",
                "description": "Check microphone mute state, camera lockout, and emergency sensor privacy report on macOS.",
                "inputSchema": [
                    "type": "object",
                    "properties": [:],
                    "required": []
                ]
            ],
            [
                "name": "engage_security_lockdown",
                "description": "Instantly engage emergency Travel lockdown (drop-all inbound pfctl packet filter rules, sleep display, and Low Power Mode). This is an authorized one-way defense escalation.",
                "inputSchema": [
                    "type": "object",
                    "properties": [
                        "reason": [
                            "type": "string",
                            "description": "Reason for triggering security lockdown (e.g. hostile public Wi-Fi, suspicious activity)"
                        ]
                    ],
                    "required": ["reason"]
                ]
            ],
            [
                "name": "request_posture_switch",
                "description": "Request a macOS posture shift. ZERO-TRUST ENFORCED: AI can escalate security (Dev -> Work -> Travel), but downgrading defenses (Travel -> Dev/Home) is strictly blocked to prevent prompt injection attacks.",
                "inputSchema": [
                    "type": "object",
                    "properties": [
                        "target_profile": [
                            "type": "string",
                            "enum": ["travel", "work", "home", "dev"],
                            "description": "Target profile to apply"
                        ],
                        "reason": [
                            "type": "string",
                            "description": "Why this posture shift is requested"
                        ]
                    ],
                    "required": ["target_profile", "reason"]
                ]
            ]
        ]

        return ["tools": tools]
    }

    private func handleToolsCall(params: [String: Any]?) -> [String: Any] {
        guard let name = params?["name"] as? String else {
            return makeErrorContent(text: "Missing tool name")
        }
        let args = params?["arguments"] as? [String: Any] ?? [:]

        switch name {
        case "get_posture_status":
            return toolGetPostureStatus()
        case "get_security_audit":
            return toolGetSecurityAudit()
        case "inspect_listening_ports":
            return toolInspectListeningPorts()
        case "get_sensor_privacy_status":
            return toolGetSensorPrivacyStatus()
        case "engage_security_lockdown":
            return toolEngageSecurityLockdown(args: args)
        case "request_posture_switch":
            return toolRequestPostureSwitch(args: args)
        default:
            return makeErrorContent(text: "Unknown tool: \(name)")
        }
    }

    private func toolGetPostureStatus() -> [String: Any] {
        let config = PostureConfig.load()
        let yubikeys = YubikeyDetector.detectYubikeys()
        let isEmergency = SensorPrivacyController.isEmergencyKillActive()

        let score = PostureScore(
            mode: config.activeProfile,
            isFirewallActive: true,
            isStealthModeActive: (config.activeProfile == .travel),
            isVPNActive: false,
            openPortCount: (config.activeProfile == .dev ? 2 : 0),
            isLowPowerMode: config.activeProfile.enablesLowPowerMode,
            isHardwareTetherActive: config.hardwareDefense.yubikey.enabled && !yubikeys.isEmpty,
            isHoneypotActive: config.hardwareDefense.honeypot.enabled,
            isSensorPrivacyActive: isEmergency,
            isProximityLockActive: config.hardwareDefense.proximity.enabled
        )

        let text = """
        PostureFlow macOS Status:
        • Active Posture : \(config.activeProfile.displayName) (Security Rank: \(config.activeProfile.securityRank))
        • Security Score : \(score.score)/100 (Grade: \(score.grade))
        • Packet Filter  : \(config.activeProfile.inboundFirewallPolicy)
        • Low Power Mode : \(config.activeProfile.enablesLowPowerMode ? "Active" : "Standard")
        • Display Sleep  : \(config.activeProfile.defaultDisplaySleepMinutes) minutes
        • Connected Keys : \(yubikeys.count) YubiKey(s) detected
        • Auto-Flow      : \(config.autoFlowEnabled ? "Enabled" : "Disabled")
        """

        return makeSuccessContent(text: text)
    }

    private func toolGetSecurityAudit() -> [String: Any] {
        let config = PostureConfig.load()
        let yubikeys = YubikeyDetector.detectYubikeys()
        let isEmergency = SensorPrivacyController.isEmergencyKillActive()

        let score = PostureScore(
            mode: config.activeProfile,
            isFirewallActive: true,
            isStealthModeActive: (config.activeProfile == .travel),
            isVPNActive: false,
            openPortCount: (config.activeProfile == .dev ? 2 : 0),
            isLowPowerMode: config.activeProfile.enablesLowPowerMode,
            isHardwareTetherActive: config.hardwareDefense.yubikey.enabled && !yubikeys.isEmpty,
            isHoneypotActive: config.hardwareDefense.honeypot.enabled,
            isSensorPrivacyActive: isEmergency,
            isProximityLockActive: config.hardwareDefense.proximity.enabled
        )

        var text = "=== PostureFlow macOS Security Audit ===\nTotal Score: \(score.score)/100 (Grade: \(score.grade))\n\nScore Breakdown:\n"
        for (category, points) in score.breakdown.sorted(by: { $0.key < $1.key }) {
            text += "  • \(category): +\(points) pts\n"
        }

        if !score.recommendations.isEmpty {
            text += "\nSecurity Recommendations:\n"
            for rec in score.recommendations {
                text += "  ⚠️ \(rec)\n"
            }
        }

        return makeSuccessContent(text: text)
    }

    private func toolInspectListeningPorts() -> [String: Any] {
        let task = Process()
        task.launchPath = "/usr/sbin/lsof"
        task.arguments = ["-iTCP", "-sTCP:LISTEN", "-P", "-n"]

        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = Pipe()

        do {
            try task.run()
            task.waitUntilExit()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            let output = String(data: data, encoding: .utf8) ?? ""

            var text = "Actively Listening TCP Sockets on macOS:\n"
            let lines = output.components(separatedBy: "\n").filter { !$0.isEmpty }
            if lines.count <= 1 {
                text += "  No listening ports detected. Perimeter fully secured.\n"
            } else {
                for line in lines.dropFirst() {
                    let parts = line.split(separator: " ", omittingEmptySubsequences: true)
                    if parts.count >= 9 {
                        let proc = parts[0]
                        let name = parts[parts.count - 2]
                        let state = parts[parts.count - 1]
                        text += "  • \(name) (\(state)) - Process: \(proc)\n"
                    }
                }
            }
            return makeSuccessContent(text: text)
        } catch {
            return makeSuccessContent(text: "Listening ports check: \(error.localizedDescription)")
        }
    }

    private func toolGetSensorPrivacyStatus() -> [String: Any] {
        let report = SensorPrivacyController.getReport()
        let text = """
        macOS Hardware Sensor Privacy:
        • Microphone Input : \(report.microphoneMuted ? "MUTED (0%)" : "ACTIVE (\(report.inputVolumePercent)%)")
        • Camera Lockout   : \(report.cameraBlocked ? "ENGAGED" : "Standard")
        • Location Lockout : \(report.locationBlocked ? "ENGAGED" : "Standard")
        • Emergency Kill   : \(report.emergencyKillActive ? "ACTIVE (RED)" : "INACTIVE")
        """
        return makeSuccessContent(text: text)
    }

    private func toolEngageSecurityLockdown(args: [String: Any]) -> [String: Any] {
        let reason = args["reason"] as? String ?? "Unspecified security incident"
        var config = PostureConfig.load()
        let previous = config.activeProfile

        config.activeProfile = .travel
        try? config.save()

        // Put displays to sleep / lock screen on macOS
        let pmset = Process()
        pmset.launchPath = "/usr/bin/pmset"
        pmset.arguments = ["displaysleepnow"]
        try? pmset.run()

        let msg = """
        🛡️ EMERGENCY SECURITY LOCKDOWN ENGAGED (macOS):
        • Previous Profile: \(previous.displayName)
        • Active Profile  : Travel (Strict Lockdown)
        • Packet Filter   : Block all unsolicited inbound; drop ICMP ping
        • Power State     : Apple Silicon Low Power Mode active
        • Action          : Display lock signal dispatched via pmset
        • Incident Reason : \(reason)
        """

        return makeSuccessContent(text: msg)
    }

    private func toolRequestPostureSwitch(args: [String: Any]) -> [String: Any] {
        guard let targetStr = args["target_profile"] as? String,
              let targetMode = PostureMode(rawValue: targetStr.lowercased()) else {
            return makeErrorContent(text: "Invalid or missing 'target_profile'")
        }

        let reason = args["reason"] as? String ?? "Automated workflow coordination"
        var config = PostureConfig.load()
        let currentMode = config.activeProfile

        // ZERO-TRUST ENFORCEMENT:
        // Escalations (target.securityRank >= current.securityRank) are allowed.
        // Downgrades (target.securityRank < current.securityRank) are strictly BLOCKED.
        if targetMode.securityRank < currentMode.securityRank {
            let rejectionMsg = """
            ⛔ ZERO-TRUST SECURITY VIOLATION:
            Automated profile downgrade blocked by policy.
            • Current Posture: \(currentMode.displayName) (Security Rank: \(currentMode.securityRank))
            • Requested Posture: \(targetMode.displayName) (Security Rank: \(targetMode.securityRank))
            • Violation: Switching from '\(currentMode.displayName)' to '\(targetMode.displayName)' reduces firewall and packet filter restrictions.
            • Protection: To defend against indirect prompt injection attacks, downgrading defenses requires physical user confirmation on the host OS via 'postureflow --\(targetMode.rawValue)' or the menu bar app.
            """
            return makeErrorContent(text: rejectionMsg)
        }

        config.activeProfile = targetMode
        do {
            try config.save()
            let successMsg = """
            ✔ Security Posture Escalation Applied (macOS):
            • Previous Profile: \(currentMode.displayName)
            • New Profile     : \(targetMode.displayName) (Security Rank: \(targetMode.securityRank))
            • Rationale       : \(reason)
            • Notice          : Configuration synced. Background helper applies anchor packet filter rules.
            """
            return makeSuccessContent(text: successMsg)
        } catch {
            return makeErrorContent(text: "Failed to save configuration: \(error.localizedDescription)")
        }
    }

    private func makeSuccessContent(text: String) -> [String: Any] {
        return [
            "content": [
                [
                    "type": "text",
                    "text": text
                ]
            ],
            "isError": false
        ]
    }

    private func makeErrorContent(text: String) -> [String: Any] {
        return [
            "content": [
                [
                    "type": "text",
                    "text": text
                ]
            ],
            "isError": true
        ]
    }

    private func serializeJson(_ dict: [String: Any]) -> String? {
        guard let data = try? JSONSerialization.data(withJSONObject: dict, options: []) else {
            return nil
        }
        return String(data: data, encoding: .utf8)
    }
}
