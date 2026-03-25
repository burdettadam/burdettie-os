// collection_pipeline — functional-style data processing with blocks
// Exercises: NSArray, NSMutableArray, NSSortDescriptor, blocks, NSNumber, NSSet
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // Input data: scores
        NSArray *scores = @[
            [NSNumber numberWithInt:85],
            [NSNumber numberWithInt:92],
            [NSNumber numberWithInt:78],
            [NSNumber numberWithInt:95],
            [NSNumber numberWithInt:88],
            [NSNumber numberWithInt:72],
            [NSNumber numberWithInt:91],
            [NSNumber numberWithInt:67],
            [NSNumber numberWithInt:83],
            [NSNumber numberWithInt:96]
        ];

        if ([scores count] == 10) { printf("CHECK 1 PASS: 10 scores\n"); checks++; }

        // Filter: scores >= 80 (manual enumeration)
        NSMutableArray *passing = [NSMutableArray array];
        [scores enumerateObjectsUsingBlock:^(NSNumber *score, NSUInteger idx, BOOL *stop) {
            if ([score intValue] >= 80) {
                [passing addObject:score];
            }
        }];

        if ([passing count] == 7) { printf("CHECK 2 PASS: 7 passing scores\n"); checks++; }

        // Sort descending
        NSArray *sorted = [passing sortedArrayUsingComparator:^NSComparisonResult(NSNumber *a, NSNumber *b) {
            return [b compare:a];  // descending
        }];

        if ([[sorted firstObject] intValue] == 96) { printf("CHECK 3 PASS: highest = 96\n"); checks++; }
        if ([[sorted lastObject] intValue] == 83) { printf("CHECK 4 PASS: lowest passing = 83\n"); checks++; }

        // Compute sum
        __block int sum = 0;
        [scores enumerateObjectsUsingBlock:^(NSNumber *score, NSUInteger idx, BOOL *stop) {
            sum += [score intValue];
        }];

        if (sum == 847) { printf("CHECK 5 PASS: sum = 847\n"); checks++; }

        // Average
        double avg = (double)sum / [scores count];
        if (avg > 84.0 && avg < 85.0) { printf("CHECK 6 PASS: avg = %.1f\n", avg); checks++; }

        // Deduplicate with NSSet
        NSArray *withDupes = @[@"a", @"b", @"a", @"c", @"b", @"d"];
        NSSet *unique = [NSSet setWithArray:withDupes];
        if ([unique count] == 4) { printf("CHECK 7 PASS: 4 unique items\n"); checks++; }

        // Check membership
        if ([unique containsObject:@"c"]) { printf("CHECK 8 PASS: set contains 'c'\n"); checks++; }

        // Top 3 scores
        NSArray *top3 = [sorted count] >= 3 ? @[[sorted objectAtIndex:0], [sorted objectAtIndex:1], [sorted objectAtIndex:2]] : sorted;
        if ([top3 count] == 3) { printf("CHECK 9 PASS: top 3 extracted\n"); checks++; }

        // Verify order of top 3: 96, 95, 92
        if ([[top3 objectAtIndex:0] intValue] == 96 &&
            [[top3 objectAtIndex:1] intValue] == 95 &&
            [[top3 objectAtIndex:2] intValue] == 92) {
            printf("CHECK 10 PASS: top 3 = [96, 95, 92]\n"); checks++;
        }

        printf("collection_pipeline: %d/10 checks passed\n", checks);
        return (checks == 10) ? 0 : 1;
    }
}
