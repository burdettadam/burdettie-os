// nsnumberformatter_basics — Phase 12 fixture: NSNumberFormatter
#import <Foundation/Foundation.h>
#include <stdio.h>

int main() {
    @autoreleasepool {
        // 1. alloc/init
        NSNumberFormatter *fmt = [[NSNumberFormatter alloc] init];
        if (!fmt) { printf("FAIL: init nil\n"); return 1; }

        // 2. Default style is NSNumberFormatterNoStyle (0)
        NSNumberFormatterStyle style = [fmt numberStyle];
        if (style != NSNumberFormatterNoStyle) { printf("FAIL: default style %ld\n", (long)style); return 2; }

        // 3. stringFromNumber: with no style
        NSString *s1 = [fmt stringFromNumber:@42];
        if (!s1) { printf("FAIL: s1 nil\n"); return 3; }
        printf("noStyle=%s\n", [s1 UTF8String]);

        // 4. Set decimal style
        [fmt setNumberStyle:NSNumberFormatterDecimalStyle];
        if ([fmt numberStyle] != NSNumberFormatterDecimalStyle) { printf("FAIL: decimal style\n"); return 4; }

        NSString *s2 = [fmt stringFromNumber:@1234567];
        if (!s2) { printf("FAIL: s2 nil\n"); return 5; }
        printf("decimal=%s\n", [s2 UTF8String]);

        // 5. numberFromString:
        NSNumber *n1 = [fmt numberFromString:@"1,234"];
        if (!n1) { printf("FAIL: n1 nil\n"); return 6; }
        printf("parsed=%ld\n", (long)[n1 integerValue]);

        // 6. Percent style
        [fmt setNumberStyle:NSNumberFormatterPercentStyle];
        NSString *s3 = [fmt stringFromNumber:@0.42];
        if (!s3) { printf("FAIL: s3 nil\n"); return 7; }
        printf("percent=%s\n", [s3 UTF8String]);

        // 7. description
        NSString *desc = [fmt description];
        if (!desc) { printf("FAIL: desc nil\n"); return 8; }

        printf("OK\n");
    }
    return 0;
}
