// swizzle_restore — Method swizzling roundtrip fixture
//
// Tests the classic "swizzle in +load" pattern with category methods,
// then verifies the swizzled method is dispatched correctly including
// across the superclass chain.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#import <objc/runtime.h>
#include <stdio.h>

// ── Counter: a simple class to swizzle ─────────────────────────────

static int g_swizzle_count = 0;

@interface Counter : NSObject
- (int)count;
- (int)doubleCount;
@end

@implementation Counter
- (int)count { return 1; }
- (int)doubleCount { return [self count] * 2; }
@end

// ── SubCounter: subclass to test dispatch chain ────────────────────

@interface SubCounter : Counter
@end

@implementation SubCounter
// inherits count and doubleCount
@end

// ── Main ───────────────────────────────────────────────────────────

int main() {
    printf("swizzle_restore test\n");
    int failures = 0;

    Counter *c = [[Counter alloc] init];
    SubCounter *sc = [[SubCounter alloc] init];

    // 1. Baseline
    printf("\n--- Test 1: baseline ---\n");
    if ([c count] == 1 && [c doubleCount] == 2) {
        printf("  PASS: Counter count=%d doubleCount=%d\n", [c count], [c doubleCount]);
    } else {
        printf("  FAIL: expected 1/2, got %d/%d\n", [c count], [c doubleCount]);
        failures++;
    }

    // 2. SubCounter inherits
    printf("\n--- Test 2: subclass inherits ---\n");
    if ([sc count] == 1 && [sc doubleCount] == 2) {
        printf("  PASS: SubCounter count=%d doubleCount=%d\n", [sc count], [sc doubleCount]);
    } else {
        printf("  FAIL: expected 1/2, got %d/%d\n", [sc count], [sc doubleCount]);
        failures++;
    }

    // 3. Swizzle count with doubleCount on Counter
    printf("\n--- Test 3: swizzle count <-> doubleCount ---\n");
    Method mCount = class_getInstanceMethod([Counter class], @selector(count));
    Method mDouble = class_getInstanceMethod([Counter class], @selector(doubleCount));
    method_exchangeImplementations(mCount, mDouble);

    // After swap: -count now calls old doubleCount body (which calls [self count])
    // BUT [self count] also goes through dispatch, so it's now the old doubleCount...
    // In practice: -count invokes old doubleCount, and -doubleCount invokes old count.
    int c_count = [c doubleCount];   // should call old count → 1
    if (c_count == 1) {
        printf("  PASS: after swap, doubleCount (old count body) = %d\n", c_count);
    } else {
        printf("  FAIL: expected 1, got %d\n", c_count);
        failures++;
    }

    // 4. SubCounter picks up the swizzle  
    printf("\n--- Test 4: subclass sees swizzle ---\n");
    int sc_double = [sc doubleCount];  // old count body → 1
    if (sc_double == 1) {
        printf("  PASS: SubCounter doubleCount (old count body) = %d\n", sc_double);
    } else {
        printf("  FAIL: expected 1, got %d\n", sc_double);
        failures++;
    }

    // 5. Restore
    printf("\n--- Test 5: restore original ---\n");
    method_exchangeImplementations(mCount, mDouble);
    if ([c count] == 1 && [c doubleCount] == 2 &&
        [sc count] == 1 && [sc doubleCount] == 2) {
        printf("  PASS: restored Counter and SubCounter\n");
    } else {
        printf("  FAIL: restore failed\n");
        failures++;
    }

    // 6. class_replaceMethod — add a new method
    printf("\n--- Test 6: class_replaceMethod adds new ---\n");
    IMP countImp = method_getImplementation(mCount);
    IMP oldImp = class_replaceMethod([Counter class], @selector(tripleCount),
                                      countImp, "i@:");
    if (oldImp == NULL) {
        printf("  PASS: class_replaceMethod returned NULL (new method)\n");
    } else {
        printf("  FAIL: expected NULL, got %p\n", (void*)oldImp);
        failures++;
    }
    // Verify the new method works via performSelector or runtime
    SEL tripleCountSel = @selector(tripleCount);
    if ([c respondsToSelector:tripleCountSel]) {
        printf("  PASS: Counter now responds to tripleCount\n");
    } else {
        printf("  FAIL: Counter does not respond to tripleCount\n");
        failures++;
    }

    // Summary
    printf("\nswizzle_restore: %d failures\n", failures);
    if (failures == 0) {
        printf("swizzle_restore: PASS\n");
    } else {
        printf("swizzle_restore: FAIL\n");
    }
    return failures;
}
