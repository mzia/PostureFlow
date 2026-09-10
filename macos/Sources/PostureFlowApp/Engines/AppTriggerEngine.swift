import Foundation
import AppKit
import PostureFlowShared

/// Monitors application lifecycle events using native NSWorkspace notifications.
/// Automatically triggers posture switches when high-priority apps launch or quit.
public final class AppTriggerEngine: @unchecked Sendable {
    public typealias AppTriggerHandler = @Sendable (_ bundleID: String, _ isRunning: Bool) -> Void

    private var observers: [NSObjectProtocol] = []
    private var onTrigger: AppTriggerHandler?

    public init() {}

    /// Begins listening to application launch and termination notifications
    public func start(handler: @escaping AppTriggerHandler) {
        self.onTrigger = handler
        let center = NSWorkspace.shared.notificationCenter

        // Application Did Launch
        let launchObserver = center.addObserver(
            forName: NSWorkspace.didLaunchApplicationNotification,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            guard let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication,
                  let bundleID = app.bundleIdentifier else {
                return
            }
            self?.onTrigger?(bundleID, true)
        }

        // Application Did Terminate
        let terminateObserver = center.addObserver(
            forName: NSWorkspace.didTerminateApplicationNotification,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            guard let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication,
                  let bundleID = app.bundleIdentifier else {
                return
            }
            self?.onTrigger?(bundleID, false)
        }

        observers = [launchObserver, terminateObserver]
    }

    /// Stops observing application notifications
    public func stop() {
        let center = NSWorkspace.shared.notificationCenter
        for observer in observers {
            center.removeObserver(observer)
        }
        observers.removeAll()
    }

    /// Inspects currently running applications
    public static func getRunningBundleIdentifiers() -> Set<String> {
        let apps = NSWorkspace.shared.runningApplications
        return Set(apps.compactMap { $0.bundleIdentifier })
    }
}
