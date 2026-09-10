import Foundation
import PostureFlowShared

/// Asynchronous typed client for communicating with the root helper daemon via Mach XPC.
public final class XPCClient: @unchecked Sendable {
    public static let shared = XPCClient()

    private var connection: NSXPCConnection?
    private let queue = DispatchQueue(label: "com.postureflow.xpcclient")

    public init() {}

    private func getOrCreateConnection() -> NSXPCConnection {
        if let conn = self.connection {
            return conn
        }

        let conn = NSXPCConnection(machServiceName: PostureFlowIPCConstants.machServiceName)
        conn.remoteObjectInterface = NSXPCInterface(with: PostureFlowHelperProtocol.self)
        
        conn.interruptionHandler = { [weak self] in
            NSLog("[XPCClient] Connection to helper interrupted")
            self?.connection = nil
        }
        
        conn.invalidationHandler = { [weak self] in
            NSLog("[XPCClient] Connection to helper invalidated")
            self?.connection = nil
        }

        conn.resume()
        self.connection = conn
        return conn
    }

    /// Queries daemon version
    public func getVersion() async throws -> String {
        let conn = getOrCreateConnection()
        return try await withCheckedThrowingContinuation { continuation in
            let helper = conn.remoteObjectProxyWithErrorHandler { error in
                continuation.resume(throwing: error)
            } as? PostureFlowHelperProtocol

            helper?.getVersion { version in
                continuation.resume(returning: version)
            }
        }
    }

    /// Queries live system state
    public func querySystemState() async throws -> (mode: PostureMode, isFirewallActive: Bool, isLowPower: Bool, isStealth: Bool) {
        let conn = getOrCreateConnection()
        return try await withCheckedThrowingContinuation { continuation in
            let helper = conn.remoteObjectProxyWithErrorHandler { error in
                continuation.resume(throwing: error)
            } as? PostureFlowHelperProtocol

            helper?.querySystemState { modeStr, isPF, isLowPower, isStealth, error in
                if let err = error {
                    continuation.resume(throwing: NSError(domain: "PostureFlow", code: -1, userInfo: [NSLocalizedDescriptionKey: err]))
                } else {
                    let mode = PostureMode(rawValue: modeStr) ?? .work
                    continuation.resume(returning: (mode, isPF, isLowPower, isStealth))
                }
            }
        }
    }

    /// Instructs the daemon to switch security posture
    public func applyPosture(mode: PostureMode) async throws -> Bool {
        let conn = getOrCreateConnection()
        return try await withCheckedThrowingContinuation { continuation in
            let helper = conn.remoteObjectProxyWithErrorHandler { error in
                continuation.resume(throwing: error)
            } as? PostureFlowHelperProtocol

            helper?.applyPosture(mode: mode.rawValue) { success, error in
                if let err = error {
                    continuation.resume(throwing: NSError(domain: "PostureFlow", code: -1, userInfo: [NSLocalizedDescriptionKey: err]))
                } else {
                    continuation.resume(returning: success)
                }
            }
        }
    }

    /// Restores factory firewall state and disables lockdown
    public func restoreDefaults() async throws -> Bool {
        let conn = getOrCreateConnection()
        return try await withCheckedThrowingContinuation { continuation in
            let helper = conn.remoteObjectProxyWithErrorHandler { error in
                continuation.resume(throwing: error)
            } as? PostureFlowHelperProtocol

            helper?.restoreFirewallDefaults { success, error in
                if let err = error {
                    continuation.resume(throwing: NSError(domain: "PostureFlow", code: -1, userInfo: [NSLocalizedDescriptionKey: err]))
                } else {
                    continuation.resume(returning: success)
                }
            }
        }
    }
}
