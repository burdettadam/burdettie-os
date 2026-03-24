// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "vm-image",
    platforms: [
        .macOS(.v13)
    ],
    targets: [
        .executableTarget(
            name: "darwin-vm",
            path: "Sources",
            linkerSettings: [
                .linkedFramework("Virtualization")
            ]
        )
    ]
)
