#import <Foundation/Foundation.h>

// NSCharacterSet fixture: test character set creation and string operations.
// Each check prints CHECK-N: PASS or CHECK-N: FAIL.

int main(void) {
    @autoreleasepool {
        int checks = 0;
        int passed = 0;

        // CHECK 1: whitespaceCharacterSet exists
        NSCharacterSet *ws = [NSCharacterSet whitespaceCharacterSet];
        checks++;
        if (ws != nil) {
            printf("CHECK-%d: PASS (whitespace set created)\n", checks);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (whitespace set is nil)\n", checks);
        }

        // CHECK 2: characterIsMember: space is whitespace
        checks++;
        if ([ws characterIsMember:' ']) {
            printf("CHECK-%d: PASS (space is whitespace)\n", checks);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (space not whitespace)\n", checks);
        }

        // CHECK 3: characterIsMember: 'A' is not whitespace
        checks++;
        if (![ws characterIsMember:'A']) {
            printf("CHECK-%d: PASS ('A' is not whitespace)\n", checks);
            passed++;
        } else {
            printf("CHECK-%d: FAIL ('A' is whitespace)\n", checks);
        }

        // CHECK 4: alphanumericCharacterSet
        NSCharacterSet *alnum = [NSCharacterSet alphanumericCharacterSet];
        checks++;
        if ([alnum characterIsMember:'Z'] && [alnum characterIsMember:'5']) {
            printf("CHECK-%d: PASS (Z and 5 are alphanumeric)\n", checks);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (alphanumeric check failed)\n", checks);
        }

        // CHECK 5: decimalDigitCharacterSet
        NSCharacterSet *digits = [NSCharacterSet decimalDigitCharacterSet];
        checks++;
        if ([digits characterIsMember:'7'] && ![digits characterIsMember:'x']) {
            printf("CHECK-%d: PASS (7 is digit, x is not)\n", checks);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (digit check failed)\n", checks);
        }

        // CHECK 6: stringByTrimmingCharactersInSet
        NSString *padded = @"  hello world  ";
        NSString *trimmed = [padded stringByTrimmingCharactersInSet:ws];
        checks++;
        if ([trimmed isEqualToString:@"hello world"]) {
            printf("CHECK-%d: PASS (trimmed: '%s')\n", checks, [trimmed UTF8String]);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (trimmed: '%s')\n", checks, [trimmed UTF8String]);
        }

        // CHECK 7: componentsSeparatedByCharactersInSet with whitespace
        NSString *sentence = @"hello world foo";
        NSArray *parts = [sentence componentsSeparatedByCharactersInSet:ws];
        checks++;
        if ([parts count] == 3 && [[parts objectAtIndex:0] isEqualToString:@"hello"]) {
            printf("CHECK-%d: PASS (split into %lu parts)\n", checks, (unsigned long)[parts count]);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (split count: %lu)\n", checks, (unsigned long)[parts count]);
        }

        // CHECK 8: rangeOfCharacterFromSet
        NSString *test = @"abc123def";
        NSRange range = [test rangeOfCharacterFromSet:digits];
        checks++;
        if (range.location == 3 && range.length == 1) {
            printf("CHECK-%d: PASS (first digit at loc=%lu len=%lu)\n", checks,
                   (unsigned long)range.location, (unsigned long)range.length);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (range loc=%lu len=%lu)\n", checks,
                   (unsigned long)range.location, (unsigned long)range.length);
        }

        // CHECK 9: characterSetWithCharactersInString
        NSCharacterSet *custom = [NSCharacterSet characterSetWithCharactersInString:@"aeiou"];
        checks++;
        if ([custom characterIsMember:'a'] && ![custom characterIsMember:'b']) {
            printf("CHECK-%d: PASS (custom set: 'a' in, 'b' not in)\n", checks);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (custom set membership wrong)\n", checks);
        }

        // CHECK 10: letterCharacterSet
        NSCharacterSet *letters = [NSCharacterSet letterCharacterSet];
        checks++;
        if ([letters characterIsMember:'M'] && ![letters characterIsMember:'3']) {
            printf("CHECK-%d: PASS (M is letter, 3 is not)\n", checks);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (letter check failed)\n", checks);
        }

        printf("\n%d/%d checks passed\n", passed, checks);
        return (passed == checks) ? 0 : 1;
    }
}
