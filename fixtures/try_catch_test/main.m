// try_catch_test — fixture for @try/@catch/@finally through mapped code
#import <Foundation/Foundation.h>
#include <stdio.h>

int main() {
    @autoreleasepool {
        // 1. Basic @try/@catch
        @try {
            @throw [NSException exceptionWithName:@"TestException"
                                           reason:@"testing"
                                         userInfo:nil];
            printf("FAIL: should not reach here\n");
            return 1;
        }
        @catch (NSException *e) {
            printf("caught: %s\n", [[e name] UTF8String]);
        }

        // 2. @try/@catch/@finally
        int finallyRan = 0;
        @try {
            printf("in try\n");
        }
        @catch (NSException *e) {
            printf("FAIL: should not catch\n");
            return 2;
        }
        @finally {
            finallyRan = 1;
        }
        if (!finallyRan) { printf("FAIL: finally\n"); return 3; }

        // 3. Nested @try/@catch
        @try {
            @try {
                @throw [NSException exceptionWithName:@"Inner"
                                               reason:@"inner"
                                             userInfo:nil];
            }
            @catch (NSException *e) {
                printf("inner caught: %s\n", [[e name] UTF8String]);
                @throw [NSException exceptionWithName:@"Outer"
                                               reason:@"rethrown"
                                             userInfo:nil];
            }
        }
        @catch (NSException *e) {
            printf("outer caught: %s\n", [[e name] UTF8String]);
        }

        printf("OK\n");
    }
    return 0;
}
