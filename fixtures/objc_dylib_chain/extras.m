// Category on Greeter — compiled into libextras.dylib
// Tests cross-image category merge: this dylib imports _OBJC_CLASS_$_Greeter
// from libgreeter.dylib and adds a method to it.

#include <objc/runtime.h>

// Forward-declare the class from libgreeter.dylib
__attribute__((objc_root_class))
@interface Greeter
- (const char *)greeting;
@end

@interface Greeter (Extras)
- (const char *)farewell;
@end

@implementation Greeter (Extras)
- (const char *)farewell {
    return "goodbye";
}
@end
