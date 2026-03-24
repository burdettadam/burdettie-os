// DarwinRuntime — idiomatic Swift wrapper for the darwin-runtime C FFI library.

import CDarwinRuntime

/// Version information for the darwin-runtime library.
public struct Version: CustomStringConvertible, Equatable, Sendable {
    public let major: UInt32
    public let minor: UInt32
    public let patch: UInt32

    public var description: String {
        "\(major).\(minor).\(patch)"
    }
}

/// Result of executing a Mach-O binary through the darwin runtime.
public struct RunResult: Sendable {
    /// Process exit code. 0 indicates success.
    public let exitCode: Int32
    /// Number of Mach-O images loaded (main binary + dynamic libraries).
    public let imagesLoaded: Int
    /// Number of shim functions installed for the execution.
    public let shimsInstalled: Int
    /// Number of initializer functions (constructors) executed.
    public let initializersRun: Int
}

/// Information about a registered Objective-C class.
public struct ClassInfo: Sendable {
    /// The class name (e.g., "NSString").
    public let name: String
    /// Number of instance methods registered on this class.
    public let methodCount: Int
    /// Number of class methods registered.
    public let classMethodCount: Int
    /// Instance size in bytes.
    public let instanceSize: UInt32
    /// Whether this is a root class (has no superclass).
    public let isRoot: Bool
}

/// Information about a registered Objective-C method.
public struct MethodInfo: Sendable {
    /// The selector name (e.g., "initWithUTF8String:").
    public let selector: String
    /// The Objective-C type encoding string.
    public let typeEncoding: String
}

/// Errors that can occur when using the darwin runtime.
public enum RuntimeError: Error, CustomStringConvertible, Sendable {
    /// The runtime returned an error while loading or executing a binary.
    case executionFailed(message: String)
    /// An invalid argument was provided.
    case invalidArgument(String)

    public var description: String {
        switch self {
        case .executionFailed(let msg):
            return "RuntimeError: \(msg)"
        case .invalidArgument(let msg):
            return "InvalidArgument: \(msg)"
        }
    }
}

// MARK: - DarwinRuntime

/// Main entry point for the darwin-runtime Swift API.
///
/// ```swift
/// let version = DarwinRuntime.version
/// print("darwin-runtime \(version)")  // "darwin-runtime 0.1.0"
///
/// let result = try DarwinRuntime.run(path: "/path/to/fixture")
/// print("exit code: \(result.exitCode)")
/// ```
public enum DarwinRuntime {

    /// Returns the library version.
    public static var version: Version {
        let v = darwin_runtime_version()
        return Version(major: v.major, minor: v.minor, patch: v.patch)
    }

    /// Run a Mach-O binary with permissive security policy.
    ///
    /// - Parameter path: Absolute path to the binary to execute.
    /// - Returns: A `RunResult` with exit code and execution statistics.
    /// - Throws: `RuntimeError.executionFailed` if loading or execution fails.
    public static func run(path: String) throws -> RunResult {
        let result = darwin_runtime_run(path)
        if result.error != 0 {
            throw RuntimeError.executionFailed(message: lastError ?? "unknown error")
        }
        return RunResult(
            exitCode: result.exit_code,
            imagesLoaded: Int(result.images_loaded),
            shimsInstalled: Int(result.shims_installed),
            initializersRun: Int(result.initializers_run)
        )
    }

    /// Run a Mach-O binary with Apple-like (secure) security policy.
    ///
    /// - Parameter path: Absolute path to the binary to execute.
    /// - Returns: A `RunResult` with exit code and execution statistics.
    /// - Throws: `RuntimeError.executionFailed` if loading or execution fails.
    public static func runSecure(path: String) throws -> RunResult {
        let result = darwin_runtime_run_secure(path)
        if result.error != 0 {
            throw RuntimeError.executionFailed(message: lastError ?? "unknown error")
        }
        return RunResult(
            exitCode: result.exit_code,
            imagesLoaded: Int(result.images_loaded),
            shimsInstalled: Int(result.shims_installed),
            initializersRun: Int(result.initializers_run)
        )
    }

    /// Returns the last error message from the runtime, or `nil` if no error.
    public static var lastError: String? {
        guard let ptr = darwin_runtime_last_error() else { return nil }
        return String(cString: ptr)
    }
}

// MARK: - ObjCRuntime

/// Introspection API for the Objective-C runtime state.
///
/// After running a binary through `DarwinRuntime.run(path:)`, the ObjC
/// runtime contains registered classes and selectors that can be queried.
///
/// ```swift
/// _ = try DarwinRuntime.run(path: fixturePath)
/// print("Classes: \(ObjCRuntime.classCount)")
/// for name in ObjCRuntime.listClasses() {
///     print(name)
/// }
/// ```
public enum ObjCRuntime {

    /// Number of registered ObjC classes (binary-parsed + synthetic Foundation).
    public static var classCount: Int {
        Int(darwin_objc_class_count())
    }

    /// Number of registered ObjC selectors.
    public static var selectorCount: Int {
        Int(darwin_objc_selector_count())
    }

    /// Number of Foundation classes registered (NSString, NSArray, etc.).
    public static var foundationClassCount: Int {
        Int(darwin_foundation_class_count())
    }

    /// Look up a class by name.
    ///
    /// - Parameter name: The Objective-C class name (e.g., "NSString").
    /// - Returns: A `ClassInfo` if found, or `nil` if no such class is registered.
    public static func getClass(name: String) -> ClassInfo? {
        let info = darwin_objc_get_class(name)
        guard let namePtr = info.name else { return nil }
        let className = String(cString: namePtr)
        darwin_string_free(namePtr)
        return ClassInfo(
            name: className,
            methodCount: Int(info.method_count),
            classMethodCount: Int(info.class_method_count),
            instanceSize: info.instance_size,
            isRoot: info.is_root != 0
        )
    }

    /// Returns a sorted list of all registered class names.
    public static func listClasses() -> [String] {
        let list = darwin_objc_list_classes()
        defer { darwin_class_list_free(list) }

        guard let names = list.names else { return [] }
        var result: [String] = []
        result.reserveCapacity(Int(list.count))
        for i in 0..<Int(list.count) {
            if let ptr = names[i] {
                result.append(String(cString: ptr))
            }
        }
        return result
    }

    /// Returns instance methods for a class.
    ///
    /// - Parameter className: The Objective-C class name.
    /// - Returns: An array of `MethodInfo` for the class's instance methods,
    ///   or an empty array if the class is not found.
    public static func listMethods(className: String) -> [MethodInfo] {
        var count: UInt = 0
        guard let methods = darwin_objc_list_methods(className, &count) else {
            return []
        }
        defer { darwin_method_list_free(methods, count) }

        var result: [MethodInfo] = []
        result.reserveCapacity(Int(count))
        for i in 0..<Int(count) {
            let m = methods[i]
            let sel = m.selector.map { String(cString: $0) } ?? ""
            let enc = m.type_encoding.map { String(cString: $0) } ?? ""
            result.append(MethodInfo(selector: sel, typeEncoding: enc))
        }
        return result
    }
}
