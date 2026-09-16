import XCTest
@testable import PostureFlowShared

final class HardwareDefenseTests: XCTestCase {

    // MARK: - YubiKey Hardware Presence Tethering Tests

    func testYubikeyTetherDisabledProducesNoAction() {
        let config = YubikeyTetherConfig(enabled: false)
        let action = YubikeyTetherStateEvaluator.evaluateTransition(
            wasPresent: true,
            isPresent: false,
            config: config,
            savedProfileBeforeDemote: .work
        )
        XCTAssertEqual(action, .none)
    }

    func testYubikeyTetherRemovalTriggersDemoteAndLock() {
        let config = YubikeyTetherConfig(
            enabled: true,
            lockOnRemoval: true,
            demoteOnRemoval: true,
            targetDemoteProfile: .travel
        )
        let action = YubikeyTetherStateEvaluator.evaluateTransition(
            wasPresent: true,
            isPresent: false,
            config: config,
            savedProfileBeforeDemote: .work
        )

        switch action {
        case .lockAndDemote(let targetProfile, let reason):
            XCTAssertEqual(targetProfile, .travel)
            XCTAssertTrue(reason.contains("Hardware YubiKey physically removed"))
        default:
            XCTFail("Expected lockAndDemote action on token removal")
        }
    }

    func testYubikeyTetherInsertionTriggersRestore() {
        let config = YubikeyTetherConfig(
            enabled: true,
            restoreOnInsert: true
        )
        let action = YubikeyTetherStateEvaluator.evaluateTransition(
            wasPresent: false,
            isPresent: true,
            config: config,
            savedProfileBeforeDemote: .dev
        )

        XCTAssertEqual(action, .restore(targetProfile: .dev))
    }

    // MARK: - Decoy Honeypot Traps Tests

    func testHoneypotDefaultConfig() {
        let config = HoneypotConfig()
        XCTAssertTrue(config.enabled)
        XCTAssertEqual(config.trapPorts, [2222, 8080, 4450])
        XCTAssertTrue(config.autoBlockOffenders)
        XCTAssertTrue(config.activeInProfiles.contains(.travel))
        XCTAssertTrue(config.activeInProfiles.contains(.home))
    }

    func testHoneypotIncidentJSONSerialization() throws {
        let incident = HoneypotIncident(
            peerIP: "192.168.1.189",
            trapPort: 2222,
            interface: "en0",
            blocked: true
        )

        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        let data = try encoder.encode(incident)

        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        let decoded = try decoder.decode(HoneypotIncident.self, from: data)

        XCTAssertEqual(decoded.id, incident.id)
        XCTAssertEqual(decoded.peerIP, "192.168.1.189")
        XCTAssertEqual(decoded.trapPort, 2222)
        XCTAssertTrue(decoded.blocked)
    }

    // MARK: - Sensor Privacy Tests

    func testSensorPrivacyConfigDefaults() {
        let config = SensorPrivacyConfig()
        XCTAssertTrue(config.muteMicOnTravel)
        XCTAssertTrue(config.muteMicOnLock)
        XCTAssertTrue(config.cameraBlockedOnTravel)
        XCTAssertFalse(config.emergencyKillActive)
    }

    func testSensorPrivacyReportModel() {
        let report = SensorPrivacyReport(
            cameraBlocked: true,
            microphoneMuted: true,
            locationBlocked: true,
            inputVolumePercent: 0,
            emergencyKillActive: true
        )
        XCTAssertTrue(report.cameraBlocked)
        XCTAssertTrue(report.microphoneMuted)
        XCTAssertEqual(report.inputVolumePercent, 0)
        XCTAssertTrue(report.emergencyKillActive)
    }

    // MARK: - Bluetooth Walk-Away Proximity Tests

    func testProximityInThresholdProducesNoAction() {
        let config = ProximityConfig(
            enabled: true,
            rssiThresholdDbm: -80,
            gracePeriodSeconds: 10
        )
        // RSSI is -65 dBm (stronger than -80 dBm)
        let action = ProximityEvaluator.evaluateProximity(
            currentRssi: -65,
            secondsOutOfRange: 0,
            config: config,
            isCurrentlyLocked: false
        )
        XCTAssertEqual(action, .none)
    }

    func testProximityOutOfRangeUnderGracePeriodProducesNoAction() {
        let config = ProximityConfig(
            enabled: true,
            rssiThresholdDbm: -80,
            gracePeriodSeconds: 10
        )
        // Device is weak (-85 dBm) but only for 4 seconds (< 10s grace)
        let action = ProximityEvaluator.evaluateProximity(
            currentRssi: -85,
            secondsOutOfRange: 4,
            config: config,
            isCurrentlyLocked: false
        )
        XCTAssertEqual(action, .none)
    }

    func testProximityWalkAwayAfterGracePeriodTriggersLock() {
        let config = ProximityConfig(
            enabled: true,
            rssiThresholdDbm: -80,
            gracePeriodSeconds: 10
        )
        // Device out of range for 12 seconds (> 10s grace)
        let action = ProximityEvaluator.evaluateProximity(
            currentRssi: nil,
            secondsOutOfRange: 12,
            config: config,
            isCurrentlyLocked: false
        )

        switch action {
        case .lockSession(let reason):
            XCTAssertTrue(reason.contains("Bluetooth device out of range for 12s"))
        default:
            XCTFail("Expected lockSession action after grace period expiration")
        }
    }

    func testProximityReturnInRangeTriggersRestore() {
        let config = ProximityConfig(
            enabled: true,
            rssiThresholdDbm: -80,
            restoreOnReturn: true
        )
        // Device returned with -60 dBm while currently locked
        let action = ProximityEvaluator.evaluateProximity(
            currentRssi: -60,
            secondsOutOfRange: 0,
            config: config,
            isCurrentlyLocked: true
        )
        XCTAssertEqual(action, .restoreSession)
    }

    // MARK: - PostureScore Hardware Defense Audit Tests

    func testPostureScoreIncludesHardwareDefensePoints() {
        let baselineScore = PostureScore(
            mode: .travel,
            isFirewallActive: true,
            isStealthModeActive: true,
            isVPNActive: true,
            openPortCount: 0,
            isLowPowerMode: true
        )

        let hardwareHardenedScore = PostureScore(
            mode: .travel,
            isFirewallActive: true,
            isStealthModeActive: true,
            isVPNActive: true,
            openPortCount: 0,
            isLowPowerMode: true,
            isHardwareTetherActive: true,
            isHoneypotActive: true,
            isSensorPrivacyActive: true,
            isProximityLockActive: true
        )

        XCTAssertNotNil(hardwareHardenedScore.breakdown["YubiKey Hardware Tether Active"])
        XCTAssertEqual(hardwareHardenedScore.breakdown["YubiKey Hardware Tether Active"], 5)
        XCTAssertNotNil(hardwareHardenedScore.breakdown["Decoy Honeypot Traps Armed"])
        XCTAssertEqual(hardwareHardenedScore.breakdown["Decoy Honeypot Traps Armed"], 5)
        XCTAssertNotNil(hardwareHardenedScore.breakdown["Sensor Hardware Privacy Locked"])
        XCTAssertEqual(hardwareHardenedScore.breakdown["Sensor Hardware Privacy Locked"], 5)
        XCTAssertNotNil(hardwareHardenedScore.breakdown["Walk-Away Proximity Auto-Lock"])
        XCTAssertEqual(hardwareHardenedScore.breakdown["Walk-Away Proximity Auto-Lock"], 5)

        XCTAssertGreaterThanOrEqual(hardwareHardenedScore.score, baselineScore.score)
    }

    func testHardwareDefenseMasterConfigSerialization() throws {
        let config = HardwareDefenseConfig()
        let encoder = JSONEncoder()
        let data = try encoder.encode(config)

        let decoder = JSONDecoder()
        let loaded = try decoder.decode(HardwareDefenseConfig.self, from: data)

        XCTAssertEqual(loaded.yubikey.lockOnRemoval, true)
        XCTAssertEqual(loaded.honeypot.trapPorts, [2222, 8080, 4450])
        XCTAssertEqual(loaded.proximity.rssiThresholdDbm, -80)
    }
}
