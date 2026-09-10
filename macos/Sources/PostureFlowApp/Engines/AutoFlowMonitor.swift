import Foundation
import Network
import CoreWLAN
import PostureFlowShared

/// Monitors network interface transitions and Wi-Fi SSID associations via CoreWLAN & NWPathMonitor.
/// Operates completely event-driven with zero CPU polling overhead.
public final class AutoFlowMonitor: @unchecked Sendable {
    private let pathMonitor: NWPathMonitor
    private let queue = DispatchQueue(label: "com.postureflow.autoflow", qos: .utility)
    private let wifiClient = CWWiFiClient.shared()

    public typealias NetworkChangeHandler = @Sendable (_ ssid: String?, _ isVPNActive: Bool) -> Void
    private var onNetworkChange: NetworkChangeHandler?

    public init() {
        self.pathMonitor = NWPathMonitor()
    }

    /// Starts monitoring network changes
    public func start(handler: @escaping NetworkChangeHandler) {
        self.onNetworkChange = handler

        pathMonitor.pathUpdateHandler = { [weak self] path in
            guard let self = self else { return }
            
            // Check if any VPN tunnel is currently active (utun0, utun1, etc.)
            let isVPN = path.availableInterfaces.contains { interface in
                interface.name.hasPrefix("utun") || interface.type == .other
            }

            // Retrieve current Wi-Fi SSID
            let ssid = self.getCurrentSSID()

            self.onNetworkChange?(ssid, isVPN)
        }

        pathMonitor.start(queue: queue)
    }

    /// Stops network monitoring
    public func stop() {
        pathMonitor.cancel()
    }

    /// Queries the currently associated Wi-Fi SSID using CoreWLAN
    public func getCurrentSSID() -> String? {
        guard let interface = wifiClient.interface() else {
            return nil
        }
        return interface.ssid()
    }
}
