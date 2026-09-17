import Foundation

/// Represents a detected YubiKey or hardware security key
public struct YubikeyDevice: Identifiable, Codable, Sendable, Equatable {
    public var id: String { serial.isEmpty ? "\(vendorId):\(productId)" : serial }
    public let name: String
    public let vendorId: String // "1050" or "0x1050"
    public let productId: String
    public let serial: String

    public init(name: String, vendorId: String, productId: String, serial: String) {
        self.name = name
        self.vendorId = vendorId
        self.productId = productId
        self.serial = serial
    }

    /// True if device vendor matches Yubico (0x1050)
    public var isYubico: Bool {
        let v = vendorId.lowercased().trimmingCharacters(in: .whitespacesAndNewlines)
        return v == "1050" || v == "0x1050"
    }
}

/// Action determined by the physical tether state machine
public enum YubiKeyTetherAction: Equatable, Sendable {
    case none
    case lockAndDemote(targetProfile: PostureMode, reason: String)
    case restore(targetProfile: PostureMode)
}

/// Configuration for YubiKey Hardware Presence Tethering
public struct YubikeyTetherConfig: Codable, Sendable, Equatable {
    public var enabled: Bool
    public var lockOnRemoval: Bool
    public var demoteOnRemoval: Bool
    public var targetDemoteProfile: PostureMode
    public var restoreOnInsert: Bool
    public var targetSerial: String?

    public init(
        enabled: Bool = false,
        lockOnRemoval: Bool = true,
        demoteOnRemoval: Bool = true,
        targetDemoteProfile: PostureMode = .travel,
        restoreOnInsert: Bool = true,
        targetSerial: String? = nil
    ) {
        self.enabled = enabled
        self.lockOnRemoval = lockOnRemoval
        self.demoteOnRemoval = demoteOnRemoval
        self.targetDemoteProfile = targetDemoteProfile
        self.restoreOnInsert = restoreOnInsert
        self.targetSerial = targetSerial
    }
}

/// Pure helper for evaluating tether state transitions
public struct YubikeyTetherStateEvaluator {
    /// Evaluates USB presence transition between previous scan and current scan
    public static func evaluateTransition(
        wasPresent: Bool,
        isPresent: Bool,
        config: YubikeyTetherConfig,
        savedProfileBeforeDemote: PostureMode?
    ) -> YubiKeyTetherAction {
        guard config.enabled else { return .none }

        // Token Removal (wasPresent -> !isPresent)
        if wasPresent && !isPresent {
            let demote = config.demoteOnRemoval ? config.targetDemoteProfile : .travel
            return .lockAndDemote(
                targetProfile: demote,
                reason: "Hardware YubiKey physically removed. Locking macOS session and enforcing lockdown posture."
            )
        }

        // Token Insertion (!wasPresent -> isPresent)
        if !wasPresent && isPresent && config.restoreOnInsert {
            if let restoreProfile = savedProfileBeforeDemote {
                return .restore(targetProfile: restoreProfile)
            }
        }

        return .none
    }
}

/// System-level USB discovery for macOS
public struct YubikeyDetector {
    /// Scans macOS USB bus using system_profiler or sysfs inspection
    public static func detectYubikeys() -> [YubikeyDevice] {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/sbin/system_profiler")
        process.arguments = ["SPUSBDataType", "-json"]

        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = Pipe()

        do {
            try process.run()
            process.waitUntilExit()

            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            guard let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let usbData = json["SPUSBDataType"] as? [[String: Any]] else {
                return []
            }

            var devices: [YubikeyDevice] = []
            parseUSBTree(items: usbData, into: &devices)
            return devices
        } catch {
            return []
        }
    }

    private static func parseUSBTree(items: [[String: Any]], into results: inout [YubikeyDevice]) {
        for item in items {
            let name = (item["_name"] as? String) ?? "Unknown USB Device"
            let vendorId = (item["vendor_id"] as? String) ?? ""
            let productId = (item["product_id"] as? String) ?? ""
            let serial = (item["serial_num"] as? String) ?? ""

            // Yubico Vendor ID is 0x1050
            if vendorId.contains("1050") || name.lowercased().contains("yubikey") {
                results.append(YubikeyDevice(
                    name: name,
                    vendorId: vendorId,
                    productId: productId,
                    serial: serial
                ))
            }

            if let subItems = item["_items"] as? [[String: Any]] {
                parseUSBTree(items: subItems, into: &results)
            }
        }
    }
}
