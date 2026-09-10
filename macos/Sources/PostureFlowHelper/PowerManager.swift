import Foundation
import IOKit.ps

/// Manages macOS power profiles, Low Power Mode, display sleep timeouts, and battery telemetry.
public final class PowerManager {
    public static let shared = PowerManager()

    public init() {}

    /// Toggles Apple Silicon / macOS Low Power Mode
    public func setLowPowerMode(enabled: Bool) -> (success: Bool, error: String?) {
        let flag = enabled ? "1" : "0"
        let result = executeCommand("/usr/bin/pmset", arguments: ["-a", "lowpowermode", flag])
        if result.exitCode == 0 {
            return (true, nil)
        } else {
            return (false, "Failed to set low power mode: \(result.output)")
        }
    }

    /// Configures the display sleep timeout in minutes
    public func setDisplaySleep(minutes: Int) -> (success: Bool, error: String?) {
        let minString = String(max(1, minutes))
        let result = executeCommand("/usr/bin/pmset", arguments: ["-a", "displaysleep", minString])
        if result.exitCode == 0 {
            return (true, nil)
        } else {
            return (false, "Failed to set display sleep: \(result.output)")
        }
    }

    /// Queries whether Low Power Mode is currently active
    public func isLowPowerModeActive() -> Bool {
        let result = executeCommand("/usr/bin/pmset", arguments: ["-g", "custom"])
        return result.output.contains("lowpowermode         1")
    }

    /// Returns the active battery percentage (0-100) and whether the device is on AC power
    public func getBatteryTelemetry() -> (percentage: Int, isCharging: Bool, isOnAC: Bool) {
        guard let snapshot = IOPSCopyPowerSourcesInfo()?.takeRetainedValue(),
              let sources = IOPSCopyPowerSourcesList(snapshot)?.takeRetainedValue() as? [CFTypeRef] else {
            return (100, false, true)
        }

        for source in sources {
            guard let desc = IOPSGetPowerSourceDescription(snapshot, source)?.takeUnretainedValue() as? [String: Any] else {
                continue
            }
            
            let isCurrentPowerSource = desc[kIOPSIsCurrentPowerSourceKey as String] as? Bool ?? false
            if !isCurrentPowerSource { continue }

            let currentCapacity = desc[kIOPSCurrentCapacityKey as String] as? Int ?? 100
            let maxCapacity = desc[kIOPSMaxCapacityKey as String] as? Int ?? 100
            let percentage = maxCapacity > 0 ? Int((Double(currentCapacity) / Double(maxCapacity)) * 100.0) : 100
            
            let powerSourceState = desc[kIOPSPowerSourceStateKey as String] as? String
            let isOnAC = powerSourceState == (kIOPSACPowerValue as String)
            let isCharging = desc[kIOPSIsChargingKey as String] as? Bool ?? false

            return (percentage, isCharging, isOnAC)
        }

        return (100, false, true)
    }

    private func executeCommand(_ binary: String, arguments: [String]) -> (exitCode: Int32, output: String) {
        let task = Process()
        task.executableURL = URL(fileURLWithPath: binary)
        task.arguments = arguments

        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = pipe

        do {
            try task.run()
            task.waitUntilExit()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            let output = String(data: data, encoding: .utf8) ?? ""
            return (task.terminationStatus, output)
        } catch {
            return (-1, error.localizedDescription)
        }
    }
}
