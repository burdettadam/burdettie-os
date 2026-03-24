// nsstring_path — fixture: NSString path utility methods.
//
// Tests pathExtension, lastPathComponent, stringByDeletingLastPathComponent,
// stringByDeletingPathExtension, stringByAppendingPathComponent:, isAbsolutePath.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>
#include <string.h>

int main() {
    typedef id (*MsgSend0)(id, SEL);
    typedef id (*MsgSend1)(id, SEL, id);
    typedef BOOL (*MsgSendBool)(id, SEL);
    typedef const char *(*MsgSendUTF8)(id, SEL);

    MsgSend0 send0 = (MsgSend0)objc_msgSend;
    MsgSend1 send1 = (MsgSend1)objc_msgSend;
    MsgSendBool sendBool = (MsgSendBool)objc_msgSend;
    MsgSendUTF8 sendUTF8 = (MsgSendUTF8)objc_msgSend;

    SEL utf8Sel = sel_registerName("UTF8String");

    id path = @"/usr/local/bin/clang.exe";

    // ── 1. pathExtension ──────────────────────────────────────────
    SEL peSel = sel_registerName("pathExtension");
    id ext = send0(path, peSel);
    const char *extStr = sendUTF8(ext, utf8Sel);
    if (!extStr || strcmp(extStr, "exe") != 0) {
        printf("FAIL: pathExtension = '%s' expected 'exe'\n", extStr ? extStr : "(null)");
        return 1;
    }

    // ── 2. lastPathComponent ──────────────────────────────────────
    SEL lpcSel = sel_registerName("lastPathComponent");
    id lastComp = send0(path, lpcSel);
    const char *compStr = sendUTF8(lastComp, utf8Sel);
    if (!compStr || strcmp(compStr, "clang.exe") != 0) {
        printf("FAIL: lastPathComponent = '%s' expected 'clang.exe'\n", compStr ? compStr : "(null)");
        return 2;
    }

    // ── 3. stringByDeletingLastPathComponent ──────────────────────
    SEL dlpcSel = sel_registerName("stringByDeletingLastPathComponent");
    id parent = send0(path, dlpcSel);
    const char *parentStr = sendUTF8(parent, utf8Sel);
    if (!parentStr || strcmp(parentStr, "/usr/local/bin") != 0) {
        printf("FAIL: stringByDeletingLastPathComponent = '%s'\n", parentStr ? parentStr : "(null)");
        return 3;
    }

    // ── 4. stringByDeletingPathExtension ──────────────────────────
    SEL dpeSel = sel_registerName("stringByDeletingPathExtension");
    id noext = send0(path, dpeSel);
    const char *noextStr = sendUTF8(noext, utf8Sel);
    if (!noextStr || strcmp(noextStr, "/usr/local/bin/clang") != 0) {
        printf("FAIL: stringByDeletingPathExtension = '%s'\n", noextStr ? noextStr : "(null)");
        return 4;
    }

    // ── 5. stringByAppendingPathComponent: ────────────────────────
    SEL apcSel = sel_registerName("stringByAppendingPathComponent:");
    id joined = send1(@"/usr/local", apcSel, @"bin");
    const char *joinedStr = sendUTF8(joined, utf8Sel);
    if (!joinedStr || strcmp(joinedStr, "/usr/local/bin") != 0) {
        printf("FAIL: stringByAppendingPathComponent = '%s'\n", joinedStr ? joinedStr : "(null)");
        return 5;
    }

    // ── 6. isAbsolutePath YES ─────────────────────────────────────
    SEL iapSel = sel_registerName("isAbsolutePath");
    BOOL isAbs = sendBool(path, iapSel);
    if (!isAbs) { printf("FAIL: isAbsolutePath should be YES\n"); return 6; }

    // ── 7. isAbsolutePath NO ──────────────────────────────────────
    BOOL isAbsRel = sendBool(@"relative/path", iapSel);
    if (isAbsRel) { printf("FAIL: isAbsolutePath relative should be NO\n"); return 7; }

    // ── 8. pathExtension with no extension ────────────────────────
    id noExtPath = @"/usr/local/bin/clang";
    id noExtResult = send0(noExtPath, peSel);
    const char *noExtStr2 = sendUTF8(noExtResult, utf8Sel);
    if (!noExtStr2 || strcmp(noExtStr2, "") != 0) {
        printf("FAIL: pathExtension no dot = '%s'\n", noExtStr2 ? noExtStr2 : "(null)");
        return 8;
    }

    printf("OK: NSString path utilities passed\n");
    return 0;
}
