// nsstring_manip — T19 fixture: NSString manipulation methods
//
// Tests hasPrefix:, hasSuffix:, stringByAppendingString:,
// substringFromIndex:, substringToIndex:, lowercaseString,
// uppercaseString, compare:, copy, hash.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>
#include <string.h>

int main() {
    // ── Setup: msg_send casts ──────────────────────────────────────
    BOOL (*sendBool1)(id, SEL, id) = (BOOL (*)(id, SEL, id))objc_msgSend;
    id (*sendId1)(id, SEL, id)     = (id (*)(id, SEL, id))objc_msgSend;
    id (*sendIdx)(id, SEL, NSUInteger) = (id (*)(id, SEL, NSUInteger))objc_msgSend;
    id (*sendId0)(id, SEL)         = (id (*)(id, SEL))objc_msgSend;
    NSInteger (*sendCmp)(id, SEL, id) = (NSInteger (*)(id, SEL, id))objc_msgSend;
    NSUInteger (*sendHash)(id, SEL) = (NSUInteger (*)(id, SEL))objc_msgSend;
    const char *(*sendUTF8)(id, SEL) = (const char *(*)(id, SEL))objc_msgSend;

    id str = @"Hello, World!";
    SEL utf8Sel = sel_registerName("UTF8String");

    // ── 1. hasPrefix: YES ──────────────────────────────────────────
    SEL hpSel = sel_registerName("hasPrefix:");
    BOOL hp = sendBool1(str, hpSel, @"Hello");
    if (!hp) { printf("FAIL: hasPrefix Hello\n"); return 1; }

    // ── 2. hasPrefix: NO ───────────────────────────────────────────
    BOOL hp2 = sendBool1(str, hpSel, @"World");
    if (hp2) { printf("FAIL: hasPrefix World should be NO\n"); return 2; }

    // ── 3. hasSuffix: YES ──────────────────────────────────────────
    SEL hsSel = sel_registerName("hasSuffix:");
    BOOL hs = sendBool1(str, hsSel, @"World!");
    if (!hs) { printf("FAIL: hasSuffix World!\n"); return 3; }

    // ── 4. hasSuffix: NO ───────────────────────────────────────────
    BOOL hs2 = sendBool1(str, hsSel, @"Hello");
    if (hs2) { printf("FAIL: hasSuffix Hello should be NO\n"); return 4; }

    // ── 5. stringByAppendingString: ────────────────────────────────
    SEL appSel = sel_registerName("stringByAppendingString:");
    id appended = sendId1(@"foo", appSel, @"bar");
    if (!appended) { printf("FAIL: append nil\n"); return 5; }
    const char *appStr = sendUTF8(appended, utf8Sel);
    if (!appStr || strcmp(appStr, "foobar") != 0) {
        printf("FAIL: append != foobar\n"); return 6;
    }

    // ── 7. substringFromIndex: ─────────────────────────────────────
    SEL fromSel = sel_registerName("substringFromIndex:");
    id sub7 = sendIdx(str, fromSel, 7);
    const char *sub7Str = sendUTF8(sub7, utf8Sel);
    if (!sub7Str || strcmp(sub7Str, "World!") != 0) {
        printf("FAIL: substringFromIndex:7 = '%s'\n", sub7Str ? sub7Str : "(null)"); return 7;
    }

    // ── 8. substringToIndex: ───────────────────────────────────────
    SEL toSel = sel_registerName("substringToIndex:");
    id sub5 = sendIdx(str, toSel, 5);
    const char *sub5Str = sendUTF8(sub5, utf8Sel);
    if (!sub5Str || strcmp(sub5Str, "Hello") != 0) {
        printf("FAIL: substringToIndex:5 = '%s'\n", sub5Str ? sub5Str : "(null)"); return 8;
    }

    // ── 9. lowercaseString ─────────────────────────────────────────
    SEL loSel = sel_registerName("lowercaseString");
    id lo = sendId0(@"HeLLo", loSel);
    const char *loStr = sendUTF8(lo, utf8Sel);
    if (!loStr || strcmp(loStr, "hello") != 0) {
        printf("FAIL: lowercaseString = '%s'\n", loStr ? loStr : "(null)"); return 9;
    }

    // ── 10. uppercaseString ────────────────────────────────────────
    SEL upSel = sel_registerName("uppercaseString");
    id up = sendId0(@"HeLLo", upSel);
    const char *upStr = sendUTF8(up, utf8Sel);
    if (!upStr || strcmp(upStr, "HELLO") != 0) {
        printf("FAIL: uppercaseString = '%s'\n", upStr ? upStr : "(null)"); return 10;
    }

    // ── 11. compare: ascending ─────────────────────────────────────
    SEL cmpSel = sel_registerName("compare:");
    NSInteger cmp1 = sendCmp(@"apple", cmpSel, @"banana");
    if (cmp1 != NSOrderedAscending) {
        printf("FAIL: compare apple<banana = %ld\n", (long)cmp1); return 11;
    }

    // ── 12. compare: descending ────────────────────────────────────
    NSInteger cmp2 = sendCmp(@"banana", cmpSel, @"apple");
    if (cmp2 != NSOrderedDescending) {
        printf("FAIL: compare banana>apple = %ld\n", (long)cmp2); return 12;
    }

    // ── 13. compare: same ──────────────────────────────────────────
    NSInteger cmp3 = sendCmp(@"same", cmpSel, @"same");
    if (cmp3 != NSOrderedSame) {
        printf("FAIL: compare same==same = %ld\n", (long)cmp3); return 13;
    }

    // ── 14. copy returns self (immutable) ──────────────────────────
    SEL copySel = sel_registerName("copy");
    id copied = sendId0(str, copySel);
    if (copied != str) {
        printf("FAIL: copy != self\n"); return 14;
    }

    // ── 15. hash consistency ───────────────────────────────────────
    SEL hashSel = sel_registerName("hash");
    NSUInteger h1 = sendHash(@"test", hashSel);
    NSUInteger h2 = sendHash(@"test", hashSel);
    if (h1 != h2) {
        printf("FAIL: hash inconsistent\n"); return 15;
    }

    // ── 16. hash differs for different strings ─────────────────────
    NSUInteger h3 = sendHash(@"different", hashSel);
    if (h1 == h3) {
        printf("FAIL: hash collision test/different\n"); return 16;
    }

    // ── 17. hasPrefix: with self ─────────────────────────────────────
    BOOL hpSelf = sendBool1(str, hpSel, str);
    if (!hpSelf) { printf("FAIL: hasPrefix self\n"); return 17; }

    // ── 18. substringFromIndex:0 == full string ────────────────────
    id sub0 = sendIdx(str, fromSel, 0);
    const char *sub0Str = sendUTF8(sub0, utf8Sel);
    if (!sub0Str || strcmp(sub0Str, "Hello, World!") != 0) {
        printf("FAIL: substringFromIndex:0\n"); return 18;
    }

    // ── 19. NSLog with manipulated string ──────────────────────────
    NSLog(@"T19 appended=%@", appended);

    printf("T19: all checks passed\n");
    return 0;
}
