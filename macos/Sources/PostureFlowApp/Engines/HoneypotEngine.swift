import Foundation
import Network
import PostureFlowShared

/// Manages asynchronous decoy honeypot TCP listeners using Apple's Network.framework
public final class HoneypotEngine: @unchecked Sendable {
    private var listeners: [Int: NWListener] = [:]
    private let queue = DispatchQueue(label: "com.postureflow.honeypot", qos: .utility)
    private var onIncident: (@Sendable (HoneypotIncident) -> Void)?

    public init() {}

    /// Starts or updates decoy TCP trap listeners based on current posture and config
    public func update(
        config: HoneypotConfig,
        currentMode: PostureMode,
        onIncident: @escaping @Sendable (HoneypotIncident) -> Void
    ) {
        self.onIncident = onIncident
        queue.async { [weak self] in
            guard let self = self else { return }

            guard config.enabled && config.activeInProfiles.contains(currentMode) else {
                self.stopAll()
                return
            }

            let desiredPorts = Set(config.trapPorts)
            let currentPorts = Set(self.listeners.keys)

            // Stop listeners on ports no longer wanted
            for port in currentPorts.subtracting(desiredPorts) {
                self.listeners[port]?.cancel()
                self.listeners.removeValue(forKey: port)
            }

            // Start listeners on new desired ports
            for port in desiredPorts.subtracting(currentPorts) {
                self.startListener(port: port, config: config)
            }
        }
    }

    private func startListener(port: Int, config: HoneypotConfig) {
        guard let nwPort = NWEndpoint.Port(rawValue: UInt16(port)) else { return }

        let tcpOptions = NWProtocolTCP.Options()
        let params = NWParameters(tls: nil, tcp: tcpOptions)
        // Allow listening on local interfaces
        params.allowLocalEndpointReuse = true

        do {
            let listener = try NWListener(using: params, on: nwPort)
            listener.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    NSLog("[HoneypotEngine] Decoy trap armed on TCP port %d", port)
                case .failed(let error):
                    NSLog("[HoneypotEngine] Failed to bind trap port %d: %@", port, error.localizedDescription)
                default:
                    break
                }
            }

            listener.newConnectionHandler = { [weak self] connection in
                self?.handleIncomingConnection(connection, trapPort: port, config: config)
            }

            listener.start(queue: queue)
            self.listeners[port] = listener
        } catch {
            NSLog("[HoneypotEngine] Error creating listener on port %d: %@", port, error.localizedDescription)
        }
    }

    private func handleIncomingConnection(_ connection: NWConnection, trapPort: Int, config: HoneypotConfig) {
        connection.start(queue: queue)

        var peerIP = "unknown"
        if case .hostPort(let host, _) = connection.endpoint {
            switch host {
            case .ipv4(let ipv4):
                peerIP = "\(ipv4)"
            case .ipv6(let ipv6):
                peerIP = "\(ipv6)"
            case .name(let name, _):
                peerIP = name
            @unknown default:
                break
            }
        }

        NSLog("[HoneypotEngine] 🚨 INTRUSION CAUGHT on trap port %d from peer IP: %@", trapPort, peerIP)

        let incident = HoneypotIncident(
            peerIP: peerIP,
            trapPort: trapPort,
            interface: "en0",
            blocked: config.autoBlockOffenders
        )

        // Record incident in persistent ledger
        HoneypotLedger.recordIncident(incident)

        // Notify app
        self.onIncident?(incident)

        // Terminate decoy connection cleanly after logging
        connection.cancel()
    }

    public func stopAll() {
        for (_, listener) in listeners {
            listener.cancel()
        }
        listeners.removeAll()
        NSLog("[HoneypotEngine] All decoy honeypot trap ports disarmed.")
    }

    deinit {
        stopAll()
    }
}
