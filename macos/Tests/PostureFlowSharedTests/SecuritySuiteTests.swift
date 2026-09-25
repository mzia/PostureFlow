import XCTest
@testable import PostureFlowShared

final class SecuritySuiteTests: XCTestCase {

    // MARK: - 1. Anti-Evil Twin & BSSID Gateway Fingerprinting Tests

    func testTrustedNetworkMatchingFingerprint() {
        let trustedFP = NetworkFingerprint(
            ssid: "Corporate-Secure",
            bssid: "aa:bb:cc:dd:ee:ff",
            gatewayMAC: "11:22:33:44:55:66",
            targetMode: .work
        )
        let registry = ["Corporate-Secure": trustedFP]

        let result = AntiEvilTwinDetector.evaluate(
            currentSSID: "Corporate-Secure",
            currentBSSID: "AA:BB:CC:DD:EE:FF", // case insensitive check
            currentGatewayMAC: "11:22:33:44:55:66",
            trustedRegistry: registry
        )

        XCTAssertEqual(result, .trusted(fingerprint: trustedFP))
    }

    func testUnregisteredNetworkEvaluation() {
        let registry: [String: NetworkFingerprint] = [:]
        let result = AntiEvilTwinDetector.evaluate(
            currentSSID: "CoffeeShop-Guest",
            currentBSSID: "00:11:22:33:44:55",
            currentGatewayMAC: "66:77:88:99:aa:bb",
            trustedRegistry: registry
        )

        XCTAssertEqual(result, .unregistered(
            ssid: "CoffeeShop-Guest",
            bssid: "00:11:22:33:44:55",
            gatewayMAC: "66:77:88:99:aa:bb"
        ))
    }

    func testEvilTwinBSSIDMismatchDetected() {
        let trustedFP = NetworkFingerprint(
            ssid: "HomeOffice-5G",
            bssid: "10:20:30:40:50:60",
            gatewayMAC: "01:02:03:04:05:06",
            targetMode: .home
        )
        let registry = ["HomeOffice-5G": trustedFP]

        // Rogue access point spoofing SSID with different BSSID
        let result = AntiEvilTwinDetector.evaluate(
            currentSSID: "HomeOffice-5G",
            currentBSSID: "de:ad:be:ef:00:01", // Mismatch!
            currentGatewayMAC: "01:02:03:04:05:06",
            trustedRegistry: registry
        )

        switch result {
        case .evilTwinDetected(let ssid, let expB, let actB, _, _):
            XCTAssertEqual(ssid, "HomeOffice-5G")
            XCTAssertEqual(expB, "10:20:30:40:50:60")
            XCTAssertEqual(actB, "de:ad:be:ef:00:01")
        default:
            XCTFail("Expected .evilTwinDetected but got \(result)")
        }
    }

    func testEvilTwinGatewayMACMismatchDetected() {
        let trustedFP = NetworkFingerprint(
            ssid: "HQ-Work",
            bssid: "aa:11:bb:22:cc:33",
            gatewayMAC: "fe:ed:fa:ce:ca:fe",
            targetMode: .work
        )
        let registry = ["HQ-Work": trustedFP]

        // Rogue gateway / ARP spoofing attack
        let result = AntiEvilTwinDetector.evaluate(
            currentSSID: "HQ-Work",
            currentBSSID: "aa:11:bb:22:cc:33",
            currentGatewayMAC: "ba:ad:f0:0d:00:00", // Mismatch!
            trustedRegistry: registry
        )

        switch result {
        case .evilTwinDetected(let ssid, _, _, let expGW, let actGW):
            XCTAssertEqual(ssid, "HQ-Work")
            XCTAssertEqual(expGW, "fe:ed:fa:ce:ca:fe")
            XCTAssertEqual(actGW, "ba:ad:f0:0d:00:00")
        default:
            XCTFail("Expected evilTwinDetected due to gateway mismatch")
        }
    }

    // MARK: - 2. Credential Cloaking Engine Tests

    func testLeavingDevPostureTriggersCloaking() {
        let config = CredentialCloakConfig(
            enabled: true,
            purgeSSHAgentOnLeaveDev: true,
            cloakAWSCredentials: true,
            lockPasswordManagers: true
        )

        let status = CredentialCloakEngine.handlePostureChange(
            from: .dev,
            to: .travel,
            config: config
        )

        XCTAssertNotNil(status.lastActionTimestamp)
    }

    func testDisabledCloakingConfigProducesNoAction() {
        let config = CredentialCloakConfig(enabled: false)
        let status = CredentialCloakEngine.handlePostureChange(
            from: .dev,
            to: .work,
            config: config
        )

        XCTAssertNil(status.lastActionTimestamp)
    }

    // MARK: - 3. Profile-Aware Encrypted DNS Tests

    func testTravelModeEnforcesEncryptedDNSAndBlocksPort53() {
        let config = EncryptedDNSConfig(
            enabled: true,
            primaryServers: ["9.9.9.9", "149.112.112.112"],
            fallbackServers: ["1.1.1.1"],
            blockPlainPort53InTravel: true,
            dnsOverTLS: true,
            dnssec: true
        )

        let report = EncryptedDNSManager.evaluateDNSStatus(mode: .travel, config: config)
        XCTAssertTrue(report.isEncrypted)
        XCTAssertTrue(report.isPort53Blocked)
        XCTAssertTrue(report.dnssecEnforced)
        XCTAssertEqual(report.activeResolvers, ["9.9.9.9", "149.112.112.112"])
    }

    func testStandardWorkModeDNSWithDoTEnabled() {
        let config = EncryptedDNSConfig(
            enabled: true,
            blockPlainPort53InTravel: true,
            dnsOverTLS: true,
            dnssec: true
        )

        let report = EncryptedDNSManager.evaluateDNSStatus(mode: .work, config: config)
        XCTAssertTrue(report.isEncrypted)
        XCTAssertFalse(report.isPort53Blocked) // Port 53 only blocked in Travel
    }

    // MARK: - 4. PostureScore Enhanced Metrics Tests

    func testEvilTwinDeductsSeverePenalty() {
        let baseScore = PostureScore(
            mode: .travel,
            isFirewallActive: true,
            isStealthModeActive: true,
            isVPNActive: false,
            openPortCount: 0,
            isLowPowerMode: true,
            isEvilTwinDetected: false
        )

        let penalizedScore = PostureScore(
            mode: .travel,
            isFirewallActive: true,
            isStealthModeActive: true,
            isVPNActive: false,
            openPortCount: 0,
            isLowPowerMode: true,
            isEvilTwinDetected: true
        )

        XCTAssertEqual(baseScore.score - penalizedScore.score, 35)
        XCTAssertTrue(penalizedScore.recommendations.contains { $0.contains("CRITICAL: Rogue Wi-Fi AP") })
    }

    func testEncryptedDNSAndCloakingAwardBonusPoints() {
        let baseScore = PostureScore(
            mode: .work,
            isFirewallActive: true,
            isStealthModeActive: false,
            isVPNActive: true,
            openPortCount: 0,
            isLowPowerMode: false,
            isEncryptedDNSActive: false,
            isCredentialCloaked: false
        )

        let boostedScore = PostureScore(
            mode: .work,
            isFirewallActive: true,
            isStealthModeActive: false,
            isVPNActive: true,
            openPortCount: 0,
            isLowPowerMode: false,
            isEncryptedDNSActive: true,
            isCredentialCloaked: true
        )

        XCTAssertEqual(boostedScore.score - baseScore.score, 10) // 5 for DNS + 5 for Cloaking
        XCTAssertEqual(boostedScore.breakdown["Encrypted DNS & Anti-Leak Active"], 5)
        XCTAssertEqual(boostedScore.breakdown["Developer Credential Cloaking Active"], 5)
    }
}
