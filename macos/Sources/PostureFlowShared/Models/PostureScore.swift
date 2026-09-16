import Foundation

/// 100-point security posture calculation engine for macOS.
public struct PostureScore: Codable, Sendable {
    public let score: Int
    public let grade: String
    public let breakdown: [String: Int]
    public let recommendations: [String]

    public init(
        mode: PostureMode,
        isFirewallActive: Bool,
        isStealthModeActive: Bool,
        isVPNActive: Bool,
        openPortCount: Int,
        isLowPowerMode: Bool,
        isHardwareTetherActive: Bool = false,
        isHoneypotActive: Bool = false,
        isSensorPrivacyActive: Bool = false,
        isProximityLockActive: Bool = false
    ) {
        var calculatedScore = 0
        var items: [String: Int] = [:]
        var recs: [String] = []

        // 1. Base Posture Allocation (Up to 30 pts)
        let modeScore: Int
        switch mode {
        case .travel:
            modeScore = 30
        case .work:
            modeScore = 25
        case .home:
            modeScore = 20
        case .dev:
            modeScore = 15
        }
        calculatedScore += modeScore
        items["Posture Baseline (\(mode.displayName))"] = modeScore

        // 2. Packet Filter (pfctl) Status (Up to 35 pts)
        if isFirewallActive {
            calculatedScore += 35
            items["Packet Filter Anchor Active"] = 35
        } else {
            recs.append("Firewall anchor is disabled. Re-enable PostureFlow packet filtering.")
        }

        // 3. Stealth Mode (Drops ICMP ping) (Up to 15 pts)
        if isStealthModeActive {
            calculatedScore += 15
            items["Stealth Mode (ICMP Dropped)"] = 15
        } else if mode == .travel {
            recs.append("Enable Stealth Mode to drop untrusted ICMP probes in public areas.")
        }

        // 4. Corporate VPN Tunneling (Up to 10 pts)
        if isVPNActive {
            calculatedScore += 10
            items["Encrypted VPN Tunnel (utun*)"] = 10
        } else if mode == .work {
            recs.append("Work posture active but no utun* VPN tunnel detected.")
        }

        // 5. Open Port Exposure Penalty
        if openPortCount == 0 {
            calculatedScore += 10
            items["Zero Unsolicited Ports Open"] = 10
        } else if openPortCount <= 3 && mode == .dev {
            calculatedScore += 5
            items["Dev Ports Open (Permitted)"] = 5
        } else if openPortCount > 5 {
            recs.append("Detected \(openPortCount) listening network sockets. Close unused servers.")
        }

        // 6. Zero-Trust Hardware Defense & Physical Tokens
        if isHardwareTetherActive {
            calculatedScore += 5
            items["YubiKey Hardware Tether Active"] = 5
        }
        if isHoneypotActive {
            calculatedScore += 5
            items["Decoy Honeypot Traps Armed"] = 5
        }
        if isSensorPrivacyActive {
            calculatedScore += 5
            items["Sensor Hardware Privacy Locked"] = 5
        }
        if isProximityLockActive {
            calculatedScore += 5
            items["Walk-Away Proximity Auto-Lock"] = 5
        }

        let finalScore = max(0, min(100, calculatedScore))
        self.score = finalScore

        // Determine letter grade
        switch finalScore {
        case 90...100:
            self.grade = "A+"
        case 80..<90:
            self.grade = "A"
        case 70..<80:
            self.grade = "B"
        case 55..<70:
            self.grade = "C"
        default:
            self.grade = "F"
        }

        self.breakdown = items
        self.recommendations = recs
    }
}
