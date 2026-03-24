// nserror_basics — fixture: NSError creation and property access.
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>

// Typed objc_msgSend casts
typedef id (*send0_t)(id, SEL);
typedef id (*send1_t)(id, SEL, id);
typedef id (*send3_t)(id, SEL, id, long, id);
typedef long (*sendL_t)(id, SEL);
typedef const char * (*sendU_t)(id, SEL);

int main() {
    send0_t send0 = (send0_t)objc_msgSend;
    send1_t send1 = (send1_t)objc_msgSend;
    send3_t send3 = (send3_t)objc_msgSend;
    sendL_t sendL = (sendL_t)objc_msgSend;
    sendU_t sendU = (sendU_t)objc_msgSend;

    Class nserrorCls = objc_getClass("NSError");
    SEL errorWithSel = sel_registerName("errorWithDomain:code:userInfo:");
    SEL domainSel = sel_registerName("domain");
    SEL codeSel = sel_registerName("code");
    SEL userInfoSel = sel_registerName("userInfo");
    SEL ldSel = sel_registerName("localizedDescription");
    SEL descSel = sel_registerName("description");

    // Create domain string
    Class nsstringCls = objc_getClass("NSString");
    SEL swuSel = sel_registerName("stringWithUTF8String:");
    typedef id (*sendS_t)(id, SEL, const char *);
    sendS_t sendS = (sendS_t)objc_msgSend;
    id domain = sendS((id)nsstringCls, swuSel, "TestErrorDomain");

    // 1. Create error with domain, code 42, nil userInfo
    id error = send3((id)nserrorCls, errorWithSel, domain, 42, nil);
    if (!error) { printf("FAIL: 1 - errorWithDomain returned nil\n"); return 1; }
    printf("CHECK 1: error created\n");

    // 2. Check domain
    id dom = send0(error, domainSel);
    if (!dom) { printf("FAIL: 2 - domain is nil\n"); return 2; }
    const char *domStr = sendU(dom, sel_registerName("UTF8String"));
    if (strcmp(domStr, "TestErrorDomain") != 0) {
        printf("FAIL: 2 - domain mismatch: %s\n", domStr); return 2;
    }
    printf("CHECK 2: domain = %s\n", domStr);

    // 3. Check code
    long code = sendL(error, codeSel);
    if (code != 42) { printf("FAIL: 3 - code %ld != 42\n", code); return 3; }
    printf("CHECK 3: code = %ld\n", code);

    // 4. Check userInfo — may be nil or empty dictionary depending on runtime
    id ui = send0(error, userInfoSel);
    if (ui != nil) {
        // Apple's runtime returns an empty dictionary for nil userInfo
        typedef unsigned long (*sendQ_t)(id, SEL);
        sendQ_t sendQ = (sendQ_t)objc_msgSend;
        unsigned long uiCount = sendQ(ui, sel_registerName("count"));
        printf("CHECK 4: userInfo has %lu entries\n", uiCount);
    } else {
        printf("CHECK 4: userInfo is nil\n");
    }

    // 5. Check localizedDescription is non-nil
    id ld = send0(error, ldSel);
    if (!ld) { printf("FAIL: 5 - localizedDescription is nil\n"); return 5; }
    const char *ldStr = sendU(ld, sel_registerName("UTF8String"));
    printf("CHECK 5: localizedDescription = %s\n", ldStr);

    // 6. Check description is non-nil
    id desc = send0(error, descSel);
    if (!desc) { printf("FAIL: 6 - description is nil\n"); return 6; }
    const char *descStr = sendU(desc, sel_registerName("UTF8String"));
    printf("CHECK 6: description = %s\n", descStr);

    printf("OK: NSError basics passed\n");
    return 0;
}
