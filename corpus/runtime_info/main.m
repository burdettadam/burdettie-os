// runtime_info — system information reporter
// Exercises: NSProcessInfo, NSBundle, NSDate, NSTimeZone, NSLocale, NSUUID
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // Process info
        NSProcessInfo *procInfo = [NSProcessInfo processInfo];
        NSString *procName = [procInfo processName];
        if ([procName length] > 0) { printf("CHECK 1 PASS: process name = %s\n", [procName UTF8String]); checks++; }

        int pid = [procInfo processIdentifier];
        if (pid > 0) { printf("CHECK 2 PASS: PID = %d\n", pid); checks++; }

        // Bundle info
        NSBundle *bundle = [NSBundle mainBundle];
        NSString *bundlePath = [bundle bundlePath];
        if ([bundlePath length] > 0) { printf("CHECK 3 PASS: bundle path = %s\n", [bundlePath UTF8String]); checks++; }

        // Current date
        NSDate *now = [NSDate date];
        NSTimeInterval epoch = [now timeIntervalSince1970];
        if (epoch > 1000000000) { printf("CHECK 4 PASS: epoch = %.0f\n", epoch); checks++; }

        // Timezone
        NSTimeZone *tz = [NSTimeZone systemTimeZone];
        NSString *tzName = [tz name];
        if ([tzName length] > 0) { printf("CHECK 5 PASS: timezone = %s\n", [tzName UTF8String]); checks++; }

        NSInteger offset = [tz secondsFromGMT];
        printf("  timezone offset: %ld seconds\n", (long)offset);

        // Locale
        NSLocale *locale = [NSLocale currentLocale];
        NSString *localeId = [locale localeIdentifier];
        if ([localeId length] > 0) { printf("CHECK 6 PASS: locale = %s\n", [localeId UTF8String]); checks++; }

        // Generate UUID
        NSUUID *uuid = [NSUUID UUID];
        NSString *uuidStr = [uuid UUIDString];
        if ([uuidStr length] == 36) { printf("CHECK 7 PASS: UUID = %s\n", [uuidStr UTF8String]); checks++; }

        // OS version string
        NSString *osVer = [procInfo operatingSystemVersionString];
        if ([osVer length] > 0) { printf("CHECK 8 PASS: OS = %s\n", [osVer UTF8String]); checks++; }

        printf("runtime_info: %d/8 checks passed\n", checks);
        return (checks == 8) ? 0 : 1;
    }
}
