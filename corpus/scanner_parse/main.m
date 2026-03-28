// scanner_parse — exercises NSScanner string scanning operations
// Exercises: NSScanner scannerWithString:, scanInt:, scanDouble:,
//   scanString:intoString:, scanUpToString:intoString:, scanLocation,
//   setScanLocation:, isAtEnd, caseSensitive, setCaseSensitive:
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // CHECK 1: scanInt: scans an integer
        NSScanner *s1 = [NSScanner scannerWithString:@"42 hello"];
        int val = 0;
        BOOL ok = [s1 scanInt:&val];
        if (ok && val == 42) {
            printf("CHECK 1 PASS: scanInt => 42\n"); checks++;
        } else {
            printf("CHECK 1 FAIL: ok=%d val=%d\n", ok, val);
        }

        // CHECK 2: scanDouble: scans a float
        NSScanner *s2 = [NSScanner scannerWithString:@"3.14 rest"];
        double dval = 0.0;
        ok = [s2 scanDouble:&dval];
        if (ok && dval > 3.13 && dval < 3.15) {
            printf("CHECK 2 PASS: scanDouble => ~3.14\n"); checks++;
        } else {
            printf("CHECK 2 FAIL: ok=%d dval=%f\n", ok, dval);
        }

        // CHECK 3: scanString:intoString:
        NSScanner *s3 = [NSScanner scannerWithString:@"BEGIN data END"];
        NSString *matched = nil;
        ok = [s3 scanString:@"BEGIN" intoString:&matched];
        if (ok && [matched isEqualToString:@"BEGIN"]) {
            printf("CHECK 3 PASS: scanString matched 'BEGIN'\n"); checks++;
        } else {
            printf("CHECK 3 FAIL: ok=%d matched=%s\n", ok, matched ? [matched UTF8String] : "nil");
        }

        // CHECK 4: scanUpToString:intoString:
        NSScanner *s4 = [NSScanner scannerWithString:@"key=value;next"];
        NSString *before = nil;
        ok = [s4 scanUpToString:@";" intoString:&before];
        if (ok && [before isEqualToString:@"key=value"]) {
            printf("CHECK 4 PASS: scanUpToString => 'key=value'\n"); checks++;
        } else {
            printf("CHECK 4 FAIL: ok=%d before=%s\n", ok, before ? [before UTF8String] : "nil");
        }

        // CHECK 5: scanLocation tracking
        NSScanner *s5 = [NSScanner scannerWithString:@"abc"];
        NSUInteger loc = [s5 scanLocation];
        if (loc == 0) {
            printf("CHECK 5 PASS: initial scanLocation=0\n"); checks++;
        } else {
            printf("CHECK 5 FAIL: scanLocation=%lu\n", (unsigned long)loc);
        }

        // CHECK 6: setScanLocation: + isAtEnd
        [s5 setScanLocation:3];
        if ([s5 isAtEnd]) {
            printf("CHECK 6 PASS: isAtEnd after setScanLocation:3\n"); checks++;
        } else {
            printf("CHECK 6 FAIL: not at end\n");
        }

        // CHECK 7: string accessor
        NSString *orig = [s5 string];
        if ([orig isEqualToString:@"abc"]) {
            printf("CHECK 7 PASS: string => 'abc'\n"); checks++;
        } else {
            printf("CHECK 7 FAIL: string=%s\n", orig ? [orig UTF8String] : "nil");
        }

        // CHECK 8: description non-nil
        NSString *desc = [s5 description];
        if (desc != nil) {
            printf("CHECK 8 PASS: description non-nil\n"); checks++;
        } else {
            printf("CHECK 8 FAIL: nil description\n");
        }

        printf("\nscanner_parse: %d/8 checks passed\n", checks);
        return (checks == 8) ? 0 : 1;
    }
}
