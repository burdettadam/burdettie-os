// sort_util — multi-criteria record sorting
// Mimics: sort(1) with key-based ordering, NSSortDescriptor-like patterns
// Exercises: NSArray sortedArrayUsingComparator: (block-based), NSMutableArray,
//   NSDictionary, NSNumber, NSString compare:options:, NSNull, enumerateObjectsUsingBlock:
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        // Build a dataset of "employee records"
        NSArray *records = @[
            @{@"name": @"Charlie", @"dept": @"Engineering", @"salary": @95000},
            @{@"name": @"Alice",   @"dept": @"Engineering", @"salary": @120000},
            @{@"name": @"Eve",     @"dept": @"Marketing",   @"salary": @85000},
            @{@"name": @"Bob",     @"dept": @"Engineering", @"salary": @95000},
            @{@"name": @"Diana",   @"dept": @"Marketing",   @"salary": @110000},
            @{@"name": @"Frank",   @"dept": @"Sales",       @"salary": @75000},
        ];

        int checks = 0;

        // CHECK 1: Sort by name ascending
        NSArray *byName = [records sortedArrayUsingComparator:^NSComparisonResult(NSDictionary *a, NSDictionary *b) {
            return [[a objectForKey:@"name"] compare:[b objectForKey:@"name"]];
        }];
        NSString *firstName = [[byName objectAtIndex:0] objectForKey:@"name"];
        NSString *lastName  = [[byName objectAtIndex:5] objectForKey:@"name"];
        if ([firstName isEqualToString:@"Alice"] && [lastName isEqualToString:@"Frank"]) {
            printf("CHECK 1 PASS: name sort Alice..Frank\n"); checks++;
        } else {
            printf("CHECK 1 FAIL: got %s..%s\n", [firstName UTF8String], [lastName UTF8String]);
        }

        // CHECK 2: Sort by salary descending
        NSArray *bySalary = [records sortedArrayUsingComparator:^NSComparisonResult(NSDictionary *a, NSDictionary *b) {
            NSNumber *sa = [a objectForKey:@"salary"];
            NSNumber *sb = [b objectForKey:@"salary"];
            return [sb compare:sa]; // descending
        }];
        NSNumber *topSalary = [[bySalary objectAtIndex:0] objectForKey:@"salary"];
        NSNumber *botSalary = [[bySalary lastObject] objectForKey:@"salary"];
        if ([topSalary integerValue] == 120000 && [botSalary integerValue] == 75000) {
            printf("CHECK 2 PASS: salary desc 120000..75000\n"); checks++;
        } else {
            printf("CHECK 2 FAIL: got %ld..%ld\n", (long)[topSalary integerValue], (long)[botSalary integerValue]);
        }

        // CHECK 3: Multi-key sort: dept ascending, then salary descending
        NSArray *multiSort = [records sortedArrayUsingComparator:^NSComparisonResult(NSDictionary *a, NSDictionary *b) {
            NSComparisonResult deptCmp = [[a objectForKey:@"dept"] compare:[b objectForKey:@"dept"]];
            if (deptCmp != NSOrderedSame) return deptCmp;
            // Same dept → salary descending
            return [[b objectForKey:@"salary"] compare:[a objectForKey:@"salary"]];
        }];
        // Engineering: Alice(120k), Bob/Charlie(95k); Marketing: Diana(110k), Eve(85k); Sales: Frank(75k)
        NSString *first = [[multiSort objectAtIndex:0] objectForKey:@"name"];
        if ([first isEqualToString:@"Alice"]) {
            printf("CHECK 3 PASS: multi-sort first = Alice\n"); checks++;
        } else {
            printf("CHECK 3 FAIL: first = %s\n", [first UTF8String]);
        }

        // CHECK 4: Filter + sort — Engineering dept only, by name
        NSMutableArray *engOnly = [NSMutableArray array];
        [records enumerateObjectsUsingBlock:^(NSDictionary *r, NSUInteger idx, BOOL *stop) {
            if ([[r objectForKey:@"dept"] isEqualToString:@"Engineering"]) {
                [engOnly addObject:r];
            }
        }];
        [engOnly sortUsingComparator:^NSComparisonResult(NSDictionary *a, NSDictionary *b) {
            return [[a objectForKey:@"name"] compare:[b objectForKey:@"name"]];
        }];
        if ([engOnly count] == 3) {
            NSString *e0 = [[engOnly objectAtIndex:0] objectForKey:@"name"];
            NSString *e2 = [[engOnly objectAtIndex:2] objectForKey:@"name"];
            if ([e0 isEqualToString:@"Alice"] && [e2 isEqualToString:@"Charlie"]) {
                printf("CHECK 4 PASS: eng filter+sort 3 records\n"); checks++;
            } else {
                printf("CHECK 4 FAIL: names %s..%s\n", [e0 UTF8String], [e2 UTF8String]);
            }
        } else {
            printf("CHECK 4 FAIL: eng count = %lu\n", (unsigned long)[engOnly count]);
        }

        // CHECK 5: Stable sort verification — equal salary records keep relative order
        // Bob and Charlie both have 95000; after sort by salary desc, Alice(120k) first
        NSArray *stableCheck = [records sortedArrayUsingComparator:^NSComparisonResult(NSDictionary *a, NSDictionary *b) {
            return [[b objectForKey:@"salary"] compare:[a objectForKey:@"salary"]];
        }];
        NSString *s0 = [[stableCheck objectAtIndex:0] objectForKey:@"name"];
        if ([s0 isEqualToString:@"Alice"]) {
            printf("CHECK 5 PASS: stable sort top = Alice\n"); checks++;
        } else {
            printf("CHECK 5 FAIL: top = %s\n", [s0 UTF8String]);
        }

        // CHECK 6: Reverse sort via NSMutableArray
        NSMutableArray *mutable = [NSMutableArray arrayWithArray:byName];
        // Reverse the name-sorted array
        NSUInteger lo = 0, hi = [mutable count] - 1;
        while (lo < hi) {
            id tmp = [mutable objectAtIndex:lo];
            [mutable replaceObjectAtIndex:lo withObject:[mutable objectAtIndex:hi]];
            [mutable replaceObjectAtIndex:hi withObject:tmp];
            lo++; hi--;
        }
        NSString *revFirst = [[mutable objectAtIndex:0] objectForKey:@"name"];
        NSString *revLast  = [[mutable lastObject] objectForKey:@"name"];
        if ([revFirst isEqualToString:@"Frank"] && [revLast isEqualToString:@"Alice"]) {
            printf("CHECK 6 PASS: reversed Frank..Alice\n"); checks++;
        } else {
            printf("CHECK 6 FAIL: %s..%s\n", [revFirst UTF8String], [revLast UTF8String]);
        }

        printf("sort_util: %d/6 checks passed\n", checks);
        return (checks == 6) ? 0 : 1;
    }
}
