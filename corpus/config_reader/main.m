// config_reader — reads and processes configuration-like structures
// Exercises: NSDictionary, NSArray, NSNumber, NSMutableDictionary, NSString, lookups
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        // Build config sections using runtime-created dictionaries
        // (avoids NSConstantDictionary which needs host ISA for key comparison)
        NSMutableDictionary *database = [NSMutableDictionary dictionary];
        [database setObject:@"localhost" forKey:@"host"];
        [database setObject:[NSNumber numberWithInt:5432] forKey:@"port"];
        [database setObject:@"myapp" forKey:@"name"];

        NSMutableDictionary *logging = [NSMutableDictionary dictionary];
        [logging setObject:@"info" forKey:@"level"];
        [logging setObject:@"/var/log/app.log" forKey:@"file"];

        NSMutableArray *features = [NSMutableArray array];
        [features addObject:@"auth"];
        [features addObject:@"cache"];
        [features addObject:@"metrics"];

        // Flat config for simple key-value lookups
        NSMutableDictionary *config = [NSMutableDictionary dictionary];
        [config setObject:@"production" forKey:@"environment"];
        [config setObject:[NSNumber numberWithInt:8080] forKey:@"port"];
        [config setObject:@"myapp" forKey:@"app_name"];
        [config setObject:@"v2.1.0" forKey:@"version"];
        [config setObject:@"/var/data" forKey:@"data_dir"];

        int checks = 0;

        // Read database values
        if ([[database objectForKey:@"host"] isEqualToString:@"localhost"]) { printf("CHECK 1 PASS: db host = localhost\n"); checks++; }
        if ([[database objectForKey:@"port"] intValue] == 5432) { printf("CHECK 2 PASS: db port = 5432\n"); checks++; }
        if ([database count] == 3) { printf("CHECK 3 PASS: db has 3 keys\n"); checks++; }

        // Read logging values
        if ([[logging objectForKey:@"level"] isEqualToString:@"info"]) { printf("CHECK 4 PASS: log level = info\n"); checks++; }
        if ([[logging objectForKey:@"file"] hasPrefix:@"/var"]) { printf("CHECK 5 PASS: log file starts with /var\n"); checks++; }

        // Read features array directly
        if ([features count] == 3) { printf("CHECK 6 PASS: 3 features\n"); checks++; }
        if ([features containsObject:@"cache"]) { printf("CHECK 7 PASS: has cache feature\n"); checks++; }

        // Read flat config
        if ([[config objectForKey:@"environment"] isEqualToString:@"production"]) { printf("CHECK 8 PASS: env = production\n"); checks++; }

        // Modify config
        [config setObject:@"debug" forKey:@"environment"];
        if ([[config objectForKey:@"environment"] isEqualToString:@"debug"]) { printf("CHECK 9 PASS: env changed to debug\n"); checks++; }

        if ([config count] == 5) { printf("CHECK 10 PASS: config has 5 keys\n"); checks++; }

        printf("config_reader: %d/10 checks passed\n", checks);
        return (checks == 10) ? 0 : 1;
    }
}
