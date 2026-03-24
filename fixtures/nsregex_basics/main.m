// T-REGEX: NSRegularExpression basics
// Compile: xcrun clang -arch arm64 -framework Foundation -isysroot $(xcrun --show-sdk-path) -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>

int main() {
    // Properly typed (non-variadic) casts for ARM64 ABI
    id (*sendCreate)(id, SEL, id, NSUInteger, id*) =
        (id (*)(id, SEL, id, NSUInteger, id*))objc_msgSend;
    NSUInteger (*sendMatches)(id, SEL, id, NSUInteger, NSUInteger, NSUInteger) =
        (NSUInteger (*)(id, SEL, id, NSUInteger, NSUInteger, NSUInteger))objc_msgSend;
    id (*sendReplace)(id, SEL, id, NSUInteger, NSUInteger, NSUInteger, id) =
        (id (*)(id, SEL, id, NSUInteger, NSUInteger, NSUInteger, id))objc_msgSend;

    // --- Test 1: Create regex ---
    Class regexCls = (Class)objc_getClass("NSRegularExpression");
    if (!regexCls) { printf("FAIL: NSRegularExpression class not found\n"); return 1; }

    id pattern = @"[0-9]+";
    id regex = sendCreate((id)regexCls,
                          sel_registerName("regularExpressionWithPattern:options:error:"),
                          pattern, 0, NULL);
    if (!regex) { printf("FAIL: regex creation failed\n"); return 2; }

    // --- Test 2: Count matches ---
    id string = @"abc 123 def 456 ghi";
    NSUInteger matches = sendMatches(regex,
                                     sel_registerName("numberOfMatchesInString:options:range:"),
                                     string, 0, 0, 19);
    if (matches != 2) { printf("FAIL: expected 2 matches, got %lu\n", matches); return 3; }

    // --- Test 3: Replace ---
    id replacement = @"NUM";
    id result = sendReplace(regex,
                            sel_registerName("stringByReplacingMatchesInString:options:range:withTemplate:"),
                            string, 0, 0, 19, replacement);
    if (!result) { printf("FAIL: replacement returned nil\n"); return 4; }

    printf("PASS\n");
    return 0;
}
