// exception_basics — fixture: NSException class methods + property access.
// Full @try/@catch/@throw coverage now lives in `minimal_throw` and
// `try_catch_test`. This fixture intentionally stays focused on NSException
// allocation, property accessors, and description through the loader.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>

// Typed objc_msgSend casts
typedef id (*send0_t)(id, SEL);
typedef id (*send3_t)(id, SEL, id, id, id);
typedef const char * (*sendU_t)(id, SEL);

int main() {
    send0_t send0 = (send0_t)objc_msgSend;
    send3_t send3 = (send3_t)objc_msgSend;
    sendU_t sendU = (sendU_t)objc_msgSend;

    Class nsexCls = objc_getClass("NSException");
    Class nsstringCls = objc_getClass("NSString");
    typedef id (*sendS_t)(id, SEL, const char *);
    sendS_t sendS = (sendS_t)objc_msgSend;

    SEL swuSel = sel_registerName("stringWithUTF8String:");

    // 1. Create NSException with name and reason
    id name = sendS((id)nsstringCls, swuSel, "TestException");
    id reason = sendS((id)nsstringCls, swuSel, "Something went wrong");
    SEL ewSel = sel_registerName("exceptionWithName:reason:userInfo:");
    id exc = send3((id)nsexCls, ewSel, name, reason, nil);
    if (!exc) { printf("FAIL: 1 - exceptionWithName returned nil\n"); return 1; }
    printf("CHECK 1: exception created\n");

    // 2. Check name
    id ename = send0(exc, sel_registerName("name"));
    if (!ename) { printf("FAIL: 2 - name is nil\n"); return 2; }
    const char *enameStr = sendU(ename, sel_registerName("UTF8String"));
    if (strcmp(enameStr, "TestException") != 0) {
        printf("FAIL: 2 - name mismatch: %s\n", enameStr); return 2;
    }
    printf("CHECK 2: name = %s\n", enameStr);

    // 3. Check reason
    id ereason = send0(exc, sel_registerName("reason"));
    if (!ereason) { printf("FAIL: 3 - reason is nil\n"); return 3; }
    const char *ereasonStr = sendU(ereason, sel_registerName("UTF8String"));
    if (strcmp(ereasonStr, "Something went wrong") != 0) {
        printf("FAIL: 3 - reason mismatch: %s\n", ereasonStr); return 3;
    }
    printf("CHECK 3: reason = %s\n", ereasonStr);

    // 4. Check userInfo is nil
    id ui = send0(exc, sel_registerName("userInfo"));
    printf("CHECK 4: userInfo = %p\n", (void *)ui);

    // 5. description
    id desc = send0(exc, sel_registerName("description"));
    if (!desc) { printf("FAIL: 5 - description is nil\n"); return 5; }
    const char *descStr = sendU(desc, sel_registerName("UTF8String"));
    printf("CHECK 5: description = %s\n", descStr);

    // 6. Create second exception with different values
    id name2 = sendS((id)nsstringCls, swuSel, "AnotherException");
    id reason2 = sendS((id)nsstringCls, swuSel, "Different reason");
    id exc2 = send3((id)nsexCls, ewSel, name2, reason2, nil);
    if (!exc2) { printf("FAIL: 6 - second exception nil\n"); return 6; }
    id ename2 = send0(exc2, sel_registerName("name"));
    const char *enameStr2 = sendU(ename2, sel_registerName("UTF8String"));
    if (strcmp(enameStr2, "AnotherException") != 0) {
        printf("FAIL: 6 - name2 mismatch: %s\n", enameStr2); return 6;
    }
    printf("CHECK 6: second exception name = %s\n", enameStr2);

    printf("OK: exception basics passed\n");
    return 0;
}
