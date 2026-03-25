// kvo_isa_swizzle — C2 fixture: KVO with isa-swizzling via dynamic classes
//
// Demonstrates how KVO works under the hood: Apple's runtime creates
// a dynamic subclass (NSKVONotifying_*) using objc_duplicateClass or
// objc_allocateClassPair, then swizzles the object's isa via
// object_setClass. This fixture simulates that pattern manually.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>
#include <string.h>

// ── Observable class ───────────────────────────────────────────────

@interface Account : NSObject
@property (nonatomic, assign) double balance;
@end

@implementation Account
@end

// ── Manual KVO-style isa swizzle ───────────────────────────────────

static int notification_count = 0;

// Override setter that calls willChange/didChange pattern
static void kvo_setBalance(id self, SEL _cmd, double value) {
    // Print the change notification
    double old = ((double (*)(id, SEL))objc_msgSend)(
        self, sel_registerName("balance"));
    printf("KVO: balance changing %.2f -> %.2f\n", old, value);
    notification_count++;

    // Call original setter via superclass
    Class origClass = [Account class];
    void (*origSetBalance)(id, SEL, double) = NULL;
    Method m = class_getInstanceMethod(origClass, sel_registerName("setBalance:"));
    if (m) {
        origSetBalance = (void (*)(id, SEL, double))method_getImplementation(m);
        origSetBalance(self, _cmd, value);
    }
}

int main() {
    @autoreleasepool {
        Account *acct = [[Account alloc] init];
        acct.balance = 100.0;
        printf("initial balance: %.2f\n", acct.balance);

        // ── 1. Get original class info ─────────────────────────────
        Class origClass = object_getClass(acct);
        printf("original class: %s\n", class_getName(origClass));

        // ── 2. Create KVO subclass via objc_duplicateClass ─────────
        Class kvoClass = objc_duplicateClass(origClass,
            "NSKVONotifying_Account", 0);
        printf("created KVO class: %s\n",
               kvoClass ? class_getName(kvoClass) : "nil");
        if (!kvoClass) {
            printf("FAIL: objc_duplicateClass returned nil\n");
            return 1;
        }

        // ── 3. Replace the setter with notification version ────────
        class_replaceMethod(kvoClass,
            sel_registerName("setBalance:"),
            (IMP)kvo_setBalance, "v@:d");

        // ── 4. Swizzle the object's isa ────────────────────────────
        Class oldClass = object_setClass(acct, kvoClass);
        printf("swizzled isa: %s -> %s\n",
               class_getName(oldClass),
               class_getName(object_getClass(acct)));

        // ── 5. Set balance — should trigger notification ───────────
        acct.balance = 200.0;
        printf("balance after first set: %.2f\n", acct.balance);

        acct.balance = 350.75;
        printf("balance after second set: %.2f\n", acct.balance);

        printf("notification count: %d\n", notification_count);

        // ── 6. Verify class hierarchy ──────────────────────────────
        Class kvoSuper = class_getSuperclass(kvoClass);
        printf("KVO class superclass: %s\n", class_getName(kvoSuper));

        // ── 7. object_getClass should return KVO class ─────────────
        printf("current class: %s\n",
               class_getName(object_getClass(acct)));

        // ── 8. -class should return original (Apple convention) ────
        // In real KVO, -class is overridden to hide the swizzle.
        // We just verify basic object_getClass behavior.
        printf("responds to setBalance: %s\n",
               class_respondsToSelector(kvoClass,
                   sel_registerName("setBalance:")) ? "YES" : "NO");

        // ── 9. Restore original class ──────────────────────────────
        object_setClass(acct, origClass);
        printf("restored class: %s\n",
               class_getName(object_getClass(acct)));

        // ── 10. Set balance without notification ───────────────────
        acct.balance = 500.0;
        printf("balance after restore: %.2f\n", acct.balance);
        printf("notification count after restore: %d\n", notification_count);

        // ── 11. Also test allocateClassPair approach ───────────────
        Class manualKvo = objc_allocateClassPair(origClass,
            "ManualKVONotifying_Account", 0);
        printf("allocated ManualKVONotifying_Account: %s\n",
               manualKvo ? "YES" : "NO");
        if (manualKvo) {
            class_addMethod(manualKvo,
                sel_registerName("setBalance:"),
                (IMP)kvo_setBalance, "v@:d");
            objc_registerClassPair(manualKvo);

            // Swizzle and test
            object_setClass(acct, manualKvo);
            notification_count = 0;
            acct.balance = 999.0;
            printf("manual KVO notification count: %d\n", notification_count);
            object_setClass(acct, origClass);
        }

        printf("OK: kvo_isa_swizzle\n");
        return 0;
    }
}
