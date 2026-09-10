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

    private var config: PostureConfig
    private let xpcClient = XPCClient.shared
    private let autoFlow = AutoFlowMonitor()
    private let appTriggers = AppTriggerEngine()
    private let circadian = CircadianScheduler()

    public init() {
        let loadedConfig = PostureConfig.load()
        self.config = loadedConfig
        self.currentMode = loadedConfig.activeProfile
        self.autoFlowEnabled = loadedConfig.autoFlowEnabled
        self.triggersEnabled = loadedConfig.triggersEnabled
        self.circadianEnabled = loadedConfig.circadianEnabled

        // Initialize score baseline
        self.postureScore = PostureScore(
            mode: loadedConfig.activeProfile,
            isFirewallActive: true,
            isStealthModeActive: (loadedConfig.activeProfile == .travel),
            isVPNActive: false,
            openPortCount: 0,
            isLowPowerMode: loadedConfig.activeProfile.enablesLowPowerMode
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
            isLowPowerMode: lowPower
        )
    }

    private func saveConfig() {
        try? config.save()
    }
}
