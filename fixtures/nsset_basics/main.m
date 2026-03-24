#import <Foundation/Foundation.h>
#include <stdio.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;
        int passed = 0;

        // --- NSSet basics ---

        // Check 1: Create a set with objects
        id objs[] = { @"apple", @"banana", @"cherry" };
        NSSet *fruits = [NSSet setWithObjects:objs count:3];
        checks++; if ([fruits count] == 3) { passed++; printf("CHECK 1 PASSED: set count == 3\n"); }
        else printf("CHECK 1 FAILED: count=%lu\n", (unsigned long)[fruits count]);

        // Check 2: containsObject
        checks++; if ([fruits containsObject:@"banana"]) { passed++; printf("CHECK 2 PASSED: containsObject banana\n"); }
        else printf("CHECK 2 FAILED\n");

        // Check 3: does not contain missing object
        checks++; if (![fruits containsObject:@"grape"]) { passed++; printf("CHECK 3 PASSED: !containsObject grape\n"); }
        else printf("CHECK 3 FAILED\n");

        // Check 4: member returns the object
        id found = [fruits member:@"cherry"];
        checks++; if (found != nil) { passed++; printf("CHECK 4 PASSED: member cherry found\n"); }
        else printf("CHECK 4 FAILED\n");

        // Check 5: anyObject returns something non-nil
        checks++; if ([fruits anyObject] != nil) { passed++; printf("CHECK 5 PASSED: anyObject non-nil\n"); }
        else printf("CHECK 5 FAILED\n");

        // Check 6: Deduplication — add duplicate
        id dupes[] = { @"a", @"b", @"a", @"c", @"b" };
        NSSet *deduped = [NSSet setWithObjects:dupes count:5];
        checks++; if ([deduped count] == 3) { passed++; printf("CHECK 6 PASSED: dedup count == 3\n"); }
        else printf("CHECK 6 FAILED: count=%lu\n", (unsigned long)[deduped count]);

        // Check 7: allObjects returns an array
        NSArray *all = [fruits allObjects];
        checks++; if ([all count] == 3) { passed++; printf("CHECK 7 PASSED: allObjects count == 3\n"); }
        else printf("CHECK 7 FAILED\n");

        // Check 8: isSubsetOfSet
        id sub[] = { @"apple", @"banana" };
        NSSet *subset = [NSSet setWithObjects:sub count:2];
        checks++; if ([subset isSubsetOfSet:fruits]) { passed++; printf("CHECK 8 PASSED: isSubsetOfSet\n"); }
        else printf("CHECK 8 FAILED\n");

        // Check 9: intersectsSet
        id inter[] = { @"banana", @"grape" };
        NSSet *interSet = [NSSet setWithObjects:inter count:2];
        checks++; if ([fruits intersectsSet:interSet]) { passed++; printf("CHECK 9 PASSED: intersectsSet\n"); }
        else printf("CHECK 9 FAILED\n");

        // Check 10: isEqualToSet
        id same[] = { @"cherry", @"banana", @"apple" };
        NSSet *sameSet = [NSSet setWithObjects:same count:3];
        checks++; if ([fruits isEqualToSet:sameSet]) { passed++; printf("CHECK 10 PASSED: isEqualToSet\n"); }
        else printf("CHECK 10 FAILED\n");

        // --- NSMutableSet ---

        // Check 11: mutable set - addObject
        NSMutableSet *mset = [NSMutableSet set];
        [mset addObject:@"x"];
        [mset addObject:@"y"];
        [mset addObject:@"z"];
        checks++; if ([mset count] == 3) { passed++; printf("CHECK 11 PASSED: mutableSet count == 3\n"); }
        else printf("CHECK 11 FAILED: count=%lu\n", (unsigned long)[mset count]);

        // Check 12: addObject duplicate is ignored
        [mset addObject:@"x"];
        checks++; if ([mset count] == 3) { passed++; printf("CHECK 12 PASSED: dup add ignored\n"); }
        else printf("CHECK 12 FAILED: count=%lu\n", (unsigned long)[mset count]);

        // Check 13: removeObject
        [mset removeObject:@"y"];
        checks++; if ([mset count] == 2) { passed++; printf("CHECK 13 PASSED: after remove count == 2\n"); }
        else printf("CHECK 13 FAILED: count=%lu\n", (unsigned long)[mset count]);

        // Check 14: unionSet
        id extras[] = { @"a", @"b" };
        NSSet *extraSet = [NSSet setWithObjects:extras count:2];
        [mset unionSet:extraSet];
        checks++; if ([mset count] == 4) { passed++; printf("CHECK 14 PASSED: after union count == 4\n"); }
        else printf("CHECK 14 FAILED: count=%lu\n", (unsigned long)[mset count]);

        // Check 15: removeAllObjects
        [mset removeAllObjects];
        checks++; if ([mset count] == 0) { passed++; printf("CHECK 15 PASSED: removeAll count == 0\n"); }
        else printf("CHECK 15 FAILED\n");

        // Check 16: empty set
        NSSet *empty = [NSSet set];
        checks++; if ([empty count] == 0) { passed++; printf("CHECK 16 PASSED: empty set count == 0\n"); }
        else printf("CHECK 16 FAILED\n");

        // Check 17: setWithArray
        NSArray *arr = @[@"p", @"q", @"p", @"r"];
        NSSet *fromArr = [NSSet setWithArray:arr];
        checks++; if ([fromArr count] == 3) { passed++; printf("CHECK 17 PASSED: setWithArray dedup count == 3\n"); }
        else printf("CHECK 17 FAILED: count=%lu\n", (unsigned long)[fromArr count]);

        printf("\n=== %d/%d checks passed ===\n", passed, checks);
    }
    return 0;
}
