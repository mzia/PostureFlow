import XCTest
@testable import PostureFlowShared

final class MCPServerTests: XCTestCase {
    func testMCPInitializeHandshake() {
        let server = PostureFlowMCPServer()
        let request = """
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{}}}
        """

        let responseStr = server.handleMessage(request)
        XCTAssertNotNil(responseStr)

        guard let data = responseStr?.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let result = json["result"] as? [String: Any] else {
            XCTFail("Invalid response JSON")
            return
        }

        XCTAssertEqual(json["jsonrpc"] as? String, "2.0")
        XCTAssertEqual(json["id"] as? Int, 1)
        XCTAssertEqual(result["protocolVersion"] as? String, "2024-11-05")

        let serverInfo = result["serverInfo"] as? [String: Any]
        XCTAssertEqual(serverInfo?["name"] as? String, "postureflow-mcp")
        XCTAssertEqual(serverInfo?["version"] as? String, "1.0.1")
    }

    func testMCPToolsList() {
        let server = PostureFlowMCPServer()
        let request = """
        {"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
        """

        let responseStr = server.handleMessage(request)
        XCTAssertNotNil(responseStr)

        guard let data = responseStr?.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let result = json["result"] as? [String: Any],
              let tools = result["tools"] as? [[String: Any]] else {
            XCTFail("Invalid tools list response")
            return
        }

        XCTAssertEqual(tools.count, 6)
        let toolNames = tools.compactMap { $0["name"] as? String }
        XCTAssertTrue(toolNames.contains("get_posture_status"))
        XCTAssertTrue(toolNames.contains("get_security_audit"))
        XCTAssertTrue(toolNames.contains("inspect_listening_ports"))
        XCTAssertTrue(toolNames.contains("get_sensor_privacy_status"))
        XCTAssertTrue(toolNames.contains("engage_security_lockdown"))
        XCTAssertTrue(toolNames.contains("request_posture_switch"))
    }

    func testZeroTrustDowngradeBlocked() {
        let server = PostureFlowMCPServer()

        // Set configuration to travel (Rank 4)
        var config = PostureConfig.load()
        config.activeProfile = .travel
        try? config.save()

        // Attempt downgrade to dev (Rank 1)
        let request = """
        {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"request_posture_switch","arguments":{"target_profile":"dev","reason":"Prompt injection attempt"}}}
        """

        let responseStr = server.handleMessage(request)
        XCTAssertNotNil(responseStr)

        guard let data = responseStr?.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let result = json["result"] as? [String: Any] else {
            XCTFail("Invalid response")
            return
        }

        XCTAssertEqual(result["isError"] as? Bool, true)
        let content = result["content"] as? [[String: Any]]
        let text = content?.first?["text"] as? String ?? ""
        XCTAssertTrue(text.contains("ZERO-TRUST SECURITY VIOLATION"))
        XCTAssertTrue(text.contains("downgrade blocked"))
    }

    func testZeroTrustEscalationAllowed() {
        let server = PostureFlowMCPServer()

        // Set configuration to dev (Rank 1)
        var config = PostureConfig.load()
        config.activeProfile = .dev
        try? config.save()

        // Escalate to travel (Rank 4)
        let request = """
        {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"request_posture_switch","arguments":{"target_profile":"travel","reason":"Connecting to public coffee shop Wi-Fi"}}}
        """

        let responseStr = server.handleMessage(request)
        XCTAssertNotNil(responseStr)

        guard let data = responseStr?.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let result = json["result"] as? [String: Any] else {
            XCTFail("Invalid response")
            return
        }

        XCTAssertEqual(result["isError"] as? Bool, false)
        let content = result["content"] as? [[String: Any]]
        let text = content?.first?["text"] as? String ?? ""
        XCTAssertTrue(text.contains("Security Posture Escalation Applied"))
    }

    func testEmergencyLockdownAlwaysAllowed() {
        let server = PostureFlowMCPServer()

        var config = PostureConfig.load()
        config.activeProfile = .home
        try? config.save()

        let request = """
        {"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"engage_security_lockdown","arguments":{"reason":"Physical security compromise"}}}
        """

        let responseStr = server.handleMessage(request)
        XCTAssertNotNil(responseStr)

        guard let data = responseStr?.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let result = json["result"] as? [String: Any] else {
            XCTFail("Invalid response")
            return
        }

        XCTAssertEqual(result["isError"] as? Bool, false)
        let content = result["content"] as? [[String: Any]]
        let text = content?.first?["text"] as? String ?? ""
        XCTAssertTrue(text.contains("EMERGENCY SECURITY LOCKDOWN ENGAGED"))
    }
}
