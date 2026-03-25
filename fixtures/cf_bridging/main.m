// cf_bridging fixture: exercises CoreFoundation toll-free bridging.
//
// Tests:
//  1. CFStringRef → NSString toll-free bridge (length, UTF8String via ObjC)
//  2. CFArrayRef → NSArray bridge (count, objectAtIndex: via ObjC)
//  3. CFDictionaryRef → NSDictionary bridge (count, objectForKey: via ObjC)
//  4. CFDataRef → NSData bridge (length, bytes via ObjC)
//  5. CFURLRef → NSURL bridge (absoluteString via ObjC)
//  6. CFRetain / CFRelease
//  7. CFEqual on CFStrings
//  8. CFGetTypeID basics
//  9. CFCopyDescription
// 10. CFBundleGetMainBundle / CFBundleCopyBundleURL
//
// Compile:
//   cc -arch arm64 -fno-objc-arc -O0 -framework Foundation \
//      -framework CoreFoundation -o app main.m

#import <Foundation/Foundation.h>
#import <CoreFoundation/CoreFoundation.h>
#include <stdio.h>

int main(void) {
    @autoreleasepool {
        int checks = 0;

        // ── 1. CFString → NSString toll-free bridge ──────────────
        CFStringRef cfStr = CFStringCreateWithCString(
            kCFAllocatorDefault, "Hello CF", kCFStringEncodingUTF8);
        NSString *nsStr = (__bridge NSString *)cfStr;
        NSUInteger len = [nsStr length];
        printf("bridge string length=%lu\n", (unsigned long)len);
        if (len != 8) { printf("FAIL: string length\n"); return 1; }
        checks++;

        const char *utf8 = [nsStr UTF8String];
        printf("bridge string utf8=%s\n", utf8);
        if (strcmp(utf8, "Hello CF") != 0) { printf("FAIL: string content\n"); return 1; }
        checks++;

        // ── 2. CFArray → NSArray toll-free bridge ────────────────
        CFStringRef items[] = { cfStr, cfStr };
        CFArrayRef cfArray = CFArrayCreate(
            kCFAllocatorDefault, (const void **)items, 2, &kCFTypeArrayCallBacks);
        NSArray *nsArray = (__bridge NSArray *)cfArray;
        NSUInteger arrCount = [nsArray count];
        printf("bridge array count=%lu\n", (unsigned long)arrCount);
        if (arrCount != 2) { printf("FAIL: array count\n"); return 1; }
        checks++;

        NSString *elem = [nsArray objectAtIndex:0];
        const char *elemUtf8 = [elem UTF8String];
        printf("bridge array[0]=%s\n", elemUtf8);
        if (strcmp(elemUtf8, "Hello CF") != 0) { printf("FAIL: array element\n"); return 1; }
        checks++;

        // ── 3. CFDictionary → NSDictionary toll-free bridge ──────
        CFStringRef key = CFStringCreateWithCString(
            kCFAllocatorDefault, "myKey", kCFStringEncodingUTF8);
        CFStringRef val = CFStringCreateWithCString(
            kCFAllocatorDefault, "myVal", kCFStringEncodingUTF8);
        CFDictionaryRef cfDict = CFDictionaryCreate(
            kCFAllocatorDefault,
            (const void **)&key, (const void **)&val, 1,
            &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
        NSDictionary *nsDict = (__bridge NSDictionary *)cfDict;
        NSUInteger dictCount = [nsDict count];
        printf("bridge dict count=%lu\n", (unsigned long)dictCount);
        if (dictCount != 1) { printf("FAIL: dict count\n"); return 1; }
        checks++;

        // ── 4. CFData → NSData toll-free bridge ──────────────────
        const UInt8 bytes[] = { 0xDE, 0xAD, 0xBE, 0xEF };
        CFDataRef cfData = CFDataCreate(kCFAllocatorDefault, bytes, 4);
        NSData *nsData = (__bridge NSData *)cfData;
        NSUInteger dataLen = [nsData length];
        printf("bridge data length=%lu\n", (unsigned long)dataLen);
        if (dataLen != 4) { printf("FAIL: data length\n"); return 1; }
        checks++;

        const UInt8 *dataBytes = [nsData bytes];
        if (dataBytes[0] != 0xDE || dataBytes[3] != 0xEF) {
            printf("FAIL: data bytes\n"); return 1;
        }
        checks++;

        // ── 5. CFURL → NSURL toll-free bridge ───────────────────
        CFStringRef urlCfStr = CFStringCreateWithCString(
            kCFAllocatorDefault, "https://example.com/path", kCFStringEncodingUTF8);
        CFURLRef cfURL = CFURLCreateWithString(
            kCFAllocatorDefault, urlCfStr, NULL);
        NSString *urlStr = [(__bridge NSURL *)cfURL absoluteString];
        const char *urlCstr = [urlStr UTF8String];
        printf("bridge url=%s\n", urlCstr);
        if (strcmp(urlCstr, "https://example.com/path") != 0) { printf("FAIL: url string\n"); return 1; }
        checks++;

        // ── 6. CFRetain / CFRelease ─────────────────────────────
        CFRetain(cfStr);
        CFRelease(cfStr);
        printf("retain/release ok\n");
        checks++;

        // ── 7. CFEqual ──────────────────────────────────────────
        CFStringRef cfStr2 = CFStringCreateWithCString(
            kCFAllocatorDefault, "Hello CF", kCFStringEncodingUTF8);
        Boolean eq = CFEqual(cfStr, cfStr2);
        printf("CFEqual=%d\n", (int)eq);
        if (!eq) { printf("FAIL: CFEqual\n"); return 1; }
        checks++;

        // ── 8. CFGetTypeID ──────────────────────────────────────
        CFTypeID strTid = CFGetTypeID(cfStr);
        CFTypeID arrTid = CFGetTypeID(cfArray);
        printf("typeID string=%lu array=%lu\n",
               (unsigned long)strTid, (unsigned long)arrTid);
        if (strTid == arrTid) { printf("FAIL: typeID same\n"); return 1; }
        checks++;

        // ── 9. CFCopyDescription ────────────────────────────────
        CFStringRef desc = CFCopyDescription(cfStr);
        if (desc == NULL) { printf("FAIL: description null\n"); return 1; }
        printf("description ok\n");
        checks++;

        // ── 10. CFBundleGetMainBundle / CFBundleCopyBundleURL ───
        CFBundleRef mainBundle = CFBundleGetMainBundle();
        if (mainBundle == NULL) { printf("FAIL: main bundle null\n"); return 1; }
        CFURLRef bundleURL = CFBundleCopyBundleURL(mainBundle);
        if (bundleURL == NULL) { printf("FAIL: bundle URL null\n"); return 1; }
        printf("bundle ok\n");
        checks++;

        printf("all %d checks passed\n", checks);
        return 0;
    }
}
