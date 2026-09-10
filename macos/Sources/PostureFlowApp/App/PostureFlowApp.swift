import SwiftUI
import PostureFlowShared

@main
struct PostureFlowApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    @StateObject private var state = PostureStateStore()

    var body: some Scene {
        MenuBarExtra {
            StatusCardView(state: state)
            Divider()
            MenuContentView(state: state) {
                appDelegate.openSettingsWindow(state: state)
            }
        } label: {
            Image(systemName: state.currentMode.sfSymbol)
                .symbolRenderingMode(.multicolor)
        }
        .menuBarExtraStyle(.window)
    }
}
