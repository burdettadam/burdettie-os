// nsprocessinfo_basics — fixture: NSProcessInfo singleton and accessors.
//
// Tests +processInfo (singleton), processName, processIdentifier,
// operatingSystemVersionString, hostName, description.
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
    typedef int (*MsgSendInt)(id, SEL);
    typedef const char *(*MsgSendUTF8)(id, SEL);

    MsgSend0 send0 = (MsgSend0)objc_msgSend;
    MsgSendInt sendInt = (MsgSendInt)objc_msgSend;
    MsgSendUTF8 sendUTF8 = (MsgSendUTF8)objc_msgSend;

    SEL utf8Sel = sel_registerName("UTF8String");

    Class piCls = objc_getClass("NSProcessInfo");
    if (!piCls) { printf("FAIL: NSProcessInfo class not found\n"); return 1; }

    // ── 1. +processInfo (singleton) ───────────────────────────────
    SEL piSel = sel_registerName("processInfo");
    id pi = send0((id)piCls, piSel);
    if (!pi) { printf("FAIL: processInfo returned nil\n"); return 2; }

    // singleton: calling again returns same instance
    id pi2 = send0((id)piCls, piSel);
    if (pi != pi2) { printf("FAIL: processInfo not singleton\n"); return 3; }

    // ── 2. processName ────────────────────────────────────────────
    SEL pnSel = sel_registerName("processName");
    id name = send0(pi, pnSel);
    if (!name) { printf("FAIL: processName nil\n"); return 4; }
    const char *nameStr = sendUTF8(name, utf8Sel);
    if (!nameStr || strlen(nameStr) == 0) {
        printf("FAIL: processName empty\n"); return 5;
    }
    printf("processName=%s\n", nameStr);

    // ── 3. processIdentifier ──────────────────────────────────────
    SEL pidSel = sel_registerName("processIdentifier");
    int pid = sendInt(pi, pidSel);
    // PID should be non-negative in the emulator
    printf("pid=%d\n", pid);

    // ── 4. operatingSystemVersionString ───────────────────────────
    SEL osvSel = sel_registerName("operatingSystemVersionString");
    id osv = send0(pi, osvSel);
    if (!osv) { printf("FAIL: osVersionString nil\n"); return 6; }
    const char *osvStr = sendUTF8(osv, utf8Sel);
    if (!osvStr || strlen(osvStr) == 0) {
        printf("FAIL: osVersionString empty\n"); return 7;
    }
    printf("osVersion=%s\n", osvStr);

    // ── 5. hostName ───────────────────────────────────────────────
    SEL hnSel = sel_registerName("hostName");
    id hn = send0(pi, hnSel);
    if (!hn) { printf("FAIL: hostName nil\n"); return 8; }
    const char *hnStr = sendUTF8(hn, utf8Sel);
    if (!hnStr || strlen(hnStr) == 0) {
        printf("FAIL: hostName empty\n"); return 9;
    }
    printf("hostName=%s\n", hnStr);

    // ── 6. description ────────────────────────────────────────────
    SEL dSel = sel_registerName("description");
    id desc = send0(pi, dSel);
    if (!desc) { printf("FAIL: description nil\n"); return 10; }

    printf("OK: NSProcessInfo basics passed\n");
    return 0;
}
