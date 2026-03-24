#import <Foundation/Foundation.h>

// NSDateFormatter fixture: test date formatting via styles and format strings.
// Each check prints CHECK-N: PASS or CHECK-N: FAIL.

int main(void) {
    @autoreleasepool {
        int checks = 0;
        int passed = 0;

        // Create a known date: 2001-01-01 00:00:00 UTC (CF reference date = 0.0)
        NSDate *refDate = [NSDate dateWithTimeIntervalSinceReferenceDate:0.0];

        // CHECK 1: dateWithTimeIntervalSinceReferenceDate returns non-nil
        checks++;
        if (refDate != nil) {
            printf("CHECK-%d: PASS (refDate is non-nil)\n", checks);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (refDate is nil)\n", checks);
        }

        // CHECK 2: Create NSDateFormatter
        NSDateFormatter *fmt = [[NSDateFormatter alloc] init];
        checks++;
        if (fmt != nil) {
            printf("CHECK-%d: PASS (formatter created)\n", checks);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (formatter is nil)\n", checks);
        }

        // CHECK 3: Set and get dateFormat
        [fmt setDateFormat:@"yyyy-MM-dd"];
        checks++;
        NSString *df = [fmt dateFormat];
        if (df != nil) {
            printf("CHECK-%d: PASS (dateFormat set)\n", checks);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (dateFormat is nil)\n", checks);
        }

        // CHECK 4: stringFromDate with custom format
        NSString *result = [fmt stringFromDate:refDate];
        checks++;
        if (result != nil && [result length] > 0) {
            printf("CHECK-%d: PASS (stringFromDate returned: %s)\n", checks, [result UTF8String]);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (stringFromDate returned nil or empty)\n", checks);
        }

        // CHECK 5: dateStyle setter/getter
        [fmt setDateStyle:NSDateFormatterShortStyle];
        checks++;
        if ([fmt dateStyle] == NSDateFormatterShortStyle) {
            printf("CHECK-%d: PASS (dateStyle is Short)\n", checks);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (dateStyle mismatch)\n", checks);
        }

        // CHECK 6: timeStyle setter/getter
        [fmt setTimeStyle:NSDateFormatterMediumStyle];
        checks++;
        if ([fmt timeStyle] == NSDateFormatterMediumStyle) {
            printf("CHECK-%d: PASS (timeStyle is Medium)\n", checks);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (timeStyle mismatch)\n", checks);
        }

        // CHECK 7: Format with dateStyle only (no custom format)
        NSDateFormatter *styleFmt = [[NSDateFormatter alloc] init];
        [styleFmt setDateFormat:nil];
        [styleFmt setDateStyle:NSDateFormatterMediumStyle];
        [styleFmt setTimeStyle:NSDateFormatterNoStyle];
        NSString *styled = [styleFmt stringFromDate:refDate];
        checks++;
        if (styled != nil && [styled length] > 0) {
            printf("CHECK-%d: PASS (style format: %s)\n", checks, [styled UTF8String]);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (style format returned nil or empty)\n", checks);
        }

        // CHECK 8: Format with yyyy/MM/dd HH:mm:ss
        NSDateFormatter *fmt2 = [[NSDateFormatter alloc] init];
        [fmt2 setDateFormat:@"yyyy/MM/dd HH:mm:ss"];
        NSString *full = [fmt2 stringFromDate:refDate];
        checks++;
        if (full != nil && [full length] > 0) {
            printf("CHECK-%d: PASS (full format: %s)\n", checks, [full UTF8String]);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (full format returned nil or empty)\n", checks);
        }

        printf("\n%d/%d checks passed\n", passed, checks);
        return (passed == checks) ? 0 : 1;
    }
}
