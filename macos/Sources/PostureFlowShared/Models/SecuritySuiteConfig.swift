import Foundation

/// Fingerprint for trusted Wi-Fi networks to detect Evil Twin rogue APs
public struct NetworkFingerprint: Codable, Sendable, Hashable, Equatable {
    public var ssid: String
    public var bssid: String?
    public var gatewayMAC: String?
    public var targetMode: PostureMode

    public init(ssid: String, bssid: String? = nil, gatewayMAC: String? = nil, targetMode: PostureMode = .home) {
        self.ssid = ssid
        self.bssid = bssid
        self.gatewayMAC = gatewayMAC
        self.targetMode = targetMode
    }
}

/// Configuration for posture-aware developer credential cloaking
public struct CredentialCloakConfig: Codable, Sendable, Equatable {
    public var enabled: Bool
    public var purgeSSHAgentOnLeaveDev: Bool
    public var cloakAWSCredentials: Bool
    public var lockPasswordManagers: Bool

    public init(
        enabled: Bool = true,
        purgeSSHAgentOnLeaveDev: Bool = true,
        cloakAWSCredentials: Bool = true,
        lockPasswordManagers: Bool = true
    ) {
        self.enabled = enabled
        self.purgeSSHAgentOnLeaveDev = purgeSSHAgentOnLeaveDev
        self.cloakAWSCredentials = cloakAWSCredentials
        self.lockPasswordManagers = lockPasswordManagers
    }
}

/// Profile-aware encrypted DNS (DoT / DoH / DNSSEC) configuration
public struct EncryptedDNSConfig: Codable, Sendable, Equatable {
    public var enabled: Bool
    public var primaryServers: [String]
    public var fallbackServers: [String]
    public var blockPlainPort53InTravel: Bool
    public var dnsOverTLS: Bool
    public var dnssec: Bool

    public init(
        enabled: Bool = true,
        primaryServers: [String] = ["9.9.9.9", "149.112.112.112"],
        fallbackServers: [String] = ["1.1.1.1", "1.0.0.1"],
        blockPlainPort53InTravel: Bool = true,
        dnsOverTLS: Bool = true,
        dnssec: Bool = true
    ) {
        self.enabled = enabled
        self.primaryServers = primaryServers
        self.fallbackServers = fallbackServers
        self.blockPlainPort53InTravel = blockPlainPort53InTravel
        self.dnsOverTLS = dnsOverTLS
        self.dnssec = dnssec
    }
}
