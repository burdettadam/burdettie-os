// kv_store — key-value configuration store with plist serialization
// Mimics: defaults(1) command, preferences management tools
// Exercises: NSUserDefaults, NSPropertyListSerialization, NSDictionary,
//   NSArray, NSNumber, NSString, NSData, BOOL/integer/double typed accessors
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // CHECK 1: NSUserDefaults basic string round-trip
        NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
        [defaults setObject:@"dark" forKey:@"theme"];
        NSString *theme = [defaults stringForKey:@"theme"];
        if ([theme isEqualToString:@"dark"]) {
            printf("CHECK 1 PASS: defaults theme = dark\n"); checks++;
        } else {
            printf("CHECK 1 FAIL: theme = %s\n", theme ? [theme UTF8String] : "(nil)");
        }

        // CHECK 2: Integer and bool defaults
        [defaults setInteger:42 forKey:@"max_retries"];
        [defaults setBool:YES forKey:@"verbose"];
        NSInteger retries = [defaults integerForKey:@"max_retries"];
        BOOL verbose = [defaults boolForKey:@"verbose"];
        if (retries == 42 && verbose) {
            printf("CHECK 2 PASS: retries=42 verbose=YES\n"); checks++;
        } else {
            printf("CHECK 2 FAIL: retries=%ld verbose=%d\n", (long)retries, verbose);
        }

        // CHECK 3: Double value
        [defaults setDouble:3.14159 forKey:@"pi"];
        double pi = [defaults doubleForKey:@"pi"];
        if (pi > 3.14 && pi < 3.15) {
            printf("CHECK 3 PASS: pi = %.5f\n", pi); checks++;
        } else {
            printf("CHECK 3 FAIL: pi = %f\n", pi);
        }

        // CHECK 4: NSPropertyListSerialization — dictionary → plist data → dictionary
        NSDictionary *config = @{
            @"version": @"2.1.0",
            @"features": @[@"caching", @"logging", @"metrics"],
            @"limits": @{
                @"max_connections": [NSNumber numberWithInteger:100],
                @"timeout_seconds": [NSNumber numberWithDouble:30.0],
            },
            @"enabled": @YES,
        };

        NSError *error = nil;
        NSData *plistData = [NSPropertyListSerialization
            dataWithPropertyList:config
            format:NSPropertyListXMLFormat_v1_0
            options:0
            error:&error];

        if (plistData != nil && error == nil) {
            printf("CHECK 4 PASS: serialized config to %lu bytes\n",
                   (unsigned long)[plistData length]);
            checks++;
        } else {
            printf("CHECK 4 FAIL: error = %s\n",
                   error ? [[error localizedDescription] UTF8String] : "nil data");
        }

        // CHECK 5: Deserialize plist back
        if (plistData) {
            NSError *readError = nil;
            NSDictionary *restored = [NSPropertyListSerialization
                propertyListWithData:plistData
                options:NSPropertyListImmutable
                format:NULL
                error:&readError];

            if (restored && readError == nil) {
                NSString *ver = [restored objectForKey:@"version"];
                NSArray *features = [restored objectForKey:@"features"];
                NSDictionary *limits = [restored objectForKey:@"limits"];
                if ([ver isEqualToString:@"2.1.0"] &&
                    [features count] == 3 &&
                    [[limits objectForKey:@"max_connections"] integerValue] == 100) {
                    printf("CHECK 5 PASS: deserialized config matches\n"); checks++;
                } else {
                    printf("CHECK 5 FAIL: data mismatch\n");
                }
            } else {
                printf("CHECK 5 FAIL: deser error = %s\n",
                       readError ? [[readError localizedDescription] UTF8String] : "nil");
            }
        } else {
            printf("CHECK 5 SKIP: no plist data\n");
        }

        // CHECK 6: Remove defaults + verify removal
        [defaults removeObjectForKey:@"theme"];
        NSString *removed = [defaults stringForKey:@"theme"];
        if (removed == nil) {
            printf("CHECK 6 PASS: removed theme key\n"); checks++;
        } else {
            printf("CHECK 6 FAIL: theme still = %s\n", [removed UTF8String]);
        }

        printf("kv_store: %d/6 checks passed\n", checks);
        return (checks == 6) ? 0 : 1;
    }
}
