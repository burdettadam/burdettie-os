// swift-client — Minimal example of using DarwinRuntime from Swift.
//
// Build:
//   cd examples/swift-client && swift build
//
// Run:
//   swift run swift-client [fixture-path]

import DarwinRuntime
import Foundation

// 1. Version
let version = DarwinRuntime.version
print("darwin-runtime \(version)")

// 2. Pre-run state
print("\nBefore running a fixture:")
print("  ObjC classes:      \(ObjCRuntime.classCount)")
print("  ObjC selectors:    \(ObjCRuntime.selectorCount)")
print("  Foundation classes: \(ObjCRuntime.foundationClassCount)")

// 3. Run a fixture
let fixturePath: String
if CommandLine.arguments.count > 1 {
    fixturePath = CommandLine.arguments[1]
} else {
    // Try to find fixtures relative to this example
    let candidates = [
        "../../fixtures/simple_cli/app",
        "../../../fixtures/simple_cli/app",
    ]
    let fm = FileManager.default
    fixturePath = candidates.first { fm.fileExists(atPath: $0) }
        ?? (ProcessInfo.processInfo.environment["DARWIN_LOADER_ROOT"]
            .map { $0 + "/fixtures/simple_cli/app" } ?? "fixtures/simple_cli/app")
}

print("\nRunning fixture: \(fixturePath)")
do {
    let result = try DarwinRuntime.run(path: fixturePath)
    print("  exit code:        \(result.exitCode)")
    print("  images loaded:    \(result.imagesLoaded)")
    print("  shims installed:  \(result.shimsInstalled)")
    print("  initializers run: \(result.initializersRun)")
} catch {
    print("  Error: \(error)")
}

// 4. Post-run introspection
print("\nAfter running fixture:")
let classCount = ObjCRuntime.classCount
let selectorCount = ObjCRuntime.selectorCount
print("  ObjC classes:      \(classCount)")
print("  ObjC selectors:    \(selectorCount)")
print("  Foundation classes: \(ObjCRuntime.foundationClassCount)")

// 5. List classes
let classes = ObjCRuntime.listClasses()
if !classes.isEmpty {
    print("\nRegistered classes (\(classes.count)):")
    for name in classes.prefix(10) {
        if let info = ObjCRuntime.getClass(name: name) {
            print("  \(name): \(info.methodCount) methods, \(info.classMethodCount) class methods")
        }
    }
    if classes.count > 10 {
        print("  ... and \(classes.count - 10) more")
    }
}

// 6. NSString method inspection
if let nsstring = ObjCRuntime.getClass(name: "NSString") {
    let methods = ObjCRuntime.listMethods(className: "NSString")
    print("\nNSString: \(nsstring.methodCount) instance methods, \(nsstring.classMethodCount) class methods")
    for m in methods.prefix(5) {
        print("  -[NSString \(m.selector)]")
    }
    if methods.count > 5 {
        print("  ... and \(methods.count - 5) more")
    }
}

print("\nDone.")
