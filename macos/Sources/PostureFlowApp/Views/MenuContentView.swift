import SwiftUI
import PostureFlowShared

/// Dropdown content view for the PostureFlow MenuBarExtra popover.
/// Formatted to mirror macOS Control Center and System Settings design standards.
public struct MenuContentView: View {
    @ObservedObject var state: PostureStateStore
    var openSettings: () -> Void

    public init(state: PostureStateStore, openSettings: @escaping () -> Void) {
        self.state = state
        self.openSettings = openSettings
    }

    public var body: some View {
        VStack(spacing: 12) {
            // MARK: - 1. Control Center Posture Selector Grid (2x2)
            VStack(alignment: .leading, spacing: 6) {
                Text("POSTURES")
                    .font(.system(size: 10, weight: .bold))
                    .foregroundColor(.secondary)
                    .padding(.horizontal, 4)

                LazyVGrid(columns: [GridItem(.flexible(), spacing: 8), GridItem(.flexible(), spacing: 8)], spacing: 8) {
                    ForEach(PostureMode.allCases) { mode in
                        PostureGridCard(
                            mode: mode,
                            isSelected: state.currentMode == mode,
                            onSelect: {
                                Task<Void, Never> {
                                    await state.switchTo(mode)
                                }
                            }
                        )
                    }
                }
            }

            // MARK: - 2. Quick Automation & Defense Controls
            VStack(alignment: .leading, spacing: 6) {
                Text("AUTOMATION & HARDWARE DEFENSE")
                    .font(.system(size: 10, weight: .bold))
                    .foregroundColor(.secondary)
                    .padding(.horizontal, 4)

                VStack(spacing: 0) {
                    AppleSettingToggleRow(
                        title: "Auto-Flow",
                        subtitle: "Wi-Fi & VPN Aware",
                        icon: "network",
                        badgeColor: .purple,
                        isOn: $state.autoFlowEnabled
                    )

                    Divider().padding(.leading, 38)

                    AppleSettingToggleRow(
                        title: "App Triggers",
                        subtitle: "Process Overrides",
                        icon: "bolt.fill",
                        badgeColor: .blue,
                        isOn: $state.triggersEnabled
                    )

                    Divider().padding(.leading, 38)

                    AppleSettingToggleRow(
                        title: "Circadian Flow",
                        subtitle: "Day/Night Scheduling",
                        icon: "clock.arrow.2.circlepath",
                        badgeColor: .orange,
                        isOn: $state.circadianEnabled
                    )

                    Divider().padding(.leading, 38)

                    AppleSettingToggleRow(
                        title: "YubiKey Tether",
                        subtitle: state.connectedYubikeys.isEmpty ? "No Token Detected" : "\(state.connectedYubikeys.count) Armed Token",
                        icon: "key.fill",
                        badgeColor: .green,
                        isOn: $state.hardwareDefense.yubikey.enabled
                    )

                    Divider().padding(.leading, 38)

                    AppleSettingToggleRow(
                        title: "Decoy Honeypot",
                        subtitle: "Scan Trap Defense",
                        icon: "shield.righthalf.filled",
                        badgeColor: .red,
                        isOn: $state.hardwareDefense.honeypot.enabled
                    )
                }
                .padding(.vertical, 4)
                .background(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(Color(NSColor.controlBackgroundColor).opacity(0.8))
                        .overlay(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                        )
                )
            }

            // MARK: - 3. Sensor Privacy Emergency Action
            Button {
                withAnimation(.spring(response: 0.35, dampingFraction: 0.75)) {
                    Task {
                        if state.sensorPrivacyReport.emergencyKillActive {
                            await state.restoreSensors()
                        } else {
                            await state.engageEmergencyKillSwitch()
                        }
                    }
                }
            } label: {
                HStack(spacing: 8) {
                    Image(systemName: state.sensorPrivacyReport.emergencyKillActive ? "mic.slash.fill" : "exclamationmark.shield.fill")
                        .font(.system(size: 13, weight: .bold))
                        .foregroundColor(state.sensorPrivacyReport.emergencyKillActive ? .orange : .red)

                    Text(state.sensorPrivacyReport.emergencyKillActive ? "Restore Microphone & Sensors" : "Emergency Sensor Kill Switch")
                        .font(.system(size: 11, weight: .bold))
                        .foregroundColor(state.sensorPrivacyReport.emergencyKillActive ? .orange : .red)

                    Spacer()

                    Image(systemName: state.sensorPrivacyReport.emergencyKillActive ? "arrow.uturn.backward" : "lock.fill")
                        .font(.system(size: 10, weight: .bold))
                        .foregroundColor(state.sensorPrivacyReport.emergencyKillActive ? .orange : .red)
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .fill(state.sensorPrivacyReport.emergencyKillActive ? Color.orange.opacity(0.12) : Color.red.opacity(0.1))
                        .overlay(
                            RoundedRectangle(cornerRadius: 10, style: .continuous)
                                .stroke((state.sensorPrivacyReport.emergencyKillActive ? Color.orange : Color.red).opacity(0.3), lineWidth: 1)
                        )
                )
            }
            .buttonStyle(.plain)

            Divider()

            // MARK: - 4. macOS Standard Window Footer
            HStack {
                Button {
                    openSettings()
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "gearshape.fill")
                            .font(.system(size: 12))
                        Text("Security Cockpit...")
                            .font(.system(size: 12, weight: .medium))
                    }
                    .foregroundColor(.primary)
                }
                .buttonStyle(.plain)

                Spacer()

                Text("⌘,")
                    .font(.system(size: 11, weight: .semibold, design: .rounded))
                    .foregroundColor(.secondary)
                    .padding(.horizontal, 5)
                    .padding(.vertical, 2)
                    .background(Color(NSColor.controlBackgroundColor))
                    .cornerRadius(4)

                Divider().frame(height: 12)

                Button {
                    Task {
                        _ = try? await XPCClient.shared.restoreDefaults()
                        NSApplication.shared.terminate(nil)
                    }
                } label: {
                    Text("Quit")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundColor(.secondary)
                }
                .buttonStyle(.plain)
                .keyboardShortcut("q", modifiers: .command)

                Text("⌘Q")
                    .font(.system(size: 11, weight: .semibold, design: .rounded))
                    .foregroundColor(.secondary)
                    .padding(.horizontal, 5)
                    .padding(.vertical, 2)
                    .background(Color(NSColor.controlBackgroundColor))
                    .cornerRadius(4)
            }
            .padding(.horizontal, 4)
            .padding(.top, 2)
        }
        .padding(12)
        .frame(width: 330)
    }
}

// MARK: - Posture Grid Card (Control Center Style)
private struct PostureGridCard: View {
    let mode: PostureMode
    let isSelected: Bool
    let onSelect: () -> Void

    var body: some View {
        Button(action: onSelect) {
            HStack(spacing: 9) {
                // Circular icon badge
                ZStack {
                    Circle()
                        .fill(
                            isSelected ?
                                Color(hex: mode.accentColorHex) :
                                Color(NSColor.controlBackgroundColor)
                        )
                        .frame(width: 28, height: 28)

                    Image(systemName: mode.sfSymbol)
                        .font(.system(size: 13, weight: .bold))
                        .foregroundColor(isSelected ? .white : Color(hex: mode.accentColorHex))
                }

                VStack(alignment: .leading, spacing: 1) {
                    Text(mode.displayName)
                        .font(.system(size: 12, weight: isSelected ? .bold : .medium))
                        .foregroundColor(isSelected ? .primary : .secondary)

                    Text(modeShortContext(mode))
                        .font(.system(size: 9.5))
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                }

                Spacer(minLength: 0)

                if isSelected {
                    Image(systemName: "checkmark")
                        .font(.system(size: 10, weight: .bold))
                        .foregroundColor(Color(hex: mode.accentColorHex))
                }
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 8)
            .background(
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .fill(
                        isSelected ?
                            Color(hex: mode.accentColorHex).opacity(0.14) :
                            Color(NSColor.controlBackgroundColor).opacity(0.6)
                    )
                    .overlay(
                        RoundedRectangle(cornerRadius: 10, style: .continuous)
                            .stroke(
                                isSelected ?
                                    Color(hex: mode.accentColorHex).opacity(0.4) :
                                    Color.primary.opacity(0.04),
                                lineWidth: 1
                            )
                    )
            )
        }
        .buttonStyle(.plain)
    }

    private func modeShortContext(_ mode: PostureMode) -> String {
        switch mode {
        case .home: return "Gaming & LAN"
        case .work: return "Office & VPN"
        case .dev: return "Coding & Ports"
        case .travel: return "Stealth Mode"
        }
    }
}

// MARK: - Apple Settings-style Toggle Row
private struct AppleSettingToggleRow: View {
    let title: String
    let subtitle: String
    let icon: String
    let badgeColor: Color
    @Binding var isOn: Bool

    var body: some View {
        HStack(spacing: 10) {
            // Iconic Rounded Square Badge
            ZStack {
                RoundedRectangle(cornerRadius: 6, style: .continuous)
                    .fill(badgeColor.gradient)
                    .frame(width: 22, height: 22)

                Image(systemName: icon)
                    .font(.system(size: 11, weight: .bold))
                    .foregroundColor(.white)
            }

            VStack(alignment: .leading, spacing: 1) {
                Text(title)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundColor(.primary)

                Text(subtitle)
                    .font(.system(size: 9.5))
                    .foregroundColor(.secondary)
            }

            Spacer()

            Toggle("", isOn: $isOn)
                .toggleStyle(.switch)
                .controlSize(.small)
                .labelsHidden()
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
    }
}
