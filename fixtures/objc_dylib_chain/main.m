// T10 fixture: multi-dylib ObjC app
// Tests cross-image class resolution and category merge.
//
// Build:
//   libgreeter.dylib defines Greeter class
//   libextras.dylib defines Greeter (Extras) category, links -lgreeter
//   app links -lgreeter -lextras
//
// Exercises:
//   1. Cross-image ObjC class symbol resolution (_OBJC_CLASS_$_Greeter)
//   2. Category from libextras applied to class from libgreeter
//   3. +load fires from libgreeter before main
//   4. objc_msgSend dispatches both original and category methods

#include <stdio.h>
#include <stdlib.h>
#include <objc/runtime.h>
#include <objc/message.h>

// Defined in libgreeter.dylib
extern int greeter_load_called;

int main() {
    // Verify +load ran before main
    if (!greeter_load_called) {
        fprintf(stderr, "FAIL: +load was not called\n");
        return 1;
    }
    fprintf(stderr, "+load verified\n");

    // Get the class (defined in libgreeter.dylib)
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

    // Call the original method (from libgreeter.dylib)
    SEL greetSel = sel_registerName("greeting");
    const char *g = ((const char *(*)(id, SEL))objc_msgSend)(obj, greetSel);
    fprintf(stderr, "greeting=%s\n", g);

    // Call the category method (from libextras.dylib)
    SEL farewellSel = sel_registerName("farewell");
    const char *f = ((const char *(*)(id, SEL))objc_msgSend)(obj, farewellSel);
    fprintf(stderr, "farewell=%s\n", f);

    if (g && f) {
        fprintf(stderr, "all checks passed\n");
    }

    free(obj);
    return 0;
}
