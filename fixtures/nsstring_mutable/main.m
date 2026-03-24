// nsstring_mutable — T21 fixture: NSMutableString + NSString enhancements
//
// Tests NSMutableString mutation methods (appendString:, insertString:atIndex:,
// deleteCharactersInRange:, replaceCharactersInRange:withString:, setString:)
// and new NSString methods (stringByReplacingOccurrencesOfString:withString:,
// componentsSeparatedByString:, rangeOfString:, integerValue, doubleValue,
// mutableCopy).
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>
#include <string.h>

int main() {
    // ── msg_send casts ─────────────────────────────────────────────
    id   (*sendId0)(id, SEL)                        = (id (*)(id, SEL))objc_msgSend;
    id   (*sendId1)(id, SEL, id)                    = (id (*)(id, SEL, id))objc_msgSend;
    id   (*sendId2)(id, SEL, id, id)                = (id (*)(id, SEL, id, id))objc_msgSend;
    void (*sendVoid1)(id, SEL, id)                  = (void (*)(id, SEL, id))objc_msgSend;
    void (*sendInsert)(id, SEL, id, NSUInteger)     = (void (*)(id, SEL, id, NSUInteger))objc_msgSend;
    void (*sendDelRange)(id, SEL, NSRange)          = (void (*)(id, SEL, NSRange))objc_msgSend;
    void (*sendReplRange)(id, SEL, NSRange, id)     = (void (*)(id, SEL, NSRange, id))objc_msgSend;
    NSUInteger (*sendLen)(id, SEL)                  = (NSUInteger (*)(id, SEL))objc_msgSend;
    NSInteger  (*sendInt)(id, SEL)                  = (NSInteger (*)(id, SEL))objc_msgSend;
    const char *(*sendUTF8)(id, SEL)                = (const char *(*)(id, SEL))objc_msgSend;
    NSRange (*sendRange1)(id, SEL, id)              = (NSRange (*)(id, SEL, id))objc_msgSend;

    SEL utf8Sel  = sel_registerName("UTF8String");
    SEL lenSel   = sel_registerName("length");

    // ════════════════════════════════════════════════════════════════
    // Part A: NSMutableString
    // ════════════════════════════════════════════════════════════════

    // ── 1. mutableCopy creates NSMutableString ─────────────────────
    id base = @"Hello";
    id mstr = sendId0(base, sel_registerName("mutableCopy"));
    if (!mstr) { printf("FAIL 1: mutableCopy nil\n"); return 1; }
    NSUInteger mlen = sendLen(mstr, lenSel);
    if (mlen != 5) { printf("FAIL 1: mutableCopy len=%lu\n", mlen); return 1; }

    // ── 2. appendString: ──────────────────────────────────────────
    sendVoid1(mstr, sel_registerName("appendString:"), @" World");
    const char *s2 = sendUTF8(mstr, utf8Sel);
    if (!s2 || strcmp(s2, "Hello World") != 0) {
        printf("FAIL 2: append='%s'\n", s2 ? s2 : "(null)"); return 2;
    }
    NSLog(@"T21 appended=%s", s2);

    // ── 3. insertString:atIndex: ──────────────────────────────────
    sendInsert(mstr, sel_registerName("insertString:atIndex:"), @"Dear ", (NSUInteger)6);
    const char *s3 = sendUTF8(mstr, utf8Sel);
    if (!s3 || strcmp(s3, "Hello Dear World") != 0) {
        printf("FAIL 3: insert='%s'\n", s3 ? s3 : "(null)"); return 3;
    }

    // ── 4. deleteCharactersInRange: ───────────────────────────────
    // Delete "Dear " (range 6..5) → "Hello World"
    sendDelRange(mstr, sel_registerName("deleteCharactersInRange:"), NSMakeRange(6, 5));
    const char *s4 = sendUTF8(mstr, utf8Sel);
    if (!s4 || strcmp(s4, "Hello World") != 0) {
        printf("FAIL 4: delete='%s'\n", s4 ? s4 : "(null)"); return 4;
    }

    // ── 5. replaceCharactersInRange:withString: ───────────────────
    // Replace "World" (range 6..5) with "Rust"
    sendReplRange(mstr, sel_registerName("replaceCharactersInRange:withString:"),
                  NSMakeRange(6, 5), @"Rust");
    const char *s5 = sendUTF8(mstr, utf8Sel);
    if (!s5 || strcmp(s5, "Hello Rust") != 0) {
        printf("FAIL 5: replace='%s'\n", s5 ? s5 : "(null)"); return 5;
    }
    NSLog(@"T21 replaced=%s", s5);

    // ── 6. setString: ─────────────────────────────────────────────
    sendVoid1(mstr, sel_registerName("setString:"), @"Reset");
    const char *s6 = sendUTF8(mstr, utf8Sel);
    if (!s6 || strcmp(s6, "Reset") != 0) {
        printf("FAIL 6: setString='%s'\n", s6 ? s6 : "(null)"); return 6;
    }

    // ── 7. Multiple append on empty +[NSMutableString string] ─────
    Class msCls = objc_getClass("NSMutableString");
    id empty = sendId0((id)msCls, sel_registerName("string"));
    if (!empty) { printf("FAIL 7: string nil\n"); return 7; }
    sendVoid1(empty, sel_registerName("appendString:"), @"A");
    sendVoid1(empty, sel_registerName("appendString:"), @"B");
    sendVoid1(empty, sel_registerName("appendString:"), @"C");
    const char *s7 = sendUTF8(empty, utf8Sel);
    if (!s7 || strcmp(s7, "ABC") != 0) {
        printf("FAIL 7: multi-append='%s'\n", s7 ? s7 : "(null)"); return 7;
    }

    // ════════════════════════════════════════════════════════════════
    // Part B: New NSString methods
    // ════════════════════════════════════════════════════════════════

    // ── 8. stringByReplacingOccurrencesOfString:withString: ───────
    id src8 = @"foo bar foo baz foo";
    id rep8 = sendId2(src8,
                      sel_registerName("stringByReplacingOccurrencesOfString:withString:"),
                      @"foo", @"qux");
    const char *s8 = sendUTF8(rep8, utf8Sel);
    if (!s8 || strcmp(s8, "qux bar qux baz qux") != 0) {
        printf("FAIL 8: replace='%s'\n", s8 ? s8 : "(null)"); return 8;
    }

    // ── 9. componentsSeparatedByString: ───────────────────────────
    id csv = @"one,two,three";
    id parts = sendId1(csv, sel_registerName("componentsSeparatedByString:"), @",");
    if (!parts) { printf("FAIL 9: components nil\n"); return 9; }
    NSUInteger partCount = sendLen(parts, sel_registerName("count"));
    if (partCount != 3) {
        printf("FAIL 9: count=%lu\n", partCount); return 9;
    }
    // Check first element
    id (*sendIdx)(id, SEL, NSUInteger) = (id (*)(id, SEL, NSUInteger))objc_msgSend;
    id first = sendIdx(parts, sel_registerName("objectAtIndex:"), (NSUInteger)0);
    const char *firstStr = sendUTF8(first, utf8Sel);
    if (!firstStr || strcmp(firstStr, "one") != 0) {
        printf("FAIL 9: first='%s'\n", firstStr ? firstStr : "(null)"); return 9;
    }
    // Check last element
    id last = sendIdx(parts, sel_registerName("objectAtIndex:"), (NSUInteger)2);
    const char *lastStr = sendUTF8(last, utf8Sel);
    if (!lastStr || strcmp(lastStr, "three") != 0) {
        printf("FAIL 9: last='%s'\n", lastStr ? lastStr : "(null)"); return 9;
    }

    // ── 10. rangeOfString: found ──────────────────────────────────
    id haystack = @"Hello World";
    NSRange r10 = sendRange1(haystack, sel_registerName("rangeOfString:"), @"World");
    if (r10.location != 6 || r10.length != 5) {
        printf("FAIL 10: range={%lu,%lu}\n", r10.location, r10.length); return 10;
    }

    // ── 11. rangeOfString: not found ──────────────────────────────
    NSRange r11 = sendRange1(haystack, sel_registerName("rangeOfString:"), @"xyz");
    if (r11.location != NSNotFound) {
        printf("FAIL 11: range.location=%lu\n", r11.location); return 11;
    }

    // ── 12. integerValue ──────────────────────────────────────────
    NSInteger iv = sendInt(@"42abc", sel_registerName("integerValue"));
    if (iv != 42) {
        printf("FAIL 12: integerValue=%ld\n", (long)iv); return 12;
    }

    // ── 13. integerValue negative ─────────────────────────────────
    NSInteger iv2 = sendInt(@"-7", sel_registerName("integerValue"));
    if (iv2 != -7) {
        printf("FAIL 13: integerValue=%ld\n", (long)iv2); return 13;
    }

    // ── 14. integerValue non-numeric ──────────────────────────────
    NSInteger iv3 = sendInt(@"abc", sel_registerName("integerValue"));
    if (iv3 != 0) {
        printf("FAIL 14: integerValue=%ld\n", (long)iv3); return 14;
    }

    // ── 15. NSMutableString length after mutations ────────────────
    id m15 = sendId0(@"test", sel_registerName("mutableCopy"));
    sendVoid1(m15, sel_registerName("appendString:"), @"ing");
    NSUInteger len15 = sendLen(m15, lenSel);
    if (len15 != 7) {
        printf("FAIL 15: len=%lu\n", len15); return 15;
    }
    const char *s15 = sendUTF8(m15, utf8Sel);
    if (!s15 || strcmp(s15, "testing") != 0) {
        printf("FAIL 15: result='%s'\n", s15 ? s15 : "(null)"); return 15;
    }

    NSLog(@"T21 final=%s", s15);

    printf("T21: all checks passed\n");
    return 0;
}
