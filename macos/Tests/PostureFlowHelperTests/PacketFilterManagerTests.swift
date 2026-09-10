import XCTest
@testable import PostureFlowShared
@testable import PostureFlowHelper

final class PacketFilterManagerTests: XCTestCase {
    let pfManager = PacketFilterManager()

    func testTravelRulesIncludeStealthMode() {
        let rules = pfManager.generateRules(for: .travel)
        let joined = rules.joined(separator: "\n")

        // Must drop ICMP
        XCTAssertTrue(joined.contains("block in quick proto icmp all"))
        XCTAssertTrue(joined.contains("block in quick proto ipv6-icmp all"))
        // Must block in log all
        XCTAssertTrue(joined.contains("block in log all"))
        // Must pass utun+ (VPN)
        XCTAssertTrue(joined.contains("pass in quick on utun+ all"))
    }

    func testDevRulesPermitDevPorts() {
        let devPorts = [3000, 5173, 8080]
        let rules = pfManager.generateRules(for: .dev, devPorts: devPorts)
        let joined = rules.joined(separator: "\n")

        XCTAssertTrue(joined.contains("pass in proto tcp to any port { 3000 5173 8080 } keep state"))
        XCTAssertTrue(joined.contains("pass in quick on utun+ all"))
    }

    func testWorkRulesBlockDevPortsFromLAN() {
        let devPorts = [3000, 8080]
        let rules = pfManager.generateRules(for: .work, devPorts: devPorts)
        let joined = rules.joined(separator: "\n")

        XCTAssertTrue(joined.contains("block in proto tcp to any port { 3000 8080 }"))
        XCTAssertTrue(joined.contains("pass in proto tcp to any port 631")) // CUPS printing
        XCTAssertTrue(joined.contains("pass in quick on utun+ all"))
    }

    func testHomeRulesPermitBonjourAndAirDrop() {
        let rules = pfManager.generateRules(for: .home)
        let joined = rules.joined(separator: "\n")

        XCTAssertTrue(joined.contains("pass in proto udp to any port 5353")) // mDNS
        XCTAssertTrue(joined.contains("pass in proto udp to any port 27031:27040")) // Steam
        XCTAssertTrue(joined.contains("block in log all")) // Block WAN inbound
    }
}
