// host_info — hostname and network info
// Exercises: NSHost, NSISO8601DateFormatter, NSHTTPURLResponse
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // NSHost — current host
        NSHost *host = [NSHost currentHost];
        if (host != nil) { printf("CHECK 1 PASS: currentHost exists\n"); checks++; }
        NSString *name = [host name];
        if (name != nil && [name length] > 0) { printf("CHECK 2 PASS: hostname = %s\n", [name UTF8String]); checks++; }

        // NSHost equality
        NSHost *host2 = [NSHost hostWithName:name];
        if ([host isEqualToHost:host2]) { printf("CHECK 3 PASS: hosts are equal\n"); checks++; }

        // NSISO8601DateFormatter
        NSISO8601DateFormatter *fmt = [[NSISO8601DateFormatter alloc] init];
        // Format a known date (2024-06-15 12:00:00 UTC)
        NSDate *date = [NSDate dateWithTimeIntervalSince1970:1718452800];
        NSString *iso = [fmt stringFromDate:date];
        if (iso != nil && [iso length] > 0) { printf("CHECK 4 PASS: ISO date = %s\n", [iso UTF8String]); checks++; }

        // Parse back
        NSDate *parsed = [fmt dateFromString:@"2024-06-15T12:00:00Z"];
        if (parsed != nil) {
            NSTimeInterval ti = [parsed timeIntervalSince1970];
            if (ti >= 1718452799 && ti <= 1718452801) { printf("CHECK 5 PASS: parsed date roundtrip\n"); checks++; }
        }

        // NSHTTPURLResponse status code string
        NSString *status = [NSHTTPURLResponse localizedStringForStatusCode:200];
        if (status != nil && [status length] > 0) { printf("CHECK 6 PASS: 200 = %s\n", [status UTF8String]); checks++; }

        NSString *s404 = [NSHTTPURLResponse localizedStringForStatusCode:404];
        if (s404 != nil && [s404 length] > 0) { printf("CHECK 7 PASS: 404 = %s\n", [s404 UTF8String]); checks++; }

        printf("host_info: %d/7 checks passed\n", checks);
        return (checks == 7) ? 0 : 1;
    }
}
