// nsstring_ops — T9 fixture: CFString/NSString creation and ObjC dispatch
#include <CoreFoundation/CoreFoundation.h>
#include <stdio.h>
#include <string.h>
#include <objc/runtime.h>
#include <objc/message.h>

int main() {
    // 1. Create CFString from C string
    CFStringRef s = CFStringCreateWithCString(NULL, "darwin-loader", kCFStringEncodingUTF8);
    if (!s) { printf("FAIL: create\n"); return 1; }

    // 2. CFStringGetLength
    CFIndex len = CFStringGetLength(s);
    if (len != 13) { printf("FAIL: cflen=%ld\n", (long)len); return 2; }

    // 3. CFStringGetCStringPtr
    const char *p = CFStringGetCStringPtr(s, kCFStringEncodingUTF8);
    if (!p) { printf("FAIL: cptr null\n"); return 3; }
    if (strcmp(p, "darwin-loader") != 0) { printf("FAIL: cptr='%s'\n", p); return 4; }

    // 4. Toll-free bridge: [nsStr length] via objc_msgSend
    id nsStr = (id)s;
    SEL lengthSel = sel_registerName("length");
    NSUInteger (*sendLen)(id, SEL) = (NSUInteger(*)(id, SEL))objc_msgSend;
    NSUInteger nsLen = sendLen(nsStr, lengthSel);
    if (nsLen != 13) { printf("FAIL: nslen=%lu\n", (unsigned long)nsLen); return 5; }

    // 5. [nsStr UTF8String] via objc_msgSend
    SEL utf8Sel = sel_registerName("UTF8String");
    const char *(*sendUTF8)(id, SEL) = (const char *(*)(id, SEL))objc_msgSend;
    const char *u = sendUTF8(nsStr, utf8Sel);
    if (!u || strcmp(u, "darwin-loader") != 0) { printf("FAIL: utf8\n"); return 6; }

    // 6. [nsStr description] returns self
    SEL descSel = sel_registerName("description");
    id (*sendDesc)(id, SEL) = (id(*)(id, SEL))objc_msgSend;
    id desc = sendDesc(nsStr, descSel);
    if (desc != nsStr) { printf("FAIL: desc\n"); return 7; }

    // 7. CFRelease
    CFRelease(s);

    printf("T9: all checks passed\n");
    return 0;
}
