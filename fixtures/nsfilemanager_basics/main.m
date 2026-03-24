// nsfilemanager_basics — fixture: NSFileManager file existence and directory queries.
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>

typedef id (*send0_t)(id, SEL);
typedef BOOL (*send1B_t)(id, SEL, id);
typedef id (*send2_t)(id, SEL, id, id *);
typedef const char * (*sendU_t)(id, SEL);
typedef id (*sendS_t)(id, SEL, const char *);

int main() {
    send0_t send0 = (send0_t)objc_msgSend;
    send1B_t send1B = (send1B_t)objc_msgSend;
    sendU_t sendU = (sendU_t)objc_msgSend;
    sendS_t sendS = (sendS_t)objc_msgSend;
    send2_t send2 = (send2_t)objc_msgSend;

    Class fmCls = objc_getClass("NSFileManager");
    Class nsstringCls = objc_getClass("NSString");
    SEL defaultMgrSel = sel_registerName("defaultManager");
    SEL fileExistsSel = sel_registerName("fileExistsAtPath:");
    SEL cwdSel = sel_registerName("currentDirectoryPath");
    SEL swuSel = sel_registerName("stringWithUTF8String:");

    // 1. Get default manager
    id fm = send0((id)fmCls, defaultMgrSel);
    if (!fm) { printf("FAIL: 1 - defaultManager returned nil\n"); return 1; }
    printf("CHECK 1: defaultManager obtained\n");

    // 2. Singleton check
    id fm2 = send0((id)fmCls, defaultMgrSel);
    if (fm != fm2) { printf("FAIL: 2 - defaultManager not singleton\n"); return 2; }
    printf("CHECK 2: defaultManager is singleton\n");

    // 3. fileExistsAtPath: for /usr (should exist)
    id usrPath = sendS((id)nsstringCls, swuSel, "/usr");
    BOOL exists = send1B(fm, fileExistsSel, usrPath);
    if (!exists) { printf("FAIL: 3 - /usr should exist\n"); return 3; }
    printf("CHECK 3: /usr exists\n");

    // 4. fileExistsAtPath: for /nonexistent_path_xyz (should not exist)
    id noPath = sendS((id)nsstringCls, swuSel, "/nonexistent_path_xyz_12345");
    BOOL noExists = send1B(fm, fileExistsSel, noPath);
    if (noExists) { printf("FAIL: 4 - nonexistent path should not exist\n"); return 4; }
    printf("CHECK 4: nonexistent path does not exist\n");

    // 5. currentDirectoryPath is non-nil
    id cwd = send0(fm, cwdSel);
    if (!cwd) { printf("FAIL: 5 - currentDirectoryPath is nil\n"); return 5; }
    const char *cwdStr = sendU(cwd, sel_registerName("UTF8String"));
    printf("CHECK 5: currentDirectoryPath = %s\n", cwdStr);

    // 6. contentsOfDirectoryAtPath:error: for /tmp
    SEL contentsSel = sel_registerName("contentsOfDirectoryAtPath:error:");
    id tmpPath = sendS((id)nsstringCls, swuSel, "/tmp");
    NSError *err = nil;
    id contents = send2(fm, contentsSel, tmpPath, &err);
    if (!contents) { printf("FAIL: 6 - contentsOfDirectory returned nil\n"); return 6; }
    printf("CHECK 6: contentsOfDirectory returned array\n");

    printf("OK: NSFileManager basics passed\n");
    return 0;
}
