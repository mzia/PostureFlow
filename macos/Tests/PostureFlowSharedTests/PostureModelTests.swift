import XCTest
@testable import PostureFlowShared

final class PostureModelTests: XCTestCase {
    func testPostureModesExist() {
        XCTAssertEqual(PostureMode.allCases.count, 4)
        XCTAssertEqual(PostureMode.home.rawValue, "home")
        XCTAssertEqual(PostureMode.work.rawValue, "work")
        XCTAssertEqual(PostureMode.dev.rawValue, "dev")
        XCTAssertEqual(PostureMode.travel.rawValue, "travel")
    }

    func testPostureModeAttributes() {
        XCTAssertEqual(PostureMode.home.sfSymbol, "house.fill")
        XCTAssertEqual(PostureMode.work.sfSymbol, "briefcase.fill")
        XCTAssertEqual(PostureMode.dev.sfSymbol, "chevron.left.forwardslash.chevron.right")
        XCTAssertEqual(PostureMode.travel.sfSymbol, "airplane")

        XCTAssertTrue(PostureMode.travel.enablesLowPowerMode)
        XCTAssertFalse(PostureMode.work.enablesLowPowerMode)
        XCTAssertFalse(PostureMode.home.enablesLowPowerMode)
    }

    func testPostureScoreCalculation() {
        // High security travel test
        let travelScore = PostureScore(
            mode: .travel,
            isFirewallActive: true,
            isStealthModeActive: true,
            isVPNActive: false,
            openPortCount: 0,
            isLowPowerMode: true
        )
        XCTAssertGreaterThanOrEqual(travelScore.score, 85)
        XCTAssertEqual(travelScore.grade, "A")

        // Inactive firewall penalty test
        let compromisedScore = PostureScore(
            mode: .dev,
            isFirewallActive: false,
            isStealthModeActive: false,
            isVPNActive: false,
            openPortCount: 6,
            isLowPowerMode: false
        )
        XCTAssertLessThan(compromisedScore.score, 50)
        XCTAssertEqual(compromisedScore.grade, "F")
        XCTAssertFalse(compromisedScore.recommendations.isEmpty)
    }

    func testDefaultConfigSerialization() throws {
        let config = PostureConfig()
        let encoder = JSONEncoder()
        let data = try encoder.encode(config)

        let decoder = JSONDecoder()
        let loaded = try decoder.decode(PostureConfig.self, data: data)
        XCTAssertEqual(loaded.activeProfile, .work)
        XCTAssertTrue(loaded.autoFlowEnabled)
        XCTAssertTrue(loaded.triggersEnabled)
        XCTAssertEqual(loaded.appTriggers["us.zoom.xos"], .work)
        XCTAssertEqual(loaded.appTriggers["com.apple.dt.Xcode"], .dev)
    }
}
