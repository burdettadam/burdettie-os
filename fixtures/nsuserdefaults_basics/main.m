// nsuserdefaults_basics — NSUserDefaults in-memory key-value store.
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>

typedef id (*send0_t)(id, SEL);
typedef id (*send1_t)(id, SEL, id);
typedef id (*send2_t)(id, SEL, id, id);
typedef id (*sendS_t)(id, SEL, const char *);
typedef id (*sendIC_t)(id, SEL, int);
typedef id (*sendB_t)(id, SEL, BOOL);
typedef NSInteger (*isend1_t)(id, SEL, id);
typedef BOOL (*bsend1_t)(id, SEL, id);
typedef BOOL (*bsend0_t)(id, SEL);
typedef const char *(*csend0_t)(id, SEL);

int main() {
    @autoreleasepool {
        send0_t send0 = (send0_t)objc_msgSend;
        send1_t send1 = (send1_t)objc_msgSend;
        send2_t send2 = (send2_t)objc_msgSend;
        sendS_t sendS = (sendS_t)objc_msgSend;
        sendIC_t sendIC = (sendIC_t)objc_msgSend;
        sendB_t sendB = (sendB_t)objc_msgSend;
        isend1_t isend1 = (isend1_t)objc_msgSend;
        bsend1_t bsend1 = (bsend1_t)objc_msgSend;
        bsend0_t bsend0 = (bsend0_t)objc_msgSend;
        csend0_t csend0 = (csend0_t)objc_msgSend;

        Class udCls = objc_getClass("NSUserDefaults");
        Class strCls = objc_getClass("NSString");
        Class numCls = objc_getClass("NSNumber");
        SEL swu = sel_registerName("stringWithUTF8String:");
        SEL utf8 = sel_registerName("UTF8String");

        // 1. Get standardUserDefaults
        id defaults = send0((id)udCls, sel_registerName("standardUserDefaults"));
        if (!defaults) { printf("FAIL: 1 - standardUserDefaults nil\n"); return 1; }
        printf("CHECK 1: standardUserDefaults obtained\n");

        // 2. Singleton identity
        id defaults2 = send0((id)udCls, sel_registerName("standardUserDefaults"));
        if (defaults != defaults2) { printf("FAIL: 2 - not singleton\n"); return 2; }
        printf("CHECK 2: singleton identity verified\n");

        // 3. setObject:forKey: + objectForKey:
        id key1 = sendS((id)strCls, swu, "username");
        id val1 = sendS((id)strCls, swu, "testuser");
        send2(defaults, sel_registerName("setObject:forKey:"), val1, key1);
        id got1 = send1(defaults, sel_registerName("objectForKey:"), key1);
        if (!got1) { printf("FAIL: 3 - objectForKey nil\n"); return 3; }
        const char *gotStr = csend0(got1, utf8);
        if (strcmp(gotStr, "testuser") != 0) {
            printf("FAIL: 3 - got %s\n", gotStr); return 3;
        }
        printf("CHECK 3: setObject/objectForKey = %s\n", gotStr);

        // 4. stringForKey:
        id got2 = send1(defaults, sel_registerName("stringForKey:"), key1);
        const char *got2Str = csend0(got2, utf8);
        if (strcmp(got2Str, "testuser") != 0) {
            printf("FAIL: 4 - stringForKey %s\n", got2Str); return 4;
        }
        printf("CHECK 4: stringForKey = %s\n", got2Str);

        // 5. integerForKey: with NSNumber
        id intKey = sendS((id)strCls, swu, "retryCount");
        id intVal = sendIC((id)numCls, sel_registerName("numberWithInt:"), 42);
        send2(defaults, sel_registerName("setObject:forKey:"), intVal, intKey);
        NSInteger intGot = isend1(defaults, sel_registerName("integerForKey:"), intKey);
        if (intGot != 42) { printf("FAIL: 5 - integerForKey %ld\n", (long)intGot); return 5; }
        printf("CHECK 5: integerForKey = %ld\n", (long)intGot);

        // 6. boolForKey: with NSNumber
        id boolKey = sendS((id)strCls, swu, "debugMode");
        id boolVal = sendB((id)numCls, sel_registerName("numberWithBool:"), YES);
        send2(defaults, sel_registerName("setObject:forKey:"), boolVal, boolKey);
        BOOL boolGot = bsend1(defaults, sel_registerName("boolForKey:"), boolKey);
        if (!boolGot) { printf("FAIL: 6 - boolForKey false\n"); return 6; }
        printf("CHECK 6: boolForKey = YES\n");

        // 7. Missing key returns nil / 0
        id missingKey = sendS((id)strCls, swu, "nonexistent");
        id missing = send1(defaults, sel_registerName("objectForKey:"), missingKey);
        if (missing != nil) { printf("FAIL: 7 - missing key not nil\n"); return 7; }
        NSInteger missingInt = isend1(defaults, sel_registerName("integerForKey:"), missingKey);
        if (missingInt != 0) { printf("FAIL: 7 - missing int %ld\n", (long)missingInt); return 7; }
        printf("CHECK 7: missing key returns nil/0\n");

        // 8. removeObjectForKey:
        send1(defaults, sel_registerName("removeObjectForKey:"), key1);
        id removed = send1(defaults, sel_registerName("objectForKey:"), key1);
        if (removed != nil) { printf("FAIL: 8 - not removed\n"); return 8; }
        printf("CHECK 8: removeObjectForKey works\n");

        // 9. synchronize returns YES
        BOOL synced = bsend0(defaults, sel_registerName("synchronize"));
        if (!synced) { printf("FAIL: 9 - synchronize returned NO\n"); return 9; }
        printf("CHECK 9: synchronize = YES\n");

        printf("OK: nsuserdefaults_basics passed\n");
        return 0;
    }
}
