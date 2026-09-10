import Foundation
import PostureFlowShared

/// Manages circadian time-based profile switching (e.g. Work hours vs Evening relax vs Night lockdown).
public final class CircadianScheduler: @unchecked Sendable {
    public typealias CircadianHandler = @Sendable (_ suggestedMode: PostureMode) -> Void

    private var timer: Timer?
    private var onScheduleShift: CircadianHandler?

    public init() {}

    /// Starts evaluation timer checking every 5 minutes
    public func start(handler: @escaping CircadianHandler) {
        self.onScheduleShift = handler

        // Run initial check
        evaluateSchedule()

        // Schedule timer on main runloop
        self.timer = Timer.scheduledTimer(withTimeInterval: 300, repeats: true) { [weak self] _ in
            self?.evaluateSchedule()
        }
    }

    /// Stops circadian schedule evaluation
    public func stop() {
        timer?.invalidate()
        timer = nil
    }

    /// Evaluates current hour and weekday to suggest posture
    public func evaluateSchedule() {
        let calendar = Calendar.current
        let now = Date()
        let hour = calendar.component(.hour, from: now)
        let weekday = calendar.component(.weekday, from: now) // 1 = Sunday, 7 = Saturday

        let isWeekend = (weekday == 1 || weekday == 7)

        let suggestedMode: PostureMode
        if isWeekend {
            // Weekend: Home default
            suggestedMode = .home
        } else {
            // Weekday
            switch hour {
            case 9..<18:
                // 9 AM to 6 PM: Work
                suggestedMode = .work
            case 18..<23:
                // 6 PM to 11 PM: Home / Dev
                suggestedMode = .home
            default:
                // 11 PM to 9 AM: Lockdown / Travel stealth
                suggestedMode = .travel
            }
        }

        onScheduleShift?(suggestedMode)
    }
}
