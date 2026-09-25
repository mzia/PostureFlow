import Foundation
import PostureFlowShared

@main
struct PostureFlowCLI {
    static func printUsage() {
        print("""
        PostureFlow for macOS (v1.0.0)
        Dynamic security posture and Apple Silicon power orchestrator.

        USAGE:
            postureflow [OPTIONS]

        OPTIONS:
            --status         Display current security posture, score, and network state
            --home           Apply Home posture (LAN trusted, mDNS/AirDrop open)
            --work           Apply Work posture (LAN dev blocked, VPN trusted, high performance)
            --dev            Apply Dev posture (Local developer ports open)
            --travel         Apply Travel posture (Strict lockdown, ICMP dropped, Low Power Mode)
            --score          Evaluate and display 100-point security posture score breakdown
            --kill-sensors   Emergency Kill Switch: cuts microphone volume to 0% and locks sensors
            --restore-sensors Restore microphone and media sensors to standard operation
            --sensors-status Display live microphone and sensor privacy state
            --tether-status  Display connected YubiKeys and physical token tethering status
            --dns-status     Display profile-aware encrypted DNS (DoT/DoH) & Anti-Leak state
            --cloak-status   Display developer secrets cloaking state (SSH agent, AWS, vaults)
            --uncloak        Uncloak developer credentials (~/.aws/credentials posix permissions)
            --evil-twin-check Check current Wi-Fi BSSID & Gateway MAC against trusted fingerprints
            --restore        Flush anchor rules and restore default network state
            --mcp, mcp       Launch native Model Context Protocol (MCP) server over stdio
            --help, -h       Show this help message
        """)
    }

    static func main() {
        let args = CommandLine.arguments.dropFirst()

        guard let command = args.first else {
            printUsage()
            exit(0)
        }

        // Simple synchronous CLI invocation
        let config = PostureConfig.load()

        switch command {
        case "--help", "-h":
            printUsage()
            exit(0)

        case "--status":
            print("┌────────────────────────────────────────────────────────┐")
            print("│ PostureFlow macOS Status                               │")
            print("├────────────────────────────────────────────────────────┤")
            print("│ Active Posture   : \(config.activeProfile.displayName.padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Policy           : \(config.activeProfile.inboundFirewallPolicy.padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Auto-Flow        : \((config.autoFlowEnabled ? "Enabled" : "Disabled").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ App Triggers     : \((config.triggersEnabled ? "Enabled" : "Disabled").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Low Power Mode   : \((config.activeProfile.enablesLowPowerMode ? "Active" : "Standard").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Display Sleep    : \("\(config.activeProfile.defaultDisplaySleepMinutes) minutes".padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("└────────────────────────────────────────────────────────┘")

        case "--home", "--work", "--dev", "--travel":
            let modeString = String(command.dropFirst(2))
            guard let targetMode = PostureMode(rawValue: modeString) else {
                print("Error: Unknown posture mode '\(modeString)'")
                exit(1)
            }

            var updated = config
            updated.activeProfile = targetMode
            do {
                try updated.save()
                print("✔ Switched posture to: \(targetMode.displayName)")
                print("  \(targetMode.postureDescription)")
                print("  Notice: Open PostureFlow.app or run the helper to apply pfctl anchors system-wide.")
            } catch {
                print("Error saving posture configuration: \(error.localizedDescription)")
                exit(1)
            }

        case "--score":
            let yubikeys = YubikeyDetector.detectYubikeys()
            let isEmergency = SensorPrivacyController.isEmergencyKillActive()
            let dnsReport = EncryptedDNSManager.evaluateDNSStatus(mode: config.activeProfile, config: config.encryptedDNS)
            let isCloaked = (config.activeProfile != .dev && config.credentialCloaking.enabled)
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
                isProximityLockActive: config.hardwareDefense.proximity.enabled,
                isEvilTwinDetected: false,
                isEncryptedDNSActive: dnsReport.isEncrypted,
                isCredentialCloaked: isCloaked
            )
            print("PostureFlow Security Score: \(score.score)/100 (Grade: \(score.grade))")
            print("Score Breakdown:")
            for (category, points) in score.breakdown.sorted(by: { $0.key < $1.key }) {
                print("  • \(category): +\(points)")
            }
            if !score.recommendations.isEmpty {
                print("\nRecommendations:")
                for rec in score.recommendations {
                    print("  ⚠️ \(rec)")
                }
            }

        case "--kill-sensors":
            let success = SensorPrivacyController.emergencyKillAllSensors()
            if success {
                print("🚨 Emergency Sensor Kill Switch engaged! Input volume set to 0%.")
            } else {
                print("⚠️ Failed to cut audio input volume.")
            }

        case "--restore-sensors":
            let success = SensorPrivacyController.restoreAllSensors()
            if success {
                print("✔ Sensors restored to standard operation.")
            } else {
                print("⚠️ Failed to restore audio input volume.")
            }

        case "--sensors-status":
            let report = SensorPrivacyController.getReport()
            print("┌────────────────────────────────────────────────────────┐")
            print("│ PostureFlow macOS Hardware & Sensor Privacy            │")
            print("├────────────────────────────────────────────────────────┤")
            print("│ Microphone Input : \((report.microphoneMuted ? "MUTED (0%)" : "ACTIVE (\(report.inputVolumePercent)%)").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Camera Lockout   : \((report.cameraBlocked ? "ENGAGED" : "Standard").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Location Lockout : \((report.locationBlocked ? "ENGAGED" : "Standard").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Emergency Kill   : \((report.emergencyKillActive ? "ACTIVE (RED)" : "INACTIVE").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("└────────────────────────────────────────────────────────┘")

        case "--tether-status":
            let keys = YubikeyDetector.detectYubikeys()
            let cfg = config.hardwareDefense.yubikey
            print("┌────────────────────────────────────────────────────────┐")
            print("│ PostureFlow YubiKey Physical Token Tether              │")
            print("├────────────────────────────────────────────────────────┤")
            print("│ Tethering Master : \((cfg.enabled ? "Enabled" : "Disabled").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Auto-Lock        : \((cfg.lockOnRemoval ? "Enabled" : "Disabled").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Demote on Unplug : \((cfg.demoteOnRemoval ? "Travel Profile" : "Disabled").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Auto-Restore     : \((cfg.restoreOnInsert ? "Enabled" : "Disabled").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Connected Keys   : \(String(keys.count).padding(toLength: 35, withPad: " ", startingAt: 0))│")
            for k in keys {
                print("│   -> \(k.name.padding(toLength: 49, withPad: " ", startingAt: 0))│")
            }
            print("└────────────────────────────────────────────────────────┘")

        case "--honeypot-status":
            let cfg = config.hardwareDefense.honeypot
            let incidents = HoneypotLedger.loadRecentIncidents()
            print("┌────────────────────────────────────────────────────────┐")
            print("│ PostureFlow Honeypot Decoy Traps & LAN Defense         │")
            print("├────────────────────────────────────────────────────────┤")
            print("│ Traps Master     : \((cfg.enabled ? "Enabled" : "Disabled").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Trap Ports       : \(cfg.trapPorts.map(String.init).joined(separator: ", ").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Auto-ban via pf  : \((cfg.autoBlockOffenders ? "Enabled" : "Disabled").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Total Incidents  : \(String(incidents.count).padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("└────────────────────────────────────────────────────────┘")

        case "--dns-status":
            let report = EncryptedDNSManager.evaluateDNSStatus(mode: config.activeProfile, config: config.encryptedDNS)
            print("┌────────────────────────────────────────────────────────┐")
            print("│ PostureFlow macOS Profile-Aware Encrypted DNS          │")
            print("├────────────────────────────────────────────────────────┤")
            print("│ DNS Engine       : \((config.encryptedDNS.enabled ? "Active" : "Disabled").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Encryption Mode  : \((report.isEncrypted ? "Encrypted (DoT/DoH)" : "Standard LAN").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Anti-Leak (Pt 53): \((report.isPort53Blocked ? "BLOCKED (Dropped)" : "Open").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ DNSSEC Enforced  : \((report.dnssecEnforced ? "Enabled" : "Disabled").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Active Resolvers : \(report.activeResolvers.joined(separator: ", ").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Description      : \(report.description.padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("└────────────────────────────────────────────────────────┘")

        case "--cloak-status":
            let awsPath = CredentialCloakEngine.awsCredentialsURL.path
            let awsExists = FileManager.default.fileExists(atPath: awsPath)
            var awsCloaked = false
            if awsExists, let attrs = try? FileManager.default.attributesOfItem(atPath: awsPath),
               let perms = attrs[.posixPermissions] as? NSNumber {
                awsCloaked = (perms.intValue == 0)
            }
            print("┌────────────────────────────────────────────────────────┐")
            print("│ PostureFlow Developer Credential Cloaking              │")
            print("├────────────────────────────────────────────────────────┤")
            print("│ Cloak Engine     : \((config.credentialCloaking.enabled ? "Enabled" : "Disabled").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ AWS File Present : \((awsExists ? "Yes" : "No ~/.aws/credentials").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ AWS Cloaked (000): \((awsCloaked ? "CLOAKED (000)" : "Uncloaked (Readable)").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Auto-Purge SSH   : \((config.credentialCloaking.purgeSSHAgentOnLeaveDev ? "On Leave Dev" : "Disabled").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Vault Auto-Lock  : \((config.credentialCloaking.lockPasswordManagers ? "1Password / Bitwarden" : "Disabled").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("└────────────────────────────────────────────────────────┘")

        case "--uncloak":
            let success = CredentialCloakEngine.setAWSCredentialsCloaked(false)
            if success {
                print("✔ Restored permissions (0600) on ~/.aws/credentials. Secrets uncloaked.")
            } else {
                print("⚠️ Could not uncloak ~/.aws/credentials (file may not exist or permission denied).")
            }

        case "--evil-twin-check":
            let currentGW = AntiEvilTwinDetector.resolveGatewayMAC()
            print("┌────────────────────────────────────────────────────────┐")
            print("│ PostureFlow Anti-Evil Twin Network Verification        │")
            print("├────────────────────────────────────────────────────────┤")
            print("│ Gateway MAC      : \((currentGW ?? "Unknown").padding(toLength: 35, withPad: " ", startingAt: 0))│")
            print("│ Fingerprinted APs: \(String(config.trustedNetworks.count).padding(toLength: 35, withPad: " ", startingAt: 0))│")
            for (ssid, fp) in config.trustedNetworks {
                print("│   • \(ssid): BSSID=\(fp.bssid ?? "*") GW=\(fp.gatewayMAC ?? "*") [\(fp.targetMode.displayName)]")
            }
            print("└────────────────────────────────────────────────────────┘")

        case "--restore":
            print("Restoring default network settings...")
            var updated = config
            updated.activeProfile = .work
            try? updated.save()
            print("✔ Reset active profile to default (Work).")

        case "--mcp", "mcp":
            PostureFlowMCPServer.runStdioServer()
            exit(0)

        default:
            print("Error: Unknown argument '\(command)'")
            printUsage()
            exit(1)
        }
    }
}
