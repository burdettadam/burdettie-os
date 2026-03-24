// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "DarwinRuntime",
    products: [
        .library(name: "DarwinRuntime", targets: ["DarwinRuntime"]),
    ],
    targets: [
        // C module wrapping the cbindgen-generated header + dylib
        .target(
            name: "CDarwinRuntime",
            path: "Sources/CDarwinRuntime",
            publicHeadersPath: "include",
            linkerSettings: [
                .unsafeFlags([
                    "-L../target/debug",
                    "-L../target/release",
                ]),
            ]
        ),
        // Idiomatic Swift wrapper
        .target(
            name: "DarwinRuntime",
            dependencies: ["CDarwinRuntime"],
            path: "Sources/DarwinRuntime"
        ),
        // Tests
        .testTarget(
            name: "DarwinRuntimeTests",
            dependencies: ["DarwinRuntime"],
            path: "Tests/DarwinRuntimeTests",
            linkerSettings: [
                .unsafeFlags([
                    "-L../target/debug",
                    "-L../target/release",
                ]),
            ]
        ),
    ]
)
