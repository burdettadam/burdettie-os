// ObjC fixture demonstrating categories, +load, and method dispatch.
// Pure ObjC runtime — no Foundation, no NSObject.
//
// The class and category are in separate compilation units to force
// the linker to emit __objc_catlist.
//
// Tests:
//   1. Class registration and instance method dispatch
//   2. Category adds a method to an existing class
//   3. +load is called before main()
//   4. objc_msgSend dispatches both original and category methods

#include <stdio.h>
#include <stdlib.h>
#include <objc/runtime.h>
#include <objc/message.h>

// Defined in greeter.m
extern int greeter_load_called;

int main() {
    // Verify +load ran before main
    if (!greeter_load_called) {
        fprintf(stderr, "FAIL: +load was not called\n");
        return 1;
    }
    fprintf(stderr, "+load verified\n");

    // Get the class
    Class cls = objc_getClass("Greeter");
    if (!cls) {
        fprintf(stderr, "FAIL: class not found\n");
        return 1;
    }

    // Create an instance
    id obj = class_createInstance(cls, 0);
    if (!obj) {
        fprintf(stderr, "FAIL: allocation failed\n");
        return 1;
    }

    // Call the original method
    SEL greetSel = sel_registerName("greeting");
    const char *g = ((const char *(*)(id, SEL))objc_msgSend)(obj, greetSel);
    fprintf(stderr, "greeting=%s\n", g);

    // Call the category method
    SEL farewellSel = sel_registerName("farewell");
    const char *f = ((const char *(*)(id, SEL))objc_msgSend)(obj, farewellSel);
    fprintf(stderr, "farewell=%s\n", f);

    free(obj);

    // Check results
    if (g == NULL || f == NULL) {
        fprintf(stderr, "FAIL: null return\n");
        return 1;
    }

    // Compare strings manually (no strcmp dependency)
    const char *expected_g = "hello";
    const char *expected_f = "goodbye";
    int ok = 1;
    for (int i = 0; expected_g[i] || g[i]; i++) {
        if (expected_g[i] != g[i]) { ok = 0; break; }
    }
    for (int i = 0; expected_f[i] || f[i]; i++) {
        if (expected_f[i] != f[i]) { ok = 0; break; }
    }

    if (!ok) {
        fprintf(stderr, "FAIL: string mismatch\n");
        return 1;
    }

    fprintf(stderr, "all checks passed\n");
    return 0;
}
