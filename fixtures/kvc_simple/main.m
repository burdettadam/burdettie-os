// kvc_simple — Minimal KVC test
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <stdio.h>

@interface Thing : NSObject {
    int _value;
}
@property (nonatomic, assign) int value;
@end

@implementation Thing
@synthesize value = _value;
@end

int main() {
    printf("kvc_simple test\n");
    int failures = 0;

    Thing *t = [[Thing alloc] init];
    [t setValue:42];
    printf("  Direct: value = %d\n", [t value]);

    // Test valueForKey:
    NSNumber *n = [t valueForKey:@"value"];
    printf("  KVC: valueForKey:@\"value\" = %d\n", [n intValue]);
    if ([n intValue] == 42) {
        printf("  PASS: valueForKey: works\n");
    } else {
        printf("  FAIL: expected 42, got %d\n", [n intValue]);
        failures++;
    }

    // Test setValue:forKey:
    [t setValue:@(99) forKey:@"value"];
    int direct = [t value];
    printf("  After setValue:forKey: direct = %d\n", direct);
    if (direct == 99) {
        printf("  PASS: setValue:forKey: works\n");
    } else {
        printf("  FAIL: expected 99, got %d\n", direct);
        failures++;
    }

    printf("\nkvc_simple: %d failures\n", failures);
    if (failures == 0) {
        printf("kvc_simple: PASS\n");
    }
    return failures;
}
