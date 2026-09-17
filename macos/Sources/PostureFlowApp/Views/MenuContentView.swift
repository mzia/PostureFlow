import SwiftUI
import PostureFlowShared

/// Dropdown menu list items for the PostureFlow MenuBarExtra.
public struct MenuContentView: View {
    @ObservedObject var state: PostureStateStore
    var openSettings: () -> Void

    public init(state: PostureStateStore, openSettings: @escaping () -> Void) {
        self.state = state
        self.openSettings = openSettings
    }

    public var body: some View {
        VStack {
            // Posture Profile Switcher
            ForEach(PostureMode.allCases) { mode in
                Button {
                    Task { await state.switchTo(mode) }
                } label: {
                    HStack {
                        if state.currentMode == mode {
                            Image(systemName: "checkmark")
                        } else {
                            Spacer().frame(width: 14)
                        }
                        Label(mode.displayName, systemImage: mode.sfSymbol)
                    }
                }
            }

            Divider()

            // Feature Automation Toggles
            Toggle(isOn: $state.autoFlowEnabled) {
                Label("Auto-Flow (Network Aware)", systemImage: "network")
            }

            Toggle(isOn: $state.triggersEnabled) {
                Label("App-Aware Triggers", systemImage: "bolt.fill")
            }

            Toggle(isOn: $state.circadianEnabled) {
                Label("Circadian Schedule", systemImage: "clock.arrow.2.circlepath")
            }

            Toggle(isOn: $state.hardwareDefense.yubikey.enabled) {
                Label("YubiKey Physical Tether", systemImage: "key.fill")
            }

            Toggle(isOn: $state.hardwareDefense.honeypot.enabled) {
                Label("Honeypot Decoy Traps", systemImage: "shield.righthalf.filled")
            }

            Button {
                Task {
                    if state.sensorPrivacyReport.emergencyKillActive {
                        await state.restoreSensors()
                    } else {
                        await state.engageEmergencyKillSwitch()
                    }
                }
            } label: {
                Label(
                    state.sensorPrivacyReport.emergencyKillActive ? "Restore Hardware Sensors" : "🚨 Emergency Sensor Kill Switch",
                    systemImage: state.sensorPrivacyReport.emergencyKillActive ? "mic.slash.fill" : "exclamationmark.shield.fill"
                )
            }

            Divider()

            // Cockpit & Actions
            Button {
                openSettings()
            } label: {
                Label("Security Cockpit & Settings...", systemImage: "gearshape")
            }
            .keyboardShortcut(",", modifiers: .command)

            Button {
                Task {
                    _ = try? await XPCClient.shared.restoreDefaults()
                    NSApplication.shared.terminate(nil)
                }
            } label: {
                Label("Quit PostureFlow", systemImage: "power")
            }
            .keyboardShortcut("q", modifiers: .command)
        }
    }
}
