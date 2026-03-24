// nsdata_basics — fixture: NSData creation and property access.
//
// Tests dataWithBytes:length:, data (empty), length, bytes,
// description, isEqualToData:.
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
    typedef id (*MsgSend2)(id, SEL, const void *, NSUInteger);
    typedef NSUInteger (*MsgSendLen)(id, SEL);
    typedef const void *(*MsgSendBytes)(id, SEL);
    typedef BOOL (*MsgSendEq)(id, SEL, id);

    MsgSend0 send0 = (MsgSend0)objc_msgSend;
    MsgSend2 send2 = (MsgSend2)objc_msgSend;
    MsgSendLen sendLen = (MsgSendLen)objc_msgSend;
    MsgSendBytes sendBytes = (MsgSendBytes)objc_msgSend;
    MsgSendEq sendEq = (MsgSendEq)objc_msgSend;

    Class nsdataCls = objc_getClass("NSData");
    if (!nsdataCls) { printf("FAIL: NSData class not found\n"); return 1; }

    // ── 1. +data (empty) ──────────────────────────────────────────
    SEL dataSel = sel_registerName("data");
    id empty = send0((id)nsdataCls, dataSel);
    if (!empty) { printf("FAIL: +data returned nil\n"); return 2; }

    SEL lenSel = sel_registerName("length");
    NSUInteger emptyLen = sendLen(empty, lenSel);
    if (emptyLen != 0) { printf("FAIL: empty length = %lu\n", emptyLen); return 3; }

    // ── 2. +dataWithBytes:length: ─────────────────────────────────
    const char bytes[] = { 0xCA, 0xFE, 0xBA, 0xBE };
    SEL dwblSel = sel_registerName("dataWithBytes:length:");
    id data = send2((id)nsdataCls, dwblSel, bytes, 4);
    if (!data) { printf("FAIL: dataWithBytes nil\n"); return 4; }

    // ── 3. -length ────────────────────────────────────────────────
    NSUInteger len = sendLen(data, lenSel);
    if (len != 4) { printf("FAIL: length = %lu\n", len); return 5; }

    // ── 4. -bytes ─────────────────────────────────────────────────
    SEL bytesSel = sel_registerName("bytes");
    const unsigned char *ptr = (const unsigned char *)sendBytes(data, bytesSel);
    if (!ptr) { printf("FAIL: bytes nil\n"); return 6; }
    if (ptr[0] != 0xCA || ptr[1] != 0xFE || ptr[2] != 0xBA || ptr[3] != 0xBE) {
        printf("FAIL: bytes mismatch\n"); return 7;
    }

    // ── 5. -isEqualToData: (YES) ──────────────────────────────────
    id data2 = send2((id)nsdataCls, dwblSel, bytes, 4);
    SEL eqSel = sel_registerName("isEqualToData:");
    BOOL eq = sendEq(data, eqSel, data2);
    if (!eq) { printf("FAIL: isEqualToData should be YES\n"); return 8; }

    // ── 6. -isEqualToData: (NO — different content) ───────────────
    const char other[] = { 0xDE, 0xAD };
    id data3 = send2((id)nsdataCls, dwblSel, other, 2);
    BOOL neq = sendEq(data, eqSel, data3);
    if (neq) { printf("FAIL: isEqualToData different should be NO\n"); return 9; }

    // ── 7. -description ───────────────────────────────────────────
    SEL descSel = sel_registerName("description");
    id desc = send0(data, descSel);
    if (!desc) { printf("FAIL: description nil\n"); return 10; }

    printf("OK: NSData basics passed\n");
    return 0;
}
