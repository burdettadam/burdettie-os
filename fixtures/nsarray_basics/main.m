// nsarray_basics — T16 fixture: NSArray creation and element access
//
// Tests [NSArray arrayWithObjects:count:], count, objectAtIndex:,
// firstObject, lastObject, containsObject:, and NSLog %@ on elements.
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
    id (*sendId)(id, SEL)              = (id (*)(id, SEL))objc_msgSend;
    id (*sendId1)(id, SEL, id)         = (id (*)(id, SEL, id))objc_msgSend;
    NSUInteger (*sendCount)(id, SEL)   = (NSUInteger (*)(id, SEL))objc_msgSend;
    id (*sendIdx)(id, SEL, NSUInteger) = (id (*)(id, SEL, NSUInteger))objc_msgSend;
    BOOL (*sendContains)(id, SEL, id)  = (BOOL (*)(id, SEL, id))objc_msgSend;
    const char *(*sendUTF8)(id, SEL)   = (const char *(*)(id, SEL))objc_msgSend;

    // ── Create some NSStrings ──────────────────────────────────────
    id (*sendStr)(id, SEL, const char *) = (id (*)(id, SEL, const char *))objc_msgSend;

    Class nsstringClass = (Class)objc_getClass("NSString");
    if (!nsstringClass) { printf("FAIL: NSString class\n"); return 1; }

    SEL swuSel = sel_registerName("stringWithUTF8String:");
    id s1 = sendStr((id)nsstringClass, swuSel, "apple");
    id s2 = sendStr((id)nsstringClass, swuSel, "banana");
    id s3 = sendStr((id)nsstringClass, swuSel, "cherry");

    if (!s1 || !s2 || !s3) { printf("FAIL: string creation\n"); return 2; }

    // ── 1. [NSArray arrayWithObjects:count:] ───────────────────────
    Class nsarrayClass = (Class)objc_getClass("NSArray");
    if (!nsarrayClass) { printf("FAIL: NSArray class lookup\n"); return 3; }

    id objects[] = { s1, s2, s3 };
    id (*sendAWOC)(id, SEL, id *, NSUInteger) =
        (id (*)(id, SEL, id *, NSUInteger))objc_msgSend;

    SEL awocSel = sel_registerName("arrayWithObjects:count:");
    id arr = sendAWOC((id)nsarrayClass, awocSel, objects, 3);
    if (!arr) { printf("FAIL: arrayWithObjects:count:\n"); return 4; }

    // ── 2. [arr count] == 3 ────────────────────────────────────────
    SEL countSel = sel_registerName("count");
    NSUInteger cnt = sendCount(arr, countSel);
    if (cnt != 3) { printf("FAIL: count=%lu\n", (unsigned long)cnt); return 5; }

    // ── 3. [arr objectAtIndex:0] → "apple" ─────────────────────────
    SEL oaiSel = sel_registerName("objectAtIndex:");
    id elem0 = sendIdx(arr, oaiSel, 0);
    if (!elem0) { printf("FAIL: objectAtIndex:0 nil\n"); return 6; }

    SEL utf8Sel = sel_registerName("UTF8String");
    const char *u0 = sendUTF8(elem0, utf8Sel);
    if (!u0 || strcmp(u0, "apple") != 0) {
        printf("FAIL: elem0='%s'\n", u0 ? u0 : "(null)"); return 7;
    }

    // ── 4. [arr objectAtIndex:2] → "cherry" ────────────────────────
    id elem2 = sendIdx(arr, oaiSel, 2);
    if (!elem2) { printf("FAIL: objectAtIndex:2 nil\n"); return 8; }
    const char *u2 = sendUTF8(elem2, utf8Sel);
    if (!u2 || strcmp(u2, "cherry") != 0) {
        printf("FAIL: elem2='%s'\n", u2 ? u2 : "(null)"); return 9;
    }

    // ── 5. [arr firstObject] → "apple" ─────────────────────────────
    SEL firstSel = sel_registerName("firstObject");
    id first = sendId(arr, firstSel);
    if (first != elem0) { printf("FAIL: firstObject\n"); return 10; }

    // ── 6. [arr lastObject] → "cherry" ─────────────────────────────
    SEL lastSel = sel_registerName("lastObject");
    id last = sendId(arr, lastSel);
    if (last != elem2) { printf("FAIL: lastObject\n"); return 11; }

    // ── 7. [arr containsObject:s2] → YES ───────────────────────────
    SEL containsSel = sel_registerName("containsObject:");
    BOOL has_s2 = sendContains(arr, containsSel, s2);
    if (!has_s2) { printf("FAIL: containsObject s2\n"); return 12; }

    // ── 8. containsObject with new string "banana" → YES ───────────
    //    (content equality, not pointer equality)
    id s2_copy = sendStr((id)nsstringClass, swuSel, "banana");
    BOOL has_copy = sendContains(arr, containsSel, s2_copy);
    if (!has_copy) { printf("FAIL: containsObject banana copy\n"); return 13; }

    // ── 9. containsObject with "grape" → NO ────────────────────────
    id s_grape = sendStr((id)nsstringClass, swuSel, "grape");
    BOOL has_grape = sendContains(arr, containsSel, s_grape);
    if (has_grape) { printf("FAIL: containsObject grape should be NO\n"); return 14; }

    // ── 10. NSLog with array element ────────────────────────────────
    NSLog(@"T16 first=%@", first);

    // ── 11. Empty array ─────────────────────────────────────────────
    id empty_objs[] = {};
    id emptyArr = sendAWOC((id)nsarrayClass, awocSel, empty_objs, 0);
    if (!emptyArr) { printf("FAIL: empty array\n"); return 15; }
    NSUInteger emptyCnt = sendCount(emptyArr, countSel);
    if (emptyCnt != 0) { printf("FAIL: empty count=%lu\n", (unsigned long)emptyCnt); return 16; }

    // ── 12. firstObject / lastObject on empty → nil ─────────────────
    id emptyFirst = sendId(emptyArr, firstSel);
    id emptyLast = sendId(emptyArr, lastSel);
    if (emptyFirst != nil) { printf("FAIL: empty firstObject\n"); return 17; }
    if (emptyLast != nil) { printf("FAIL: empty lastObject\n"); return 18; }

    printf("T16: all checks passed\n");
    return 0;
}
