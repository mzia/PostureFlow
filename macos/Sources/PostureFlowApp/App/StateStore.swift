import Foundation
import SwiftUI
import PostureFlowShared

/// Reactive single source of truth for the PostureFlow macOS interface.
@MainActor
public final class PostureStateStore: ObservableObject {
    @Published public var currentMode: PostureMode = .work
    @Published public var postureScore: PostureScore
    @Published public var isVPNActive: Bool = false
    @Published public var connectedSSID: String? = nil
    @Published public var helperConnected: Bool = false
    @Published public var statusMessage: String = "Ready"

    @Published public var autoFlowEnabled: Bool = true {
        didSet { config.autoFlowEnabled = autoFlowEnabled; saveConfig() }
    }
    @Published public var triggersEnabled: Bool = true {
        didSet { config.triggersEnabled = triggersEnabled; saveConfig() }
    }
    @Published public var circadianEnabled: Bool = false {
        didSet { config.circadianEnabled = circadianEnabled; saveConfig() }
    }

    // Hardware Defenses State
    @Published public var hardwareDefense: HardwareDefenseConfig {
        didSet {
            config.hardwareDefense = hardwareDefense
            saveConfig()
            updateHardwareEngines()
        }
    }
    @Published public var connectedYubikeys: [YubikeyDevice] = []
    @Published public var recentHoneypotIncidents: [HoneypotIncident] = []
    @Published public var sensorPrivacyReport: SensorPrivacyReport = SensorPrivacyReport()
    @Published public var proximityRSSI: Int? = nil

    private var config: PostureConfig
    private let xpcClient = XPCClient.shared
    private let autoFlow = AutoFlowMonitor()
    private let appTriggers = AppTriggerEngine()
    private let circadian = CircadianScheduler()

    private let yubiKeyMonitor = YubiKeyTetherMonitor()
    private let honeypotEngine = HoneypotEngine()
    private let proximityEngine = ProximityEngine()
    private var profileBeforeTetherDemote: PostureMode?

    public init() {
        let loadedConfig = PostureConfig.load()
        self.config = loadedConfig
        self.currentMode = loadedConfig.activeProfile
        self.autoFlowEnabled = loadedConfig.autoFlowEnabled
        self.triggersEnabled = loadedConfig.triggersEnabled
        self.circadianEnabled = loadedConfig.circadianEnabled
        self.hardwareDefense = loadedConfig.hardwareDefense

        self.sensorPrivacyReport = SensorPrivacyController.getReport()
        self.recentHoneypotIncidents = HoneypotLedger.loadRecentIncidents()
        self.connectedYubikeys = YubikeyDetector.detectYubikeys()

        // Initialize score baseline
        self.postureScore = PostureScore(
            mode: loadedConfig.activeProfile,
            isFirewallActive: true,
            isStealthModeActive: (loadedConfig.activeProfile == .travel),
            isVPNActive: false,
            openPortCount: 0,
            isLowPowerMode: loadedConfig.activeProfile.enablesLowPowerMode,
            isHardwareTetherActive: loadedConfig.hardwareDefense.yubikey.enabled && !YubikeyDetector.detectYubikeys().isEmpty,
            isHoneypotActive: loadedConfig.hardwareDefense.honeypot.enabled,
            isSensorPrivacyActive: SensorPrivacyController.isEmergencyKillActive(),
            isProximityLockActive: loadedConfig.hardwareDefense.proximity.enabled
        )

        setupEngines()
        Task {
            await syncWithDaemon()
        }
    }

    /// Connects background engines and monitors
    private func setupEngines() {
        // 1. Auto-Flow Network Monitor
        autoFlow.start { [weak self] ssid, isVPN in
            Task { @MainActor [weak self] in
                guard let self = self else { return }
                self.connectedSSID = ssid
                self.isVPNActive = isVPN
                self.recalculateScore()

                if self.autoFlowEnabled, let ssid = ssid, let mappedMode = self.config.whitelistedSSIDs[ssid] {
                    if mappedMode != self.currentMode {
                        self.statusMessage = "Auto-Flow: Switched to \(mappedMode.displayName) on '\(ssid)'"
                        await self.switchTo(mappedMode)
                    }
                }
            }
        }

        // 2. App-Aware Dynamic Triggers
        appTriggers.start { [weak self] bundleID, isRunning in
            Task { @MainActor [weak self] in
                guard let self = self, self.triggersEnabled else { return }
                if isRunning, let targetMode = self.config.appTriggers[bundleID] {
                    if targetMode != self.currentMode {
                        self.statusMessage = "Trigger: Launch of \(bundleID) activated \(targetMode.displayName)"
                        await self.switchTo(targetMode)
                    }
                }
            }
        }

        // 3. Circadian Scheduler
        circadian.start { [weak self] suggestedMode in
            Task { @MainActor [weak self] in
                guard let self = self, self.circadianEnabled else { return }
                if suggestedMode != self.currentMode {
                    self.statusMessage = "Circadian: Shifted to \(suggestedMode.displayName)"
                    await self.switchTo(suggestedMode)
                }
            }
        }

        // 4. YubiKey Hardware Tethering Monitor
        yubiKeyMonitor.start(
            configProvider: { [weak self] in self?.hardwareDefense.yubikey ?? YubikeyTetherConfig() },
            onAction: { [weak self] action in
                Task { @MainActor [weak self] in
                    guard let self = self else { return }
                    switch action {
                    case .lockAndDemote(let targetProfile, let reason):
                        self.statusMessage = "🔑 YubiKey Unplugged: \(reason)"
                        self.profileBeforeTetherDemote = self.currentMode
                        self.yubiKeyMonitor.setSavedProfileBeforeDemote(self.currentMode)
                        // Lock screen
                        _ = try? await self.xpcClient.lockScreen()
                        // Demote profile
                        await self.switchTo(targetProfile)
                    case .restore(let targetProfile):
                        self.statusMessage = "🔑 YubiKey Restored: Resuming \(targetProfile.displayName) posture"
                        await self.switchTo(targetProfile)
                    case .none:
                        break
                    }
                }
            }
        )

        // 5. Honeypot Decoy Trap Engine
        updateHoneypotEngine()

        // 6. Bluetooth Walk-Away Proximity Engine
        updateProximityEngine()
    }

    private func updateHardwareEngines() {
        updateHoneypotEngine()
        updateProximityEngine()
        recalculateScore()
    }

    private func updateHoneypotEngine() {
        honeypotEngine.update(
            config: hardwareDefense.honeypot,
            currentMode: currentMode
        ) { [weak self] incident in
            Task { @MainActor [weak self] in
                guard let self = self else { return }
                self.recentHoneypotIncidents.append(incident)
                self.statusMessage = "🚨 Port Scan Caught: \(incident.peerIP) on trap port \(incident.trapPort)"

                if self.hardwareDefense.honeypot.autoBlockOffenders {
                    _ = try? await self.xpcClient.blockOffenderIP(ip: incident.peerIP)
                }
            }
        }
    }

    private func updateProximityEngine() {
        proximityEngine.start(
            config: hardwareDefense.proximity
        ) { [weak self] action in
            Task { @MainActor [weak self] in
                guard let self = self else { return }
                switch action {
                case .lockSession(let reason):
                    self.statusMessage = "🛰️ Proximity Lock: \(reason)"
                    _ = try? await self.xpcClient.lockScreen()
                case .restoreSession:
                    self.statusMessage = "🛰️ Proximity: Device returned in-range"
                case .none:
                    break
                }
            }
        }
    }

    // MARK: - Sensor & Emergency Actions

    public func engageEmergencyKillSwitch() async {
        _ = try? await xpcClient.emergencyKillSensors()
        _ = SensorPrivacyController.emergencyKillAllSensors()
        self.sensorPrivacyReport = SensorPrivacyController.getReport()
        self.statusMessage = "🚨 Emergency Sensor Kill Switch Engaged!"
        recalculateScore()
    }

    public func restoreSensors() async {
        _ = try? await xpcClient.restoreSensors()
        _ = SensorPrivacyController.restoreAllSensors()
        self.sensorPrivacyReport = SensorPrivacyController.getReport()
        self.statusMessage = "Sensors restored to standard profile"
        recalculateScore()
    }

    public func toggleMicrophoneMute() {
        let currentlyMuted = sensorPrivacyReport.microphoneMuted
        _ = SensorPrivacyController.setMicrophoneMuted(!currentlyMuted)
        self.sensorPrivacyReport = SensorPrivacyController.getReport()
        self.statusMessage = currentlyMuted ? "Microphone active" : "Microphone muted"
        recalculateScore()
    }

    public func refreshHardwareStatus() {
        self.connectedYubikeys = YubikeyDetector.detectYubikeys()
        self.sensorPrivacyReport = SensorPrivacyController.getReport()
        self.recentHoneypotIncidents = HoneypotLedger.loadRecentIncidents()
        recalculateScore()
    }

    /// Queries the privileged helper daemon to synchronize live state
    public func syncWithDaemon() async {
        do {
            _ = try await xpcClient.getVersion()
            self.helperConnected = true
            let state = try await xpcClient.querySystemState()
            self.currentMode = state.mode
            recalculateScore(isFirewallActive: state.isFirewallActive, isLowPower: state.isLowPower, isStealth: state.isStealth)
            self.statusMessage = "Synchronized with helper daemon"
        } catch {
            self.helperConnected = false
            self.statusMessage = "Helper daemon not registered (running in mock mode)"
        }
    }

    /// Switches the active posture profile
    public func switchTo(_ mode: PostureMode) async {
        self.currentMode = mode
        self.config.activeProfile = mode
        saveConfig()
        updateHoneypotEngine()
        recalculateScore()

        do {
            let success = try await xpcClient.applyPosture(mode: mode)
            if success {
                self.statusMessage = "Active: \(mode.displayName)"
            } else {
                self.statusMessage = "Failed to apply \(mode.displayName)"
            }
        } catch {
            self.statusMessage = "Applied \(mode.displayName) locally (Helper unavailable)"
        }
    }

    /// Recalculates the posture score based on current system conditions
    public func recalculateScore(isFirewallActive: Bool = true, isLowPower: Bool? = nil, isStealth: Bool? = nil) {
        let lowPower = isLowPower ?? currentMode.enablesLowPowerMode
        let stealth = isStealth ?? (currentMode == .travel)
        self.postureScore = PostureScore(
            mode: currentMode,
            isFirewallActive: isFirewallActive,
            isStealthModeActive: stealth,
            isVPNActive: isVPNActive,
            openPortCount: (currentMode == .dev ? 2 : 0),
            isLowPowerMode: lowPower,
            isHardwareTetherActive: hardwareDefense.yubikey.enabled && !connectedYubikeys.isEmpty,
            isHoneypotActive: hardwareDefense.honeypot.enabled,
            isSensorPrivacyActive: sensorPrivacyReport.emergencyKillActive || sensorPrivacyReport.microphoneMuted,
            isProximityLockActive: hardwareDefense.proximity.enabled
        )
    }

    private func saveConfig() {
        try? config.save()
    }
}
