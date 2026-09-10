// swift-tools-version: 5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "PostureFlow",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .library(
            name: "PostureFlowShared",
            targets: ["PostureFlowShared"]
        ),
        .executable(
            name: "PostureFlowApp",
            targets: ["PostureFlowApp"]
        ),
        .executable(
            name: "PostureFlowHelper",
            targets: ["PostureFlowHelper"]
        ),
        .executable(
            name: "postureflow",
            targets: ["postureflow-cli"]
        )
    ],
    dependencies: [],
    targets: [
        // Shared core library (models, configs, XPC protocol, scoring)
        .target(
            name: "PostureFlowShared",
            dependencies: [],
            path: "Sources/PostureFlowShared"
        ),
        
        // Native SwiftUI MenuBarExtra App
        .executableTarget(
            name: "PostureFlowApp",
            dependencies: ["PostureFlowShared"],
            path: "Sources/PostureFlowApp"
        ),
        
        // Root Privileged Helper Daemon (PacketFilter pfctl, pmset, XPC server)
        .executableTarget(
            name: "PostureFlowHelper",
            dependencies: ["PostureFlowShared"],
            path: "Sources/PostureFlowHelper"
        ),
        
        // Terminal CLI utility for macOS
        .executableTarget(
            name: "postureflow-cli",
            dependencies: ["PostureFlowShared"],
            path: "Sources/postureflow-cli"
        ),
        
        // Tests
        .testTarget(
            name: "PostureFlowSharedTests",
            dependencies: ["PostureFlowShared"],
            path: "Tests/PostureFlowSharedTests"
        ),
        .testTarget(
            name: "PostureFlowHelperTests",
            dependencies: ["PostureFlowShared", "PostureFlowHelper"],
            path: "Tests/PostureFlowHelperTests"
        )
    ]
)
