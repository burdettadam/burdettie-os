// Greeter class — compiled into libgreeter.dylib
// Defines a root ObjC class with +load, instance method, and a global flag.

#include <stdio.h>
#include <objc/runtime.h>

// Track whether +load was called — exported for main to check
int greeter_load_called = 0;

__attribute__((objc_root_class))
@interface Greeter
- (const char *)greeting;
@end

@implementation Greeter

+ (void)load {
    greeter_load_called = 1;
    fprintf(stderr, "[+load] Greeter loaded\n");
}

- (const char *)greeting {
    return "hello";
}

@end
