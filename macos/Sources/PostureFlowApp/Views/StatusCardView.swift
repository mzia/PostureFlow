import SwiftUI
import PostureFlowShared

/// Compact status header rendered at the top of the PostureFlow macOS MenuBarExtra popover.
/// Adheres strictly to Apple Human Interface Guidelines for Control Center and Menu Bar accessories.
public struct StatusCardView: View {
    @ObservedObject var state: PostureStateStore

    public init(state: PostureStateStore) {
        self.state = state
    }

    public var body: some View {
        VStack(spacing: 12) {
            // Hero Status Row
            HStack(alignment: .center, spacing: 12) {
                // Vibrant App Icon Badge with Soft Glow
                ZStack {
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .fill(
                            LinearGradient(
                                colors: [
                                    Color(hex: state.currentMode.accentColorHex),
                                    Color(hex: state.currentMode.accentColorHex).opacity(0.8)
                                ],
                                startPoint: .topLeading,
                                endPoint: .bottomTrailing
                            )
                        )
                        .frame(width: 38, height: 38)
                        .shadow(color: Color(hex: state.currentMode.accentColorHex).opacity(0.35), radius: 6, x: 0, y: 2)

                    Image(systemName: state.currentMode.shieldSymbol)
                        .font(.system(size: 18, weight: .bold))
                        .foregroundColor(.white)
                }

                // Posture Name & Inbound Policy
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: 6) {
                        Text(state.currentMode.displayName)
                            .font(.system(.headline, design: .rounded))
                            .fontWeight(.bold)
                            .foregroundColor(.primary)

                        if state.currentMode == .travel {
                            Text("STEALTH")
                                .font(.system(size: 9, weight: .heavy, design: .rounded))
                                .padding(.horizontal, 5)
                                .padding(.vertical, 1.5)
                                .background(Color.red.opacity(0.18))
                                .foregroundColor(.red)
                                .clipShape(Capsule())
                        }
                    }

                    Text(state.currentMode.inboundFirewallPolicy)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                }

                Spacer()

                // Apple-style Score Capsule
                ScoreBadge(score: state.postureScore.score, grade: state.postureScore.grade, accentHex: state.currentMode.accentColorHex)
            }

            // Critical Rogue AP / Evil Twin Alert
            if state.isEvilTwinDetected {
                HStack(spacing: 6) {
                    Image(systemName: "exclamationmark.shield.fill")
                        .font(.system(size: 13, weight: .bold))
                        .foregroundColor(.red)
                    Text("ROGUE AP DETECTED: Lockdown Active")
                        .font(.system(size: 10.5, weight: .heavy, design: .rounded))
                        .foregroundColor(.red)
                        .lineLimit(1)
                    Spacer()
                }
                .padding(.horizontal, 8)
                .padding(.vertical, 5)
                .background(Color.red.opacity(0.12))
                .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .stroke(Color.red.opacity(0.3), lineWidth: 1)
                )
            }

            // Quick Telemetry Ribbon (Wi-Fi, VPN, and Root Helper)
            HStack(spacing: 8) {
                // Wi-Fi Chip
                HStack(spacing: 5) {
                    Image(systemName: state.connectedSSID != nil ? "wifi" : "wifi.slash")
                        .font(.system(size: 11, weight: .medium))
                    Text(state.connectedSSID ?? "Disconnected")
                        .font(.system(size: 11, weight: .medium))
                        .lineLimit(1)
                }
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(Color(NSColor.controlBackgroundColor).opacity(0.6))
                .clipShape(Capsule())
                .foregroundColor(.secondary)

                Spacer()

                // VPN Indicator Chip
                HStack(spacing: 5) {
                    Image(systemName: state.isVPNActive ? "lock.shield.fill" : "shield.slash")
                        .font(.system(size: 11, weight: .medium))
                        .foregroundColor(state.isVPNActive ? .green : .secondary)
                    Text(state.isVPNActive ? "VPN Active" : "No VPN")
                        .font(.system(size: 11, weight: .medium))
                        .foregroundColor(state.isVPNActive ? .primary : .secondary)
                }
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(
                    (state.isVPNActive ? Color.green.opacity(0.12) : Color(NSColor.controlBackgroundColor).opacity(0.6))
                )
                .clipShape(Capsule())

                // Privileged Daemon Status Dot
                Circle()
                    .fill(state.helperConnected ? Color.green : Color.orange)
                    .frame(width: 8, height: 8)
                    .overlay(
                        Circle()
                            .stroke((state.helperConnected ? Color.green : Color.orange).opacity(0.4), lineWidth: 2)
                            .scaleEffect(state.helperConnected ? 1.4 : 1.0)
                    )
                    .help(state.helperConnected ? "Root PacketFilter Helper Active" : "Helper Unregistered (Mock Mode)")
            }
        }
        .padding(14)
        .background(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .fill(Color(NSColor.windowBackgroundColor).opacity(0.75))
                .overlay(
                    RoundedRectangle(cornerRadius: 14, style: .continuous)
                        .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                )
        )
    }
}

// MARK: - Score Badge Subview
private struct ScoreBadge: View {
    let score: Int
    let grade: String
    let accentHex: String

    var gradeColor: Color {
        switch grade {
        case "A+": return Color.green
        case "A":  return Color(hex: "#10B981")
        case "B":  return Color.blue
        case "C":  return Color.orange
        default:   return Color.red
        }
    }

    var body: some View {
        HStack(spacing: 6) {
            Text(grade)
                .font(.system(size: 11, weight: .heavy, design: .rounded))
                .foregroundColor(.white)
                .padding(.horizontal, 6)
                .padding(.vertical, 2.5)
                .background(gradeColor)
                .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))

            Text("\(score)%")
                .font(.system(size: 13, weight: .bold, design: .rounded))
                .foregroundColor(.primary)
        }
        .padding(4)
        .padding(.trailing, 4)
        .background(Color(NSColor.controlBackgroundColor).opacity(0.8))
        .clipShape(Capsule())
        .overlay(
            Capsule()
                .stroke(gradeColor.opacity(0.25), lineWidth: 1)
        )
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
