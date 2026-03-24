#import <Foundation/Foundation.h>

// Test NSInvocation and NSMethodSignature basics:
// 1. Create NSMethodSignature from type encoding
// 2. Create NSInvocation with signature
// 3. Set target and selector
// 4. Invoke and get return value

int main(void) {
    @autoreleasepool {
        // Test 1: NSMethodSignature from type encoding
        // "@@:" means: returns id(@), self(@), _cmd(:)
        NSMethodSignature *sig = [NSMethodSignature signatureWithObjCTypes:"@@:"];
        if (!sig) {
            NSLog(@"FAIL: Test 1 - signatureWithObjCTypes returned nil");
            return 1;
        }
        NSUInteger numArgs = [sig numberOfArguments];
        NSLog(@"Test 1 PASS: numberOfArguments=%lu (expected 2)", (unsigned long)numArgs);
        if (numArgs != 2) {
            NSLog(@"FAIL: expected 2 args, got %lu", (unsigned long)numArgs);
            return 1;
        }

        // Test 2: NSMethodSignature with arguments
        // "@@:@" means: returns id, self, _cmd, one id arg
        NSMethodSignature *sig2 = [NSMethodSignature signatureWithObjCTypes:"@@:@"];
        if (!sig2) {
            NSLog(@"FAIL: Test 2 - signatureWithObjCTypes returned nil");
            return 1;
        }
        NSUInteger numArgs2 = [sig2 numberOfArguments];
        NSLog(@"Test 2 PASS: numberOfArguments=%lu (expected 3)", (unsigned long)numArgs2);
        if (numArgs2 != 3) {
            NSLog(@"FAIL: expected 3 args, got %lu", (unsigned long)numArgs2);
            return 1;
        }

        // Test 3: Create NSInvocation and invoke uppercaseString on a string
        NSMethodSignature *upperSig = [NSMethodSignature signatureWithObjCTypes:"@@:"];
        NSInvocation *inv = [NSInvocation invocationWithMethodSignature:upperSig];
        if (!inv) {
            NSLog(@"FAIL: Test 3 - invocationWithMethodSignature returned nil");
            return 1;
        }

        NSString *target = @"hello";
        SEL sel = @selector(uppercaseString);
        [inv setTarget:target];
        [inv setSelector:sel];
        [inv invoke];

        __unsafe_unretained NSString *result = nil;
        [inv getReturnValue:&result];
        NSLog(@"Test 3 PASS: uppercaseString result=%@", result);
        if (![result isEqualToString:@"HELLO"]) {
            NSLog(@"FAIL: expected HELLO, got %@", result);
            return 1;
        }

        // Test 4: NSMethodSignature methodReturnLength
        NSUInteger retLen = [upperSig methodReturnLength];
        NSLog(@"Test 4 PASS: methodReturnLength=%lu (expected 8)", (unsigned long)retLen);
        if (retLen != 8) {
            NSLog(@"FAIL: expected 8, got %lu", (unsigned long)retLen);
            return 1;
        }

        // Test 5: NSInvocation with setArgument for stringByAppendingString:
        NSMethodSignature *appendSig = [NSMethodSignature signatureWithObjCTypes:"@@:@"];
        NSInvocation *inv2 = [NSInvocation invocationWithMethodSignature:appendSig];
        NSString *base = @"Hello";
        SEL appendSel = @selector(stringByAppendingString:);
        [inv2 setTarget:base];
        [inv2 setSelector:appendSel];
        NSString *suffix = @", World!";
        [inv2 setArgument:&suffix atIndex:2];
        [inv2 invoke];

        __unsafe_unretained NSString *appendResult = nil;
        [inv2 getReturnValue:&appendResult];
        NSLog(@"Test 5 PASS: append result=%@", appendResult);
        if (![appendResult isEqualToString:@"Hello, World!"]) {
            NSLog(@"FAIL: expected 'Hello, World!', got %@", appendResult);
            return 1;
        }

        NSLog(@"All NSInvocation tests passed!");
        return 0;
    }
}
