import Foundation

/// Constants for PostureFlow Inter-Process Communication (XPC)
public enum PostureFlowIPCConstants {
    public static let machServiceName = "com.postureflow.helper"
}

/// The formal Objective-C compatible protocol implemented by the privileged helper daemon
/// and consumed by the unprivileged PostureFlow macOS app and CLI.
@objc(PostureFlowHelperProtocol)
public protocol PostureFlowHelperProtocol {
    /// Returns the daemon version
    func getVersion(with reply: @escaping (String) -> Void)
    
    /// Queries the live system state (activeMode, firewallActive, lowPowerMode, stealthMode, error)
    func querySystemState(with reply: @escaping (String, Bool, Bool, Bool, String?) -> Void)
    
    /// Applies a specific posture mode
    func applyPosture(mode: String, with reply: @escaping (Bool, String?) -> Void)
    
    /// Generates and loads custom pfctl anchor rules
    func applyFirewallRules(rules: [String], anchor: String, with reply: @escaping (Bool, String?) -> Void)
    
    /// Sets macOS Low Power Mode (via pmset -a lowpowermode)
    func setLowPowerMode(enabled: Bool, with reply: @escaping (Bool, String?) -> Void)
    
    /// Sets display sleep timeout in minutes (via pmset -a displaysleep)
    func setDisplaySleep(minutes: Int, with reply: @escaping (Bool, String?) -> Void)
    
    /// Toggles Bluetooth radio state
    func setBluetoothEnabled(_ enabled: Bool, with reply: @escaping (Bool, String?) -> Void)
    
    /// Flushes PostureFlow pfctl anchor rules and restores factory networking state
    func restoreFirewallDefaults(with reply: @escaping (Bool, String?) -> Void)

    /// Blocks a malicious peer IP via Packet Filter (pfctl)
    func blockOffenderIP(ip: String, with reply: @escaping (Bool, String?) -> Void)

    /// Engages emergency sensor privacy kill switch (cuts audio input volume to 0)
    func emergencyKillSensors(with reply: @escaping (Bool, String?) -> Void)

    /// Restores audio and sensor privacy to standard settings
    func restoreSensors(with reply: @escaping (Bool, String?) -> Void)

    /// Immediately locks the macOS desktop session
    func lockScreen(with reply: @escaping (Bool, String?) -> Void)
}
