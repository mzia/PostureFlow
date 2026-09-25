import Foundation

/// Result of evaluating current Wi-Fi network against trusted fingerprints
public enum NetworkTrustEvaluation: Sendable, Equatable {
    case trusted(fingerprint: NetworkFingerprint)
    case unregistered(ssid: String, bssid: String?, gatewayMAC: String?)
    case evilTwinDetected(ssid: String, expectedBSSID: String?, actualBSSID: String?, expectedGateway: String?, actualGateway: String?)
    case disconnected
}

/// Detects rogue Wi-Fi access points advertising legitimate SSIDs (Evil Twin attack defense)
public struct AntiEvilTwinDetector: Sendable {
    public init() {}

    /// Evaluates current Wi-Fi parameters against the configured trusted network registry
    public static func evaluate(
        currentSSID: String?,
        currentBSSID: String?,
        currentGatewayMAC: String?,
        trustedRegistry: [String: NetworkFingerprint]
    ) -> NetworkTrustEvaluation {
        guard let ssid = currentSSID, !ssid.isEmpty else {
            return .disconnected
        }

        // Check if this SSID has a trusted fingerprint recorded
        guard let expected = trustedRegistry[ssid] else {
            return .unregistered(ssid: ssid, bssid: currentBSSID, gatewayMAC: currentGatewayMAC)
        }

        var isMismatched = false
        var bssidMismatch = false
        var gatewayMismatch = false

        // Normalize MAC addresses (lowercase, remove leading zeros if varied)
        let normCurrentBSSID = currentBSSID?.lowercased().trimmingCharacters(in: .whitespacesAndNewlines)
        let normExpectedBSSID = expected.bssid?.lowercased().trimmingCharacters(in: .whitespacesAndNewlines)
        
        let normCurrentGW = currentGatewayMAC?.lowercased().trimmingCharacters(in: .whitespacesAndNewlines)
        let normExpectedGW = expected.gatewayMAC?.lowercased().trimmingCharacters(in: .whitespacesAndNewlines)

        if let expB = normExpectedBSSID, !expB.isEmpty,
           let curB = normCurrentBSSID, !curB.isEmpty,
           expB != curB {
            isMismatched = true
            bssidMismatch = true
        }

        if let expGW = normExpectedGW, !expGW.isEmpty,
           let curGW = normCurrentGW, !curGW.isEmpty,
           expGW != curGW {
            isMismatched = true
            gatewayMismatch = true
        }

        if isMismatched {
            return .evilTwinDetected(
                ssid: ssid,
                expectedBSSID: bssidMismatch ? expected.bssid : nil,
                actualBSSID: bssidMismatch ? currentBSSID : nil,
                expectedGateway: gatewayMismatch ? expected.gatewayMAC : nil,
                actualGateway: gatewayMismatch ? currentGatewayMAC : nil
            )
        }

        return .trusted(fingerprint: expected)
    }

    /// Reads default gateway MAC address from macOS ARP cache
    public static func resolveGatewayMAC() -> String? {
        let task = Process()
        task.launchPath = "/usr/sbin/arp"
        task.arguments = ["-an"]

        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = Pipe()

        do {
            try task.run()
            task.waitUntilExit()

            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            guard let output = String(data: data, encoding: .utf8) else { return nil }

            // Typical format: "? (192.168.1.1) at 0:11:22:33:44:55 on en0 ifscope [ethernet]"
            for line in output.components(separatedBy: "\n") {
                if line.contains("en0") || line.contains("en1") {
                    let parts = line.components(separatedBy: " ")
                    if let atIndex = parts.firstIndex(of: "at"), atIndex + 1 < parts.count {
                        let mac = parts[atIndex + 1]
                        if mac.contains(":") && mac != "(incomplete)" {
                            return mac
                        }
                    }
                }
            }
        } catch {
            return nil
        }
        return nil
    }
}
