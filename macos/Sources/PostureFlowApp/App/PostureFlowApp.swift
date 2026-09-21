import SwiftUI
import PostureFlowShared

@main
struct PostureFlowApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    @StateObject private var state = PostureStateStore()

    var body: some Scene {
        MenuBarExtra {
            VStack(spacing: 0) {
                StatusCardView(state: state)
                    .padding(.bottom, 6)

                MenuContentView(state: state) {
                    appDelegate.openSettingsWindow(state: state)
                }
            }
            .padding(10)
            .frame(width: 340)
        } label: {
            Image(systemName: state.menuBarShieldIcon)
                .symbolRenderingMode(.hierarchical)
        }
        .menuBarExtraStyle(.window)
    }
}
