import Foundation

/// Represents a discovered or paired Bluetooth Low Energy device
public struct ProximityDevice: Identifiable, Codable, Sendable, Equatable {
    public var id: String { identifier }
    public let name: String
    public let identifier: String // UUID or MAC
    public var rssi: Int
    public var lastSeen: Date

    public init(name: String, identifier: String, rssi: Int, lastSeen: Date = Date()) {
        self.name = name
        self.identifier = identifier
        self.rssi = rssi
        self.lastSeen = lastSeen
    }
}

/// Action determined by the Bluetooth proximity evaluator
public enum ProximityAction: Equatable, Sendable {
    case none
    case lockSession(reason: String)
    case restoreSession
}

/// Configuration for Bluetooth RSSI Proximity Auto-Lock ("Walk-Away Security")
public struct ProximityConfig: Codable, Sendable, Equatable {
    public var enabled: Bool
    public var targetDeviceName: String?
    public var targetDeviceIdentifier: String?
    public var rssiThresholdDbm: Int // Default: -80 dBm
    public var gracePeriodSeconds: Int // Default: 10 seconds
    public var autoLockOnDistance: Bool
    public var restoreOnReturn: Bool

    public init(
        enabled: Bool = false,
        targetDeviceName: String? = nil,
        targetDeviceIdentifier: String? = nil,
        rssiThresholdDbm: Int = -80,
        gracePeriodSeconds: Int = 10,
        autoLockOnDistance: Bool = true,
        restoreOnReturn: Bool = true
    ) {
        self.enabled = enabled
        self.targetDeviceName = targetDeviceName
        self.targetDeviceIdentifier = targetDeviceIdentifier
        self.rssiThresholdDbm = rssiThresholdDbm
        self.gracePeriodSeconds = gracePeriodSeconds
        self.autoLockOnDistance = autoLockOnDistance
        self.restoreOnReturn = restoreOnReturn
    }
}

/// Pure state machine evaluator for Bluetooth proximity
public struct ProximityEvaluator {
    /// Evaluates signal strength and duration out of range
    public static func evaluateProximity(
        currentRssi: Int?,
        secondsOutOfRange: Int,
        config: ProximityConfig,
        isCurrentlyLocked: Bool
    ) -> ProximityAction {
        guard config.enabled else { return .none }

        // Case 1: Device is visible with strong signal
        if let rssi = currentRssi, rssi >= config.rssiThresholdDbm {
            if isCurrentlyLocked && config.restoreOnReturn {
                return .restoreSession
            }
            return .none
        }

        // Case 2: Device is out of range or signal below threshold
        let isBelowThreshold = (currentRssi != nil && currentRssi! < config.rssiThresholdDbm)
        let isAbsent = (currentRssi == nil)

        if (isBelowThreshold || isAbsent) && !isCurrentlyLocked && config.autoLockOnDistance {
            if secondsOutOfRange >= config.gracePeriodSeconds {
                let reason = isAbsent ?
                    "Bluetooth device out of range for \(secondsOutOfRange)s (Grace: \(config.gracePeriodSeconds)s)" :
                    "Bluetooth RSSI (\(currentRssi!) dBm) dropped below threshold (\(config.rssiThresholdDbm) dBm) for \(secondsOutOfRange)s"
                return .lockSession(reason: reason)
            }
        }

        return .none
    }
}
