import Foundation
import CoreBluetooth
import PostureFlowShared

/// Monitors Bluetooth signal strength (RSSI) to paired device for Walk-Away Auto-Lock on macOS
public final class ProximityEngine: NSObject, CBCentralManagerDelegate, @unchecked Sendable {
    private var centralManager: CBCentralManager?
    private var config: ProximityConfig = ProximityConfig()
    private var isCurrentlyLocked: Bool = false
    private var secondsOutOfRange: Int = 0
    private var lastSeenDate: Date?
    private var timer: Timer?
    private var onAction: (@Sendable (ProximityAction) -> Void)?
    private let queue = DispatchQueue(label: "com.postureflow.proximity", qos: .utility)

    @Published public private(set) var discoveredDevices: [ProximityDevice] = []
    @Published public private(set) var currentTrackedRSSI: Int?

    public override init() {
        super.init()
    }

    public func start(
        config: ProximityConfig,
        onAction: @escaping @Sendable (ProximityAction) -> Void
    ) {
        self.config = config
        self.onAction = onAction
        self.isCurrentlyLocked = false
        self.secondsOutOfRange = 0

        guard config.enabled else {
            stop()
            return
        }

        if centralManager == nil {
            centralManager = CBCentralManager(delegate: self, queue: queue)
        } else if centralManager?.state == .poweredOn {
            startScanning()
        }

        startWatchdogTimer()
    }

    public func stop() {
        timer?.invalidate()
        timer = nil
        centralManager?.stopScan()
        currentTrackedRSSI = nil
    }

    private func startScanning() {
        centralManager?.scanForPeripherals(
            withServices: nil,
            options: [CBCentralManagerScanOptionAllowDuplicatesKey: true]
        )
    }

    private func startWatchdogTimer() {
        DispatchQueue.main.async { [weak self] in
            self?.timer?.invalidate()
            self?.timer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
                self?.tickWatchdog()
            }
        }
    }

    private func tickWatchdog() {
        queue.async { [weak self] in
            guard let self = self, self.config.enabled else { return }

            // Check how long since last seen
            if let lastSeen = self.lastSeenDate {
                let elapsed = Int(Date().timeIntervalSince(lastSeen))
                self.secondsOutOfRange = elapsed
                if elapsed > 3 {
                    self.currentTrackedRSSI = nil
                }
            } else {
                self.secondsOutOfRange += 1
            }

            let action = ProximityEvaluator.evaluateProximity(
                currentRssi: self.currentTrackedRSSI,
                secondsOutOfRange: self.secondsOutOfRange,
                config: self.config,
                isCurrentlyLocked: self.isCurrentlyLocked
            )

            switch action {
            case .lockSession:
                self.isCurrentlyLocked = true
                self.onAction?(action)
            case .restoreSession:
                self.isCurrentlyLocked = false
                self.onAction?(action)
            case .none:
                break
            }
        }
    }

    // MARK: - CBCentralManagerDelegate

    public func centralManagerDidUpdateState(_ central: CBCentralManager) {
        if central.state == .poweredOn && config.enabled {
            startScanning()
        }
    }

    public func centralManager(
        _ central: CBCentralManager,
        didDiscover peripheral: CBPeripheral,
        advertisementData: [String: Any],
        rssi RSSI: NSNumber
    ) {
        let name = peripheral.name ?? (advertisementData[CBAdvertisementDataLocalNameKey] as? String) ?? "Unknown BLE Device"
        let identifier = peripheral.identifier.uuidString
        let rssiVal = RSSI.intValue

        // Check if this matches target
        let matchesTarget = (config.targetDeviceIdentifier == identifier) ||
                            (config.targetDeviceName != nil && config.targetDeviceName == name)

        if matchesTarget {
            self.lastSeenDate = Date()
            self.secondsOutOfRange = 0
            self.currentTrackedRSSI = rssiVal
        }

        // Keep list of discovered devices
        DispatchQueue.main.async {
            if let idx = self.discoveredDevices.firstIndex(where: { $0.identifier == identifier }) {
                self.discoveredDevices[idx].rssi = rssiVal
                self.discoveredDevices[idx].lastSeen = Date()
            } else if self.discoveredDevices.count < 30 {
                self.discoveredDevices.append(ProximityDevice(
                    name: name,
                    identifier: identifier,
                    rssi: rssiVal
                ))
            }
        }
    }
}
