// blocks_foundation — T20 fixture: blocks inside Foundation workflows.
//
// Tests enumerateObjectsUsingBlock:, sortedArrayUsingComparator:,
// sortUsingComparator:, enumerateKeysAndObjectsUsingBlock:, dispatch_sync.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>
#include <string.h>
#include <dispatch/dispatch.h>

int main() {
    typedef id (*MsgSend0)(id, SEL);
    typedef id (*MsgSend1)(id, SEL, id);
    typedef id (*MsgSendArrFactory)(id, SEL, id*, NSUInteger);
    typedef id (*MsgSendDictFactory)(id, SEL, id*, id*, NSUInteger);
    typedef NSUInteger (*MsgSendLen)(id, SEL);
    typedef id (*MsgSendIdx)(id, SEL, NSUInteger);
    typedef void (*MsgSendVoid1)(id, SEL, id);
    typedef const char *(*MsgSendUTF8)(id, SEL);

    MsgSend0 send0 = (MsgSend0)objc_msgSend;
    MsgSend1 send1 = (MsgSend1)objc_msgSend;
    MsgSendArrFactory sendArrFactory = (MsgSendArrFactory)objc_msgSend;
    MsgSendDictFactory sendDictFactory = (MsgSendDictFactory)objc_msgSend;
    MsgSendLen sendLen = (MsgSendLen)objc_msgSend;
    MsgSendIdx sendIdx = (MsgSendIdx)objc_msgSend;
    MsgSendVoid1 sendVoid1 = (MsgSendVoid1)objc_msgSend;
    MsgSendUTF8 sendUTF8 = (MsgSendUTF8)objc_msgSend;

    SEL utf8Sel = sel_registerName("UTF8String");

    // Build NSArray via arrayWithObjects:count:
    Class nsarrayCls = objc_getClass("NSArray");
    SEL awocSel = sel_registerName("arrayWithObjects:count:");
    id objs[3] = { @"apple", @"banana", @"cherry" };
    id arr = sendArrFactory((id)nsarrayCls, awocSel, objs, 3);
    if (!arr) { printf("FAIL: array nil\n"); return 1; }

    // ── 1. enumerateObjectsUsingBlock: on NSArray ─────────────────
    __block int enumCount = 0;
    SEL enumSel = sel_registerName("enumerateObjectsUsingBlock:");
    sendVoid1(arr, enumSel, (id)^(id obj, NSUInteger idx, BOOL *stop) {
        enumCount++;
    });
    if (enumCount != 3) {
        printf("FAIL: enumerate count = %d, expected 3\n", enumCount);
        return 2;
    }

    // ── 2. enumerateObjectsUsingBlock: with early stop ────────────
    __block int stopCount = 0;
    sendVoid1(arr, enumSel, (id)^(id obj, NSUInteger idx, BOOL *stop) {
        stopCount++;
        if (idx == 1) *stop = YES;
    });
    if (stopCount != 2) {
        printf("FAIL: stop count = %d, expected 2\n", stopCount);
        return 3;
    }

    // ── 3. enumerateObjectsUsingBlock: captures element values ────
    __block const char *secondItem = NULL;
    sendVoid1(arr, enumSel, (id)^(id obj, NSUInteger idx, BOOL *stop) {
        if (idx == 1) {
            secondItem = sendUTF8(obj, utf8Sel);
        }
    });
    if (!secondItem || strcmp(secondItem, "banana") != 0) {
        printf("FAIL: second item = '%s'\n", secondItem ? secondItem : "(null)");
        return 4;
    }

    // ── 4. sortedArrayUsingComparator: ────────────────────────────
    id unsortedObjs[3] = { @"cherry", @"apple", @"banana" };
    id unsorted = sendArrFactory((id)nsarrayCls, awocSel, unsortedObjs, 3);
    SEL sortSel = sel_registerName("sortedArrayUsingComparator:");
    id sorted = send1(unsorted, sortSel, (id)^NSComparisonResult(id a, id b) {
        const char *sa = sendUTF8(a, utf8Sel);
        const char *sb = sendUTF8(b, utf8Sel);
        return (NSComparisonResult)strcmp(sa, sb);
    });
    if (!sorted) { printf("FAIL: sorted nil\n"); return 5; }

    SEL countSel = sel_registerName("count");
    NSUInteger sortedCount = sendLen(sorted, countSel);
    if (sortedCount != 3) { printf("FAIL: sorted count = %lu\n", sortedCount); return 6; }

    SEL objAtSel = sel_registerName("objectAtIndex:");
    const char *first = sendUTF8(sendIdx(sorted, objAtSel, 0), utf8Sel);
    const char *second = sendUTF8(sendIdx(sorted, objAtSel, 1), utf8Sel);
    const char *third = sendUTF8(sendIdx(sorted, objAtSel, 2), utf8Sel);
    if (!first || strcmp(first, "apple") != 0) {
        printf("FAIL: sorted[0] = '%s'\n", first ? first : "(null)"); return 7;
    }
    if (!second || strcmp(second, "banana") != 0) {
        printf("FAIL: sorted[1] = '%s'\n", second ? second : "(null)"); return 8;
    }
    if (!third || strcmp(third, "cherry") != 0) {
        printf("FAIL: sorted[2] = '%s'\n", third ? third : "(null)"); return 9;
    }

    // ── 5. NSMutableArray sortUsingComparator: ────────────────────
    Class mutArrCls = objc_getClass("NSMutableArray");
    SEL arraySel = sel_registerName("array");
    id mutArr = send0((id)mutArrCls, arraySel);
    SEL addSel = sel_registerName("addObject:");
    sendVoid1(mutArr, addSel, @"zebra");
    sendVoid1(mutArr, addSel, @"alpha");
    sendVoid1(mutArr, addSel, @"middle");

    SEL sortMutSel = sel_registerName("sortUsingComparator:");
    sendVoid1(mutArr, sortMutSel, (id)^NSComparisonResult(id a, id b) {
        const char *sa = sendUTF8(a, utf8Sel);
        const char *sb = sendUTF8(b, utf8Sel);
        return (NSComparisonResult)strcmp(sa, sb);
    });

    const char *mfirst = sendUTF8(sendIdx(mutArr, objAtSel, 0), utf8Sel);
    const char *msecond = sendUTF8(sendIdx(mutArr, objAtSel, 1), utf8Sel);
    if (!mfirst || strcmp(mfirst, "alpha") != 0) {
        printf("FAIL: mutArr[0] = '%s'\n", mfirst ? mfirst : "(null)"); return 10;
    }
    if (!msecond || strcmp(msecond, "middle") != 0) {
        printf("FAIL: mutArr[1] = '%s'\n", msecond ? msecond : "(null)"); return 11;
    }

    // ── 6. enumerateKeysAndObjectsUsingBlock: on NSDictionary ─────
    Class nsdictCls = objc_getClass("NSDictionary");
    SEL dictFactSel = sel_registerName("dictionaryWithObjects:forKeys:count:");
    id keys[2] = { @"name", @"age" };
    id vals[2] = { @"Alice", @"30" };
    id dict = sendDictFactory((id)nsdictCls, dictFactSel, vals, keys, 2);
    if (!dict) { printf("FAIL: dict nil\n"); return 12; }

    __block int dictEnumCount = 0;
    __block int foundName = 0;
    SEL dictEnumSel = sel_registerName("enumerateKeysAndObjectsUsingBlock:");
    sendVoid1(dict, dictEnumSel, (id)^(id key, id obj, BOOL *stop) {
        dictEnumCount++;
        const char *ks = sendUTF8(key, utf8Sel);
        const char *vs = sendUTF8(obj, utf8Sel);
        if (ks && strcmp(ks, "name") == 0 && vs && strcmp(vs, "Alice") == 0) {
            foundName = 1;
        }
    });
    if (dictEnumCount != 2) {
        printf("FAIL: dict enumerate count = %d\n", dictEnumCount); return 13;
    }
    if (!foundName) {
        printf("FAIL: didn't find name=Alice\n"); return 14;
    }

    // ── 7. dispatch_sync ──────────────────────────────────────────
    __block int syncVal = 0;
    dispatch_sync(dispatch_get_main_queue(), ^{
        syncVal = 42;
    });
    if (syncVal != 42) {
        printf("FAIL: dispatch_sync val = %d\n", syncVal); return 15;
    }

    printf("OK: blocks + Foundation passed\n");
    return 0;
}
