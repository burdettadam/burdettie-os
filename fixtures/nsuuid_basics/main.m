// nsuuid_basics — Phase 11 fixture: NSUUID creation and comparison
#import <Foundation/Foundation.h>
#include <stdio.h>

int main() {
    @autoreleasepool {
        // 1. Create random UUID
        NSUUID *u1 = [NSUUID UUID];
        if (!u1) { printf("FAIL: UUID nil\n"); return 1; }

        NSString *s1 = [u1 UUIDString];
        if (!s1) { printf("FAIL: UUIDString nil\n"); return 2; }
        printf("uuid1=%s\n", [s1 UTF8String]);

        // 2. UUID string format: 8-4-4-4-12
        if ([s1 length] != 36) { printf("FAIL: len=%lu\n", (unsigned long)[s1 length]); return 3; }

        // 3. Create from string
        NSUUID *u2 = [[NSUUID alloc] initWithUUIDString:s1];
        if (!u2) { printf("FAIL: initWithUUIDString nil\n"); return 4; }

        // 4. isEqual
        if (![u2 isEqual:u1]) { printf("FAIL: not equal\n"); return 5; }

        // 5. Two random UUIDs should differ
        NSUUID *u3 = [NSUUID UUID];
        NSString *s3 = [u3 UUIDString];
        printf("uuid3=%s\n", [s3 UTF8String]);
        // Extremely unlikely to be equal

        // 6. description
        NSString *desc = [u1 description];
        if (!desc) { printf("FAIL: desc nil\n"); return 6; }
        printf("desc=%s\n", [desc UTF8String]);

        printf("OK\n");
    }
    return 0;
}
