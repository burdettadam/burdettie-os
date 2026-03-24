// runtime_introspection — T23 fixture: ObjC runtime introspection APIs
//
// Tests class_addMethod, class_getInstanceMethod, class_copyMethodList,
// method_getName, method_getTypeEncoding, method_getImplementation,
// method_getNumberOfArguments, sel_getName, class_getMethodImplementation,
// class_getInstanceSize, class_isMetaClass.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <stdio.h>
#include <string.h>

// ── Simple test class ──────────────────────────────────────────────

@interface Probe : NSObject
- (int)alpha;
- (int)beta:(int)x;
@end

@implementation Probe
- (int)alpha { return 1; }
- (int)beta:(int)x { return x + 10; }
@end

// ── Dynamic method to add via class_addMethod ──────────────────────

int dynamicGamma(id self, SEL _cmd) {
    (void)self; (void)_cmd;
    return 99;
}

int main(void) {
    int failures = 0;

    // ── 1. class_getInstanceSize ────────────────────────────────────
    Class probeCls = objc_getClass("Probe");
    size_t isize = class_getInstanceSize(probeCls);
    if (isize < 8) {
        fprintf(stderr, "FAIL: class_getInstanceSize returned %zu (expected >= 8)\n", isize);
        failures++;
    } else {
        NSLog(@"T23 instanceSize=%zu", isize);
    }

    // ── 2. class_isMetaClass ───────────────────────────────────────
    BOOL isMeta = class_isMetaClass(probeCls);
    if (isMeta) {
        fprintf(stderr, "FAIL: class_isMetaClass(Probe) returned YES\n");
        failures++;
    } else {
        NSLog(@"T23 isMetaClass=NO");
    }

    // ── 3. class_getInstanceMethod ─────────────────────────────────
    Method alphaMethod = class_getInstanceMethod(probeCls, @selector(alpha));
    if (!alphaMethod) {
        fprintf(stderr, "FAIL: class_getInstanceMethod returned NULL for alpha\n");
        failures++;
    } else {
        NSLog(@"T23 getInstanceMethod=ok");
    }

    // ── 4. method_getName ──────────────────────────────────────────
    if (alphaMethod) {
        SEL name = method_getName(alphaMethod);
        const char *nameStr = sel_getName(name);
        if (strcmp(nameStr, "alpha") != 0) {
            fprintf(stderr, "FAIL: method_getName returned '%s' (expected 'alpha')\n", nameStr);
            failures++;
        } else {
            NSLog(@"T23 methodName=%s", nameStr);
        }
    }

    // ── 5. method_getTypeEncoding ──────────────────────────────────
    if (alphaMethod) {
        const char *enc = method_getTypeEncoding(alphaMethod);
        if (!enc) {
            fprintf(stderr, "FAIL: method_getTypeEncoding returned NULL\n");
            failures++;
        } else {
            NSLog(@"T23 typeEncoding=%s", enc);
        }
    }

    // ── 6. method_getImplementation ────────────────────────────────
    if (alphaMethod) {
        IMP imp = method_getImplementation(alphaMethod);
        if (!imp) {
            fprintf(stderr, "FAIL: method_getImplementation returned NULL\n");
            failures++;
        } else {
            NSLog(@"T23 getImplementation=ok");
        }
    }

    // ── 7. method_getNumberOfArguments ─────────────────────────────
    if (alphaMethod) {
        unsigned nargs = method_getNumberOfArguments(alphaMethod);
        // alpha takes (self, _cmd) = 2 args
        if (nargs != 2) {
            fprintf(stderr, "FAIL: method_getNumberOfArguments(alpha) = %u (expected 2)\n", nargs);
            failures++;
        } else {
            NSLog(@"T23 numArgs=%u", nargs);
        }
    }

    // ── 8. sel_getName ─────────────────────────────────────────────
    SEL betaSel = @selector(beta:);
    const char *betaName = sel_getName(betaSel);
    if (strcmp(betaName, "beta:") != 0) {
        fprintf(stderr, "FAIL: sel_getName returned '%s' (expected 'beta:')\n", betaName);
        failures++;
    } else {
        NSLog(@"T23 selName=%s", betaName);
    }

    // ── 9. class_getMethodImplementation ───────────────────────────
    IMP alphaImp = class_getMethodImplementation(probeCls, @selector(alpha));
    if (!alphaImp) {
        fprintf(stderr, "FAIL: class_getMethodImplementation returned NULL\n");
        failures++;
    } else {
        NSLog(@"T23 classGetMethodImpl=ok");
    }

    // ── 10. class_addMethod ────────────────────────────────────────
    BOOL added = class_addMethod(probeCls, @selector(gamma), (IMP)dynamicGamma, "i@:");
    if (!added) {
        fprintf(stderr, "FAIL: class_addMethod returned NO\n");
        failures++;
    } else {
        NSLog(@"T23 addMethod=YES");
    }

    // Adding same selector again should fail
    BOOL addedAgain = class_addMethod(probeCls, @selector(gamma), (IMP)dynamicGamma, "i@:");
    if (addedAgain) {
        fprintf(stderr, "FAIL: class_addMethod returned YES for duplicate\n");
        failures++;
    } else {
        NSLog(@"T23 addMethodDup=NO");
    }

    // ── 11. class_copyMethodList ───────────────────────────────────
    unsigned int count = 0;
    Method *methods = class_copyMethodList(probeCls, &count);
    // Should have at least alpha, beta:, gamma (we just added gamma)
    if (count < 3) {
        fprintf(stderr, "FAIL: class_copyMethodList returned count=%u (expected >= 3)\n", count);
        failures++;
    } else {
        NSLog(@"T23 methodCount=%u", count);
        // Verify we can read names from the list
        for (unsigned int i = 0; i < count; i++) {
            SEL msel = method_getName(methods[i]);
            const char *mname = sel_getName(msel);
            NSLog(@"T23 method[%u]=%s", i, mname);
        }
    }
    if (methods) free(methods);

    // ── Summary ────────────────────────────────────────────────────
    if (failures == 0) {
        NSLog(@"T23 done=ok");
    } else {
        fprintf(stderr, "T23 FAILED with %d failures\n", failures);
    }

    return failures;
}
