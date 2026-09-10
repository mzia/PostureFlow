import Foundation
import PostureFlowShared

/// Manages macOS Packet Filter (`pfctl`) rules isolated inside a dedicated anchor.
/// This prevents tampering with the operating system's base `/etc/pf.conf`.
public final class PacketFilterManager {
    public static let shared = PacketFilterManager()
    
    public static let anchorName = "com.postureflow"
    public static let anchorPath = "/etc/pf.anchors/com.postureflow"
    
    private let fileManager = FileManager.default

    public init() {}

    /// Generates pf rules for the specified posture mode
    public func generateRules(for mode: PostureMode, devPorts: [Int] = [3000, 5173, 8000, 8080]) -> [String] {
        var rules: [String] = [
            "# PostureFlow macOS Packet Filter Anchor: \(mode.displayName)",
            "# Generated automatically on \(ISO8601DateFormatter().string(from: Date()))",
            ""
        ]

        // 1. Loopback traffic is always permitted
        rules.append("set skip on lo0")
        
        // 2. Always pass VPN virtual interfaces (utun*)
        rules.append("# Trust corporate & private VPN tunnels")
        rules.append("pass in quick on utun+ all")
        rules.append("pass out quick on utun+ all")
        rules.append("")

        switch mode {
        case .home:
            rules.append("# Home Posture: Allow local LAN, AirDrop/Bonjour, block inbound WAN")
            rules.append("# Allow Bonjour / mDNS & AirDrop")
            rules.append("pass in proto udp to any port 5353")
            rules.append("# Allow local subnet traffic")
            rules.append("pass in from 192.168.0.0/16 to any keep state")
            rules.append("pass in from 10.0.0.0/8 to any keep state")
            rules.append("pass in from 172.16.0.0/12 to any keep state")
            rules.append("# Allow Steam Remote Play & local gaming")
            rules.append("pass in proto udp to any port 27031:27040")
            rules.append("# Default drop other inbound")
            rules.append("block in log all")
            rules.append("pass out all keep state")

        case .work:
            rules.append("# Work Posture: Isolate development ports from LAN, pass corporate services")
            rules.append("# Block dev ports from local LAN")
            let portList = devPorts.map(String.init).joined(separator: " ")
            rules.append("block in proto tcp to any port { \(portList) }")
            rules.append("# Allow local network printing (IPP/CUPS)")
            rules.append("pass in proto tcp to any port 631")
            rules.append("pass out all keep state")

        case .dev:
            rules.append("# Dev Posture: Developer servers exposed for local testing and debugging")
            let portList = devPorts.map(String.init).joined(separator: " ")
            rules.append("pass in proto tcp to any port { \(portList) } keep state")
            rules.append("pass in proto udp to any port { \(portList) } keep state")
            rules.append("pass out all keep state")

        case .travel:
            rules.append("# Travel Posture: Hostile perimeter lockdown")
            rules.append("# Stealth mode: Drop unsolicited ICMP echo requests (pings)")
            rules.append("block in quick proto icmp all")
            rules.append("block in quick proto ipv6-icmp all")
            rules.append("# Block all inbound traffic")
            rules.append("block in log all")
            rules.append("# Allow strictly stateful outbound traffic")
            rules.append("pass out all keep state")
        }

        rules.append("")
        return rules
    }

    /// Writes rules to the anchor file and loads them into pfctl
    public func applyRules(for mode: PostureMode, devPorts: [Int] = [3000, 5173, 8000, 8080]) -> (success: Bool, error: String?) {
        let rules = generateRules(for: mode, devPorts: devPorts)
        let content = rules.joined(separator: "\n")

        do {
            // Write rules to anchor file
            let anchorURL = URL(fileURLWithPath: Self.anchorPath)
            let parentDir = anchorURL.deletingLastPathComponent()
            if !fileManager.fileExists(atPath: parentDir.path) {
                try fileManager.createDirectory(at: parentDir, withIntermediateDirectories: true)
            }
            try content.write(to: anchorURL, atomically: true, encoding: .utf8)

            // Ensure pf is enabled: pfctl -e
            _ = executeCommand("/sbin/pfctl", arguments: ["-e"])

            // Load anchor: pfctl -a com.postureflow -f /etc/pf.anchors/com.postureflow
            let loadResult = executeCommand("/sbin/pfctl", arguments: [
                "-a", Self.anchorName,
                "-f", Self.anchorPath
            ])

            if loadResult.exitCode == 0 {
                return (true, nil)
            } else {
                return (false, "Failed to load pfctl anchor: \(loadResult.output)")
            }
        } catch {
            return (false, "Error writing anchor rules: \(error.localizedDescription)")
        }
    }

    /// Flushes anchor rules and clears PostureFlow packet filter modifications
    public func flushAnchor() -> (success: Bool, error: String?) {
        let result = executeCommand("/sbin/pfctl", arguments: [
            "-a", Self.anchorName,
            "-F", "all"
        ])
        if result.exitCode == 0 {
            // Remove anchor file if it exists
            try? fileManager.removeItem(atPath: Self.anchorPath)
            return (true, nil)
        } else {
            return (false, result.output)
        }
    }

    /// Verifies if pfctl is enabled and our anchor contains active rules
    public func isAnchorActive() -> Bool {
        let result = executeCommand("/sbin/pfctl", arguments: [
            "-a", Self.anchorName,
            "-s", "rules"
        ])
        return result.exitCode == 0 && !result.output.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private func executeCommand(_ binary: String, arguments: [String]) -> (exitCode: Int32, output: String) {
        let task = Process()
        task.executableURL = URL(fileURLWithPath: binary)
        task.arguments = arguments

        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = pipe

        do {
            try task.run()
            task.waitUntilExit()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            let output = String(data: data, encoding: .utf8) ?? ""
            return (task.terminationStatus, output)
        } catch {
            return (-1, error.localizedDescription)
        }
    }
}
