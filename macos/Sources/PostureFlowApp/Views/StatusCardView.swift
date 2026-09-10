import SwiftUI
import PostureFlowShared

/// Compact status card rendered at the top of the PostureFlow MenuBarExtra popover.
public struct StatusCardView: View {
    @ObservedObject var state: PostureStateStore

    public init(state: PostureStateStore) {
        self.state = state
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            // Header: Posture name + Score Grade
            HStack {
                Image(systemName: state.currentMode.sfSymbol)
                    .font(.system(size: 20, weight: .bold))
                    .foregroundColor(Color(hex: state.currentMode.accentColorHex))

                VStack(alignment: .leading, spacing: 2) {
                    Text(state.currentMode.displayName)
                        .font(.headline)
                        .fontWeight(.semibold)

                    Text(state.currentMode.inboundFirewallPolicy)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                }

                Spacer()

                // Score Badge
                VStack(alignment: .trailing, spacing: 1) {
                    Text("\(state.postureScore.score)/100")
                        .font(.caption)
                        .fontWeight(.bold)
                    Text("Grade \(state.postureScore.grade)")
                        .font(.system(size: 9, weight: .bold))
                        .padding(.horizontal, 5)
                        .padding(.vertical, 1)
                        .background(Color(hex: state.currentMode.accentColorHex).opacity(0.2))
                        .foregroundColor(Color(hex: state.currentMode.accentColorHex))
                        .cornerRadius(4)
                }
            }

            Divider()

            // Sub-status indicators: SSID & VPN
            HStack(spacing: 12) {
                // Wi-Fi / SSID
                HStack(spacing: 4) {
                    Image(systemName: "wifi")
                        .font(.caption)
                    Text(state.connectedSSID ?? "No Wi-Fi")
                        .font(.caption2)
                        .lineLimit(1)
                }
                .foregroundColor(.secondary)

                Spacer()

                // VPN Status
                HStack(spacing: 4) {
                    Image(systemName: state.isVPNActive ? "lock.shield.fill" : "shield.slash")
                        .font(.caption)
                        .foregroundColor(state.isVPNActive ? .green : .secondary)
                    Text(state.isVPNActive ? "VPN Active" : "No VPN")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }

                // Helper Status Indicator
                Circle()
                    .fill(state.helperConnected ? Color.green : Color.orange)
                    .frame(width: 7, height: 7)
                    .help(state.helperConnected ? "Root Helper Connected" : "Helper Unregistered (Mock Mode)")
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }
}

// MARK: - Color Hex Initializer Extension
extension Color {
    init(hex: String) {
        let hex = hex.trimmingCharacters(in: CharacterSet.alphanumerics.inverted)
        var int: UInt64 = 0
        Scanner(string: hex).scanHexInt64(&int)
        let a, r, g, b: UInt64
        switch hex.count {
        case 3: // RGB (12-bit)
            (a, r, g, b) = (255, (int >> 8) * 17, (int >> 4 & 0xF) * 17, (int & 0xF) * 17)
        case 6: // RGB (24-bit)
            (a, r, g, b) = (255, int >> 16, int >> 8 & 0xFF, int & 0xFF)
        case 8: // ARGB (32-bit)
            (a, r, g, b) = (int >> 24, int >> 16 & 0xFF, int >> 8 & 0xFF, int & 0xFF)
        default:
            (a, r, g, b) = (1, 1, 1, 0)
        }
        self.init(
            .sRGB,
            red: Double(r) / 255,
            green: Double(g) / 255,
            blue:  Double(b) / 255,
            opacity: Double(a) / 255
        )
    }
}
