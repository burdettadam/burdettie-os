// ordered_collections — exercises NSOrderedSet, NSMutableOrderedSet, NSCountedSet
// Exercises: orderedSetWithArray:, orderedSet, count, objectAtIndex:,
//   firstObject, lastObject, containsObject:, indexOfObject:, array,
//   addObject:, removeObject:, insertObject:atIndex:,
//   NSCountedSet set, setWithArray:, countForObject:, addObject:, removeObject:
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // CHECK 1: NSOrderedSet deduplicates and preserves order
        NSOrderedSet *os = [NSOrderedSet orderedSetWithArray:@[@"apple", @"banana", @"apple", @"cherry"]];
        if ([os count] == 3) {
            printf("CHECK 1 PASS: orderedSet count=3 (deduped)\n"); checks++;
        } else {
            printf("CHECK 1 FAIL: count=%lu\n", (unsigned long)[os count]);
        }

        // CHECK 2: firstObject / lastObject
        NSString *first = [os firstObject];
        NSString *last = [os lastObject];
        if ([first isEqualToString:@"apple"] && [last isEqualToString:@"cherry"]) {
            printf("CHECK 2 PASS: first=apple last=cherry\n"); checks++;
        } else {
            printf("CHECK 2 FAIL: first=%s last=%s\n",
                first ? [first UTF8String] : "nil", last ? [last UTF8String] : "nil");
        }

        // CHECK 3: containsObject: and indexOfObject:
        if ([os containsObject:@"banana"] && [os indexOfObject:@"banana"] == 1) {
            printf("CHECK 3 PASS: contains banana at index 1\n"); checks++;
        } else {
            printf("CHECK 3 FAIL\n");
        }

        // CHECK 4: array conversion
        NSArray *arr = [os array];
        if ([arr count] == 3) {
            printf("CHECK 4 PASS: array count=3\n"); checks++;
        } else {
            printf("CHECK 4 FAIL: array count=%lu\n", (unsigned long)[arr count]);
        }

        // CHECK 5: NSMutableOrderedSet addObject: with dedup
        NSMutableOrderedSet *mos = [NSMutableOrderedSet orderedSet];
        [mos addObject:@"x"];
        [mos addObject:@"y"];
        [mos addObject:@"x"]; // duplicate — should be ignored
        if ([mos count] == 2) {
            printf("CHECK 5 PASS: mutable orderedSet deduped count=2\n"); checks++;
        } else {
            printf("CHECK 5 FAIL: count=%lu\n", (unsigned long)[mos count]);
        }

        // CHECK 6: NSMutableOrderedSet removeObject:
        [mos removeObject:@"x"];
        if ([mos count] == 1) {
            printf("CHECK 6 PASS: after remove count=1\n"); checks++;
        } else {
            printf("CHECK 6 FAIL: count=%lu\n", (unsigned long)[mos count]);
        }

        // CHECK 7: NSCountedSet countForObject:
        NSCountedSet *cs = [NSCountedSet setWithArray:@[@"a", @"b", @"a", @"a", @"c"]];
        NSUInteger countA = [cs countForObject:@"a"];
        NSUInteger countB = [cs countForObject:@"b"];
        if (countA == 3 && countB == 1) {
            printf("CHECK 7 PASS: countForObject a=3 b=1\n"); checks++;
        } else {
            printf("CHECK 7 FAIL: a=%lu b=%lu\n", (unsigned long)countA, (unsigned long)countB);
        }

        // CHECK 8: NSCountedSet count (unique)
        if ([cs count] == 3) {
            printf("CHECK 8 PASS: countedSet unique count=3\n"); checks++;
        } else {
            printf("CHECK 8 FAIL: count=%lu\n", (unsigned long)[cs count]);
        }

        // CHECK 9: NSCountedSet removeObject: decrements
        [cs removeObject:@"a"];
        NSUInteger countA2 = [cs countForObject:@"a"];
        if (countA2 == 2) {
            printf("CHECK 9 PASS: after remove countForObject:a=2\n"); checks++;
        } else {
            printf("CHECK 9 FAIL: countForObject:a=%lu\n", (unsigned long)countA2);
        }

        // CHECK 10: NSCountedSet allObjects
        NSArray *allObjs = [cs allObjects];
        if ([allObjs count] == 3) {
            printf("CHECK 10 PASS: allObjects count=3\n"); checks++;
        } else {
            printf("CHECK 10 FAIL: allObjects count=%lu\n", (unsigned long)[allObjs count]);
        }

        printf("\nordered_collections: %d/10 checks passed\n", checks);
        return (checks == 10) ? 0 : 1;
    }
}
