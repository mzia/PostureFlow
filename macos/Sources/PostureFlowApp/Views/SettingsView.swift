import SwiftUI
import ServiceManagement
import PostureFlowShared

/// Full Security Cockpit & Preferences window for macOS.
public struct SettingsView: View {
    @ObservedObject var state: PostureStateStore
    @State private var selectedTab: Int = 0

    public init(state: PostureStateStore) {
        self.state = state
    }

    public var body: some View {
        TabView(selection: $selectedTab) {
            GeneralTab(state: state)
                .tabItem {
                    Label("General", systemImage: "slider.horizontal.3")
                }
                .tag(0)

            AutoFlowTab(state: state)
                .tabItem {
                    Label("Auto-Flow", systemImage: "network")
                }
                .tag(1)

            AppTriggersTab(state: state)
                .tabItem {
                    Label("App Triggers", systemImage: "bolt.fill")
                }
                .tag(2)

            SecurityAndPowerTab(state: state)
                .tabItem {
                    Label("Security & Power", systemImage: "shield.checkered")
                }
                .tag(3)
        }
        .frame(width: 580, height: 440)
        .padding(16)
    }
}

// MARK: - General Tab
struct GeneralTab: View {
    @ObservedObject var state: PostureStateStore

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            GroupBox(label: Label("Current Posture", systemImage: state.currentMode.sfSymbol)) {
                VStack(alignment: .leading, spacing: 8) {
                    Picker("Active Posture:", selection: Binding(
                        get: { state.currentMode },
                        set: { newMode in
                            Task { await state.switchTo(newMode) }
                        }
                    )) {
                        ForEach(PostureMode.allCases) { mode in
                            Text(mode.displayName).tag(mode)
                        }
                    }
                    .pickerStyle(.segmented)

                    Text(state.currentMode.postureDescription)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .padding(.top, 4)
                }
                .padding(8)
            }

            GroupBox(label: Label("Posture Score Assessment", systemImage: "gauge.with.needle")) {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("Overall Score:")
                            .fontWeight(.medium)
                        Text("\(state.postureScore.score) / 100 (\(state.postureScore.grade))")
                            .fontWeight(.bold)
                            .foregroundColor(state.postureScore.score >= 80 ? .green : .orange)
                        Spacer()
                    }

                    ForEach(Array(state.postureScore.breakdown.keys.sorted()), id: \.self) { key in
                        HStack {
                            Text("• \(key)")
                                .font(.caption)
                            Spacer()
                            Text("+\(state.postureScore.breakdown[key] ?? 0)")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }

                    if !state.postureScore.recommendations.isEmpty {
                        Divider()
                        Text("Recommendations:")
                            .font(.caption)
                            .fontWeight(.semibold)
                        ForEach(state.postureScore.recommendations, id: \.self) { rec in
                            Text("⚠️ \(rec)")
                                .font(.caption2)
                                .foregroundColor(.orange)
                        }
                    }
                }
                .padding(8)
            }

            Spacer()
        }
    }
}

// MARK: - Auto-Flow Tab
struct AutoFlowTab: View {
    @ObservedObject var state: PostureStateStore

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Toggle("Enable Autonomous Network Switching (Auto-Flow)", isOn: $state.autoFlowEnabled)
                .font(.headline)

            Text("Automatically switches posture profiles based on the connected Wi-Fi SSID or VPN tunnel.")
                .font(.caption)
                .foregroundColor(.secondary)

            GroupBox(label: Label("Network Mappings", systemImage: "wifi")) {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("Current Wi-Fi SSID:")
                            .font(.caption)
                        Text(state.connectedSSID ?? "Disconnected")
                            .font(.caption)
                            .fontWeight(.semibold)
                        Spacer()
                        Text("VPN:")
                            .font(.caption)
                        Text(state.isVPNActive ? "Connected" : "Inactive")
                            .font(.caption)
                            .fontWeight(.semibold)
                            .foregroundColor(state.isVPNActive ? .green : .secondary)
                    }
                    Divider()
                    Text("Configured Networks:")
                        .font(.caption)
                        .fontWeight(.medium)
                    Text("• Home Wi-Fi → Home Posture")
                        .font(.caption)
                    Text("• Corporate Office Wi-Fi → Work Posture")
                        .font(.caption)
                    Text("• Public / Airport Wi-Fi → Travel Posture")
                        .font(.caption)
                }
                .padding(8)
            }

            Spacer()
        }
    }
}

// MARK: - App Triggers Tab
struct AppTriggersTab: View {
    @ObservedObject var state: PostureStateStore

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Toggle("Enable Application Lifecycle Triggers", isOn: $state.triggersEnabled)
                .font(.headline)

            Text("Monitors running apps to elevate security and optimize power. For example, video calls trigger Work mode, while developer tools trigger Dev mode.")
                .font(.caption)
                .foregroundColor(.secondary)

            GroupBox(label: Label("Registered App Triggers", systemImage: "app.badge.checkmark")) {
                ScrollView {
                    VStack(alignment: .leading, spacing: 6) {
                        TriggerRow(icon: "video.fill", name: "Zoom / Teams / Slack", bundle: "us.zoom.xos, com.microsoft.teams2", posture: "Work")
                        TriggerRow(icon: "hammer.fill", name: "Xcode / VS Code / IntelliJ", bundle: "com.apple.dt.Xcode, com.microsoft.VSCode", posture: "Dev")
                        TriggerRow(icon: "gamecontroller.fill", name: "Steam / Epic Games", bundle: "com.valvesoftware.steam", posture: "Home")
                    }
                    .padding(8)
                }
            }

            Spacer()
        }
    }
}

struct TriggerRow: View {
    let icon: String
    let name: String
    let bundle: String
    let posture: String

    var body: some View {
        HStack {
            Image(systemName: icon)
                .frame(width: 20)
                .foregroundColor(.accentColor)
            VStack(alignment: .leading) {
                Text(name).font(.caption).fontWeight(.medium)
                Text(bundle).font(.system(size: 9)).foregroundColor(.secondary)
            }
            Spacer()
            Text(posture)
                .font(.caption2)
                .fontWeight(.bold)
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(Color.secondary.opacity(0.2))
                .cornerRadius(4)
        }
    }
}

// MARK: - Security & Power Tab
struct SecurityAndPowerTab: View {
    @ObservedObject var state: PostureStateStore

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            GroupBox(label: Label("Privileged Helper Tool", systemImage: "wrench.and.screwdriver")) {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("Helper Status:")
                            .font(.caption)
                        Text(state.helperConnected ? "Active & Registered" : "Not Registered")
                            .font(.caption)
                            .fontWeight(.bold)
                            .foregroundColor(state.helperConnected ? .green : .orange)
                        Spacer()
                        Button(state.helperConnected ? "Re-register Helper" : "Register Helper Tool") {
                            registerPrivilegedHelper()
                        }
                        .font(.caption)
                    }
                    Text("The helper daemon runs as root to manage /etc/pf.anchors/com.postureflow and pmset without entering passwords repeatedly.")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
                .padding(8)
            }

            GroupBox(label: Label("Hardware & Power Profiles", systemImage: "battery.100.bolt")) {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("Low Power Mode:")
                            .font(.caption)
                        Text(state.currentMode.enablesLowPowerMode ? "Enabled (Travel)" : "Disabled (Work/Dev/Home)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    HStack {
                        Text("Display Sleep Timeout:")
                            .font(.caption)
                        Text("\(state.currentMode.defaultDisplaySleepMinutes) minutes")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                .padding(8)
            }

            Spacer()
        }
    }

    private func registerPrivilegedHelper() {
        if #available(macOS 13.0, *) {
            let service = SMAppService.daemon(plistName: "com.postureflow.helper.plist")
            do {
                try service.register()
                Task { await state.syncWithDaemon() }
            } catch {
                NSLog("[SettingsView] Failed to register daemon: %@", error.localizedDescription)
            }
        }
    }
}
