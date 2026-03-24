// nsmutablearray_basics — T21 fixture: NSMutableArray mutation methods
//
// Tests +[NSMutableArray array], addObject:, removeLastObject,
// removeObjectAtIndex:, insertObject:atIndex:,
// replaceObjectAtIndex:withObject:, removeAllObjects, count,
// objectAtIndex:.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>
#include <string.h>

int main() {
    NSUInteger (*sendCount)(id, SEL) = (NSUInteger (*)(id, SEL))objc_msgSend;
    id (*sendId0)(id, SEL)           = (id (*)(id, SEL))objc_msgSend;
    id (*sendId1)(id, SEL, id)       = (id (*)(id, SEL, id))objc_msgSend;
    id (*sendIdx)(id, SEL, NSUInteger) = (id (*)(id, SEL, NSUInteger))objc_msgSend;
    void (*sendVoid1)(id, SEL, id)   = (void (*)(id, SEL, id))objc_msgSend;
    void (*sendVoidIdx)(id, SEL, NSUInteger) = (void (*)(id, SEL, NSUInteger))objc_msgSend;
    void (*sendInsert)(id, SEL, id, NSUInteger) = (void (*)(id, SEL, id, NSUInteger))objc_msgSend;
    void (*sendReplace)(id, SEL, NSUInteger, id) = (void (*)(id, SEL, NSUInteger, id))objc_msgSend;
    const char *(*sendUTF8)(id, SEL) = (const char *(*)(id, SEL))objc_msgSend;

    SEL countSel   = sel_registerName("count");
    SEL addSel     = sel_registerName("addObject:");
    SEL objAtSel   = sel_registerName("objectAtIndex:");
    SEL removeLast = sel_registerName("removeLastObject");
    SEL removeAt   = sel_registerName("removeObjectAtIndex:");
    SEL insertAt   = sel_registerName("insertObject:atIndex:");
    SEL replaceAt  = sel_registerName("replaceObjectAtIndex:withObject:");
    SEL removeAll  = sel_registerName("removeAllObjects");
    SEL utf8Sel    = sel_registerName("UTF8String");

    // 1. Create empty mutable array
    id arr = [NSMutableArray array];
    NSUInteger cnt = sendCount(arr, countSel);
    if (cnt != 0) {
        printf("FAIL 1: expected count 0, got %lu\n", cnt);
        return 1;
    }
    printf("T21 check 1 OK: empty array count=0\n");

    // 2. addObject: x3
    sendVoid1(arr, addSel, @"alpha");
    sendVoid1(arr, addSel, @"beta");
    sendVoid1(arr, addSel, @"gamma");
    cnt = sendCount(arr, countSel);
    if (cnt != 3) {
        printf("FAIL 2: expected count 3, got %lu\n", cnt);
        return 1;
    }
    printf("T21 check 2 OK: count=3 after 3 adds\n");

    // 3. objectAtIndex: verification
    {
        id obj0 = sendIdx(arr, objAtSel, 0);
        const char *s0 = sendUTF8(obj0, utf8Sel);
        if (strcmp(s0, "alpha") != 0) {
            printf("FAIL 3: expected 'alpha' got '%s'\n", s0);
            return 1;
        }
        id obj2 = sendIdx(arr, objAtSel, 2);
        const char *s2 = sendUTF8(obj2, utf8Sel);
        if (strcmp(s2, "gamma") != 0) {
            printf("FAIL 3b: expected 'gamma' got '%s'\n", s2);
            return 1;
        }
    }
    printf("T21 check 3 OK: objectAtIndex correct\n");

    // 4. removeLastObject
    sendId0(arr, removeLast);
    cnt = sendCount(arr, countSel);
    if (cnt != 2) {
        printf("FAIL 4: expected count 2, got %lu\n", cnt);
        return 1;
    }
    printf("T21 check 4 OK: removeLastObject -> count=2\n");

    // 5. insertObject:atIndex: — insert "aaa" at index 0
    sendInsert(arr, insertAt, @"aaa", 0);
    cnt = sendCount(arr, countSel);
    if (cnt != 3) {
        printf("FAIL 5: expected count 3, got %lu\n", cnt);
        return 1;
    }
    {
        id obj0 = sendIdx(arr, objAtSel, 0);
        const char *s0 = sendUTF8(obj0, utf8Sel);
        if (strcmp(s0, "aaa") != 0) {
            printf("FAIL 5b: expected 'aaa' at index 0, got '%s'\n", s0);
            return 1;
        }
        id obj1 = sendIdx(arr, objAtSel, 1);
        const char *s1 = sendUTF8(obj1, utf8Sel);
        if (strcmp(s1, "alpha") != 0) {
            printf("FAIL 5c: expected 'alpha' at index 1, got '%s'\n", s1);
            return 1;
        }
    }
    printf("T21 check 5 OK: insertObject:atIndex:0 correct\n");

    // 6. replaceObjectAtIndex:withObject: — replace index 1 with "ALPHA"
    sendReplace(arr, replaceAt, 1, @"ALPHA");
    {
        id obj1 = sendIdx(arr, objAtSel, 1);
        const char *s1 = sendUTF8(obj1, utf8Sel);
        if (strcmp(s1, "ALPHA") != 0) {
            printf("FAIL 6: expected 'ALPHA' at index 1, got '%s'\n", s1);
            return 1;
        }
    }
    printf("T21 check 6 OK: replaceObjectAtIndex:withObject: correct\n");

    // 7. removeObjectAtIndex: — remove index 0 ("aaa")
    sendVoidIdx(arr, removeAt, 0);
    cnt = sendCount(arr, countSel);
    if (cnt != 2) {
        printf("FAIL 7: expected count 2, got %lu\n", cnt);
        return 1;
    }
    {
        id obj0 = sendIdx(arr, objAtSel, 0);
        const char *s0 = sendUTF8(obj0, utf8Sel);
        if (strcmp(s0, "ALPHA") != 0) {
            printf("FAIL 7b: expected 'ALPHA' at index 0, got '%s'\n", s0);
            return 1;
        }
    }
    printf("T21 check 7 OK: removeObjectAtIndex:0 correct\n");

    // 8. removeAllObjects
    sendId0(arr, removeAll);
    cnt = sendCount(arr, countSel);
    if (cnt != 0) {
        printf("FAIL 8: expected count 0 after removeAllObjects, got %lu\n", cnt);
        return 1;
    }
    printf("T21 check 8 OK: removeAllObjects -> count=0\n");

    // 9. NSLog the array description
    sendVoid1(arr, addSel, @"one");
    sendVoid1(arr, addSel, @"two");
    NSLog(@"T21 mutarr=%@", arr);

    printf("T21: all checks passed\n");
    return 0;
}
