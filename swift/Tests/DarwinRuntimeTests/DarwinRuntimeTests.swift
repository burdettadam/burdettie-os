import Testing
import Foundation
import DarwinRuntime

@Suite("DarwinRuntime Version")
struct VersionTests {
    @Test func versionIs0_1_0() {
        let v = DarwinRuntime.version
        #expect(v.major == 0)
        #expect(v.minor == 1)
        #expect(v.patch == 0)
    }

    @Test func versionDescription() {
        #expect(DarwinRuntime.version.description == "0.1.0")
    }

    @Test func versionEquatable() {
        let a = DarwinRuntime.version
        let b = DarwinRuntime.version
        #expect(a == b)
    }
}

@Suite("DarwinRuntime Error Handling")
struct ErrorTests {
    @Test func runNonexistentPathThrows() {
        #expect(throws: RuntimeError.self) {
            try DarwinRuntime.run(path: "/nonexistent/binary")
        }
    }

    @Test func runSecureNonexistentThrows() {
        #expect(throws: RuntimeError.self) {
            try DarwinRuntime.runSecure(path: "/nonexistent/binary")
        }
    }

    @Test func lastErrorAfterFailure() {
        _ = try? DarwinRuntime.run(path: "/no/such/file")
        let err = DarwinRuntime.lastError
        #expect(err != nil)
    }
}

@Suite("ObjCRuntime Introspection (pre-run)")
struct ObjCPreRunTests {
    @Test func classCountDoesNotCrash() {
        // Before any runtime run, should return 0 or a valid count
        let _ = ObjCRuntime.classCount
    }

    @Test func selectorCountDoesNotCrash() {
        let _ = ObjCRuntime.selectorCount
    }

    @Test func foundationClassCountDoesNotCrash() {
        let _ = ObjCRuntime.foundationClassCount
    }

    @Test func getClassUnknownReturnsNil() {
        let info = ObjCRuntime.getClass(name: "NoSuchClass999")
        #expect(info == nil)
    }

    @Test func listClassesReturnsEmptyOrValid() {
        let classes = ObjCRuntime.listClasses()
        // May be empty if runtime not yet initialized — just ensure no crash
        let _ = classes
    }

    @Test func listMethodsUnknownClassReturnsEmpty() {
        let methods = ObjCRuntime.listMethods(className: "NonExistent")
        #expect(methods.isEmpty)
    }
}

/// Helper to find a fixture path relative to the workspace root.
func fixturePath(_ relative: String) -> String? {
    // Walk up from the swift/ directory to find the workspace root
    let fm = FileManager.default
    // Try common paths relative to where tests run
    let candidates = [
        "../fixtures/\(relative)",                    // from swift/
        "../../fixtures/\(relative)",                 // from swift/.build/
        "../../../fixtures/\(relative)",              // from deeper .build paths
        "../../../../fixtures/\(relative)",
        "../../../../../fixtures/\(relative)",
    ]
    for c in candidates {
        if fm.fileExists(atPath: c) { return c }
    }

    // Try absolute path from env
    if let root = ProcessInfo.processInfo.environment["DARWIN_LOADER_ROOT"] {
        let p = "\(root)/fixtures/\(relative)"
        if fm.fileExists(atPath: p) { return p }
    }

    return nil
}

@Suite(.serialized)
struct FixtureTests {
    @Test func runSimpleCli() throws {
        guard let path = fixturePath("simple_cli/app") else {
            print("skipping: simple_cli fixture not found")
            return
        }
        let result = try DarwinRuntime.run(path: path)
        #expect(result.exitCode == 0)
        #expect(result.imagesLoaded >= 1)
        #expect(result.shimsInstalled > 0)
    }

    @Test func runReturn42() throws {
        guard let path = fixturePath("return42/return42") else {
            print("skipping: return42 fixture not found")
            return
        }
        let result = try DarwinRuntime.run(path: path)
        #expect(result.exitCode == 42)
    }

    @Test func introspectAfterNSStringFixture() throws {
        guard let path = fixturePath("nsstring_ops/app") else {
            print("skipping: nsstring_ops fixture not found")
            return
        }
        let result = try DarwinRuntime.run(path: path)
        #expect(result.exitCode == 0)

        // After running, ObjC runtime should have classes
        #expect(ObjCRuntime.classCount > 0)
        #expect(ObjCRuntime.selectorCount > 0)
        #expect(ObjCRuntime.foundationClassCount > 0)

        // NSString should be registered
        let nsString = ObjCRuntime.getClass(name: "NSString")
        #expect(nsString != nil)
        #expect(nsString?.name == "NSString")
        #expect((nsString?.methodCount ?? 0) > 0)

        // List classes should include NSString
        let classes = ObjCRuntime.listClasses()
        #expect(classes.contains("NSString"))

        // List NSString methods
        let methods = ObjCRuntime.listMethods(className: "NSString")
        #expect(!methods.isEmpty)
        let selectors = methods.map { $0.selector }
        // NSString should have common methods
        #expect(selectors.contains(where: { $0.contains("init") || $0.contains("UTF8") || $0.contains("length") }))
    }
}
