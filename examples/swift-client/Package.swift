// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "swift-client",
    platforms: [.macOS(.v13)],
    dependencies: [
        .package(path: "../../swift"),
    ],
    targets: [
        .executableTarget(
            name: "swift-client",
            dependencies: [
                .product(name: "DarwinRuntime", package: "swift"),
            ],
            path: "Sources",
            linkerSettings: [
                .unsafeFlags(["-L../../target/debug"]),
                .unsafeFlags(["-L../../target/release"]),
            ]
        ),
    ]
)
