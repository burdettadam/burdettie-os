// Category on Greeter — compiled separately to force __objc_catlist.

#include <objc/runtime.h>

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
