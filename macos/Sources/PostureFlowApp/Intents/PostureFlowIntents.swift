import Foundation
import AppIntents
import PostureFlowShared

/// AppEnum representation of PostureMode for native Apple Shortcuts and Siri integration
@available(macOS 13.0, *)
public enum PostureModeAppEnum: String, AppEnum, Sendable {
    case home
    case work
    case dev
    case travel

    public static var typeDisplayRepresentation: TypeDisplayRepresentation = "Posture Mode"

    public static var caseDisplayRepresentations: [PostureModeAppEnum: DisplayRepresentation] = [
        .home: DisplayRepresentation(title: "Home", subtitle: "Trusted LAN, AirDrop open", image: .init(systemName: "house.fill")),
        .work: DisplayRepresentation(title: "Work", subtitle: "LAN dev blocked, VPN trusted", image: .init(systemName: "briefcase.fill")),
        .dev: DisplayRepresentation(title: "Development", subtitle: "Dev ports open, secrets uncloaked", image: .init(systemName: "terminal.fill")),
        .travel: DisplayRepresentation(title: "Travel (Lockdown)", subtitle: "Hostile network lockdown, anti-leak DNS", image: .init(systemName: "airplane"))
    ]

    public var asPostureMode: PostureMode {
        switch self {
        case .home: return .home
        case .work: return .work
        case .dev: return .dev
        case .travel: return .travel
        }
    }
}

/// Apple Shortcut Intent to switch macOS security posture
@available(macOS 13.0, *)
public struct ApplyPostureIntent: AppIntent {
    public static var title: LocalizedStringResource = "Apply Security Posture"
    public static var description = IntentDescription("Switches macOS packet filter firewall, hardware defense, and network posture.")

    @Parameter(title: "Target Mode")
    public var targetMode: PostureModeAppEnum

    public init() {
        self.targetMode = .work
    }

    public init(targetMode: PostureModeAppEnum) {
        self.targetMode = targetMode
    }

    public func perform() async throws -> some IntentResult & ProvidesDialog & ReturnsValue<String> {
        let mode = targetMode.asPostureMode
        var config = PostureConfig.load()
        let previous = config.activeProfile
        config.activeProfile = mode
        try? config.save()

        // Apply cloaking & DNS
        _ = CredentialCloakEngine.handlePostureChange(from: previous, to: mode, config: config.credentialCloaking)
        _ = EncryptedDNSManager.evaluateDNSStatus(mode: mode, config: config.encryptedDNS)

        // Request helper via XPC if available
        let client = XPCClient.shared
        _ = try? await client.applyPosture(mode: mode)

        let msg = "PostureFlow switched to \(mode.displayName) posture"
        return .result(value: msg, dialog: IntentDialog(stringLiteral: msg))
    }
}

/// Apple Shortcut Intent to query current PostureFlow state
@available(macOS 13.0, *)
public struct GetPostureStatusIntent: AppIntent {
    public static var title: LocalizedStringResource = "Get Security Posture Status"
    public static var description = IntentDescription("Returns the active PostureFlow security profile and security score.")

    public init() {}

    public func perform() async throws -> some IntentResult & ProvidesDialog & ReturnsValue<String> {
        let config = PostureConfig.load()
        let mode = config.activeProfile
        let msg = "Active Posture: \(mode.displayName). Auto-Flow is \(config.autoFlowEnabled ? "enabled" : "disabled")."
        return .result(value: msg, dialog: IntentDialog(stringLiteral: msg))
    }
}

/// Apple Shortcut Intent to engage emergency travel lockdown and kill sensors
@available(macOS 13.0, *)
public struct EmergencyLockdownIntent: AppIntent {
    public static var title: LocalizedStringResource = "Engage Emergency Lockdown"
    public static var description = IntentDescription("Engages Travel posture lockdown, drops ICMP, and engages sensor kill switch.")

    public init() {}

    public func perform() async throws -> some IntentResult & ProvidesDialog & ReturnsValue<String> {
        var config = PostureConfig.load()
        config.activeProfile = .travel
        try? config.save()

        _ = SensorPrivacyController.emergencyKillAllSensors()
        _ = CredentialCloakEngine.handlePostureChange(from: .dev, to: .travel, config: config.credentialCloaking)

        let client = XPCClient.shared
        _ = try? await client.applyPosture(mode: .travel)
        _ = try? await client.emergencyKillSensors()

        let msg = "🚨 Emergency Lockdown Engaged! Travel posture enforced and sensors killed."
        return .result(value: msg, dialog: IntentDialog(stringLiteral: msg))
    }
}

/// Native Apple Shortcuts App discovery provider
@available(macOS 13.0, *)
public struct PostureFlowShortcuts: AppShortcutsProvider {
    public static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: ApplyPostureIntent(),
            phrases: [
                "Switch \(.applicationName) posture",
                "Set \(.applicationName) profile",
                "Change \(.applicationName) mode"
            ],
            shortTitle: "Switch Posture",
            systemImageName: "shield.fill"
        )
        AppShortcut(
            intent: EmergencyLockdownIntent(),
            phrases: [
                "Engage \(.applicationName) emergency lockdown",
                "Emergency lock in \(.applicationName)"
            ],
            shortTitle: "Emergency Lockdown",
            systemImageName: "lock.shield.fill"
        )
    }
}
