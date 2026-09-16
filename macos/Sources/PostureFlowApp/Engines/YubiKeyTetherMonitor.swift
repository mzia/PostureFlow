import Foundation
import PostureFlowShared

/// Background presence monitor for YubiKey physical hardware tokens on macOS
public final class YubiKeyTetherMonitor: @unchecked Sendable {
    private var timer: Timer?
    private let queue = DispatchQueue(label: "com.postureflow.yubikeytether", qos: .utility)
    private var wasPresent: Bool = false
    private var savedProfileBeforeDemote: PostureMode?

    public init() {}

    /// Starts polling USB presence every `interval` seconds (default 2s)
    public func start(
        interval: TimeInterval = 2.0,
        configProvider: @escaping @Sendable () -> YubikeyTetherConfig,
        onAction: @escaping @Sendable (YubiKeyTetherAction) -> Void
    ) {
        stop()

        // Initial detection
        let initialDevices = YubikeyDetector.detectYubikeys()
        self.wasPresent = !initialDevices.isEmpty

        DispatchQueue.main.async { [weak self] in
            guard let self = self else { return }
            self.timer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { [weak self] _ in
                guard let self = self else { return }
                self.queue.async {
                    let config = configProvider()
                    guard config.enabled else { return }

                    let currentDevices = YubikeyDetector.detectYubikeys()
                    let isPresent: Bool
                    if let targetSerial = config.targetSerial, !targetSerial.isEmpty {
                        isPresent = currentDevices.contains { $0.serial == targetSerial }
                    } else {
                        isPresent = !currentDevices.isEmpty
                    }

                    let action = YubikeyTetherStateEvaluator.evaluateTransition(
                        wasPresent: self.wasPresent,
                        isPresent: isPresent,
                        config: config,
                        savedProfileBeforeDemote: self.savedProfileBeforeDemote
                    )

                    self.wasPresent = isPresent

                    switch action {
                    case .lockAndDemote:
                        onAction(action)
                    case .restore:
                        self.savedProfileBeforeDemote = nil
                        onAction(action)
                    case .none:
                        break
                    }
                }
            }
        }
    }

    public func setSavedProfileBeforeDemote(_ profile: PostureMode?) {
        self.savedProfileBeforeDemote = profile
    }

    public func stop() {
        DispatchQueue.main.async { [weak self] in
            self?.timer?.invalidate()
            self?.timer = nil
        }
    }

    deinit {
        stop()
    }
}
