import Foundation

/// Master hardware defense configuration aggregating physical token, honeypots, sensors, and proximity
public struct HardwareDefenseConfig: Codable, Sendable, Equatable {
    public var yubikey: YubikeyTetherConfig
    public var honeypot: HoneypotConfig
    public var sensors: SensorPrivacyConfig
    public var proximity: ProximityConfig

    public init(
        yubikey: YubikeyTetherConfig = YubikeyTetherConfig(),
        honeypot: HoneypotConfig = HoneypotConfig(),
        sensors: SensorPrivacyConfig = SensorPrivacyConfig(),
        proximity: ProximityConfig = ProximityConfig()
    ) {
        self.yubikey = yubikey
        self.honeypot = honeypot
        self.sensors = sensors
        self.proximity = proximity
    }

    public static var defaultConfigURL: URL {
        let homeDir = FileManager.default.homeDirectoryForCurrentUser
        return homeDir
            .appendingPathComponent(".config", isDirectory: true)
            .appendingPathComponent("postureflow", isDirectory: true)
            .appendingPathComponent("hardware.json")
    }

    public static func load() -> HardwareDefenseConfig {
        let url = defaultConfigURL
        guard FileManager.default.fileExists(atPath: url.path),
              let data = try? Data(contentsOf: url),
              let cfg = try? JSONDecoder().decode(HardwareDefenseConfig.self, from: data) else {
            return HardwareDefenseConfig()
        }
        return cfg
    }

    public func save() throws {
        let url = Self.defaultConfigURL
        let dir = url.deletingLastPathComponent()
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)

        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(self)
        try data.write(to: url, options: .atomic)
    }
}
