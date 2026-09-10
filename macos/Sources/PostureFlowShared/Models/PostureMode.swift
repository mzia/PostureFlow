import Foundation

/// Defines the security and hardware operational postures available in PostureFlow.
public enum PostureMode: String, CaseIterable, Identifiable, Codable, Sendable {
    case home
    case work
    case dev
    case travel

    public var id: String { rawValue }

    /// Human-readable title
    public var displayName: String {
        switch self {
        case .home:
            return "Home"
        case .work:
            return "Work"
        case .dev:
            return "Development"
        case .travel:
            return "Travel (Lockdown)"
        }
    }

    /// Descriptive overview of the profile's security posture
    public var postureDescription: String {
        switch self {
        case .home:
            return "Trusted LAN perimeter. Bonjour, AirDrop, and LAN gaming traffic permitted. External WAN inbound blocked."
        case .work:
            return "Hardened corporate posture. Inbound dev ports blocked from local network. VPN (utun*) interfaces fully trusted."
        case .dev:
            return "Developer agility posture. Local development ports (3000, 5173, 8080, 8000) open on loopback and LAN."
        case .travel:
            return "Hostile network lockdown. All inbound traffic dropped. Stealth mode enabled (drops ping). Low Power Mode active."
        }
    }

    /// Primary SF Symbol for macOS menu bar and controls
    public var sfSymbol: String {
        switch self {
        case .home:
            return "house.fill"
        case .work:
            return "briefcase.fill"
        case .dev:
            return "chevron.left.forwardslash.chevron.right"
        case .travel:
            return "airplane"
        }
    }

    /// Secondary status badge symbol
    public var badgeSymbol: String {
        switch self {
        case .home:
            return "shield.checkerboard"
        case .work:
            return "network.badge.shield.half.filled"
        case .dev:
            return "terminal.fill"
        case .travel:
            return "lock.shield.fill"
        }
    }

    /// Hex accent color matching PostureFlow Linux indicators
    public var accentColorHex: String {
        switch self {
        case .home:
            return "#10B981" // Emerald Green
        case .work:
            return "#3B82F6" // Sapphire Blue
        case .dev:
            return "#F59E0B" // Amber Gold
        case .travel:
            return "#EF4444" // Crimson Red
        }
    }

    /// Baseline display idle sleep timeout in minutes
    public var defaultDisplaySleepMinutes: Int {
        switch self {
        case .home:
            return 15
        case .work:
            return 10
        case .dev:
            return 30
        case .travel:
            return 3
        }
    }

    /// Whether Apple Silicon Low Power Mode is enforced
    public var enablesLowPowerMode: Bool {
        switch self {
        case .travel:
            return true
        case .home, .work, .dev:
            return false
        }
    }

    /// Inbound firewall default rule description
    public var inboundFirewallPolicy: String {
        switch self {
        case .home:
            return "Block external WAN; pass LAN & mDNS/AirDrop"
        case .work:
            return "Block LAN dev ports; pass VPN (utun*) & CUPS"
        case .dev:
            return "Pass local dev servers (3000, 5173, 8080, 8000)"
        case .travel:
            return "Block all unsolicited inbound; drop ICMP ping"
        }
    }
}
