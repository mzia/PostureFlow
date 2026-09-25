import Cocoa
import SwiftUI

/// Lifecycle delegate for the PostureFlow macOS application.
@MainActor
public final class AppDelegate: NSObject, NSApplicationDelegate {
    private var settingsWindow: NSWindow?
    private var globalEventMonitor: Any?
    private var localEventMonitor: Any?
    private var isHotkeysConfigured: Bool = false

    public func applicationDidFinishLaunching(_ notification: Notification) {
        // Load security shield application icon if available
        if let iconPath = Bundle.main.path(forResource: "AppIcon", ofType: "icns") ??
                          Bundle.main.path(forResource: "AppIcon", ofType: "png"),
           let iconImage = NSImage(contentsOfFile: iconPath) {
            NSApp.applicationIconImage = iconImage
        }

        // Run as menu bar accessory (hides Dock icon)
        NSApp.setActivationPolicy(.accessory)
    }

    /// Sets up global shortcuts linked to the PostureStateStore
    public func setup(state: PostureStateStore) {
        guard !isHotkeysConfigured else { return }
        isHotkeysConfigured = true

        guard state.config.globalShortcutsEnabled else { return }

        let targetFlags: NSEvent.ModifierFlags = [.control, .option, .command]

        let handleKey: (NSEvent) -> Void = { event in
            let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
            guard flags == targetFlags else { return }

            switch event.charactersIgnoringModifiers?.lowercased() {
            case "1":
                Task { @MainActor in await state.switchTo(.home) }
            case "2":
                Task { @MainActor in await state.switchTo(.work) }
            case "3":
                Task { @MainActor in await state.switchTo(.dev) }
            case "4":
                Task { @MainActor in await state.switchTo(.travel) }
            case "k":
                Task { @MainActor in await state.engageEmergencyKillSwitch() }
            default:
                break
            }
        }

        self.globalEventMonitor = NSEvent.addGlobalMonitorForEvents(matching: .keyDown) { event in
            handleKey(event)
        }

        self.localEventMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
            handleKey(event)
            return event
        }
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
