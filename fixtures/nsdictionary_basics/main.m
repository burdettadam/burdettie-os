// nsdictionary_basics — T18 fixture: NSDictionary creation and access
//
// Tests [NSDictionary dictionaryWithObjects:forKeys:count:],
// objectForKey:, objectForKeyedSubscript:, allKeys, allValues, count,
// and NSLog %@ on dictionaries.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>

int main() {
    // ── Setup: msg_send casts ──────────────────────────────────────
    id (*sendId)(id, SEL, ...)       = (id (*)(id, SEL, ...))objc_msgSend;
    NSUInteger (*sendCount)(id, SEL) = (NSUInteger (*)(id, SEL))objc_msgSend;
    id (*sendObj1)(id, SEL, id)      = (id (*)(id, SEL, id))objc_msgSend;
    id (*sendObjAtIdx)(id, SEL, NSUInteger) = (id (*)(id, SEL, NSUInteger))objc_msgSend;

    // Factory cast: dictionaryWithObjects:forKeys:count:
    id (*sendFactory)(id, SEL, id*, id*, NSUInteger) =
        (id (*)(id, SEL, id*, id*, NSUInteger))objc_msgSend;

    // ── 1. NSDictionary class lookup ───────────────────────────────
    Class nsdictClass = (Class)objc_getClass("NSDictionary");
    if (!nsdictClass) { printf("FAIL: NSDictionary class\n"); return 1; }

    // ── 2. Create dictionary with 2 key-value pairs ────────────────
    SEL factorySel = sel_registerName("dictionaryWithObjects:forKeys:count:");
    id keys[] = { @"name", @"city" };
    id vals[] = { @"Alice", @"Portland" };
    id dict = sendFactory((id)nsdictClass, factorySel, vals, keys, 2);
    if (!dict) { printf("FAIL: dictionaryWithObjects\n"); return 2; }

    // ── 3. count == 2 ──────────────────────────────────────────────
    SEL countSel = sel_registerName("count");
    NSUInteger c = sendCount(dict, countSel);
    if (c != 2) { printf("FAIL: count=%lu\n", (unsigned long)c); return 3; }

    // ── 4. objectForKey: hit ───────────────────────────────────────
    SEL ofkSel = sel_registerName("objectForKey:");
    id nameKey = @"name";
    id nameVal = sendObj1(dict, ofkSel, nameKey);
    if (!nameVal) { printf("FAIL: objectForKey name nil\n"); return 4; }

    // ── 5. Check value content ─────────────────────────────────────
    SEL utf8Sel = sel_registerName("UTF8String");
    const char *nameStr = ((const char *(*)(id, SEL))objc_msgSend)(nameVal, utf8Sel);
    if (!nameStr || strcmp(nameStr, "Alice") != 0) {
        printf("FAIL: objectForKey name != Alice\n"); return 5;
    }

    // ── 6. objectForKey: miss ──────────────────────────────────────
    id missKey = @"missing";
    id missVal = sendObj1(dict, ofkSel, missKey);
    if (missVal) { printf("FAIL: objectForKey missing should be nil\n"); return 6; }

    // ── 7. objectForKeyedSubscript: (dict[@"city"]) ────────────────
    SEL subSel = sel_registerName("objectForKeyedSubscript:");
    id cityKey = @"city";
    id cityVal = sendObj1(dict, subSel, cityKey);
    if (!cityVal) { printf("FAIL: subscript city nil\n"); return 7; }

    const char *cityStr = ((const char *(*)(id, SEL))objc_msgSend)(cityVal, utf8Sel);
    if (!cityStr || strcmp(cityStr, "Portland") != 0) {
        printf("FAIL: subscript city != Portland\n"); return 8;
    }

    // ── 9. allKeys returns NSArray ─────────────────────────────────
    SEL akSel = sel_registerName("allKeys");
    id allK = sendId(dict, akSel);
    if (!allK) { printf("FAIL: allKeys nil\n"); return 9; }

    NSUInteger akCount = sendCount(allK, countSel);
    if (akCount != 2) { printf("FAIL: allKeys count=%lu\n", (unsigned long)akCount); return 10; }

    // ── 11. allValues returns NSArray ───────────────────────────────
    SEL avSel = sel_registerName("allValues");
    id allV = sendId(dict, avSel);
    if (!allV) { printf("FAIL: allValues nil\n"); return 11; }

    NSUInteger avCount = sendCount(allV, countSel);
    if (avCount != 2) { printf("FAIL: allValues count=%lu\n", (unsigned long)avCount); return 12; }

    // ── 13. Empty dictionary ───────────────────────────────────────
    id emptyDict = sendFactory((id)nsdictClass, factorySel, NULL, NULL, 0);
    if (!emptyDict) { printf("FAIL: empty dict nil\n"); return 13; }

    NSUInteger emptyC = sendCount(emptyDict, countSel);
    if (emptyC != 0) { printf("FAIL: empty count=%lu\n", (unsigned long)emptyC); return 14; }

    // ── 15. objectForKey: on empty dict → nil ──────────────────────
    id emptyHit = sendObj1(emptyDict, ofkSel, @"x");
    if (emptyHit) { printf("FAIL: empty objectForKey should be nil\n"); return 15; }

    // ── 16. NSLog with dictionary description ──────────────────────
    NSLog(@"T18 dict=%@", dict);

    printf("T18: all checks passed\n");
    return 0;
}
