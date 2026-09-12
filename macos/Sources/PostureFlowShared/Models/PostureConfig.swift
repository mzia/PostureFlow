import Foundation

/// PostureFlow configuration schema for macOS.
public struct PostureConfig: Codable, Sendable {
    public var activeProfile: PostureMode
    public var autoFlowEnabled: Bool
    public var triggersEnabled: Bool
    public var circadianEnabled: Bool
    public var whitelistedSSIDs: [String: PostureMode]
    public var appTriggers: [String: PostureMode]
    public var devPorts: [Int]

    public init(
        activeProfile: PostureMode = .work,
        autoFlowEnabled: Bool = true,
        triggersEnabled: Bool = true,
        circadianEnabled: Bool = false,
        whitelistedSSIDs: [String: PostureMode] = [:],
        appTriggers: [String: PostureMode] = Self.defaultAppTriggers,
        devPorts: [Int] = [3000, 5173, 8000, 8080]
    ) {
        self.activeProfile = activeProfile
        self.autoFlowEnabled = autoFlowEnabled
        self.triggersEnabled = triggersEnabled
        self.circadianEnabled = circadianEnabled
        self.whitelistedSSIDs = whitelistedSSIDs
        self.appTriggers = appTriggers
        self.devPorts = devPorts
    }

    /// Standard macOS bundle identifiers mapped to postures
    public static var defaultAppTriggers: [String: PostureMode] {
        return [
            // Video & Work Collaboration -> Work
            "us.zoom.xos": .work,
            "com.microsoft.teams2": .work,
            "com.tinyspeck.slackmacgap": .work,
            "com.cisco.webexmeetingsapp": .work,
            
            // Developer Tools -> Dev
            "com.apple.dt.Xcode": .dev,
            "com.microsoft.VSCode": .dev,
            "com.sublimetext.4": .dev,
            "com.jetbrains.intellij": .dev,
            "com.mitchellh.ghostty": .dev,
            "com.googlecode.iterm2": .dev,

            // Gaming & Leisure -> Home
            "com.valvesoftware.steam": .home,
            "com.epicgames.EpicGamesLauncher": .home
        ]
    }

    /// Default file system configuration path (~/.config/postureflow/config.json)
    public static var defaultConfigURL: URL {
        let homeDir = FileManager.default.homeDirectoryForCurrentUser
        return homeDir
            .appendingPathComponent(".config", isDirectory: true)
            .appendingPathComponent("postureflow", isDirectory: true)
            .appendingPathComponent("config.json")
    }

    /// Loads configuration from disk, falling back to defaults if not found
    public static func load() -> PostureConfig {
        let url = defaultConfigURL
        guard FileManager.default.fileExists(atPath: url.path),
              let data = try? Data(contentsOf: url),
              let config = try? JSONDecoder().decode(PostureConfig.self, from: data) else {
            return PostureConfig()
        }
        return config
    }

    /// Persists configuration to ~/.config/postureflow/config.json
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
