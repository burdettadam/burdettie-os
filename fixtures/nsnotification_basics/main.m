#import <Foundation/Foundation.h>

// NSNotificationCenter fixture: test that notification operations don't crash.
// Since we use no-op stubs, we just verify the API calls succeed without errors.
// Each check prints CHECK-N: PASS or CHECK-N: FAIL.

@interface Observer : NSObject
@property (nonatomic, assign) BOOL received;
- (void)handleNotification:(NSNotification *)note;
@end

@implementation Observer
- (void)handleNotification:(NSNotification *)note {
    self.received = YES;
}
@end

int main(void) {
    @autoreleasepool {
        int checks = 0;
        int passed = 0;

        // CHECK 1: defaultCenter returns non-nil
        NSNotificationCenter *center = [NSNotificationCenter defaultCenter];
        checks++;
        if (center != nil) {
            printf("CHECK-%d: PASS (defaultCenter is non-nil)\n", checks);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (defaultCenter is nil)\n", checks);
        }

        // CHECK 2: defaultCenter is a singleton
        NSNotificationCenter *center2 = [NSNotificationCenter defaultCenter];
        checks++;
        if (center == center2) {
            printf("CHECK-%d: PASS (defaultCenter is singleton)\n", checks);
            passed++;
        } else {
            printf("CHECK-%d: FAIL (defaultCenter not singleton)\n", checks);
        }

        // CHECK 3: addObserver doesn't crash
        Observer *obs = [[Observer alloc] init];
        checks++;
        @try {
            [center addObserver:obs
                       selector:@selector(handleNotification:)
                           name:@"TestNotification"
                         object:nil];
            printf("CHECK-%d: PASS (addObserver didn't crash)\n", checks);
            passed++;
        } @catch (NSException *e) {
            printf("CHECK-%d: FAIL (addObserver crashed: %s)\n", checks, [[e reason] UTF8String]);
        }

        // CHECK 4: postNotificationName:object: doesn't crash
        checks++;
        @try {
            [center postNotificationName:@"TestNotification" object:nil];
            printf("CHECK-%d: PASS (postNotification didn't crash)\n", checks);
            passed++;
        } @catch (NSException *e) {
            printf("CHECK-%d: FAIL (postNotification crashed: %s)\n", checks, [[e reason] UTF8String]);
        }

        // CHECK 5: postNotificationName:object:userInfo: doesn't crash
        checks++;
        @try {
            [center postNotificationName:@"TestNotification"
                                  object:nil
                                userInfo:@{@"key": @"value"}];
            printf("CHECK-%d: PASS (postNotification with userInfo didn't crash)\n", checks);
            passed++;
        } @catch (NSException *e) {
            printf("CHECK-%d: FAIL (postNotification with userInfo crashed)\n", checks);
        }

        // CHECK 6: removeObserver doesn't crash
        checks++;
        @try {
            [center removeObserver:obs];
            printf("CHECK-%d: PASS (removeObserver didn't crash)\n", checks);
            passed++;
        } @catch (NSException *e) {
            printf("CHECK-%d: FAIL (removeObserver crashed)\n", checks);
        }

        printf("\n%d/%d checks passed\n", passed, checks);
        return (passed == checks) ? 0 : 1;
    }
}
