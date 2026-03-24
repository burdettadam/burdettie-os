// minimal_throw — simplest @try/@catch test
#import <Foundation/Foundation.h>
#include <stdio.h>

int main() {
    printf("start\n");
    @try {
        printf("in try\n");
        @throw [NSException exceptionWithName:@"Test"
                                       reason:@"testing"
                                     userInfo:nil];
        printf("FAIL\n");
    }
    @catch (NSException *e) {
        printf("caught: %s\n", [[e name] UTF8String]);
    }
    printf("OK\n");
    return 0;
}
