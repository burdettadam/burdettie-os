// nstimezone_basics — Phase 11 fixture: NSTimeZone queries
#import <Foundation/Foundation.h>
#include <stdio.h>

int main() {
    @autoreleasepool {
        // 1. systemTimeZone
        NSTimeZone *sys = [NSTimeZone systemTimeZone];
        if (!sys) { printf("FAIL: systemTimeZone nil\n"); return 1; }

        NSString *name = [sys name];
        if (!name) { printf("FAIL: name nil\n"); return 2; }
        printf("system=%s\n", [name UTF8String]);

        // 2. secondsFromGMT
        NSInteger offset = [sys secondsFromGMT];
        printf("offset=%ld\n", (long)offset);

        // 3. timeZoneWithName: UTC
        NSTimeZone *utc = [NSTimeZone timeZoneWithName:@"UTC"];
        if (!utc) { printf("FAIL: UTC nil\n"); return 3; }
        NSString *utcName = [utc name];
        printf("utc=%s\n", [utcName UTF8String]);
        if ([utc secondsFromGMT] != 0) { printf("FAIL: UTC offset != 0\n"); return 4; }

        // 4. timeZoneWithAbbreviation:
        NSTimeZone *pst = [NSTimeZone timeZoneWithAbbreviation:@"PST"];
        if (!pst) { printf("FAIL: PST nil\n"); return 5; }
        printf("pst_offset=%ld\n", (long)[pst secondsFromGMT]);

        // 5. localTimeZone (alias for system)
        NSTimeZone *local = [NSTimeZone localTimeZone];
        if (!local) { printf("FAIL: localTimeZone nil\n"); return 6; }

        // 6. description
        NSString *desc = [sys description];
        if (!desc) { printf("FAIL: desc nil\n"); return 7; }
        printf("desc=%s\n", [desc UTF8String]);

        printf("OK\n");
    }
    return 0;
}
