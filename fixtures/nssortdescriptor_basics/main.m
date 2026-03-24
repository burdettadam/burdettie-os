// nssortdescriptor_basics — Phase 12 fixture: NSSortDescriptor
#import <Foundation/Foundation.h>
#include <stdio.h>

int main() {
    @autoreleasepool {
        // 1. Create ascending descriptor on "self" (NSNumber responds to compare:)
        NSSortDescriptor *asc = [NSSortDescriptor sortDescriptorWithKey:@"self" ascending:YES];
        if (!asc) { printf("FAIL: asc nil\n"); return 1; }

        // 2. key / ascending
        NSString *key = [asc key];
        if (!key || ![key isEqualToString:@"self"]) { printf("FAIL: key\n"); return 2; }
        if (![asc ascending]) { printf("FAIL: ascending\n"); return 3; }

        // 3. compareObject:toObject:
        NSNumber *a = @10;
        NSNumber *b = @20;
        NSComparisonResult r1 = [asc compareObject:a toObject:b];
        if (r1 != NSOrderedAscending) { printf("FAIL: compare 10<20 = %ld\n", (long)r1); return 4; }
        NSComparisonResult r2 = [asc compareObject:b toObject:a];
        if (r2 != NSOrderedDescending) { printf("FAIL: compare 20>10 = %ld\n", (long)r2); return 5; }
        NSComparisonResult r3 = [asc compareObject:a toObject:a];
        if (r3 != NSOrderedSame) { printf("FAIL: compare equal = %ld\n", (long)r3); return 6; }

        // 4. reversedSortDescriptor
        NSSortDescriptor *desc = [asc reversedSortDescriptor];
        if (!desc) { printf("FAIL: reversed nil\n"); return 7; }
        if ([desc ascending]) { printf("FAIL: reversed ascending\n"); return 8; }
        NSComparisonResult r4 = [desc compareObject:a toObject:b];
        if (r4 != NSOrderedDescending) { printf("FAIL: reversed compare\n"); return 9; }

        // 5. description
        NSString *d = [asc description];
        if (!d) { printf("FAIL: desc nil\n"); return 10; }

        printf("OK\n");
    }
    return 0;
}
