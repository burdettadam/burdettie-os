// nsindexset_basics — Phase 12 fixture: NSIndexSet & NSMutableIndexSet
#import <Foundation/Foundation.h>
#include <stdio.h>

int main() {
    @autoreleasepool {
        // 1. Empty index set
        NSIndexSet *empty = [NSIndexSet indexSet];
        if (!empty) { printf("FAIL: empty nil\n"); return 1; }
        if ([empty count] != 0) { printf("FAIL: empty count %lu\n", (unsigned long)[empty count]); return 2; }

        // 2. Single index
        NSIndexSet *single = [NSIndexSet indexSetWithIndex:5];
        if ([single count] != 1) { printf("FAIL: single count\n"); return 3; }
        if (![single containsIndex:5]) { printf("FAIL: !contains 5\n"); return 4; }
        if ([single containsIndex:3]) { printf("FAIL: contains 3\n"); return 5; }

        // 3. firstIndex / lastIndex
        if ([single firstIndex] != 5) { printf("FAIL: first\n"); return 6; }
        if ([single lastIndex] != 5) { printf("FAIL: last\n"); return 7; }

        // 4. Range-based
        NSIndexSet *range = [NSIndexSet indexSetWithIndexesInRange:NSMakeRange(10, 5)];
        if ([range count] != 5) { printf("FAIL: range count %lu\n", (unsigned long)[range count]); return 8; }
        if ([range firstIndex] != 10) { printf("FAIL: range first\n"); return 9; }
        if ([range lastIndex] != 14) { printf("FAIL: range last\n"); return 10; }

        // 5. NSMutableIndexSet
        NSMutableIndexSet *mut = [[NSMutableIndexSet alloc] init];
        if (!mut) { printf("FAIL: mut nil\n"); return 11; }
        [mut addIndex:3];
        [mut addIndex:7];
        [mut addIndex:1];
        if ([mut count] != 3) { printf("FAIL: mut count %lu\n", (unsigned long)[mut count]); return 12; }
        if ([mut firstIndex] != 1) { printf("FAIL: mut first\n"); return 13; }
        if ([mut lastIndex] != 7) { printf("FAIL: mut last\n"); return 14; }

        // 6. removeIndex:
        [mut removeIndex:3];
        if ([mut count] != 2) { printf("FAIL: after remove count\n"); return 15; }
        if ([mut containsIndex:3]) { printf("FAIL: still contains 3\n"); return 16; }

        // 7. removeAllIndexes
        [mut removeAllIndexes];
        if ([mut count] != 0) { printf("FAIL: after removeAll count\n"); return 17; }

        // 8. description
        NSString *desc = [single description];
        if (!desc) { printf("FAIL: desc nil\n"); return 18; }

        printf("OK\n");
    }
    return 0;
}
