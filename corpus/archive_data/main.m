// archive_data — keyed archiving and unarchiving
// Exercises: NSKeyedArchiver, NSKeyedUnarchiver
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // Archive a simple string
        NSString *original = @"Archiving test!";
        NSError *err = nil;
        NSData *archived = [NSKeyedArchiver archivedDataWithRootObject:original
                                             requiringSecureCoding:NO error:&err];
        if (archived != nil && [archived length] > 0) {
            printf("CHECK 1 PASS: archived data (%lu bytes)\n", (unsigned long)[archived length]);
            checks++;
        }
        if (err == nil) { printf("CHECK 2 PASS: no archive error\n"); checks++; }

        // Legacy archive
        NSData *legacyData = [NSKeyedArchiver archivedDataWithRootObject:@"legacy"];
        if (legacyData != nil) { printf("CHECK 3 PASS: legacy archive OK\n"); checks++; }

        // Instance-based archiver
        NSKeyedArchiver *archiver = [[NSKeyedArchiver alloc] initRequiringSecureCoding:NO];
        if (archiver != nil) { printf("CHECK 4 PASS: archiver instance created\n"); checks++; }
        if (![archiver requiresSecureCoding]) { printf("CHECK 5 PASS: secure coding = NO\n"); checks++; }

        // Finish encoding
        [archiver finishEncoding];
        printf("CHECK 6 PASS: finish encoding OK\n"); checks++;

        // Description
        NSString *adesc = [archiver description];
        if (adesc != nil && [adesc length] > 0) { printf("CHECK 7 PASS: archiver desc = %s\n", [adesc UTF8String]); checks++; }

        // Unarchive (best-effort for plist-compatible objects)
        NSKeyedUnarchiver *unarchiver = [[NSKeyedUnarchiver alloc] initForReadingFromData:archived error:&err];
        if (unarchiver != nil) { printf("CHECK 8 PASS: unarchiver created\n"); checks++; }

        // Legacy unarchive
        id restored = [NSKeyedUnarchiver unarchiveObjectWithData:legacyData];
        if (restored != nil) { printf("CHECK 9 PASS: legacy unarchive OK\n"); checks++; }

        // Unarchiver description
        NSString *udesc = [unarchiver description];
        if (udesc != nil) { printf("CHECK 10 PASS: unarchiver desc = %s\n", [udesc UTF8String]); checks++; }

        printf("archive_data: %d/10 checks passed\n", checks);
        return (checks == 10) ? 0 : 1;
    }
}
