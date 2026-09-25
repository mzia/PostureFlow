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

    // Security Suite State
    @Published public var connectedBSSID: String? = nil
    @Published public var connectedGatewayMAC: String? = nil
    @Published public var evilTwinAlert: String? = nil
    @Published public var isEvilTwinDetected: Bool = false
    @Published public var credentialCloakStatus: CredentialCloakStatus = CredentialCloakStatus()
    @Published public var dnsStatusReport: DNSStatusReport = DNSStatusReport()

    /// Current security shield icon for menu bar display
    public var menuBarShieldIcon: String {
        return currentMode.shieldSymbol
    }

    public var config: PostureConfig
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
        self.dnsStatusReport = EncryptedDNSManager.evaluateDNSStatus(mode: loadedConfig.activeProfile, config: loadedConfig.encryptedDNS)

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
            isProximityLockActive: loadedConfig.hardwareDefense.proximity.enabled,
            isEvilTwinDetected: false,
            isEncryptedDNSActive: loadedConfig.encryptedDNS.enabled && (loadedConfig.activeProfile == .travel || loadedConfig.encryptedDNS.dnsOverTLS),
            isCredentialCloaked: (loadedConfig.activeProfile != .dev && loadedConfig.credentialCloaking.enabled)
        )

        setupEngines()
        Task<Void, Never> {
            await syncWithDaemon()
        }
    }

    /// Connects background engines and monitors
    private func setupEngines() {
        // 1. Auto-Flow Network Monitor with Anti-Evil Twin Evaluation
        autoFlow.start { [weak self] ssid, bssid, gatewayMAC, isVPN in
            Task { @MainActor [weak self] in
                guard let self = self else { return }
                self.connectedSSID = ssid
                self.connectedBSSID = bssid
                self.connectedGatewayMAC = gatewayMAC
                self.isVPNActive = isVPN

                // Anti-Evil Twin & BSSID Gateway Fingerprinting check
                let evaluation = AntiEvilTwinDetector.evaluate(
                    currentSSID: ssid,
                    currentBSSID: bssid,
                    currentGatewayMAC: gatewayMAC,
                    trustedRegistry: self.config.trustedNetworks
                )

                switch evaluation {
                case .evilTwinDetected(let s, _, _, _, _):
                    self.isEvilTwinDetected = true
                    let alert = "🚨 ROGUE AP / EVIL TWIN ATTACK DETECTED on '\(s)'! Spoofed BSSID or Gateway MAC. Lockdown engaged."
                    self.evilTwinAlert = alert
                    self.statusMessage = alert
                    self.recalculateScore()
                    if self.currentMode != .travel {
                        await self.switchTo(.travel)
                    }

                case .trusted(let fingerprint):
                    self.isEvilTwinDetected = false
                    self.evilTwinAlert = nil
                    self.recalculateScore()
                    if self.autoFlowEnabled && fingerprint.targetMode != self.currentMode {
                        self.statusMessage = "Auto-Flow: Trusted network '\(fingerprint.ssid)'. Switched to \(fingerprint.targetMode.displayName)"
                        await self.switchTo(fingerprint.targetMode)
                    }

                case .unregistered, .disconnected:
                    self.isEvilTwinDetected = false
                    self.evilTwinAlert = nil
                    self.recalculateScore()
                    if self.autoFlowEnabled, let ssid = ssid, let mappedMode = self.config.whitelistedSSIDs[ssid] {
                        if mappedMode != self.currentMode {
                            self.statusMessage = "Auto-Flow: Switched to \(mappedMode.displayName) on '\(ssid)'"
                            await self.switchTo(mappedMode)
                        }
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
            config: hardwareDefense.yubikey,
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
        yubiKeyMonitor.update(config: hardwareDefense.yubikey)
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
        let previousMode = self.currentMode
        self.currentMode = mode
        self.config.activeProfile = mode
        saveConfig()
        updateHoneypotEngine()

        // Handle Posture-Aware Credential Cloaking
        self.credentialCloakStatus = CredentialCloakEngine.handlePostureChange(
            from: previousMode,
            to: mode,
            config: config.credentialCloaking
        )

        // Evaluate Profile-Aware Encrypted DNS
        self.dnsStatusReport = EncryptedDNSManager.evaluateDNSStatus(
            mode: mode,
            config: config.encryptedDNS
        )

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

    /// Manually uncloaks developer credentials (e.g. ~/.aws/credentials permissions)
    public func uncloakCredentials() {
        _ = CredentialCloakEngine.setAWSCredentialsCloaked(false)
        self.credentialCloakStatus.isAWSCredentialsCloaked = false
        self.statusMessage = "Developer credentials uncloaked"
        recalculateScore()
    }

    /// Registers the current Wi-Fi SSID, BSSID, and Gateway MAC as a trusted fingerprint
    public func trustCurrentNetwork(targetMode: PostureMode) {
        guard let ssid = connectedSSID, !ssid.isEmpty else { return }
        let fingerprint = NetworkFingerprint(
            ssid: ssid,
            bssid: connectedBSSID,
            gatewayMAC: connectedGatewayMAC,
            targetMode: targetMode
        )
        self.config.trustedNetworks[ssid] = fingerprint
        saveConfig()
        self.isEvilTwinDetected = false
        self.evilTwinAlert = nil
        self.statusMessage = "Fingerprinted '\(ssid)' as trusted network for \(targetMode.displayName)"
        recalculateScore()
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
            isProximityLockActive: hardwareDefense.proximity.enabled,
            isEvilTwinDetected: isEvilTwinDetected,
            isEncryptedDNSActive: dnsStatusReport.isEncrypted,
            isCredentialCloaked: (currentMode != .dev && config.credentialCloaking.enabled)
        )
    }

    private func saveConfig() {
        try? config.save()
    }
}
