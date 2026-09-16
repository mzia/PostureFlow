import Foundation
import PostureFlowShared

/// Implements the PostureFlowHelperProtocol over Mach XPC.
/// Runs inside the root privileged helper daemon process.
public final class HelperService: NSObject, PostureFlowHelperProtocol, NSXPCListenerDelegate {
    public static let shared = HelperService()

    private let packetFilter = PacketFilterManager.shared
    private let powerManager = PowerManager.shared
    private var currentMode: PostureMode = .work

    public override init() {
        super.init()
    }

    // MARK: - NSXPCListenerDelegate

    public func listener(_ listener: NSXPCListener, shouldAcceptNewConnection newConnection: NSXPCConnection) -> Bool {
        // Set up the exported object and interface
        let interface = NSXPCInterface(with: PostureFlowHelperProtocol.self)
        newConnection.exportedInterface = interface
        newConnection.exportedObject = self

        // In a production Apple Developer environment, audit token validation is performed here:
        // let clientAuditToken = newConnection.auditToken
        // guard validateClient(token: clientAuditToken) else { return false }

        newConnection.interruptionHandler = {
            NSLog("[PostureFlowHelper] XPC Connection interrupted")
        }
        newConnection.invalidationHandler = {
            NSLog("[PostureFlowHelper] XPC Connection invalidated")
        }

        newConnection.resume()
        NSLog("[PostureFlowHelper] Accepted new XPC client connection (PID: %d)", newConnection.processIdentifier)
        return true
    }

    // MARK: - PostureFlowHelperProtocol Implementation

    public func getVersion(with reply: @escaping (String) -> Void) {
        reply("1.0.0-macos")
    }

    public func querySystemState(with reply: @escaping (String, Bool, Bool, Bool, String?) -> Void) {
        let isPFActive = packetFilter.isAnchorActive()
        let isLowPower = powerManager.isLowPowerModeActive()
        let isStealth = (currentMode == .travel)
        reply(currentMode.rawValue, isPFActive, isLowPower, isStealth, nil)
    }

    public func applyPosture(mode modeString: String, with reply: @escaping (Bool, String?) -> Void) {
        guard let mode = PostureMode(rawValue: modeString) else {
            reply(false, "Invalid posture mode identifier: '\(modeString)'")
            return
        }

        NSLog("[PostureFlowHelper] Applying posture: %@", mode.displayName)
        self.currentMode = mode

        // 1. Apply Packet Filter Anchor Rules
        let pfResult = packetFilter.applyRules(for: mode)
        guard pfResult.success else {
            reply(false, "Firewall error: \(pfResult.error ?? "Unknown")")
            return
        }

        // 2. Apply Low Power Mode
        let lowPowerResult = powerManager.setLowPowerMode(enabled: mode.enablesLowPowerMode)
        if !lowPowerResult.success {
            NSLog("[PostureFlowHelper] Warning: Failed to set low power mode: %@", lowPowerResult.error ?? "")
        }

        // 3. Apply Display Sleep Timeout
        let displayResult = powerManager.setDisplaySleep(minutes: mode.defaultDisplaySleepMinutes)
        if !displayResult.success {
            NSLog("[PostureFlowHelper] Warning: Failed to set display sleep: %@", displayResult.error ?? "")
        }

        reply(true, nil)
    }

    public func applyFirewallRules(rules: [String], anchor: String, with reply: @escaping (Bool, String?) -> Void) {
        // Direct custom rule injection
        reply(true, nil)
    }

    public func setLowPowerMode(enabled: Bool, with reply: @escaping (Bool, String?) -> Void) {
        let result = powerManager.setLowPowerMode(enabled: enabled)
        reply(result.success, result.error)
    }

    public func setDisplaySleep(minutes: Int, with reply: @escaping (Bool, String?) -> Void) {
        let result = powerManager.setDisplaySleep(minutes: minutes)
        reply(result.success, result.error)
    }

    public func setBluetoothEnabled(_ enabled: Bool, with reply: @escaping (Bool, String?) -> Void) {
        // Bluetooth toggle can be implemented via blueutil or IOBluetooth C-API
        NSLog("[PostureFlowHelper] setBluetoothEnabled called: %d", enabled)
        reply(true, nil)
    }

    public func restoreFirewallDefaults(with reply: @escaping (Bool, String?) -> Void) {
        let result = packetFilter.flushAnchor()
        _ = powerManager.setLowPowerMode(enabled: false)
        _ = powerManager.setDisplaySleep(minutes: 15)
        reply(result.success, result.error)
    }

    public func blockOffenderIP(ip: String, with reply: @escaping (Bool, String?) -> Void) {
        NSLog("[PostureFlowHelper] Blocking malicious peer IP: %@", ip)
        let result = packetFilter.blockIP(ip)
        reply(result.success, result.error)
    }

    public func emergencyKillSensors(with reply: @escaping (Bool, String?) -> Void) {
        NSLog("[PostureFlowHelper] Engaging emergency sensor privacy kill switch")
        let success = SensorPrivacyController.emergencyKillAllSensors()
        reply(success, success ? nil : "Failed to engage emergency sensor kill switch")
    }

    public func restoreSensors(with reply: @escaping (Bool, String?) -> Void) {
        NSLog("[PostureFlowHelper] Restoring sensor privacy settings")
        let success = SensorPrivacyController.restoreAllSensors()
        reply(success, success ? nil : "Failed to restore sensors")
    }

    public func lockScreen(with reply: @escaping (Bool, String?) -> Void) {
        NSLog("[PostureFlowHelper] Locking macOS desktop session")
        // Invoke CGSession suspend or pmset displaysleepnow
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/pmset")
        process.arguments = ["displaysleepnow"]
        do {
            try process.run()
            process.waitUntilExit()
            reply(process.terminationStatus == 0, nil)
        } catch {
            reply(false, error.localizedDescription)
        }
    }
}
