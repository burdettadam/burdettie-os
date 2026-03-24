// nsbundle_basics — fixture: NSBundle mainBundle and path queries.
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>

typedef id (*send0_t)(id, SEL);
typedef const char * (*sendU_t)(id, SEL);

int main() {
    send0_t send0 = (send0_t)objc_msgSend;
    sendU_t sendU = (sendU_t)objc_msgSend;

    Class nsbundleCls = objc_getClass("NSBundle");
    SEL mainBundleSel = sel_registerName("mainBundle");
    SEL bundlePathSel = sel_registerName("bundlePath");
    SEL execPathSel = sel_registerName("executablePath");
    SEL bundleIdSel = sel_registerName("bundleIdentifier");

    // 1. Get main bundle
    id bundle = send0((id)nsbundleCls, mainBundleSel);
    if (!bundle) { printf("FAIL: 1 - mainBundle returned nil\n"); return 1; }
    printf("CHECK 1: mainBundle obtained\n");

    // 2. bundlePath is non-nil
    id bpath = send0(bundle, bundlePathSel);
    if (!bpath) { printf("FAIL: 2 - bundlePath is nil\n"); return 2; }
    const char *bpathStr = sendU(bpath, sel_registerName("UTF8String"));
    printf("CHECK 2: bundlePath = %s\n", bpathStr);

    // 3. executablePath is non-nil
    id epath = send0(bundle, execPathSel);
    if (!epath) { printf("FAIL: 3 - executablePath is nil\n"); return 3; }
    const char *epathStr = sendU(epath, sel_registerName("UTF8String"));
    printf("CHECK 3: executablePath = %s\n", epathStr);

    // 4. bundleIdentifier should be nil for a CLI tool
    id bid = send0(bundle, bundleIdSel);
    // For flat CLI tools, bundleIdentifier is nil — we don't fail if it is
    if (bid == nil) {
        printf("CHECK 4: bundleIdentifier is nil (expected for CLI)\n");
    } else {
        const char *bidStr = sendU(bid, sel_registerName("UTF8String"));
        printf("CHECK 4: bundleIdentifier = %s\n", bidStr);
    }

    // 5. Calling mainBundle again returns the same singleton
    id bundle2 = send0((id)nsbundleCls, mainBundleSel);
    if (bundle != bundle2) { printf("FAIL: 5 - mainBundle not singleton\n"); return 5; }
    printf("CHECK 5: mainBundle is singleton\n");

    printf("OK: NSBundle basics passed\n");
    return 0;
}
