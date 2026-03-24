// T-CACHE: NSCache basics
// Compile: xcrun clang -arch arm64 -framework Foundation -isysroot $(xcrun --show-sdk-path) -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>

int main() {
    // Properly typed (non-variadic) casts for ARM64 ABI
    id (*sendId0)(id, SEL) = (id (*)(id, SEL))objc_msgSend;
    id (*sendId1)(id, SEL, id) = (id (*)(id, SEL, id))objc_msgSend;
    void (*sendVoid2)(id, SEL, id, id) = (void (*)(id, SEL, id, id))objc_msgSend;
    void (*sendVoid1)(id, SEL, id) = (void (*)(id, SEL, id))objc_msgSend;

    // --- Test 1: Create cache ---
    Class cacheCls = (Class)objc_getClass("NSCache");
    if (!cacheCls) { printf("FAIL: NSCache class not found\n"); return 1; }

    id cache = sendId0((id)cacheCls, sel_registerName("alloc"));
    cache = sendId0(cache, sel_registerName("init"));
    if (!cache) { printf("FAIL: cache init failed\n"); return 2; }

    // --- Test 2: Set and get ---
    id key = @"greeting";
    id val = @"hello";
    sendVoid2(cache, sel_registerName("setObject:forKey:"), val, key);

    id got = sendId1(cache, sel_registerName("objectForKey:"), key);
    if (got != val) { printf("FAIL: objectForKey returned wrong value\n"); return 3; }

    // --- Test 3: Remove ---
    sendVoid1(cache, sel_registerName("removeObjectForKey:"), key);
    id got2 = sendId1(cache, sel_registerName("objectForKey:"), key);
    if (got2 != nil) { printf("FAIL: objectForKey should return nil after remove\n"); return 4; }

    printf("PASS\n");
    return 0;
}
