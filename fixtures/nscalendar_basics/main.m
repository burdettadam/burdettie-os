// nscalendar_basics — Phase 11 fixture: NSCalendar date components
#import <Foundation/Foundation.h>
#include <stdio.h>

int main() {
    @autoreleasepool {
        // 1. currentCalendar
        NSCalendar *cal = [NSCalendar currentCalendar];
        if (!cal) { printf("FAIL: currentCalendar nil\n"); return 1; }

        NSString *ident = [cal calendarIdentifier];
        if (!ident) { printf("FAIL: calendarIdentifier nil\n"); return 2; }
        printf("calendar=%s\n", [ident UTF8String]);

        // 2. component:fromDate: — extract single components from current date
        NSDate *now = [NSDate date];
        if (!now) { printf("FAIL: date nil\n"); return 3; }

        NSInteger year = [cal component:NSCalendarUnitYear fromDate:now];
        NSInteger month = [cal component:NSCalendarUnitMonth fromDate:now];
        NSInteger day = [cal component:NSCalendarUnitDay fromDate:now];
        printf("year=%ld month=%ld day=%ld\n", (long)year, (long)month, (long)day);

        // Sanity: year should be >= 2024
        if (year < 2024) { printf("FAIL: year=%ld\n", (long)year); return 5; }
        if (month < 1 || month > 12) { printf("FAIL: month=%ld\n", (long)month); return 6; }
        if (day < 1 || day > 31) { printf("FAIL: day=%ld\n", (long)day); return 7; }

        printf("OK\n");
    }
    return 0;
}

