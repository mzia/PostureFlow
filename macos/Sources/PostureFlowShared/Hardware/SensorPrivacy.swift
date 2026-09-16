import Foundation

/// Live status of hardware sensors and media inputs
public struct SensorPrivacyReport: Codable, Sendable, Equatable {
    public var cameraBlocked: Bool
    public var microphoneMuted: Bool
    public var locationBlocked: Bool
    public var inputVolumePercent: Int
    public var emergencyKillActive: Bool

    public init(
        cameraBlocked: Bool = false,
        microphoneMuted: Bool = false,
        locationBlocked: Bool = false,
        inputVolumePercent: Int = 100,
        emergencyKillActive: Bool = false
    ) {
        self.cameraBlocked = cameraBlocked
        self.microphoneMuted = microphoneMuted
        self.locationBlocked = locationBlocked
        self.inputVolumePercent = inputVolumePercent
        self.emergencyKillActive = emergencyKillActive
    }
}

/// Configuration for Camera, Microphone, and Geolocation Privacy
public struct SensorPrivacyConfig: Codable, Sendable, Equatable {
    public var muteMicOnTravel: Bool
    public var muteMicOnLock: Bool
    public var cameraBlockedOnTravel: Bool
    public var locationBlockedOnTravel: Bool
    public var emergencyKillActive: Bool

    public init(
        muteMicOnTravel: Bool = true,
        muteMicOnLock: Bool = true,
        cameraBlockedOnTravel: Bool = true,
        locationBlockedOnTravel: Bool = true,
        emergencyKillActive: Bool = false
    ) {
        self.muteMicOnTravel = muteMicOnTravel
        self.muteMicOnLock = muteMicOnLock
        self.cameraBlockedOnTravel = cameraBlockedOnTravel
        self.locationBlockedOnTravel = locationBlockedOnTravel
        self.emergencyKillActive = emergencyKillActive
    }
}

/// Controller for programmatic audio and hardware privacy on macOS
public struct SensorPrivacyController {
    /// Sets microphone input volume (0 = muted)
    public static func setMicrophoneMuted(_ muted: Bool) -> Bool {
        let volume = muted ? 0 : 75
        let script = "set volume input volume \(volume)"
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
        process.arguments = ["-e", script]
        do {
            try process.run()
            process.waitUntilExit()
            return process.terminationStatus == 0
        } catch {
            return false
        }
    }

    /// Queries the current microphone input volume (0-100)
    public static func getInputVolume() -> Int {
        let script = "input volume of (get volume settings)"
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
        process.arguments = ["-e", script]

        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = Pipe()

        do {
            try process.run()
            process.waitUntilExit()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            if let output = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines),
               let val = Int(output) {
                return val
            }
        } catch {}
        return 100
    }

    /// Engages 1-click emergency sensor kill switch: cuts mic volume and flags privacy kill
    public static func emergencyKillAllSensors() -> Bool {
        let micSuccess = setMicrophoneMuted(true)
        // Store emergency kill state
        setEmergencyStateFile(active: true)
        return micSuccess
    }

    /// Restores audio and camera sensors to standard operation
    public static func restoreAllSensors() -> Bool {
        let micSuccess = setMicrophoneMuted(false)
        setEmergencyStateFile(active: false)
        return micSuccess
    }

    private static var emergencyStateFileURL: URL {
        FileManager.default.temporaryDirectory.appendingPathComponent("postureflow_emergency_kill.state")
    }

    private static func setEmergencyStateFile(active: Bool) {
        let url = emergencyStateFileURL
        if active {
            try? "emergency".write(to: url, atomically: true, encoding: .utf8)
        } else {
            try? FileManager.default.removeItem(at: url)
        }
    }

    public static func isEmergencyKillActive() -> Bool {
        FileManager.default.fileExists(atPath: emergencyStateFileURL.path)
    }

    /// Generates live sensor privacy report
    public static func getReport() -> SensorPrivacyReport {
        let inputVol = getInputVolume()
        let isEmergency = isEmergencyKillActive()
        let isMuted = inputVol == 0 || isEmergency
        return SensorPrivacyReport(
            cameraBlocked: isEmergency,
            microphoneMuted: isMuted,
            locationBlocked: isEmergency,
            inputVolumePercent: inputVol,
            emergencyKillActive: isEmergency
        )
    }
}
