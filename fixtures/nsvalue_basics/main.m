// nsvalue_basics — Phase 11 fixture: NSValue wrapping
#import <Foundation/Foundation.h>
#include <stdio.h>

int main() {
    @autoreleasepool {
        // 1. valueWithPointer:
        int x = 42;
        NSValue *pv = [NSValue valueWithPointer:&x];
        if (!pv) { printf("FAIL: valueWithPointer nil\n"); return 1; }
        void *ptr = [pv pointerValue];
        if (ptr != &x) { printf("FAIL: pointer mismatch\n"); return 2; }

        // 2. objCType
        const char *type = [pv objCType];
        if (!type) { printf("FAIL: objCType nil\n"); return 3; }
        printf("objCType=%s\n", type);

        // 3. description
        NSString *desc = [pv description];
        if (!desc) { printf("FAIL: desc nil\n"); return 4; }
        printf("desc=%s\n", [desc UTF8String]);

        // 4. valueWithBytes:objCType: (wrap a single integer)
        int val = 99;
        NSValue *bv = [NSValue valueWithBytes:&val objCType:@encode(int)];
        if (!bv) { printf("FAIL: valueWithBytes nil\n"); return 5; }
        const char *btype = [bv objCType];
        if (!btype) { printf("FAIL: btype nil\n"); return 6; }
        printf("btype=%s\n", btype);

        printf("OK\n");
    }
    return 0;
}
