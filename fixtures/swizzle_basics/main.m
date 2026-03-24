// swizzle_basics — Method swizzling fixture
//
// Tests method_exchangeImplementations, method_setImplementation,
// and class_replaceMethod.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#import <objc/runtime.h>
#include <stdio.h>

// ── Greeter class ──────────────────────────────────────────────────

@interface Greeter : NSObject
- (int)greetingCode;
- (int)farewellCode;
@end

@implementation Greeter
- (int)greetingCode { return 42; }
- (int)farewellCode { return 99; }
@end

// ── Main ───────────────────────────────────────────────────────────

int main() {
    printf("swizzle_basics test\n");
    int failures = 0;

    Greeter *g = [[Greeter alloc] init];

    // 1. Verify original behavior
    printf("\n--- Test 1: original behavior ---\n");
    int greeting = [g greetingCode];
    int farewell = [g farewellCode];
    if (greeting == 42 && farewell == 99) {
        printf("  PASS: greetingCode=%d farewellCode=%d\n", greeting, farewell);
    } else {
        printf("  FAIL: expected 42/99, got %d/%d\n", greeting, farewell);
        failures++;
    }

    // 2. method_exchangeImplementations
    printf("\n--- Test 2: method_exchangeImplementations ---\n");
    Method m1 = class_getInstanceMethod([Greeter class], @selector(greetingCode));
    Method m2 = class_getInstanceMethod([Greeter class], @selector(farewellCode));
    method_exchangeImplementations(m1, m2);

    int swapped_greeting = [g greetingCode];
    int swapped_farewell = [g farewellCode];
    if (swapped_greeting == 99 && swapped_farewell == 42) {
        printf("  PASS: after swap greetingCode=%d farewellCode=%d\n",
               swapped_greeting, swapped_farewell);
    } else {
        printf("  FAIL: expected 99/42, got %d/%d\n",
               swapped_greeting, swapped_farewell);
        failures++;
    }

    // 3. Swap back to restore
    printf("\n--- Test 3: swap back restores original ---\n");
    method_exchangeImplementations(m1, m2);
    int restored_greeting = [g greetingCode];
    int restored_farewell = [g farewellCode];
    if (restored_greeting == 42 && restored_farewell == 99) {
        printf("  PASS: restored greetingCode=%d farewellCode=%d\n",
               restored_greeting, restored_farewell);
    } else {
        printf("  FAIL: expected 42/99, got %d/%d\n",
               restored_greeting, restored_farewell);
        failures++;
    }

    // 4. method_setImplementation
    printf("\n--- Test 4: method_setImplementation ---\n");
    IMP oldImp = method_getImplementation(m1);
    IMP farewellImp = method_getImplementation(m2);
    IMP returned = method_setImplementation(m1, farewellImp);
    if (returned == oldImp) {
        printf("  PASS: method_setImplementation returned old IMP\n");
    } else {
        printf("  FAIL: returned IMP mismatch\n");
        failures++;
    }
    int after_set = [g greetingCode];
    if (after_set == 99) {
        printf("  PASS: greetingCode now returns %d (farewell's impl)\n", after_set);
    } else {
        printf("  FAIL: expected 99, got %d\n", after_set);
        failures++;
    }

    // Restore
    method_setImplementation(m1, oldImp);

    // 5. class_replaceMethod — replace existing
    printf("\n--- Test 5: class_replaceMethod (replace existing) ---\n");
    IMP oldReplace = class_replaceMethod([Greeter class], @selector(greetingCode),
                                         farewellImp, "i@:");
    if (oldReplace == oldImp) {
        printf("  PASS: class_replaceMethod returned old IMP\n");
    } else {
        printf("  FAIL: returned IMP mismatch\n");
        failures++;
    }
    int after_replace = [g greetingCode];
    if (after_replace == 99) {
        printf("  PASS: greetingCode now returns %d\n", after_replace);
    } else {
        printf("  FAIL: expected 99, got %d\n", after_replace);
        failures++;
    }

    // Restore
    class_replaceMethod([Greeter class], @selector(greetingCode), oldImp, "i@:");

    // Summary
    printf("\nswizzle_basics: %d failures\n", failures);
    if (failures == 0) {
        printf("swizzle_basics: PASS\n");
    } else {
        printf("swizzle_basics: FAIL\n");
    }
    return failures;
}
