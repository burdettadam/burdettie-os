// date_calculator — date arithmetic and formatting
// Exercises: NSDate, NSDateFormatter, NSCalendar, NSTimeZone, NSNumber
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // Create a known date (2024-06-15 12:00:00 UTC)
        // Unix timestamp for 2024-06-15 12:00:00 UTC = 1718452800
        NSDate *date = [NSDate dateWithTimeIntervalSince1970:1718452800];
        NSTimeInterval ts = [date timeIntervalSince1970];
        if (ts == 1718452800) { printf("CHECK 1 PASS: timestamp roundtrip\n"); checks++; }

        // Date arithmetic: add 1 day (86400 seconds)
        NSDate *tomorrow = [date dateByAddingTimeInterval:86400];
        NSTimeInterval diff = [tomorrow timeIntervalSinceDate:date];
        if (diff == 86400) { printf("CHECK 2 PASS: 1 day = 86400s\n"); checks++; }

        // Date comparison
        NSComparisonResult cmp = [tomorrow compare:date];
        if (cmp == NSOrderedDescending) { printf("CHECK 3 PASS: tomorrow > date\n"); checks++; }

        // Current date
        NSDate *now = [NSDate date];
        if ([now timeIntervalSince1970] > 0) { printf("CHECK 4 PASS: now has positive timestamp\n"); checks++; }

        // Date formatter
        NSDateFormatter *fmt = [[NSDateFormatter alloc] init];
        [fmt setDateFormat:@"yyyy-MM-dd"];
        NSString *formatted = [fmt stringFromDate:date];
        if ([formatted isEqualToString:@"2024-06-15"]) { printf("CHECK 5 PASS: formatted = 2024-06-15\n"); checks++; }

        // Different format — check the date portion is non-empty
        // (time depends on local timezone)
        [fmt setDateFormat:@"yyyy"];
        NSString *year = [fmt stringFromDate:date];
        if ([year isEqualToString:@"2024"]) { printf("CHECK 6 PASS: year = 2024\n"); checks++; }

        // Calendar components
        NSCalendar *cal = [NSCalendar currentCalendar];
        NSString *calId = [cal calendarIdentifier];
        if ([calId length] > 0) { printf("CHECK 7 PASS: calendar id = %s\n", [calId UTF8String]); checks++; }

        // TimeZone
        NSTimeZone *tz = [NSTimeZone systemTimeZone];
        if ([[tz name] length] > 0) { printf("CHECK 8 PASS: timezone = %s\n", [[tz name] UTF8String]); checks++; }

        printf("date_calculator: %d/8 checks passed\n", checks);
        return (checks == 8) ? 0 : 1;
    }
}
