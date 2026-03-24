// nsstring_format — T20 fixture: +[NSString stringWithFormat:]
//
// Tests the variadic class method stringWithFormat: which creates
// a new NSString from a format string and arguments.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>
#include <string.h>

int main() {
    const char *(*sendUTF8)(id, SEL) = (const char *(*)(id, SEL))objc_msgSend;
    SEL utf8Sel = sel_registerName("UTF8String");

    // 1. stringWithFormat: with %@ object substitution
    {
        id name = @"world";
        id result = [NSString stringWithFormat:@"Hello, %@!", name];
        const char *cstr = sendUTF8(result, utf8Sel);
        if (strcmp(cstr, "Hello, world!") != 0) {
            printf("FAIL 1: expected 'Hello, world!' got '%s'\n", cstr);
            return 1;
        }
        printf("T20 check 1 OK: %s\n", cstr);
    }

    // 2. stringWithFormat: with %d integer substitution
    {
        id result = [NSString stringWithFormat:@"value=%d", 42];
        const char *cstr = sendUTF8(result, utf8Sel);
        if (strcmp(cstr, "value=42") != 0) {
            printf("FAIL 2: expected 'value=42' got '%s'\n", cstr);
            return 1;
        }
        printf("T20 check 2 OK: %s\n", cstr);
    }

    // 3. stringWithFormat: with mixed %@ and %d
    {
        id label = @"count";
        id result = [NSString stringWithFormat:@"%@=%d", label, 7];
        const char *cstr = sendUTF8(result, utf8Sel);
        if (strcmp(cstr, "count=7") != 0) {
            printf("FAIL 3: expected 'count=7' got '%s'\n", cstr);
            return 1;
        }
        printf("T20 check 3 OK: %s\n", cstr);
    }

    // 4. stringWithFormat: literal (no substitutions)
    {
        id result = [NSString stringWithFormat:@"plain"];
        const char *cstr = sendUTF8(result, utf8Sel);
        if (strcmp(cstr, "plain") != 0) {
            printf("FAIL 4: expected 'plain' got '%s'\n", cstr);
            return 1;
        }
        printf("T20 check 4 OK: %s\n", cstr);
    }

    // 5. Result used with isEqualToString:
    {
        id result = [NSString stringWithFormat:@"test%d", 99];
        BOOL (*sendEq)(id, SEL, id) = (BOOL (*)(id, SEL, id))objc_msgSend;
        SEL eqSel = sel_registerName("isEqualToString:");
        BOOL eq = sendEq(result, eqSel, @"test99");
        if (!eq) {
            printf("FAIL 5: stringWithFormat result not equal to expected\n");
            return 1;
        }
        printf("T20 check 5 OK: isEqualToString matched\n");
    }

    // 6. NSLog the formatted string (captured by runtime)
    {
        id result = [NSString stringWithFormat:@"item=%@", @"widget"];
        NSLog(@"T20 formatted=%@", result);
    }

    printf("T20: all checks passed\n");
    return 0;
}
