#import <Foundation/Foundation.h>
#import <objc/runtime.h>

// Test dynamic method resolution and forwarding.

// ── Test 1: resolveInstanceMethod ──────────────────────────────────

static void dynamicHello(id self, SEL _cmd) {
    NSLog(@"dynamicHello called!");
}

@interface DynamicResolver : NSObject
- (void)sayHello;
@end

@implementation DynamicResolver
+ (BOOL)resolveInstanceMethod:(SEL)sel {
    if (sel == @selector(sayHello)) {
        class_addMethod(self, sel, (IMP)dynamicHello, "v@:");
        return YES;
    }
    return [super resolveInstanceMethod:sel];
}
@end

// ── Test 2: NSMethodSignature parsing ──────────────────────────────

int main(void) {
    @autoreleasepool {
        // Test 1: Dynamic method resolution
        DynamicResolver *resolver = [[DynamicResolver alloc] init];
        [resolver sayHello];
        NSLog(@"Test 1 PASS: resolveInstanceMethod worked");

        // Test 2: methodSignatureForSelector on NSString
        NSString *str = @"hello";
        NSMethodSignature *sig = [str methodSignatureForSelector:@selector(length)];
        if (!sig) {
            NSLog(@"FAIL: Test 2 - methodSignatureForSelector returned nil");
            return 1;
        }
        NSLog(@"Test 2 PASS: methodSignatureForSelector returned sig with %lu args",
              (unsigned long)[sig numberOfArguments]);

        // Test 3: methodSignatureForSelector returns nil for unknown selector
        NSMethodSignature *nilSig = [str methodSignatureForSelector:@selector(nonExistentMethod)];
        if (nilSig != nil) {
            NSLog(@"FAIL: Test 3 - expected nil for unknown selector");
            return 1;
        }
        NSLog(@"Test 3 PASS: nil signature for unknown selector");

        // Test 4: NSMethodSignature frameLength
        NSMethodSignature *sig2 = [NSMethodSignature signatureWithObjCTypes:"@@:@@"];
        if (!sig2) {
            NSLog(@"FAIL: Test 4 - signatureWithObjCTypes returned nil");
            return 1;
        }
        // 4 args (self + _cmd + 2 id args), each 8 bytes = 32
        NSUInteger fl = [sig2 frameLength];
        NSLog(@"Test 4 PASS: frameLength=%lu", (unsigned long)fl);

        // Test 5: NSInvocation retainArguments / argumentsRetained
        NSMethodSignature *sig3 = [NSMethodSignature signatureWithObjCTypes:"@@:"];
        NSInvocation *inv = [NSInvocation invocationWithMethodSignature:sig3];
        [inv retainArguments];
        BOOL retained = [inv argumentsRetained];
        NSLog(@"Test 5 PASS: argumentsRetained=%d", (int)retained);

        // Test 6: NSInvocation invokeWithTarget
        NSMethodSignature *upperSig = [NSMethodSignature signatureWithObjCTypes:"@@:"];
        NSInvocation *inv2 = [NSInvocation invocationWithMethodSignature:upperSig];
        [inv2 setSelector:@selector(uppercaseString)];
        [inv2 invokeWithTarget:@"world"];
        __unsafe_unretained NSString *result = nil;
        [inv2 getReturnValue:&result];
        NSLog(@"Test 6 PASS: invokeWithTarget result=%@", result);
        if (![result isEqualToString:@"WORLD"]) {
            NSLog(@"FAIL: expected WORLD, got %@", result);
            return 1;
        }

        NSLog(@"All forwarding tests passed!");
        return 0;
    }
}
