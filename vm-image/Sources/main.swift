import Foundation
import Virtualization

// MARK: - Configuration

/// Paths and resource configuration for the VM.
struct VMConfig {
    /// Path to the macOS restore IPSW image (downloaded or cached).
    let restoreImagePath: String
    /// Directory shared between host and guest containing runtime + fixtures.
    let sharedDirectory: String
    /// VM resource allocation.
    let cpuCount: Int
    let memorySize: UInt64  // bytes
    /// Path to store the VM bundle (disk image, aux storage, etc.).
    let bundlePath: String

    static func fromEnvironment() -> VMConfig {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        let projectRoot = ProcessInfo.processInfo.environment["DARWIN_LOADER_ROOT"]
            ?? FileManager.default.currentDirectoryPath

        return VMConfig(
            restoreImagePath: ProcessInfo.processInfo.environment["VM_IPSW"]
                ?? "\(home)/.burdettie-os/RestoreImage.ipsw",
            sharedDirectory: ProcessInfo.processInfo.environment["VM_SHARED_DIR"]
                ?? "\(projectRoot)/dist",
            cpuCount: Int(ProcessInfo.processInfo.environment["VM_CPUS"] ?? "2") ?? 2,
            memorySize: UInt64(ProcessInfo.processInfo.environment["VM_MEMORY_GB"] ?? "4")
                .map { $0 * 1024 * 1024 * 1024 } ?? 4 * 1024 * 1024 * 1024,
            bundlePath: ProcessInfo.processInfo.environment["VM_BUNDLE"]
                ?? "\(home)/.burdettie-os/vm-bundle"
        )
    }
}

// MARK: - VM Bundle on disk

/// Manages the persistent VM state (disk image, auxiliary storage, machine ID).
struct VMBundle {
    let path: String

    var diskImagePath: String { "\(path)/disk.img" }
    var auxiliaryStoragePath: String { "\(path)/auxiliary-storage" }
    var machineIdentifierPath: String { "\(path)/machine-id" }
    var hardwareModelPath: String { "\(path)/hardware-model" }

    var exists: Bool {
        FileManager.default.fileExists(atPath: diskImagePath)
    }

    func create(diskSizeGB: Int = 64) throws {
        try FileManager.default.createDirectory(
            atPath: path, withIntermediateDirectories: true)

        // Create a sparse disk image
        let diskURL = URL(fileURLWithPath: diskImagePath)
        FileManager.default.createFile(atPath: diskImagePath, contents: nil)
        let handle = try FileHandle(forWritingTo: diskURL)
        try handle.truncate(atOffset: UInt64(diskSizeGB) * 1024 * 1024 * 1024)
        try handle.close()

        print("  Created \(diskSizeGB)GB sparse disk image")
    }
}

// MARK: - VM Builder

/// Creates and configures a VZVirtualMachine.
@available(macOS 13.0, *)
class VMBuilder {
    let config: VMConfig
    let bundle: VMBundle

    init(config: VMConfig) {
        self.config = config
        self.bundle = VMBundle(path: config.bundlePath)
    }

    /// Download the macOS restore image if not already cached.
    func ensureRestoreImage() async throws -> URL {
        let url = URL(fileURLWithPath: config.restoreImagePath)
        if FileManager.default.fileExists(atPath: config.restoreImagePath) {
            print("  Using cached restore image: \(config.restoreImagePath)")
            return url
        }

        let cacheDir = URL(fileURLWithPath: config.restoreImagePath).deletingLastPathComponent()
        try FileManager.default.createDirectory(at: cacheDir, withIntermediateDirectories: true)

        print("  Fetching latest supported restore image URL...")
        let image = try await withCheckedThrowingContinuation {
            (cont: CheckedContinuation<VZMacOSRestoreImage, Error>) in
            VZMacOSRestoreImage.fetchLatestSupported { result in
                switch result {
                case .success(let img): cont.resume(returning: img)
                case .failure(let err): cont.resume(throwing: err)
                }
            }
        }

        let downloadURL = image.url

        print("  Downloading restore image (~13GB)...")
        print("  From: \(downloadURL)")
        print("  To:   \(config.restoreImagePath)")

        let (tmpURL, _) = try await URLSession.shared.download(from: downloadURL)
        try FileManager.default.moveItem(at: tmpURL, to: url)
        print("  Download complete.")
        return url
    }

    /// Install macOS into the VM bundle from the restore image.
    func install() async throws -> VZVirtualMachine {
        let restoreURL = try await ensureRestoreImage()

        print("  Loading restore image...")
        let restoreImage = try await withCheckedThrowingContinuation {
            (cont: CheckedContinuation<VZMacOSRestoreImage, Error>) in
            VZMacOSRestoreImage.load(from: restoreURL) { result in
                switch result {
                case .success(let img): cont.resume(returning: img)
                case .failure(let err): cont.resume(throwing: err)
                }
            }
        }

        guard let requirements = restoreImage.mostFeaturefulSupportedConfiguration else {
            throw VMError.unsupportedHardware
        }

        print("  Minimum CPU: \(requirements.minimumSupportedCPUCount)")
        print("  Minimum RAM: \(requirements.minimumSupportedMemorySize / 1024 / 1024)MB")

        // Create bundle
        if !bundle.exists {
            try bundle.create()
        }

        // Build VM configuration for installation
        let vzConfig = VZVirtualMachineConfiguration()
        vzConfig.cpuCount = max(config.cpuCount, requirements.minimumSupportedCPUCount)
        vzConfig.memorySize = max(config.memorySize, requirements.minimumSupportedMemorySize)

        // Platform
        let platform = VZMacPlatformConfiguration()
        platform.auxiliaryStorage = try VZMacAuxiliaryStorage(
            creatingStorageAt: URL(fileURLWithPath: bundle.auxiliaryStoragePath),
            hardwareModel: requirements.hardwareModel,
            options: [.allowOverwrite]
        )
        platform.hardwareModel = requirements.hardwareModel
        platform.machineIdentifier = VZMacMachineIdentifier()

        // Save machine identifier and hardware model for later boots
        let idData = platform.machineIdentifier.dataRepresentation
        try idData.write(to: URL(fileURLWithPath: bundle.machineIdentifierPath))
        let hwData = requirements.hardwareModel.dataRepresentation
        try hwData.write(to: URL(fileURLWithPath: bundle.hardwareModelPath))

        vzConfig.platform = platform

        // Boot loader
        vzConfig.bootLoader = VZMacOSBootLoader()

        // Storage
        let diskAttachment = try VZDiskImageStorageDeviceAttachment(
            url: URL(fileURLWithPath: bundle.diskImagePath), readOnly: false)
        vzConfig.storageDevices = [VZVirtioBlockDeviceConfiguration(attachment: diskAttachment)]

        // Network
        let network = VZVirtioNetworkDeviceConfiguration()
        network.attachment = VZNATNetworkDeviceAttachment()
        vzConfig.networkDevices = [network]

        // Graphics
        let graphics = VZMacGraphicsDeviceConfiguration()
        graphics.displays = [
            VZMacGraphicsDisplayConfiguration(
                widthInPixels: 1920, heightInPixels: 1080, pixelsPerInch: 144)
        ]
        vzConfig.graphicsDevices = [graphics]

        // Keyboard + pointing
        vzConfig.keyboards = [VZUSBKeyboardConfiguration()]
        vzConfig.pointingDevices = [VZUSBScreenCoordinatePointingDeviceConfiguration()]

        // Shared directory
        addSharedDirectory(to: vzConfig)

        try vzConfig.validate()

        let vm = VZVirtualMachine(configuration: vzConfig)

        // Install
        print("  Installing macOS (this takes a while)...")
        let installer = VZMacOSInstaller(virtualMachine: vm, restoringFromImageAt: restoreURL)

        // Progress reporting
        let observation = installer.progress.observe(\.fractionCompleted) { progress, _ in
            let pct = Int(progress.fractionCompleted * 100)
            print("\r  Install progress: \(pct)%", terminator: "")
            fflush(stdout)
        }

        try await withCheckedThrowingContinuation { (cont: CheckedContinuation<Void, Error>) in
            installer.install { result in
                switch result {
                case .success: cont.resume()
                case .failure(let err): cont.resume(throwing: err)
                }
            }
        }

        observation.invalidate()
        print("\n  macOS installation complete!")

        return vm
    }

    /// Boot an already-installed VM.
    func boot() throws -> VZVirtualMachine {
        guard bundle.exists else {
            throw VMError.notInstalled
        }

        let vzConfig = VZVirtualMachineConfiguration()
        vzConfig.cpuCount = config.cpuCount
        vzConfig.memorySize = config.memorySize

        // Platform — restore from saved state
        let platform = VZMacPlatformConfiguration()
        platform.auxiliaryStorage = VZMacAuxiliaryStorage(
            contentsOf: URL(fileURLWithPath: bundle.auxiliaryStoragePath))

        let idData = try Data(contentsOf: URL(fileURLWithPath: bundle.machineIdentifierPath))
        guard let machineId = VZMacMachineIdentifier(dataRepresentation: idData) else {
            throw VMError.invalidMachineIdentifier
        }
        platform.machineIdentifier = machineId

        // Hardware model — restored from saved data
        let hwData = try Data(contentsOf: URL(fileURLWithPath: bundle.hardwareModelPath))
        guard let hardwareModel = VZMacHardwareModel(dataRepresentation: hwData) else {
            throw VMError.invalidHardwareModel
        }
        platform.hardwareModel = hardwareModel

        vzConfig.platform = platform
        vzConfig.bootLoader = VZMacOSBootLoader()

        // Storage
        let diskAttachment = try VZDiskImageStorageDeviceAttachment(
            url: URL(fileURLWithPath: bundle.diskImagePath), readOnly: false)
        vzConfig.storageDevices = [VZVirtioBlockDeviceConfiguration(attachment: diskAttachment)]

        // Network
        let network = VZVirtioNetworkDeviceConfiguration()
        network.attachment = VZNATNetworkDeviceAttachment()
        vzConfig.networkDevices = [network]

        // Graphics
        let graphics = VZMacGraphicsDeviceConfiguration()
        graphics.displays = [
            VZMacGraphicsDisplayConfiguration(
                widthInPixels: 1920, heightInPixels: 1080, pixelsPerInch: 144)
        ]
        vzConfig.graphicsDevices = [graphics]

        // Input
        vzConfig.keyboards = [VZUSBKeyboardConfiguration()]
        vzConfig.pointingDevices = [VZUSBScreenCoordinatePointingDeviceConfiguration()]

        // Shared directory
        addSharedDirectory(to: vzConfig)

        try vzConfig.validate()
        return VZVirtualMachine(configuration: vzConfig)
    }

    private func addSharedDirectory(to vzConfig: VZVirtualMachineConfiguration) {
        let sharedURL = URL(fileURLWithPath: config.sharedDirectory)
        guard FileManager.default.fileExists(atPath: config.sharedDirectory) else {
            print("  ⚠ Shared directory not found: \(config.sharedDirectory)")
            print("    Run ./packaging/bundle-runtime.sh first")
            return
        }

        let share = VZSharedDirectory(url: sharedURL, readOnly: true)
        let singleDir = VZSingleDirectoryShare(directory: share)
        let tag = VZVirtioFileSystemDeviceConfiguration(tag: "darwin-runtime")
        tag.share = singleDir
        vzConfig.directorySharingDevices = [tag]
        print("  Sharing: \(config.sharedDirectory) → darwin-runtime")
    }
}

// MARK: - VM Delegate

@available(macOS 13.0, *)
class VMDelegate: NSObject, VZVirtualMachineDelegate {
    let onStop: () -> Void

    init(onStop: @escaping () -> Void) {
        self.onStop = onStop
    }

    func virtualMachine(_ vm: VZVirtualMachine, didStopWithError error: Error) {
        print("\n  VM stopped with error: \(error)")
        onStop()
    }

    func guestDidStop(_ vm: VZVirtualMachine) {
        print("\n  VM guest stopped.")
        onStop()
    }
}

// MARK: - Errors

enum VMError: Error, CustomStringConvertible {
    case unsupportedHardware
    case notInstalled
    case invalidMachineIdentifier
    case invalidHardwareModel

    var description: String {
        switch self {
        case .unsupportedHardware: return "This Mac does not support the restore image"
        case .notInstalled: return "VM not installed. Run with 'install' first."
        case .invalidMachineIdentifier: return "Saved machine identifier is invalid"
        case .invalidHardwareModel: return "Saved hardware model is invalid"
        }
    }
}

// MARK: - CLI Entry Point

@available(macOS 13.0, *)
@main
struct DarwinVM {
    static func main() async throws {
        let args = CommandLine.arguments
        let command = args.count > 1 ? args[1] : "boot"

        let config = VMConfig.fromEnvironment()
        let builder = VMBuilder(config: config)

        print("╔══════════════════════════════════════════════════════╗")
        print("║  burdettie-os — VM launcher                    ║")
        print("╚══════════════════════════════════════════════════════╝")
        print("")
        print("  Command:    \(command)")
        print("  CPUs:       \(config.cpuCount)")
        print("  Memory:     \(config.memorySize / 1024 / 1024)MB")
        print("  Shared dir: \(config.sharedDirectory)")
        print("  VM bundle:  \(config.bundlePath)")
        print("")

        switch command {
        case "install":
            print("▶ Installing macOS into VM...")
            let vm = try await builder.install()
            print("▶ Starting VM after install...")
            try await runVM(vm)

        case "boot", "start", "run":
            print("▶ Booting VM...")
            let vm = try builder.boot()
            try await runVM(vm)

        case "status":
            if builder.bundle.exists {
                let diskSize = try FileManager.default
                    .attributesOfItem(atPath: builder.bundle.diskImagePath)[.size] as? UInt64 ?? 0
                print("  VM bundle exists at: \(config.bundlePath)")
                print("  Disk image: \(diskSize / 1024 / 1024)MB")
            } else {
                print("  No VM installed. Run: darwin-vm install")
            }

        case "clean":
            if builder.bundle.exists {
                print("▶ Removing VM bundle at: \(config.bundlePath)")
                try FileManager.default.removeItem(atPath: config.bundlePath)
                print("  ✓ Cleaned")
            } else {
                print("  Nothing to clean.")
            }

        case "help", "--help", "-h":
            printUsage()

        default:
            print("Unknown command: \(command)")
            printUsage()
            Foundation.exit(1)
        }
    }

    static func runVM(_ vm: VZVirtualMachine) async throws {
        let delegate = VMDelegate {
            Foundation.exit(0)
        }
        vm.delegate = delegate

        // Handle Ctrl+C for clean shutdown
        signal(SIGINT) { _ in
            print("\n  Ctrl+C received, requesting stop...")
            Foundation.exit(0)
        }

        try await vm.start()
        print("  ✓ VM is running")
        print("  Press Ctrl+C to stop")
        print("")

        // Keep delegate alive; sleep indefinitely until VM stops or Ctrl+C
        _ = delegate
        while true {
            try await Task.sleep(nanoseconds: 1_000_000_000)
        }
    }

    static func printUsage() {
        print("""
        Usage: darwin-vm <command>

        Commands:
          install   Download macOS IPSW and install into a new VM
          boot      Boot an already-installed VM (default)
          status    Show VM bundle status
          clean     Remove the VM bundle
          help      Show this help

        Environment:
          VM_IPSW          Path to macOS restore IPSW (default: ~/.burdettie-os/RestoreImage.ipsw)
          VM_SHARED_DIR    Directory to share with guest (default: ./dist)
          VM_CPUS          Number of CPUs (default: 2)
          VM_MEMORY_GB     Memory in GB (default: 4)
          VM_BUNDLE        VM bundle path (default: ~/.burdettie-os/vm-bundle)
          DARWIN_LOADER_ROOT  Project root (default: current directory)
        """)
    }
}
