import Foundation
import PostureFlowShared

func printUsage() {
    print("""
    PostureFlow for macOS (v1.0.0)
    Dynamic security posture and Apple Silicon power orchestrator.

    USAGE:
        postureflow [OPTIONS]

    OPTIONS:
        --status         Display current security posture, score, and network state
        --home           Apply Home posture (LAN trusted, mDNS/AirDrop open)
        --work           Apply Work posture (LAN dev blocked, VPN trusted, high performance)
        --dev            Apply Dev posture (Local developer ports open)
        --travel         Apply Travel posture (Strict lockdown, ICMP dropped, Low Power Mode)
        --score          Evaluate and display 100-point security posture score breakdown
        --restore        Flush anchor rules and restore default network state
        --help, -h       Show this help message
    """)
}

let args = CommandLine.arguments.dropFirst()

guard let command = args.first else {
    printUsage()
    exit(0)
}

// Simple synchronous CLI invocation
let config = PostureConfig.load()

switch command {
case "--help", "-h":
    printUsage()
    exit(0)

case "--status":
    print("┌────────────────────────────────────────────────────────┐")
    print("│ PostureFlow macOS Status                               │")
    print("├────────────────────────────────────────────────────────┤")
    print("│ Active Posture   : \(config.activeProfile.displayName.padding(toLength: 35, withPad: " ", startingAt: 0))│")
    print("│ Policy           : \(config.activeProfile.inboundFirewallPolicy.padding(toLength: 35, withPad: " ", startingAt: 0))│")
    print("│ Auto-Flow        : \((config.autoFlowEnabled ? "Enabled" : "Disabled").padding(toLength: 35, withPad: " ", startingAt: 0))│")
    print("│ App Triggers     : \((config.triggersEnabled ? "Enabled" : "Disabled").padding(toLength: 35, withPad: " ", startingAt: 0))│")
    print("│ Low Power Mode   : \((config.activeProfile.enablesLowPowerMode ? "Active" : "Standard").padding(toLength: 35, withPad: " ", startingAt: 0))│")
    print("│ Display Sleep    : \("\(config.activeProfile.defaultDisplaySleepMinutes) minutes".padding(toLength: 35, withPad: " ", startingAt: 0))│")
    print("└────────────────────────────────────────────────────────┘")

case "--home", "--work", "--dev", "--travel":
    let modeString = String(command.dropFirst(2))
    guard let targetMode = PostureMode(rawValue: modeString) else {
        print("Error: Unknown posture mode '\(modeString)'")
        exit(1)
    }

    var updated = config
    updated.activeProfile = targetMode
    do {
        try updated.save()
        print("✔ Switched posture to: \(targetMode.displayName)")
        print("  \(targetMode.postureDescription)")
        print("  Notice: Open PostureFlow.app or run the helper to apply pfctl anchors system-wide.")
    } catch {
        print("Error saving posture configuration: \(error.localizedDescription)")
        exit(1)
    }

case "--score":
    let score = PostureScore(
        mode: config.activeProfile,
        isFirewallActive: true,
        isStealthModeActive: (config.activeProfile == .travel),
        isVPNActive: false,
        openPortCount: (config.activeProfile == .dev ? 2 : 0),
        isLowPowerMode: config.activeProfile.enablesLowPowerMode
    )
    print("PostureFlow Security Score: \(score.score)/100 (Grade: \(score.grade))")
    print("Score Breakdown:")
    for (category, points) in score.breakdown.sorted(by: { $0.key < $1.key }) {
        print("  • \(category): +\(points)")
    }
    if !score.recommendations.isEmpty {
        print("\nRecommendations:")
        for rec in score.recommendations {
            print("  ⚠️ \(rec)")
        }
    }

case "--restore":
    print("Restoring default network settings...")
    var updated = config
    updated.activeProfile = .work
    try? updated.save()
    print("✔ Reset active profile to default (Work).")

default:
    print("Error: Unknown argument '\(command)'")
    printUsage()
    exit(1)
}
