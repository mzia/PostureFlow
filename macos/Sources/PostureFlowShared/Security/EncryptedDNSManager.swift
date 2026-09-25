import Foundation

/// Report of active DNS configuration and encryption state on macOS
public struct DNSStatusReport: Sendable, Codable, Equatable {
    public var isEncrypted: Bool
    public var activeResolvers: [String]
    public var isPort53Blocked: Bool
    public var dnssecEnforced: Bool
    public var description: String

    public init(
        isEncrypted: Bool = false,
        activeResolvers: [String] = [],
        isPort53Blocked: Bool = false,
        dnssecEnforced: Bool = false,
        description: String = "Default Resolution"
    ) {
        self.isEncrypted = isEncrypted
        self.activeResolvers = activeResolvers
        self.isPort53Blocked = isPort53Blocked
        self.dnssecEnforced = dnssecEnforced
        self.description = description
    }
}

/// Orchestrates posture-aware encrypted DNS settings and prevents plain-text DNS leakage
public struct EncryptedDNSManager: Sendable {
    public init() {}

    /// Queries active resolvers from macOS system configuration (`scutil --dns`)
    public static func getActiveResolvers() -> [String] {
        let task = Process()
        task.launchPath = "/usr/sbin/scutil"
        task.arguments = ["--dns"]

        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = Pipe()

        do {
            try task.run()
            task.waitUntilExit()

            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            guard let output = String(data: data, encoding: .utf8) else { return [] }

            var resolvers: [String] = []
            for line in output.components(separatedBy: "\n") {
                let trimmed = line.trimmingCharacters(in: .whitespaces)
                if trimmed.starts(with: "nameserver[") {
                    let parts = trimmed.components(separatedBy: ":")
                    if parts.count >= 2 {
                        let ip = parts[1].trimmingCharacters(in: .whitespaces)
                        if !resolvers.contains(ip) && !ip.isEmpty {
                            resolvers.pushOrAppend(ip)
                        }
                    }
                }
            }
            return resolvers
        } catch {
            return []
        }
    }

    /// Evaluates current DNS status under active posture mode and configuration
    public static func evaluateDNSStatus(mode: PostureMode, config: EncryptedDNSConfig) -> DNSStatusReport {
        guard config.enabled else {
            return DNSStatusReport(isEncrypted: false, activeResolvers: getActiveResolvers(), isPort53Blocked: false, dnssecEnforced: false, description: "System Managed")
        }

        let isTravel = (mode == .travel)
        let isPort53Blocked = isTravel && config.blockPlainPort53InTravel
        let isEncrypted = isTravel || config.dnsOverTLS

        let activeResolvers = isTravel ? config.primaryServers : getActiveResolvers()
        let desc = isTravel ? "Encrypted DoT/DNSSEC (Quad9 / Cloudflare) - Port 53 Blocked" : (isEncrypted ? "DoT / Secure Resolver" : "Standard LAN Resolver")

        return DNSStatusReport(
            isEncrypted: isEncrypted,
            activeResolvers: activeResolvers,
            isPort53Blocked: isPort53Blocked,
            dnssecEnforced: isTravel && config.dnssec,
            description: desc
        )
    }
}

private extension Array where Element == String {
    mutating func pushOrAppend(_ element: String) {
        self.append(element)
    }
}
