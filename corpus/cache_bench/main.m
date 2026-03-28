// cache_bench — in-memory cache management with eviction
// Mimics: LRU cache patterns from build tools, package managers
// Exercises: NSCache (cost limits, eviction), NSMutableDictionary (manual cache),
//   NSNumber, NSString, NSMutableArray for access ordering
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // CHECK 1: Basic NSCache put/get
        NSCache *cache = [[NSCache alloc] init];
        [cache setName:@"TestCache"];
        [cache setObject:@"value_a" forKey:@"key_a"];
        [cache setObject:@"value_b" forKey:@"key_b"];
        [cache setObject:@"value_c" forKey:@"key_c"];

        NSString *got = [cache objectForKey:@"key_b"];
        if ([got isEqualToString:@"value_b"]) {
            printf("CHECK 1 PASS: cache get key_b = value_b\n"); checks++;
        } else {
            printf("CHECK 1 FAIL: got %s\n", got ? [got UTF8String] : "(nil)");
        }

        // CHECK 2: Cache miss returns nil
        id miss = [cache objectForKey:@"nonexistent"];
        if (miss == nil) {
            printf("CHECK 2 PASS: cache miss = nil\n"); checks++;
        } else {
            printf("CHECK 2 FAIL: got non-nil\n");
        }

        // CHECK 3: Cache overwrite
        [cache setObject:@"new_value_a" forKey:@"key_a"];
        NSString *updated = [cache objectForKey:@"key_a"];
        if ([updated isEqualToString:@"new_value_a"]) {
            printf("CHECK 3 PASS: overwrite key_a = new_value_a\n"); checks++;
        } else {
            printf("CHECK 3 FAIL: got %s\n", [updated UTF8String]);
        }

        // CHECK 4: Cache removal
        [cache removeObjectForKey:@"key_b"];
        id removed = [cache objectForKey:@"key_b"];
        if (removed == nil) {
            printf("CHECK 4 PASS: removed key_b = nil\n"); checks++;
        } else {
            printf("CHECK 4 FAIL: key_b still present\n");
        }

        // CHECK 5: Cost-based cache — add items with costs
        NSCache *costCache = [[NSCache alloc] init];
        [costCache setCountLimit:3];
        [costCache setObject:@"item1" forKey:@"k1" cost:100];
        [costCache setObject:@"item2" forKey:@"k2" cost:200];
        [costCache setObject:@"item3" forKey:@"k3" cost:300];
        // With countLimit=3 and adding a 4th, eviction may occur
        [costCache setObject:@"item4" forKey:@"k4" cost:400];
        // At least k4 should be present after insertion
        NSString *k4val = [costCache objectForKey:@"k4"];
        if ([k4val isEqualToString:@"item4"]) {
            printf("CHECK 5 PASS: cost cache k4 present after eviction\n"); checks++;
        } else {
            printf("CHECK 5 FAIL: k4 = %s\n", k4val ? [k4val UTF8String] : "(nil)");
        }

        // CHECK 6: Manual LRU-like cache using NSDictionary + NSMutableArray
        NSMutableDictionary *lruMap = [NSMutableDictionary dictionary];
        NSMutableArray *lruOrder = [NSMutableArray array];
        NSUInteger capacity = 3;

        // Insert items with LRU eviction
        NSArray *keys = @[@"a", @"b", @"c", @"d", @"e"];
        for (NSUInteger i = 0; i < [keys count]; i++) {
            NSString *key = [keys objectAtIndex:i];
            NSNumber *val = [NSNumber numberWithInteger:(NSInteger)(i + 1) * 10];

            if ([lruMap objectForKey:key]) {
                [lruOrder removeObject:key];
            } else if ([lruOrder count] >= capacity) {
                NSString *evict = [lruOrder objectAtIndex:0];
                [lruOrder removeObjectAtIndex:0];
                [lruMap removeObjectForKey:evict];
            }
            [lruMap setObject:val forKey:key];
            [lruOrder addObject:key];
        }

        // After inserting a,b,c,d,e with capacity 3: a,b evicted — c,d,e remain
        BOOL cPresent = [lruMap objectForKey:@"c"] != nil;
        BOOL dPresent = [lruMap objectForKey:@"d"] != nil;
        BOOL ePresent = [lruMap objectForKey:@"e"] != nil;
        BOOL aGone = [lruMap objectForKey:@"a"] == nil;
        BOOL bGone = [lruMap objectForKey:@"b"] == nil;
        if (cPresent && dPresent && ePresent && aGone && bGone && [lruMap count] == 3) {
            printf("CHECK 6 PASS: LRU eviction c,d,e present, a,b evicted\n"); checks++;
        } else {
            printf("CHECK 6 FAIL: count=%lu a=%d b=%d c=%d d=%d e=%d\n",
                   (unsigned long)[lruMap count], !aGone, !bGone, cPresent, dPresent, ePresent);
        }

        printf("cache_bench: %d/6 checks passed\n", checks);
        return (checks == 6) ? 0 : 1;
    }
}
