// nsstring_dynamic — T15 fixture: dynamic NSString creation and comparison
//
// Tests [[NSString alloc] initWithUTF8String:], [NSString stringWithUTF8String:],
// length, UTF8String, isEqualToString:, and ObjC introspection helpers.
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
    id (*sendId)(id, SEL)           = (id (*)(id, SEL))objc_msgSend;
    id (*sendId1)(id, SEL, id)      = (id (*)(id, SEL, id))objc_msgSend;
    id (*sendStr)(id, SEL, const char *) = (id (*)(id, SEL, const char *))objc_msgSend;

    NSUInteger (*sendLen)(id, SEL)  = (NSUInteger (*)(id, SEL))objc_msgSend;
    const char *(*sendUTF8)(id, SEL) = (const char *(*)(id, SEL))objc_msgSend;
    BOOL (*sendEq)(id, SEL, id)     = (BOOL (*)(id, SEL, id))objc_msgSend;

    // ── 1. [[NSString alloc] initWithUTF8String:"hello"] ───────────
    Class nsstringClass = (Class)objc_getClass("NSString");
    if (!nsstringClass) { printf("FAIL: NSString class lookup\n"); return 1; }

    SEL allocSel = sel_registerName("alloc");
    SEL initUTF8Sel = sel_registerName("initWithUTF8String:");

    id raw = sendId((id)nsstringClass, allocSel);
    if (!raw) { printf("FAIL: NSString alloc\n"); return 2; }

    id str1 = sendStr(raw, initUTF8Sel, "hello");
    if (!str1) { printf("FAIL: initWithUTF8String\n"); return 3; }

    // ── 2. [str1 length] == 5 ──────────────────────────────────────
    SEL lengthSel = sel_registerName("length");
    NSUInteger len1 = sendLen(str1, lengthSel);
    if (len1 != 5) { printf("FAIL: length=%lu\n", (unsigned long)len1); return 4; }

    // ── 3. [str1 UTF8String] == "hello" ────────────────────────────
    SEL utf8Sel = sel_registerName("UTF8String");
    const char *u1 = sendUTF8(str1, utf8Sel);
    if (!u1 || strcmp(u1, "hello") != 0) { printf("FAIL: UTF8String='%s'\n", u1); return 5; }

    // ── 4. [NSString stringWithUTF8String:"hello"] ─────────────────
    SEL swuSel = sel_registerName("stringWithUTF8String:");
    id str2 = sendStr((id)nsstringClass, swuSel, "hello");
    if (!str2) { printf("FAIL: stringWithUTF8String\n"); return 6; }

    // ── 5. [str2 length] == 5, UTF8String == "hello" ───────────────
    NSUInteger len2 = sendLen(str2, lengthSel);
    if (len2 != 5) { printf("FAIL: str2 length=%lu\n", (unsigned long)len2); return 7; }
    const char *u2 = sendUTF8(str2, utf8Sel);
    if (!u2 || strcmp(u2, "hello") != 0) { printf("FAIL: str2 UTF8='%s'\n", u2); return 8; }

    // ── 6. [str1 isEqualToString:str2] == YES ──────────────────────
    SEL eqSel = sel_registerName("isEqualToString:");
    BOOL eq = sendEq(str1, eqSel, str2);
    if (!eq) { printf("FAIL: isEqualToString same\n"); return 9; }

    // ── 7. Different string, isEqualToString: == NO ────────────────
    id str3 = sendStr((id)nsstringClass, swuSel, "world");
    if (!str3) { printf("FAIL: str3 create\n"); return 10; }
    BOOL neq = sendEq(str1, eqSel, str3);
    if (neq) { printf("FAIL: isEqualToString diff\n"); return 11; }

    // ── 8. Literal @"hello" isEqualToString: dynamic string ────────
    NSString *literal = @"hello";
    BOOL litEq = sendEq((id)literal, eqSel, str1);
    if (!litEq) { printf("FAIL: literal eq dynamic\n"); return 12; }

    // ── 9. Introspection: object_getClass returns non-null class ──
    //    (On real macOS, class clusters mean object_getClass returns a
    //    private subclass like __NSCFString, not NSString itself.)
    Class cls1 = object_getClass((id)str1);
    if (!cls1) { printf("FAIL: object_getClass null\n"); return 13; }

    // ── 10. Introspection: class_getSuperclass ─────────────────────
    Class superCls = class_getSuperclass(nsstringClass);
    Class nsobjectClass = (Class)objc_getClass("NSObject");
    if (superCls != nsobjectClass) {
        printf("FAIL: superclass=%p expected NSObject=%p\n", (void*)superCls, (void*)nsobjectClass);
        return 14;
    }

    // ── 11. Introspection: class_respondsToSelector ────────────────
    BOOL responds = class_respondsToSelector(nsstringClass, lengthSel);
    if (!responds) { printf("FAIL: respondsToSelector length\n"); return 15; }

    // ── 12. Introspection: sel_isEqual ─────────────────────────────
    SEL sel_a = sel_registerName("length");
    SEL sel_b = sel_registerName("length");
    if (!sel_isEqual(sel_a, sel_b)) { printf("FAIL: sel_isEqual\n"); return 16; }

    // ── 13. [str3 length] == 5 (for "world") ───────────────────────
    NSUInteger len3 = sendLen(str3, lengthSel);
    if (len3 != 5) { printf("FAIL: str3 length=%lu\n", (unsigned long)len3); return 17; }

    // ── 14. NSLog(@"%@", str1) ─────────────────────────────────────
    NSLog(@"T15 str1=%@", str1);

    printf("T15: all checks passed\n");
    return 0;
}
