// collection_enum — NSEnumerator basics: objectEnumerator + keyEnumerator.
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>

typedef id (*send0_t)(id, SEL);
typedef id (*send1_t)(id, SEL, id);
typedef id (*sendS_t)(id, SEL, const char *);
typedef id (*sendOC_t)(id, SEL, id *, NSUInteger);
typedef NSUInteger (*usend0_t)(id, SEL);
typedef const char *(*csend0_t)(id, SEL);

int main() {
    @autoreleasepool {
        send0_t send0 = (send0_t)objc_msgSend;
        send1_t send1 = (send1_t)objc_msgSend;
        sendS_t sendS = (sendS_t)objc_msgSend;
        usend0_t usend0 = (usend0_t)objc_msgSend;
        csend0_t csend0 = (csend0_t)objc_msgSend;

        Class strCls = objc_getClass("NSString");
        Class arrCls = objc_getClass("NSArray");
        Class dictCls = objc_getClass("NSDictionary");
        SEL swu = sel_registerName("stringWithUTF8String:");
        SEL utf8 = sel_registerName("UTF8String");

        // 1. Build an NSArray and enumerate with objectEnumerator
        id items[3];
        items[0] = sendS((id)strCls, swu, "alpha");
        items[1] = sendS((id)strCls, swu, "beta");
        items[2] = sendS((id)strCls, swu, "gamma");
        id arr = ((id(*)(id,SEL,id*,NSUInteger))objc_msgSend)(
            (id)arrCls, sel_registerName("arrayWithObjects:count:"), items, 3);

        id enumerator = send0(arr, sel_registerName("objectEnumerator"));
        if (!enumerator) { printf("FAIL: 1 - objectEnumerator nil\n"); return 1; }
        printf("CHECK 1: objectEnumerator created\n");

        // 2. nextObject iteration
        int enumCount = 0;
        id obj;
        while ((obj = send0(enumerator, sel_registerName("nextObject"))) != nil) {
            enumCount++;
        }
        if (enumCount != 3) { printf("FAIL: 2 - enumCount %d\n", enumCount); return 2; }
        printf("CHECK 2: enumerated %d objects\n", enumCount);

        // 3. nextObject returns nil after exhaustion
        obj = send0(enumerator, sel_registerName("nextObject"));
        if (obj != nil) { printf("FAIL: 3 - nextObject not nil after exhaustion\n"); return 3; }
        printf("CHECK 3: nextObject returns nil after exhaustion\n");

        // 4. allObjects — get remaining from a fresh enumerator after consuming 1
        id enum2 = send0(arr, sel_registerName("objectEnumerator"));
        send0(enum2, sel_registerName("nextObject")); // consume first
        id remaining = send0(enum2, sel_registerName("allObjects"));
        NSUInteger remCount = usend0(remaining, sel_registerName("count"));
        if (remCount != 2) { printf("FAIL: 4 - remaining count %lu\n", (unsigned long)remCount); return 4; }
        printf("CHECK 4: allObjects remaining = %lu\n", (unsigned long)remCount);

        // 5. NSDictionary keyEnumerator
        id keys[2] = {
            sendS((id)strCls, swu, "name"),
            sendS((id)strCls, swu, "age"),
        };
        id vals[2] = {
            sendS((id)strCls, swu, "Alice"),
            sendS((id)strCls, swu, "30"),
        };
        id dict = ((id(*)(id,SEL,id*,id*,NSUInteger))objc_msgSend)(
            (id)dictCls, sel_registerName("dictionaryWithObjects:forKeys:count:"),
            vals, keys, 2);

        id keyEnum = send0(dict, sel_registerName("keyEnumerator"));
        if (!keyEnum) { printf("FAIL: 5 - keyEnumerator nil\n"); return 5; }
        int keyCount = 0;
        while ((obj = send0(keyEnum, sel_registerName("nextObject"))) != nil) {
            keyCount++;
        }
        if (keyCount != 2) { printf("FAIL: 5 - keyCount %d\n", keyCount); return 5; }
        printf("CHECK 5: keyEnumerator enumerated %d keys\n", keyCount);

        // 6. allObjects on a fresh keyEnumerator
        id keyEnum2 = send0(dict, sel_registerName("keyEnumerator"));
        id allKeys = send0(keyEnum2, sel_registerName("allObjects"));
        NSUInteger akCount = usend0(allKeys, sel_registerName("count"));
        if (akCount != 2) { printf("FAIL: 6 - allKeys count %lu\n", (unsigned long)akCount); return 6; }
        printf("CHECK 6: allObjects on keyEnumerator = %lu\n", (unsigned long)akCount);

        printf("OK: collection_enum passed\n");
        return 0;
    }
}
