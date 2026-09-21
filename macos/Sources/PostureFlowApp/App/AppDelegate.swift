import Cocoa
import SwiftUI

/// Lifecycle delegate for the PostureFlow macOS application.
public final class AppDelegate: NSObject, NSApplicationDelegate {
    private var settingsWindow: NSWindow?

    public func applicationDidFinishLaunching(_ notification: Notification) {
        // Run as menu bar accessory (hides Dock icon)
        NSApp.setActivationPolicy(.accessory)
    }

    /// Opens or brings to focus the Settings & Security Cockpit window
    public func openSettingsWindow(state: PostureStateStore) {
        if let existing = settingsWindow {
            existing.makeKeyAndOrderFront(nil)
            NSApp.activate(ignoringOtherApps: true)
            return
        }

        let contentView = SettingsView(state: state)
        let hostingController = NSHostingController(rootView: contentView)

        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 820, height: 560),
            styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
            backing: .buffered,
            defer: false
        )
        window.title = "PostureFlow"
        window.titleVisibility = .visible
        window.titlebarAppearsTransparent = false
        window.toolbarStyle = .unified
        window.minSize = NSSize(width: 740, height: 500)
        window.center()
        window.contentViewController = hostingController
        window.isReleasedWhenClosed = false

        self.settingsWindow = window
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }
}
