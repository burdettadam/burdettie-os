// predicate_filter — NSPredicate collection filtering
// Exercises: NSPredicate, NSArray, NSDictionary, NSNumber, NSString,
//            filteredArrayUsingPredicate:, compound predicates, format strings
#import <Foundation/Foundation.h>

int main(void) {
    @autoreleasepool {
        // Build a dataset of "records"
        NSArray *people = @[
            @{@"name": @"Alice",   @"age": @(30), @"city": @"Portland"},
            @{@"name": @"Bob",     @"age": @(25), @"city": @"Seattle"},
            @{@"name": @"Charlie", @"age": @(35), @"city": @"Portland"},
            @{@"name": @"Diana",   @"age": @(28), @"city": @"Denver"},
            @{@"name": @"Eve",     @"age": @(32), @"city": @"Seattle"},
        ];

        // 1. Simple comparison predicate
        NSPredicate *ageOver30 = [NSPredicate predicateWithFormat:@"age > 30"];
        NSArray *over30 = [people filteredArrayUsingPredicate:ageOver30];
        // Charlie(35), Eve(32)
        if ([over30 count] == 2) {
            printf("CHECK 1 PASS: %lu people over 30\n", (unsigned long)[over30 count]);
        } else {
            printf("CHECK 1 FAIL: count = %lu\n", (unsigned long)[over30 count]);
            return 1;
        }

        // 2. String equality
        NSPredicate *inPortland = [NSPredicate predicateWithFormat:@"city == %@", @"Portland"];
        NSArray *portland = [people filteredArrayUsingPredicate:inPortland];
        if ([portland count] == 2) {
            printf("CHECK 2 PASS: %lu in Portland\n", (unsigned long)[portland count]);
        } else {
            printf("CHECK 2 FAIL: count = %lu\n", (unsigned long)[portland count]);
            return 1;
        }

        // 3. Compound predicate (AND)
        NSPredicate *compound = [NSPredicate predicateWithFormat:
            @"age >= 28 AND city == %@", @"Seattle"];
        NSArray *seattleOlder = [people filteredArrayUsingPredicate:compound];
        // Eve(32, Seattle)
        if ([seattleOlder count] == 1 &&
            [seattleOlder[0][@"name"] isEqualToString:@"Eve"]) {
            printf("CHECK 3 PASS: Eve is the only Seattle person >= 28\n");
        } else {
            printf("CHECK 3 FAIL: count = %lu\n", (unsigned long)[seattleOlder count]);
            return 1;
        }

        // 4. String BEGINSWITH
        NSPredicate *startsWithC = [NSPredicate predicateWithFormat:
            @"name BEGINSWITH %@", @"C"];
        NSArray *cNames = [people filteredArrayUsingPredicate:startsWithC];
        if ([cNames count] == 1 &&
            [cNames[0][@"name"] isEqualToString:@"Charlie"]) {
            printf("CHECK 4 PASS: Charlie begins with C\n");
        } else {
            printf("CHECK 4 FAIL: count = %lu\n", (unsigned long)[cNames count]);
            return 1;
        }

        // 5. BETWEEN predicate
        NSPredicate *between = [NSPredicate predicateWithFormat:
            @"age BETWEEN %@", @[@(28), @(32)]];
        NSArray *mid = [people filteredArrayUsingPredicate:between];
        // Alice(30), Diana(28), Eve(32)
        if ([mid count] == 3) {
            printf("CHECK 5 PASS: %lu people aged 28-32\n", (unsigned long)[mid count]);
        } else {
            printf("CHECK 5 FAIL: count = %lu\n", (unsigned long)[mid count]);
            return 1;
        }

        // 6. IN predicate
        NSPredicate *inCities = [NSPredicate predicateWithFormat:
            @"city IN %@", @[@"Portland", @"Denver"]];
        NSArray *subset = [people filteredArrayUsingPredicate:inCities];
        // Alice, Charlie, Diana
        if ([subset count] == 3) {
            printf("CHECK 6 PASS: %lu in Portland or Denver\n", (unsigned long)[subset count]);
        } else {
            printf("CHECK 6 FAIL: count = %lu\n", (unsigned long)[subset count]);
            return 1;
        }

        // 7. evaluateWithObject: on a single dict
        NSDictionary *testPerson = @{@"name": @"Frank", @"age": @(40), @"city": @"Portland"};
        BOOL matches = [ageOver30 evaluateWithObject:testPerson];
        if (matches) {
            printf("CHECK 7 PASS: Frank(40) matches age > 30\n");
        } else {
            printf("CHECK 7 FAIL: predicate should match\n");
            return 1;
        }

        // 8. CONTAINS predicate
        NSPredicate *nameContainsE = [NSPredicate predicateWithFormat:
            @"name CONTAINS[c] %@", @"e"];
        NSArray *withE = [people filteredArrayUsingPredicate:nameContainsE];
        // Alice, Charlie, Eve
        if ([withE count] == 3) {
            printf("CHECK 8 PASS: %lu names contain 'e' (case-insensitive)\n",
                   (unsigned long)[withE count]);
        } else {
            printf("CHECK 8 FAIL: count = %lu\n", (unsigned long)[withE count]);
            return 1;
        }
    }
    return 0;
}
