// Base class with +load method.
// Compiled separately from the category to force __objc_catlist usage.

#include <stdio.h>
#include <objc/runtime.h>

// Track whether +load was called — shared with main
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
