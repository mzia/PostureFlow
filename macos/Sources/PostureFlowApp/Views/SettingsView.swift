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

            HardwareAndTrapsTab(state: state)
                .tabItem {
                    Label("Hardware & Traps", systemImage: "lock.shield.fill")
                }
                .tag(4)
        }
        .frame(width: 640, height: 500)
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

// MARK: - Hardware & Traps Tab
struct HardwareAndTrapsTab: View {
    @ObservedObject var state: PostureStateStore

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                // Section 1: YubiKey Hardware Presence Tethering
                GroupBox(label: Label("YubiKey Presence Tethering (\"Zero-Trust Physical Token\")", systemImage: "key.fill")) {
                    VStack(alignment: .leading, spacing: 10) {
                        Toggle("Enable Continuous USB Hardware Tethering", isOn: $state.hardwareDefense.yubikey.enabled)
                            .font(.caption)
                            .fontWeight(.medium)

                        if state.hardwareDefense.yubikey.enabled {
                            HStack(spacing: 16) {
                                Toggle("Lock on Unplug", isOn: $state.hardwareDefense.yubikey.lockOnRemoval)
                                Toggle("Demote to Travel", isOn: $state.hardwareDefense.yubikey.demoteOnRemoval)
                                Toggle("Auto-Restore on Re-insert", isOn: $state.hardwareDefense.yubikey.restoreOnInsert)
                            }
                            .font(.caption2)
                            .foregroundColor(.secondary)
                            .padding(.leading, 12)

                            Divider()

                            HStack {
                                Text("Detected Hardware Keys:")
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                                Spacer()
                                Button("Rescan USB Bus") {
                                    state.refreshHardwareStatus()
                                }
                                .font(.caption2)
                            }

                            if state.connectedYubikeys.isEmpty {
                                Text("⚠️ No YubiKey detected on USB bus.")
                                    .font(.caption2)
                                    .foregroundColor(.orange)
                            } else {
                                ForEach(state.connectedYubikeys) { dev in
                                    HStack {
                                        Image(systemName: "checkmark.shield.fill")
                                            .foregroundColor(.green)
                                        Text("\(dev.name) (\(dev.vendorId):\(dev.productId))")
                                            .font(.caption)
                                            .fontWeight(.medium)
                                        Spacer()
                                        Text("ARMED & TETHERED")
                                            .font(.system(size: 9, weight: .bold))
                                            .padding(.horizontal, 6)
                                            .padding(.vertical, 2)
                                            .background(Color.green.opacity(0.2))
                                            .foregroundColor(.green)
                                            .cornerRadius(4)
                                    }
                                }
                            }
                        }
                    }
                    .padding(8)
                }

                // Section 2: Decoy Honeypot Port Traps
                GroupBox(label: Label("Honeypot Port Traps & Scan Defense", systemImage: "shield.righthalf.filled")) {
                    VStack(alignment: .leading, spacing: 10) {
                        Toggle("Arm Decoy Honeypot TCP Listeners", isOn: $state.hardwareDefense.honeypot.enabled)
                            .font(.caption)
                            .fontWeight(.medium)

                        if state.hardwareDefense.honeypot.enabled {
                            HStack {
                                Text("Trap Ports: 2222 (SSH), 8080 (Web Admin), 4450 (SMB)")
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                                Spacer()
                                Toggle("Auto-ban scanning IPs via pfctl", isOn: $state.hardwareDefense.honeypot.autoBlockOffenders)
                                    .font(.caption2)
                            }
                            .padding(.leading, 12)

                            Divider()

                            Text("Recent Caught Intrusions (\(state.recentHoneypotIncidents.count)):")
                                .font(.caption2)
                                .foregroundColor(.secondary)

                            if state.recentHoneypotIncidents.isEmpty {
                                Text("No unauthorized port scans detected yet. Perimeter secure.")
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                            } else {
                                ForEach(state.recentHoneypotIncidents.suffix(3)) { inc in
                                    HStack {
                                        Image(systemName: "exclamationmark.triangle.fill")
                                            .foregroundColor(.red)
                                        Text("\(inc.peerIP) -> Port \(inc.trapPort)")
                                            .font(.caption)
                                        Spacer()
                                        Text(inc.blocked ? "BLOCKED (pfctl)" : "LOGGED")
                                            .font(.system(size: 9, weight: .bold))
                                            .foregroundColor(inc.blocked ? .red : .orange)
                                    }
                                }
                            }
                        }
                    }
                    .padding(8)
                }

                // Section 3: Sensor Hardware Privacy Kill Switch
                GroupBox(label: Label("Camera, Microphone & Sensor Privacy", systemImage: "mic.fill")) {
                    VStack(alignment: .leading, spacing: 10) {
                        HStack {
                            VStack(alignment: .leading) {
                                Text("Microphone Input:")
                                    .font(.caption)
                                Text(state.sensorPrivacyReport.microphoneMuted ? "MUTED (0% input)" : "ACTIVE (\(state.sensorPrivacyReport.inputVolumePercent)%)")
                                    .font(.caption)
                                    .fontWeight(.bold)
                                    .foregroundColor(state.sensorPrivacyReport.microphoneMuted ? .green : .orange)
                            }

                            Spacer()

                            Button(state.sensorPrivacyReport.microphoneMuted ? "Unmute Mic" : "Mute Mic") {
                                state.toggleMicrophoneMute()
                            }
                            .font(.caption)

                            Button("🚨 Emergency Kill Switch") {
                                Task { await state.engageEmergencyKillSwitch() }
                            }
                            .font(.caption)
                            .foregroundColor(.white)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(Color.red)
                            .cornerRadius(6)

                            if state.sensorPrivacyReport.emergencyKillActive {
                                Button("Restore Sensors") {
                                    Task { await state.restoreSensors() }
                                }
                                .font(.caption)
                            }
                        }

                        HStack(spacing: 16) {
                            Toggle("Auto-mute mic on Travel profile", isOn: $state.hardwareDefense.sensors.muteMicOnTravel)
                            Toggle("Auto-mute mic on Session Lock", isOn: $state.hardwareDefense.sensors.muteMicOnLock)
                        }
                        .font(.caption2)
                        .foregroundColor(.secondary)
                    }
                    .padding(8)
                }

                // Section 4: Bluetooth Walk-Away Proximity Auto-Lock
                GroupBox(label: Label("Bluetooth Walk-Away Proximity Auto-Lock", systemImage: "wave.3.right.circle.fill")) {
                    VStack(alignment: .leading, spacing: 10) {
                        Toggle("Enable BLE Proximity Walk-Away Lockdown", isOn: $state.hardwareDefense.proximity.enabled)
                            .font(.caption)
                            .fontWeight(.medium)

                        if state.hardwareDefense.proximity.enabled {
                            HStack {
                                Text("Threshold: \(state.hardwareDefense.proximity.rssiThresholdDbm) dBm")
                                    .font(.caption2)
                                    .frame(width: 120, alignment: .leading)
                                Slider(
                                    value: Binding(
                                        get: { Double(state.hardwareDefense.proximity.rssiThresholdDbm) },
                                        set: { state.hardwareDefense.proximity.rssiThresholdDbm = Int($0) }
                                    ),
                                    in: -95...(-50),
                                    step: 5
                                )
                            }

                            HStack {
                                Text("Grace Period: \(state.hardwareDefense.proximity.gracePeriodSeconds)s")
                                    .font(.caption2)
                                    .frame(width: 120, alignment: .leading)
                                Slider(
                                    value: Binding(
                                        get: { Double(state.hardwareDefense.proximity.gracePeriodSeconds) },
                                        set: { state.hardwareDefense.proximity.gracePeriodSeconds = Int($0) }
                                    ),
                                    in: 5...30,
                                    step: 5
                                )
                            }
                        }
                    }
                    .padding(8)
                }
            }
            .padding(.trailing, 4)
        }
    }
}

