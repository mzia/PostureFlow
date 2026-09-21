import SwiftUI
import ServiceManagement
import PostureFlowShared

/// Navigation sections for the macOS System Settings sidebar layout.
public enum SettingsSection: String, CaseIterable, Identifiable {
    case overview = "Overview & Score"
    case autoFlow = "Auto-Flow (Network)"
    case appTriggers = "App Triggers"
    case firewall = "Security & Firewall"
    case power = "Hardware & Power"
    case hardwareDefense = "Zero-Trust Defense"

    public var id: String { rawValue }

    var icon: String {
        switch self {
        case .overview: return "gauge.with.needle"
        case .autoFlow: return "network"
        case .appTriggers: return "bolt.fill"
        case .firewall: return "shield.checkered"
        case .power: return "battery.100.bolt"
        case .hardwareDefense: return "lock.shield.fill"
        }
    }

    var badgeColor: Color {
        switch self {
        case .overview: return .blue
        case .autoFlow: return .purple
        case .appTriggers: return .orange
        case .firewall: return .green
        case .power: return .yellow
        case .hardwareDefense: return .red
        }
    }
}

/// Full Security Cockpit & Preferences window designed following modern macOS System Settings standards.
public struct SettingsView: View {
    @ObservedObject var state: PostureStateStore
    @State private var selectedSection: SettingsSection? = .overview

    public init(state: PostureStateStore) {
        self.state = state
    }

    public var body: some View {
        NavigationSplitView {
            // MARK: - Modern macOS Sidebar
            List(SettingsSection.allCases, selection: $selectedSection) { section in
                NavigationLink(value: section) {
                    HStack(spacing: 10) {
                        ZStack {
                            RoundedRectangle(cornerRadius: 6, style: .continuous)
                                .fill(section.badgeColor.gradient)
                                .frame(width: 22, height: 22)

                            Image(systemName: section.icon)
                                .font(.system(size: 11, weight: .bold))
                                .foregroundColor(.white)
                        }

                        Text(section.rawValue)
                            .font(.system(size: 13, weight: .medium))
                    }
                    .padding(.vertical, 2)
                }
            }
            .navigationSplitViewColumnWidth(min: 190, ideal: 220, max: 260)
            .listStyle(.sidebar)
        } detail: {
            // MARK: - Detail Pane
            Group {
                switch selectedSection ?? .overview {
                case .overview:
                    OverviewDetailView(state: state)
                case .autoFlow:
                    AutoFlowDetailView(state: state)
                case .appTriggers:
                    AppTriggersDetailView(state: state)
                case .firewall:
                    FirewallDetailView(state: state)
                case .power:
                    PowerDetailView(state: state)
                case .hardwareDefense:
                    HardwareDefenseDetailView(state: state)
                }
            }
            .navigationTitle(selectedSection?.rawValue ?? "PostureFlow")
        }
        .frame(minWidth: 740, minHeight: 520)
    }
}

// MARK: - 1. Overview & Score Detail View
private struct OverviewDetailView: View {
    @ObservedObject var state: PostureStateStore

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                // Hero Posture Banner
                HStack(spacing: 14) {
                    ZStack {
                        Circle()
                            .fill(
                                LinearGradient(
                                    colors: [
                                        Color(hex: state.currentMode.accentColorHex),
                                        Color(hex: state.currentMode.accentColorHex).opacity(0.75)
                                    ],
                                    startPoint: .topLeading,
                                    endPoint: .bottomTrailing
                                )
                            )
                            .frame(width: 48, height: 48)
                            .shadow(color: Color(hex: state.currentMode.accentColorHex).opacity(0.3), radius: 8, x: 0, y: 3)

                        Image(systemName: state.currentMode.sfSymbol)
                            .font(.system(size: 22, weight: .bold))
                            .foregroundColor(.white)
                    }

                    VStack(alignment: .leading, spacing: 3) {
                        HStack {
                            Text(state.currentMode.displayName)
                                .font(.system(.title2, design: .rounded))
                                .fontWeight(.bold)

                            Text("ACTIVE POSTURE")
                                .font(.system(size: 9, weight: .heavy, design: .rounded))
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(Color(hex: state.currentMode.accentColorHex).opacity(0.18))
                                .foregroundColor(Color(hex: state.currentMode.accentColorHex))
                                .clipShape(Capsule())
                        }

                        Text(state.currentMode.postureDescription)
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                }
                .padding(14)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(Color(NSColor.controlBackgroundColor))
                        .overlay(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                        )
                )

                // Segmented Posture Switcher
                VStack(alignment: .leading, spacing: 8) {
                    Text("MANUAL POSTURE SELECTION")
                        .font(.system(size: 11, weight: .bold))
                        .foregroundColor(.secondary)

                    Picker("Active Posture", selection: Binding(
                        get: { state.currentMode },
                        set: { newMode in
                            Task<Void, Never> {
                                await state.switchTo(newMode)
                            }
                        }
                    )) {
                        ForEach(PostureMode.allCases) { mode in
                            Label(mode.displayName, systemImage: mode.sfSymbol).tag(mode)
                        }
                    }
                    .pickerStyle(.segmented)
                }

                // 100-Point Security Score Card
                VStack(alignment: .leading, spacing: 12) {
                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("SECURITY POSTURE SCORE")
                                .font(.system(size: 11, weight: .bold))
                                .foregroundColor(.secondary)

                            Text("Live posture health & defense compliance audit")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }

                        Spacer()

                        HStack(spacing: 8) {
                            Text(state.postureScore.grade)
                                .font(.system(size: 14, weight: .heavy, design: .rounded))
                                .foregroundColor(.white)
                                .padding(.horizontal, 8)
                                .padding(.vertical, 3)
                                .background(state.postureScore.score >= 80 ? Color.green : Color.orange)
                                .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))

                            Text("\(state.postureScore.score) / 100")
                                .font(.system(.title3, design: .rounded))
                                .fontWeight(.bold)
                        }
                    }

                    // Score Progress Bar
                    GeometryReader { geo in
                        ZStack(alignment: .leading) {
                            Capsule()
                                .fill(Color.secondary.opacity(0.15))
                                .frame(height: 8)

                            Capsule()
                                .fill(
                                    LinearGradient(
                                        colors: [Color.green, Color(hex: "#10B981")],
                                        startPoint: .leading,
                                        endPoint: .trailing
                                    )
                                )
                                .frame(width: max(0, min(geo.size.width, geo.size.width * CGFloat(state.postureScore.score) / 100)), height: 8)
                        }
                    }
                    .frame(height: 8)

                    Divider()

                    // Audit Point Breakdown
                    Text("Audit Points Earned:")
                        .font(.system(size: 12, weight: .semibold))

                    LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 8) {
                        ForEach(Array(state.postureScore.breakdown.keys.sorted()), id: \.self) { key in
                            HStack(spacing: 6) {
                                Image(systemName: "checkmark.circle.fill")
                                    .foregroundColor(.green)
                                    .font(.system(size: 12))

                                Text(key)
                                    .font(.system(size: 11.5))
                                    .lineLimit(1)

                                Spacer()

                                Text("+\(state.postureScore.breakdown[key] ?? 0)")
                                    .font(.system(size: 11.5, weight: .bold))
                                    .foregroundColor(.secondary)
                            }
                            .padding(7)
                            .background(Color(NSColor.controlBackgroundColor).opacity(0.5))
                            .cornerRadius(6)
                        }
                    }

                    if !state.postureScore.recommendations.isEmpty {
                        Divider()
                        VStack(alignment: .leading, spacing: 6) {
                            Label("Recommendations to Improve Score:", systemImage: "lightbulb.fill")
                                .font(.system(size: 11.5, weight: .bold))
                                .foregroundColor(.orange)

                            ForEach(state.postureScore.recommendations, id: \.self) { rec in
                                HStack(alignment: .top, spacing: 6) {
                                    Text("•")
                                        .foregroundColor(.orange)
                                    Text(rec)
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                }
                            }
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.orange.opacity(0.08))
                        .cornerRadius(8)
                    }
                }
                .padding(14)
                .background(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(Color(NSColor.controlBackgroundColor))
                        .overlay(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                        )
                )
            }
            .padding(20)
        }
    }
}

// MARK: - 2. Auto-Flow Detail View
private struct AutoFlowDetailView: View {
    @ObservedObject var state: PostureStateStore

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                // Header Toggle Card
                VStack(alignment: .leading, spacing: 8) {
                    Toggle("Enable Autonomous Network Switching (Auto-Flow)", isOn: $state.autoFlowEnabled)
                        .font(.system(.headline, design: .rounded))
                        .toggleStyle(.switch)

                    Text("Watches Wi-Fi SSID transitions and VPN tunnel connections to automatically switch postures.")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }
                .padding(14)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(Color(NSColor.controlBackgroundColor))
                        .overlay(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                        )
                )

                // Current Telemetry
                VStack(alignment: .leading, spacing: 12) {
                    Text("LIVE NETWORK STATE")
                        .font(.system(size: 11, weight: .bold))
                        .foregroundColor(.secondary)

                    HStack(spacing: 20) {
                        HStack(spacing: 8) {
                            Image(systemName: "wifi")
                                .font(.system(size: 16))
                                .foregroundColor(.accentColor)
                            VStack(alignment: .leading) {
                                Text("Wi-Fi SSID")
                                    .font(.caption2).foregroundColor(.secondary)
                                Text(state.connectedSSID ?? "Disconnected")
                                    .font(.system(size: 13, weight: .semibold))
                            }
                        }

                        Divider().frame(height: 28)

                        HStack(spacing: 8) {
                            Image(systemName: state.isVPNActive ? "lock.shield.fill" : "shield.slash")
                                .font(.system(size: 16))
                                .foregroundColor(state.isVPNActive ? .green : .secondary)
                            VStack(alignment: .leading) {
                                Text("VPN Tunnel (utun*)")
                                    .font(.caption2).foregroundColor(.secondary)
                                Text(state.isVPNActive ? "Encrypted / Connected" : "Inactive")
                                    .font(.system(size: 13, weight: .semibold))
                            }
                        }
                    }
                    .padding(12)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color(NSColor.windowBackgroundColor).opacity(0.5))
                    .cornerRadius(8)
                }
                .padding(14)
                .background(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(Color(NSColor.controlBackgroundColor))
                        .overlay(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                        )
                )

                // Known Network Mappings
                VStack(alignment: .leading, spacing: 10) {
                    Text("AUTOMATIC NETWORK MAPPINGS")
                        .font(.system(size: 11, weight: .bold))
                        .foregroundColor(.secondary)

                    VStack(spacing: 8) {
                        NetworkMappingRow(icon: "house.fill", networkName: "Home Wi-Fi (Recognized LAN)", postureName: "Home", colorHex: "#10B981")
                        Divider()
                        NetworkMappingRow(icon: "briefcase.fill", networkName: "Corporate Office Wi-Fi / VPN", postureName: "Work", colorHex: "#3B82F6")
                        Divider()
                        NetworkMappingRow(icon: "airplane", networkName: "Public Hotspots / Airports", postureName: "Travel", colorHex: "#EF4444")
                    }
                    .padding(12)
                    .background(Color(NSColor.windowBackgroundColor).opacity(0.5))
                    .cornerRadius(8)
                }
                .padding(14)
                .background(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(Color(NSColor.controlBackgroundColor))
                        .overlay(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                        )
                )
            }
            .padding(20)
        }
    }
}

private struct NetworkMappingRow: View {
    let icon: String
    let networkName: String
    let postureName: String
    let colorHex: String

    var body: some View {
        HStack {
            Image(systemName: icon)
                .font(.system(size: 14))
                .foregroundColor(Color(hex: colorHex))
                .frame(width: 24)

            Text(networkName)
                .font(.system(size: 13, weight: .medium))

            Spacer()

            Text(postureName)
                .font(.system(size: 11, weight: .bold, design: .rounded))
                .foregroundColor(Color(hex: colorHex))
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
                .background(Color(hex: colorHex).opacity(0.15))
                .clipShape(Capsule())
        }
    }
}

// MARK: - 3. App Triggers Detail View
private struct AppTriggersDetailView: View {
    @ObservedObject var state: PostureStateStore

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                VStack(alignment: .leading, spacing: 8) {
                    Toggle("Enable Application Lifecycle Triggers", isOn: $state.triggersEnabled)
                        .font(.system(.headline, design: .rounded))
                        .toggleStyle(.switch)

                    Text("Detects running processes in real time and automatically applies target postures while active, cleanly reverting when the app terminates.")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }
                .padding(14)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(Color(NSColor.controlBackgroundColor))
                        .overlay(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                        )
                )

                VStack(alignment: .leading, spacing: 10) {
                    Text("REGISTERED TRIGGER RULES")
                        .font(.system(size: 11, weight: .bold))
                        .foregroundColor(.secondary)

                    VStack(spacing: 8) {
                        AppTriggerCard(
                            icon: "video.fill",
                            title: "Zoom / Microsoft Teams / Slack",
                            bundleId: "us.zoom.xos, com.microsoft.teams2",
                            targetPosture: "Work",
                            colorHex: "#3B82F6"
                        )
                        Divider()
                        AppTriggerCard(
                            icon: "hammer.fill",
                            title: "Xcode / Visual Studio Code / IntelliJ",
                            bundleId: "com.apple.dt.Xcode, com.microsoft.VSCode",
                            targetPosture: "Development",
                            colorHex: "#F59E0B"
                        )
                        Divider()
                        AppTriggerCard(
                            icon: "gamecontroller.fill",
                            title: "Steam / Epic Games Launcher",
                            bundleId: "com.valvesoftware.steam",
                            targetPosture: "Home",
                            colorHex: "#10B981"
                        )
                    }
                    .padding(12)
                    .background(Color(NSColor.windowBackgroundColor).opacity(0.5))
                    .cornerRadius(8)
                }
                .padding(14)
                .background(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(Color(NSColor.controlBackgroundColor))
                        .overlay(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                        )
                )
            }
            .padding(20)
        }
    }
}

private struct AppTriggerCard: View {
    let icon: String
    let title: String
    let bundleId: String
    let targetPosture: String
    let colorHex: String

    var body: some View {
        HStack(spacing: 12) {
            ZStack {
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .fill(Color(hex: colorHex).opacity(0.15))
                    .frame(width: 32, height: 32)

                Image(systemName: icon)
                    .font(.system(size: 14, weight: .bold))
                    .foregroundColor(Color(hex: colorHex))
            }

            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.system(size: 13, weight: .medium))
                Text(bundleId)
                    .font(.system(size: 10))
                    .foregroundColor(.secondary)
            }

            Spacer()

            Text(targetPosture)
                .font(.system(size: 11, weight: .bold, design: .rounded))
                .foregroundColor(Color(hex: colorHex))
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
                .background(Color(hex: colorHex).opacity(0.15))
                .clipShape(Capsule())
        }
    }
}

// MARK: - 4. Security & Firewall Detail View
private struct FirewallDetailView: View {
    @ObservedObject var state: PostureStateStore

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                // Privileged Helper Status Card
                VStack(alignment: .leading, spacing: 12) {
                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("PRIVILEGED PACKETFILTER HELPER DAEMON")
                                .font(.system(size: 11, weight: .bold))
                                .foregroundColor(.secondary)

                            Text("Root-privileged XPC service managing /etc/pf.anchors and pmset")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }

                        Spacer()

                        Button(state.helperConnected ? "Re-register Helper" : "Register Helper Tool") {
                            registerPrivilegedHelper()
                        }
                        .controlSize(.regular)
                    }

                    HStack(spacing: 12) {
                        HStack(spacing: 6) {
                            Circle()
                                .fill(state.helperConnected ? Color.green : Color.orange)
                                .frame(width: 8, height: 8)

                            Text(state.helperConnected ? "Helper Connected & Registered" : "Helper Unregistered (Mock Mode)")
                                .font(.system(size: 12, weight: .semibold))
                                .foregroundColor(state.helperConnected ? .green : .orange)
                        }

                        Spacer()

                        Text("XPC Domain: com.postureflow.helper")
                            .font(.system(size: 10))
                            .foregroundColor(.secondary)
                    }
                    .padding(10)
                    .background(Color(NSColor.windowBackgroundColor).opacity(0.5))
                    .cornerRadius(8)
                }
                .padding(14)
                .background(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(Color(NSColor.controlBackgroundColor))
                        .overlay(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                        )
                )

                // Packet Filter Rules Summary
                VStack(alignment: .leading, spacing: 12) {
                    Text("PACKET FILTER (PFCTL) POLICY")
                        .font(.system(size: 11, weight: .bold))
                        .foregroundColor(.secondary)

                    VStack(spacing: 8) {
                        LabeledContent("Inbound Policy") {
                            Text(state.currentMode.inboundFirewallPolicy)
                                .font(.system(size: 12, weight: .semibold))
                        }
                        Divider()
                        LabeledContent("Loopback IPC Guarantee") {
                            HStack(spacing: 4) {
                                Image(systemName: "checkmark.shield.fill").foregroundColor(.green)
                                Text("Pass Quick on 'lo0'").font(.system(size: 12, weight: .semibold))
                            }
                        }
                        Divider()
                        LabeledContent("Stealth ICMP Dropping") {
                            Text(state.currentMode == .travel ? "Enabled (Silent Drop)" : "Standard Response")
                                .font(.system(size: 12, weight: .semibold))
                                .foregroundColor(state.currentMode == .travel ? .green : .secondary)
                        }
                    }
                    .padding(12)
                    .background(Color(NSColor.windowBackgroundColor).opacity(0.5))
                    .cornerRadius(8)
                }
                .padding(14)
                .background(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(Color(NSColor.controlBackgroundColor))
                        .overlay(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                        )
                )
            }
            .padding(20)
        }
    }

    private func registerPrivilegedHelper() {
        if #available(macOS 13.0, *) {
            let service = SMAppService.daemon(plistName: "com.postureflow.helper.plist")
            do {
                try service.register()
                Task { @MainActor in
                    await state.syncWithDaemon()
                }
            } catch {
                NSLog("[SettingsView] Failed to register daemon: %@", error.localizedDescription)
            }
        }
    }
}

// MARK: - 5. Hardware & Power Detail View
private struct PowerDetailView: View {
    @ObservedObject var state: PostureStateStore

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                VStack(alignment: .leading, spacing: 12) {
                    Text("APPLE SILICON ENERGY & POWER TUNING")
                        .font(.system(size: 11, weight: .bold))
                        .foregroundColor(.secondary)

                    VStack(spacing: 12) {
                        LabeledContent("Apple Silicon Low Power Mode") {
                            Text(state.currentMode.enablesLowPowerMode ? "Enabled (Travel Mode)" : "Disabled (Standard)")
                                .font(.system(size: 12, weight: .semibold))
                                .foregroundColor(state.currentMode.enablesLowPowerMode ? .green : .secondary)
                        }

                        Divider()

                        LabeledContent("Display Sleep Timeout") {
                            Text("\(state.currentMode.defaultDisplaySleepMinutes) minutes")
                                .font(.system(size: 12, weight: .semibold))
                        }
                    }
                    .padding(12)
                    .background(Color(NSColor.windowBackgroundColor).opacity(0.5))
                    .cornerRadius(8)
                }
                .padding(14)
                .background(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(Color(NSColor.controlBackgroundColor))
                        .overlay(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                        )
                )
            }
            .padding(20)
        }
    }
}

// MARK: - 6. Hardware Defense Detail View
private struct HardwareDefenseDetailView: View {
    @ObservedObject var state: PostureStateStore

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                // Section 1: YubiKey Tethering
                VStack(alignment: .leading, spacing: 10) {
                    HStack {
                        Label("YubiKey Physical Presence Tethering", systemImage: "key.fill")
                            .font(.system(.headline, design: .rounded))
                            .foregroundColor(.green)

                        Spacer()

                        Toggle("", isOn: $state.hardwareDefense.yubikey.enabled)
                            .toggleStyle(.switch)
                            .labelsHidden()
                    }

                    Text("Zero-Trust hardware binding: continuously checks for your physical YubiKey token. Unplugging instantly locks the macOS screen and shifts to Travel lockdown mode.")
                        .font(.caption)
                        .foregroundColor(.secondary)

                    if state.hardwareDefense.yubikey.enabled {
                        Divider()

                        HStack(spacing: 14) {
                            Toggle("Lock Screen on Unplug", isOn: $state.hardwareDefense.yubikey.lockOnRemoval)
                            Toggle("Demote to Travel", isOn: $state.hardwareDefense.yubikey.demoteOnRemoval)
                            Toggle("Auto-Restore", isOn: $state.hardwareDefense.yubikey.restoreOnInsert)
                        }
                        .font(.system(size: 11.5))
                        .foregroundColor(.secondary)

                        Divider()

                        HStack {
                            Text("Detected USB Security Tokens:")
                                .font(.system(size: 11.5, weight: .medium))
                            Spacer()
                            Button("Scan USB Bus") {
                                state.refreshHardwareStatus()
                            }
                            .controlSize(.small)
                        }

                        if state.connectedYubikeys.isEmpty {
                            Text("⚠️ No YubiKey currently detected on USB bus.")
                                .font(.caption)
                                .foregroundColor(.orange)
                        } else {
                            ForEach(state.connectedYubikeys) { key in
                                HStack {
                                    Image(systemName: "checkmark.shield.fill").foregroundColor(.green)
                                    Text("\(key.name) (\(key.vendorId):\(key.productId))")
                                        .font(.system(size: 12, weight: .medium))
                                    Spacer()
                                    Text("ARMED & TETHERED")
                                        .font(.system(size: 9, weight: .heavy, design: .rounded))
                                        .padding(.horizontal, 6)
                                        .padding(.vertical, 2)
                                        .background(Color.green.opacity(0.18))
                                        .foregroundColor(.green)
                                        .clipShape(Capsule())
                                }
                            }
                        }
                    }
                }
                .padding(14)
                .background(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(Color(NSColor.controlBackgroundColor))
                        .overlay(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                        )
                )

                // Section 2: Decoy Honeypots
                VStack(alignment: .leading, spacing: 10) {
                    HStack {
                        Label("Decoy Honeypot Port Traps", systemImage: "shield.righthalf.filled")
                            .font(.system(.headline, design: .rounded))
                            .foregroundColor(.red)

                        Spacer()

                        Toggle("", isOn: $state.hardwareDefense.honeypot.enabled)
                            .toggleStyle(.switch)
                            .labelsHidden()
                    }

                    Text("Arms deceptive TCP listeners on ports 2222 (SSH), 8080 (Web), and 4450 (SMB) to catch unauthorized network port scanners on local subnets.")
                        .font(.caption)
                        .foregroundColor(.secondary)

                    if state.hardwareDefense.honeypot.enabled {
                        Divider()

                        Toggle("Auto-ban port scanning IP addresses via pfctl firewall table", isOn: $state.hardwareDefense.honeypot.autoBlockOffenders)
                            .font(.system(size: 11.5))

                        Divider()

                        Text("Recent Caught Intrusions (\(state.recentHoneypotIncidents.count)):")
                            .font(.system(size: 11.5, weight: .semibold))

                        if state.recentHoneypotIncidents.isEmpty {
                            Text("No unauthorized port scans detected. Perimeter secure.")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        } else {
                            ForEach(state.recentHoneypotIncidents.suffix(3)) { inc in
                                HStack {
                                    Image(systemName: "exclamationmark.triangle.fill").foregroundColor(.red)
                                    Text("\(inc.peerIP) ➔ Trap Port \(inc.trapPort)")
                                        .font(.system(size: 12, weight: .medium))
                                    Spacer()
                                    Text(inc.blocked ? "BLOCKED (pfctl)" : "LOGGED")
                                        .font(.system(size: 9, weight: .bold))
                                        .foregroundColor(inc.blocked ? .red : .orange)
                                }
                            }
                        }
                    }
                }
                .padding(14)
                .background(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(Color(NSColor.controlBackgroundColor))
                        .overlay(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                        )
                )

                // Section 3: Sensor Privacy Kill Switch
                VStack(alignment: .leading, spacing: 10) {
                    HStack {
                        Label("Sensor & Audio Hardware Privacy", systemImage: "mic.fill")
                            .font(.system(.headline, design: .rounded))
                            .foregroundColor(.orange)

                        Spacer()

                        Button(state.sensorPrivacyReport.microphoneMuted ? "Unmute Mic" : "Mute Mic") {
                            state.toggleMicrophoneMute()
                        }
                        .controlSize(.small)

                        Button(state.sensorPrivacyReport.emergencyKillActive ? "Restore All Sensors" : "🚨 Emergency Kill Switch") {
                            Task {
                                if state.sensorPrivacyReport.emergencyKillActive {
                                    await state.restoreSensors()
                                } else {
                                    await state.engageEmergencyKillSwitch()
                                }
                            }
                        }
                        .controlSize(.small)
                        .buttonStyle(.borderedProminent)
                        .tint(state.sensorPrivacyReport.emergencyKillActive ? .orange : .red)
                    }

                    HStack {
                        Text("Microphone Input:")
                            .font(.caption)
                        Text(state.sensorPrivacyReport.microphoneMuted ? "MUTED (0% Input)" : "ACTIVE (\(state.sensorPrivacyReport.inputVolumePercent)%)")
                            .font(.caption)
                            .fontWeight(.bold)
                            .foregroundColor(state.sensorPrivacyReport.microphoneMuted ? .green : .orange)
                    }

                    Divider()

                    HStack(spacing: 16) {
                        Toggle("Auto-mute mic in Travel mode", isOn: $state.hardwareDefense.sensors.muteMicOnTravel)
                        Toggle("Auto-mute mic on Screen Lock", isOn: $state.hardwareDefense.sensors.muteMicOnLock)
                    }
                    .font(.system(size: 11.5))
                    .foregroundColor(.secondary)
                }
                .padding(14)
                .background(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(Color(NSColor.controlBackgroundColor))
                        .overlay(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                        )
                )

                // Section 4: Bluetooth Walk-Away Proximity
                VStack(alignment: .leading, spacing: 10) {
                    HStack {
                        Label("Bluetooth Walk-Away Proximity Auto-Lock", systemImage: "wave.3.right.circle.fill")
                            .font(.system(.headline, design: .rounded))
                            .foregroundColor(.blue)

                        Spacer()

                        Toggle("", isOn: $state.hardwareDefense.proximity.enabled)
                            .toggleStyle(.switch)
                            .labelsHidden()
                    }

                    Text("Monitors signal strength (RSSI) from your paired iPhone or Apple Watch. If you walk away from your Mac, the screen automatically locks.")
                        .font(.caption)
                        .foregroundColor(.secondary)

                    if state.hardwareDefense.proximity.enabled {
                        Divider()

                        HStack {
                            Text("Distance Threshold: \(state.hardwareDefense.proximity.rssiThresholdDbm) dBm")
                                .font(.system(size: 11.5))
                                .frame(width: 170, alignment: .leading)

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
                                .font(.system(size: 11.5))
                                .frame(width: 170, alignment: .leading)

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
                .padding(14)
                .background(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .fill(Color(NSColor.controlBackgroundColor))
                        .overlay(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .stroke(Color.primary.opacity(0.06), lineWidth: 1)
                        )
                )
            }
            .padding(20)
        }
    }
}
