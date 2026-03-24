#import <Foundation/Foundation.h>
#include <stdio.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;
        int passed = 0;

        // Check 1: NSNull singleton
        id null1 = [NSNull null];
        checks++; if (null1 != nil) { passed++; printf("CHECK 1 PASSED: NSNull non-nil\n"); }
        else printf("CHECK 1 FAILED\n");

        // Check 2: singleton identity
        id null2 = [NSNull null];
        checks++; if (null1 == null2) { passed++; printf("CHECK 2 PASSED: singleton identity\n"); }
        else printf("CHECK 2 FAILED\n");

        // Check 3: description
        NSString *desc = [null1 description];
        checks++; if ([desc isEqualToString:@"<null>"]) { passed++; printf("CHECK 3 PASSED: description == <null>\n"); }
        else printf("CHECK 3 FAILED: desc=%s\n", [desc UTF8String]);

        // Check 4: use NSNull in an array
        NSArray *arr = @[@"hello", [NSNull null], @"world"];
        checks++; if ([arr count] == 3) { passed++; printf("CHECK 4 PASSED: array with NSNull count == 3\n"); }
        else printf("CHECK 4 FAILED\n");

        // Check 5: retrieve NSNull from array
        id item = [arr objectAtIndex:1];
        checks++; if (item == [NSNull null]) { passed++; printf("CHECK 5 PASSED: retrieved NSNull from array\n"); }
        else printf("CHECK 5 FAILED\n");

        // Check 6: NSNull in dictionary
        NSDictionary *dict = @{@"key": [NSNull null]};
        id val = [dict objectForKey:@"key"];
        checks++; if (val == [NSNull null]) { passed++; printf("CHECK 6 PASSED: NSNull in dictionary\n"); }
        else printf("CHECK 6 FAILED\n");

        printf("\n=== %d/%d checks passed ===\n", passed, checks);
    }
    return 0;
}
