// uuid_registry — entity registry with UUID keys and set deduplication
// Mimics: device/session management tools (Homebrew bundle IDs, CocoaPods spec registry)
// Exercises: NSUUID, NSMutableDictionary, NSMutableSet, NSMutableArray,
//   NSDate, NSString, NSDictionary, isEqual:, hash
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // CHECK 1: Generate UUIDs, verify format (8-4-4-4-12 hex)
        NSUUID *uuid1 = [[NSUUID alloc] init];
        NSUUID *uuid2 = [[NSUUID alloc] init];
        NSString *str1 = [uuid1 UUIDString];
        NSString *str2 = [uuid2 UUIDString];
        if ([str1 length] == 36 && [str2 length] == 36 && ![str1 isEqualToString:str2]) {
            printf("CHECK 1 PASS: two unique UUIDs generated (%s)\n", [str1 UTF8String]);
            checks++;
        } else {
            printf("CHECK 1 FAIL: str1=%s str2=%s\n",
                   [str1 UTF8String], [str2 UTF8String]);
        }

        // CHECK 2: UUID roundtrip — parse from string
        NSUUID *parsed = [[NSUUID alloc] initWithUUIDString:str1];
        NSString *roundtrip = [parsed UUIDString];
        if ([roundtrip isEqualToString:str1]) {
            printf("CHECK 2 PASS: UUID roundtrip\n"); checks++;
        } else {
            printf("CHECK 2 FAIL: %s != %s\n", [roundtrip UTF8String], [str1 UTF8String]);
        }

        // CHECK 3: Build entity registry — UUID → metadata dict
        NSMutableDictionary *registry = [NSMutableDictionary dictionary];
        NSArray *names = @[@"Widget", @"Gadget", @"Doohickey", @"Thingamajig"];
        NSMutableArray *ids = [NSMutableArray array];

        for (NSUInteger i = 0; i < [names count]; i++) {
            NSUUID *entityId = [[NSUUID alloc] init];
            NSString *key = [entityId UUIDString];
            [ids addObject:key];

            NSDictionary *meta = @{
                @"name": [names objectAtIndex:i],
                @"index": [NSNumber numberWithInteger:(NSInteger)i],
                @"created": [NSDate date],
            };
            [registry setObject:meta forKey:key];
        }

        if ([registry count] == 4) {
            printf("CHECK 3 PASS: registry has 4 entities\n"); checks++;
        } else {
            printf("CHECK 3 FAIL: count = %lu\n", (unsigned long)[registry count]);
        }

        // CHECK 4: Look up entity by UUID key
        NSString *lookupKey = [ids objectAtIndex:2]; // Doohickey
        NSDictionary *found = [registry objectForKey:lookupKey];
        NSString *foundName = [found objectForKey:@"name"];
        if ([foundName isEqualToString:@"Doohickey"]) {
            printf("CHECK 4 PASS: lookup Doohickey by UUID\n"); checks++;
        } else {
            printf("CHECK 4 FAIL: name = %s\n", foundName ? [foundName UTF8String] : "(nil)");
        }

        // CHECK 5: Set-based deduplication of UUIDs
        NSMutableSet *seen = [NSMutableSet set];
        NSMutableArray *allIds = [NSMutableArray arrayWithArray:ids];
        // Add duplicates
        [allIds addObject:[ids objectAtIndex:0]];
        [allIds addObject:[ids objectAtIndex:1]];
        [allIds addObject:[ids objectAtIndex:0]];

        NSMutableArray *unique = [NSMutableArray array];
        for (NSUInteger i = 0; i < [allIds count]; i++) {
            NSString *uid = [allIds objectAtIndex:i];
            if (![seen containsObject:uid]) {
                [seen addObject:uid];
                [unique addObject:uid];
            }
        }
        if ([unique count] == 4 && [allIds count] == 7) {
            printf("CHECK 5 PASS: dedup 7 → 4 unique\n"); checks++;
        } else {
            printf("CHECK 5 FAIL: unique=%lu all=%lu\n",
                   (unsigned long)[unique count], (unsigned long)[allIds count]);
        }

        // CHECK 6: Remove entity and verify
        NSString *removeKey = [ids objectAtIndex:1]; // Gadget
        [registry removeObjectForKey:removeKey];
        id removedEntity = [registry objectForKey:removeKey];
        if (removedEntity == nil && [registry count] == 3) {
            printf("CHECK 6 PASS: removed Gadget, registry=3\n"); checks++;
        } else {
            printf("CHECK 6 FAIL: count=%lu removed=%s\n",
                   (unsigned long)[registry count],
                   removedEntity ? "still present" : "nil");
        }

        printf("uuid_registry: %d/6 checks passed\n", checks);
        return (checks == 6) ? 0 : 1;
    }
}
