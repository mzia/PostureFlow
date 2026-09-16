import Foundation

/// Represents an intrusion attempt caught by decoy honeypot traps
public struct HoneypotIncident: Identifiable, Codable, Sendable, Equatable {
    public var id: UUID
    public var timestamp: Date
    public var peerIP: String
    public var trapPort: Int
    public var interface: String
    public var blocked: Bool

    public init(
        id: UUID = UUID(),
        timestamp: Date = Date(),
        peerIP: String,
        trapPort: Int,
        interface: String = "en0",
        blocked: Bool = true
    ) {
        self.id = id
        self.timestamp = timestamp
        self.peerIP = peerIP
        self.trapPort = trapPort
        self.interface = interface
        self.blocked = blocked
    }
}

/// Configuration for Decoy Honeypot Port Traps
public struct HoneypotConfig: Codable, Sendable, Equatable {
    public var enabled: Bool
    public var trapPorts: [Int]
    public var autoBlockOffenders: Bool
    public var alertUser: Bool
    public var activeInProfiles: [PostureMode]

    public init(
        enabled: Bool = true,
        trapPorts: [Int] = [2222, 8080, 4450],
        autoBlockOffenders: Bool = true,
        alertUser: Bool = true,
        activeInProfiles: [PostureMode] = [.travel, .home, .work]
    ) {
        self.enabled = enabled
        self.trapPorts = trapPorts
        self.autoBlockOffenders = autoBlockOffenders
        self.alertUser = alertUser
        self.activeInProfiles = activeInProfiles
    }

    /// Default trap ports: 2222 (SSH decoy), 8080 (Web admin decoy), 4450 (SMB decoy)
    public static var defaultTrapPorts: [Int] {
        [2222, 8080, 4450]
    }
}

/// Helper for managing persistent incident records
public struct HoneypotLedger {
    public static var incidentLogURL: URL {
        let homeDir = FileManager.default.homeDirectoryForCurrentUser
        return homeDir
            .appendingPathComponent(".config", isDirectory: true)
            .appendingPathComponent("postureflow", isDirectory: true)
            .appendingPathComponent("honeypot_incidents.json")
    }

    public static func loadRecentIncidents(limit: Int = 50) -> [HoneypotIncident] {
        let url = incidentLogURL
        guard FileManager.default.fileExists(atPath: url.path),
              let data = try? Data(contentsOf: url),
              let incidents = try? JSONDecoder().decode([HoneypotIncident].self, from: data) else {
            return []
        }
        return Array(incidents.suffix(limit))
    }

    public static func recordIncident(_ incident: HoneypotIncident) {
        var incidents = loadRecentIncidents(limit: 100)
        incidents.append(incident)

        let url = incidentLogURL
        let dir = url.deletingLastPathComponent()
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)

        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        encoder.dateEncodingStrategy = .iso8601
        if let data = try? encoder.encode(incidents) {
            try? data.write(to: url, options: .atomic)
        }
    }
}
