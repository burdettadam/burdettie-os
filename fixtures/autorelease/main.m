// autorelease — T22 fixture: autorelease pools + retain/release ownership
//
// Tests @autoreleasepool, retain, release, retainCount, autorelease,
// and object lifetime across pool drains. Compiled with -fno-objc-arc
// to exercise MRC code paths.
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
    const char *(*sendUTF8)(id, SEL)    = (const char *(*)(id, SEL))objc_msgSend;
    NSUInteger (*sendLen)(id, SEL)      = (NSUInteger (*)(id, SEL))objc_msgSend;
    id   (*sendId0)(id, SEL)            = (id (*)(id, SEL))objc_msgSend;
    id   (*sendId1)(id, SEL, id)        = (id (*)(id, SEL, id))objc_msgSend;
    void (*sendVoid0)(id, SEL)          = (void (*)(id, SEL))objc_msgSend;
    NSUInteger (*sendRC)(id, SEL)       = (NSUInteger (*)(id, SEL))objc_msgSend;

    SEL utf8Sel    = sel_registerName("UTF8String");
    SEL lenSel     = sel_registerName("length");
    SEL retainSel  = sel_registerName("retain");
    SEL releaseSel = sel_registerName("release");
    SEL arcSel     = sel_registerName("autorelease");
    SEL rcSel      = sel_registerName("retainCount");

    // ════════════════════════════════════════════════════════════════
    // Part A: @autoreleasepool basic flow
    // ════════════════════════════════════════════════════════════════

    // ── 1. @autoreleasepool push / pop ─────────────────────────────
    @autoreleasepool {
        id str1 = @"hello autorelease";
        NSUInteger len = sendLen(str1, lenSel);
        if (len != 17) {
            printf("FAIL 1: len=%lu\n", len); return 1;
        }
        NSLog(@"T22 pool1=%s", sendUTF8(str1, utf8Sel));
    }
    // Pool drained — test passed if we got here without crash

    // ════════════════════════════════════════════════════════════════
    // Part B: retain / release / retainCount (MRC)
    // ════════════════════════════════════════════════════════════════

    // ── 2. retainCount starts at 1 ────────────────────────────────
    // Use NSMutableString to avoid tagged pointers on real macOS
    id obj2 = [[NSMutableString alloc] initWithString:@"test_retaincount_long"];
    NSUInteger rc2 = sendRC(obj2, rcSel);
    if (rc2 != 1) {
        printf("FAIL 2: retainCount=%lu\n", rc2); return 2;
    }

    // ── 3. retain increments retainCount ──────────────────────────
    sendId0(obj2, retainSel);
    NSUInteger rc3 = sendRC(obj2, rcSel);
    if (rc3 != 2) {
        printf("FAIL 3: retainCount after retain=%lu\n", rc3); return 3;
    }

    // ── 4. release decrements retainCount ─────────────────────────
    sendVoid0(obj2, releaseSel);
    NSUInteger rc4 = sendRC(obj2, rcSel);
    if (rc4 != 1) {
        printf("FAIL 4: retainCount after release=%lu\n", rc4); return 4;
    }

    // ── 5. balance the alloc with a release ───────────────────────
    sendVoid0(obj2, releaseSel);

    // ════════════════════════════════════════════════════════════════
    // Part C: autorelease + pool drain
    // ════════════════════════════════════════════════════════════════

    // ── 6. autorelease adds to pool ───────────────────────────────
    @autoreleasepool {
        id obj6 = [[NSMutableString alloc] initWithString:@"autorelease_test_string"];
        sendId0(obj6, arcSel);  // autorelease → will be released at pool pop
        // Object is still alive in the pool
        const char *s6 = sendUTF8(obj6, utf8Sel);
        if (!s6 || strcmp(s6, "autorelease_test_string") != 0) {
            printf("FAIL 6: str='%s'\n", s6 ? s6 : "(null)"); return 6;
        }
    }
    // autorelease pool drained — obj6 released

    // ════════════════════════════════════════════════════════════════
    // Part D: nested autoreleasepool
    // ════════════════════════════════════════════════════════════════

    // ── 7. nested pools ───────────────────────────────────────────
    @autoreleasepool {
        id outer = [[NSMutableString alloc] initWithString:@"outer_pool_string"];
        sendId0(outer, arcSel);

        @autoreleasepool {
            id inner = [[NSMutableString alloc] initWithString:@"inner_pool_string"];
            sendId0(inner, arcSel);
            const char *si = sendUTF8(inner, utf8Sel);
            if (!si || strcmp(si, "inner_pool_string") != 0) {
                printf("FAIL 7a: inner='%s'\n", si ? si : "(null)"); return 7;
            }
        }
        // inner pool drained, outer still alive

        const char *so = sendUTF8(outer, utf8Sel);
        if (!so || strcmp(so, "outer_pool_string") != 0) {
            printf("FAIL 7b: outer='%s'\n", so ? so : "(null)"); return 7;
        }
    }
    // both pools drained

    // ════════════════════════════════════════════════════════════════
    // Part E: retain across autorelease pool
    // ════════════════════════════════════════════════════════════════

    // ── 8. retain keeps object alive past pool drain ──────────────
    id survivor;
    @autoreleasepool {
        survivor = [[NSMutableString alloc] initWithString:@"survivor_string_value"];
        sendId0(survivor, arcSel);  // autorelease → pool holds a ref
        sendId0(survivor, retainSel);  // +1 → survive pool drain
    }
    // Pool drained → releases the autorelease ref, but our retain keeps it alive
    const char *s8 = sendUTF8(survivor, utf8Sel);
    if (!s8 || strcmp(s8, "survivor_string_value") != 0) {
        printf("FAIL 8: survivor='%s'\n", s8 ? s8 : "(null)"); return 8;
    }
    sendVoid0(survivor, releaseSel);  // balance our extra retain

    // ════════════════════════════════════════════════════════════════
    // Part F: Factory method returns autoreleased object
    // ════════════════════════════════════════════════════════════════

    // ── 9. +stringWithUTF8String: pattern ─────────────────────────
    @autoreleasepool {
        id factory = [NSString stringWithUTF8String:"factory"];
        NSUInteger flen = sendLen(factory, lenSel);
        if (flen != 7) {
            printf("FAIL 9: factory len=%lu\n", flen); return 9;
        }
        const char *sf = sendUTF8(factory, utf8Sel);
        if (!sf || strcmp(sf, "factory") != 0) {
            printf("FAIL 9: factory='%s'\n", sf ? sf : "(null)"); return 9;
        }
    }

    // ── 10. Multiple retains and releases ─────────────────────────
    id obj10 = [[NSMutableString alloc] initWithString:@"multi_retaincount_test"];
    sendId0(obj10, retainSel);   // rc=2
    sendId0(obj10, retainSel);   // rc=3
    NSUInteger rc10 = sendRC(obj10, rcSel);
    if (rc10 != 3) {
        printf("FAIL 10: retainCount=%lu\n", rc10); return 10;
    }
    sendVoid0(obj10, releaseSel);  // rc=2
    sendVoid0(obj10, releaseSel);  // rc=1
    NSUInteger rc10b = sendRC(obj10, rcSel);
    if (rc10b != 1) {
        printf("FAIL 10b: retainCount=%lu\n", rc10b); return 10;
    }
    sendVoid0(obj10, releaseSel);  // rc=0 → dealloc

    NSLog(@"T22 done=ok");
    printf("T22: all checks passed\n");
    return 0;
}
