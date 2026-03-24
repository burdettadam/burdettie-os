// nsmutabledictionary_basics — T22 fixture: NSMutableDictionary mutation methods
//
// Tests +[NSMutableDictionary dictionary], setObject:forKey:,
// objectForKey:, removeObjectForKey:, removeAllObjects, count,
// subscript setter/getter.
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
    void (*sendSet)(id, SEL, id, id) = (void (*)(id, SEL, id, id))objc_msgSend;
    void (*sendVoid1)(id, SEL, id)   = (void (*)(id, SEL, id))objc_msgSend;
    const char *(*sendUTF8)(id, SEL) = (const char *(*)(id, SEL))objc_msgSend;

    SEL countSel     = sel_registerName("count");
    SEL setSel       = sel_registerName("setObject:forKey:");
    SEL getSel       = sel_registerName("objectForKey:");
    SEL removeSel    = sel_registerName("removeObjectForKey:");
    SEL removeAllSel = sel_registerName("removeAllObjects");
    SEL utf8Sel      = sel_registerName("UTF8String");

    // 1. Create empty mutable dictionary
    id dict = [NSMutableDictionary dictionary];
    NSUInteger cnt = sendCount(dict, countSel);
    if (cnt != 0) {
        printf("FAIL 1: expected count 0, got %lu\n", cnt);
        return 1;
    }
    printf("T22 check 1 OK: empty dict count=0\n");

    // 2. setObject:forKey: — add two entries
    sendSet(dict, setSel, @"Alice", @"name");
    sendSet(dict, setSel, @"30", @"age");
    cnt = sendCount(dict, countSel);
    if (cnt != 2) {
        printf("FAIL 2: expected count 2, got %lu\n", cnt);
        return 1;
    }
    printf("T22 check 2 OK: count=2 after 2 sets\n");

    // 3. objectForKey: hit
    {
        id val = sendId1(dict, getSel, @"name");
        const char *s = sendUTF8(val, utf8Sel);
        if (strcmp(s, "Alice") != 0) {
            printf("FAIL 3: expected 'Alice' got '%s'\n", s);
            return 1;
        }
    }
    printf("T22 check 3 OK: objectForKey hit\n");

    // 4. objectForKey: miss
    {
        id val = sendId1(dict, getSel, @"missing");
        if (val != nil) {
            printf("FAIL 4: expected nil for missing key\n");
            return 1;
        }
    }
    printf("T22 check 4 OK: objectForKey miss returns nil\n");

    // 5. Update existing key
    sendSet(dict, setSel, @"Bob", @"name");
    cnt = sendCount(dict, countSel);
    if (cnt != 2) {
        printf("FAIL 5: expected count still 2, got %lu\n", cnt);
        return 1;
    }
    {
        id val = sendId1(dict, getSel, @"name");
        const char *s = sendUTF8(val, utf8Sel);
        if (strcmp(s, "Bob") != 0) {
            printf("FAIL 5b: expected 'Bob' got '%s'\n", s);
            return 1;
        }
    }
    printf("T22 check 5 OK: update existing key\n");

    // 6. removeObjectForKey:
    sendVoid1(dict, removeSel, @"age");
    cnt = sendCount(dict, countSel);
    if (cnt != 1) {
        printf("FAIL 6: expected count 1, got %lu\n", cnt);
        return 1;
    }
    {
        id val = sendId1(dict, getSel, @"age");
        if (val != nil) {
            printf("FAIL 6b: expected nil after remove\n");
            return 1;
        }
    }
    printf("T22 check 6 OK: removeObjectForKey\n");

    // 7. removeAllObjects
    sendSet(dict, setSel, @"v1", @"k1");
    sendSet(dict, setSel, @"v2", @"k2");
    sendId0(dict, removeAllSel);
    cnt = sendCount(dict, countSel);
    if (cnt != 0) {
        printf("FAIL 7: expected count 0, got %lu\n", cnt);
        return 1;
    }
    printf("T22 check 7 OK: removeAllObjects\n");

    // 8. Subscript setter/getter: dict[@"key"] = val
    dict[@"color"] = @"blue";
    {
        id val = dict[@"color"];
        const char *s = sendUTF8(val, utf8Sel);
        if (strcmp(s, "blue") != 0) {
            printf("FAIL 8: expected 'blue' got '%s'\n", s);
            return 1;
        }
    }
    printf("T22 check 8 OK: subscript access\n");

    // 9. NSLog the dict description
    dict[@"x"] = @"1";
    dict[@"y"] = @"2";
    NSLog(@"T22 mutdict=%@", dict);

    printf("T22: all checks passed\n");
    return 0;
}
