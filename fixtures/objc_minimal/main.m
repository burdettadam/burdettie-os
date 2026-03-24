// Minimal ObjC fixture — no Foundation, pure ObjC runtime.
//
// Exercises: class registration from __objc_classlist,
// objc_getClass, class_createInstance, sel_registerName,
// and objc_msgSend dispatch.

#include <stdio.h>
#include <objc/runtime.h>
#include <objc/message.h>

__attribute__((objc_root_class))
@interface Answer
- (int)value;
@end

@implementation Answer
- (int)value {
    return 42;
}
@end

int main() {
    // Look up class (must be registered from __objc_classlist by our runtime)
    Class cls = (Class)objc_getClass("Answer");
    if (!cls) {
        printf("FAIL: class not found\n");
        return 1;
    }

    // Allocate an instance
    id obj = class_createInstance(cls, 0);
    if (!obj) {
        printf("FAIL: alloc failed\n");
        return 1;
    }

    // Dispatch -[Answer value] via objc_msgSend
    SEL sel = sel_registerName("value");
    int result = ((int(*)(id, SEL))objc_msgSend)(obj, sel);
    printf("value=%d\n", result);

    free(obj);
    return result == 42 ? 0 : 1;
}
