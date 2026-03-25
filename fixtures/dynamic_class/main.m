// dynamic_class — C2 fixture: runtime class creation and manipulation
//
// Tests objc_allocateClassPair, objc_registerClassPair,
// class_addMethod, class_addIvar, class_addProperty,
// class_addProtocol, objc_disposeClassPair, and dispatch
// through dynamically-created classes.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>
#include <string.h>

// ── IMP functions for dynamic methods ──────────────────────────────

static void greet_imp(id self, SEL _cmd) {
    // Read the "name" ivar
    Ivar nameIvar = class_getInstanceVariable(object_getClass(self), "name");
    id nameObj = object_getIvar(self, nameIvar);
    const char *name = nameObj ? [(NSString *)nameObj UTF8String] : "unknown";
    printf("Hello from %s!\n", name);
}

static void setName_imp(id self, SEL _cmd, id value) {
    Ivar nameIvar = class_getInstanceVariable(object_getClass(self), "name");
    object_setIvar(self, nameIvar, value);
}

static id getName_imp(id self, SEL _cmd) {
    Ivar nameIvar = class_getInstanceVariable(object_getClass(self), "name");
    return object_getIvar(self, nameIvar);
}

static id init_imp(id self, SEL _cmd) {
    self = ((id (*)(id, SEL))objc_msgSend)(self, sel_registerName("init"));
    return self;
}

// ── Protocol for dynamic class ─────────────────────────────────────

@protocol Greetable
- (void)greet;
@end

int main() {
    @autoreleasepool {
        // ── 1. Allocate a new class pair ───────────────────────────
        Class newClass = objc_allocateClassPair(
            [NSObject class], "DynamicPerson", 0);
        printf("allocated DynamicPerson: %s\n", newClass ? "YES" : "NO");
        if (!newClass) {
            printf("FAIL: could not allocate class\n");
            return 1;
        }

        // ── 2. Add ivars (before registration) ────────────────────
        BOOL ivarAdded = class_addIvar(newClass, "name", sizeof(id), 
                                        log2(sizeof(id)), "@");
        printf("added 'name' ivar: %s\n", ivarAdded ? "YES" : "NO");

        BOOL ivarAge = class_addIvar(newClass, "age", sizeof(int),
                                      log2(sizeof(int)), "i");
        printf("added 'age' ivar: %s\n", ivarAge ? "YES" : "NO");

        // Duplicate ivar should fail
        BOOL ivarDup = class_addIvar(newClass, "name", sizeof(id),
                                      log2(sizeof(id)), "@");
        printf("duplicate 'name' ivar: %s\n", ivarDup ? "YES (bad)" : "NO (correct)");

        // ── 3. Add methods ─────────────────────────────────────────
        BOOL greetAdded = class_addMethod(newClass, 
            sel_registerName("greet"), (IMP)greet_imp, "v@:");
        printf("added greet method: %s\n", greetAdded ? "YES" : "NO");

        BOOL setNameAdded = class_addMethod(newClass,
            sel_registerName("setName:"), (IMP)setName_imp, "v@:@");
        printf("added setName: method: %s\n", setNameAdded ? "YES" : "NO");

        BOOL getNameAdded = class_addMethod(newClass,
            sel_registerName("name"), (IMP)getName_imp, "@@:");
        printf("added name method: %s\n", getNameAdded ? "YES" : "NO");

        // ── 4. Add property ────────────────────────────────────────
        objc_property_attribute_t attrs[] = {
            { "T", "@\"NSString\"" },
            { "V", "name" },
        };
        BOOL propAdded = class_addProperty(newClass, "name", attrs, 2);
        printf("added 'name' property: %s\n", propAdded ? "YES" : "NO");

        // ── 5. Add protocol conformance ────────────────────────────
        Protocol *greetable = objc_getProtocol("Greetable");
        if (greetable) {
            BOOL protoAdded = class_addProtocol(newClass, greetable);
            printf("added Greetable protocol: %s\n", protoAdded ? "YES" : "NO");
        }

        // ── 6. Register the class ──────────────────────────────────
        objc_registerClassPair(newClass);
        printf("registered DynamicPerson\n");

        // Verify it's now visible
        Class found = objc_getClass("DynamicPerson");
        printf("objc_getClass DynamicPerson: %s\n", found ? "found" : "nil");
        printf("class_getName: %s\n", class_getName(found));

        // ── 7. Instantiate and use ─────────────────────────────────
        id person = class_createInstance(found, 0);
        if (!person) {
            printf("FAIL: class_createInstance returned nil\n");
            return 1;
        }
        printf("created instance: %s\n", person ? "YES" : "NO");

        // Set the name ivar
        ((void (*)(id, SEL, id))objc_msgSend)(
            person, sel_registerName("setName:"), @"Alice");
        
        // Call greet
        ((void (*)(id, SEL))objc_msgSend)(
            person, sel_registerName("greet"));

        // Get name
        id nameResult = ((id (*)(id, SEL))objc_msgSend)(
            person, sel_registerName("name"));
        printf("name getter: %s\n",
               nameResult ? [(NSString *)nameResult UTF8String] : "nil");

        // ── 8. Verify superclass ───────────────────────────────────
        Class super_ = class_getSuperclass(found);
        printf("superclass: %s\n", class_getName(super_));

        // ── 9. Instance size includes ivars ────────────────────────
        size_t size = class_getInstanceSize(found);
        printf("instance size: %zu\n", size);

        // ── 10. Duplicate name should fail ─────────────────────────
        Class dup = objc_allocateClassPair([NSObject class], "DynamicPerson", 0);
        printf("duplicate allocate: %s\n", dup ? "unexpected" : "nil (correct)");

        // ── 11. Ivar after registration should fail ────────────────
        BOOL lateIvar = class_addIvar(found, "late", sizeof(int),
                                       log2(sizeof(int)), "i");
        printf("late ivar add: %s\n", lateIvar ? "YES (bad)" : "NO (correct)");

        // ── 12. Verify protocol conformance ────────────────────────
        if (greetable) {
            printf("DynamicPerson conforms to Greetable: %s\n",
                   class_conformsToProtocol(found, greetable) ? "YES" : "NO");
        }

        // ── 13. Property introspection ─────────────────────────────
        unsigned int propCount = 0;
        objc_property_t *props = class_copyPropertyList(found, &propCount);
        printf("property count: %u\n", propCount);
        for (unsigned int i = 0; i < propCount; i++) {
            printf("  property[%u]: %s\n", i, property_getName(props[i]));
        }
        free(props);

        // ── 14. Ivar introspection ─────────────────────────────────
        unsigned int ivarCount = 0;
        Ivar *ivars = class_copyIvarList(found, &ivarCount);
        printf("ivar count: %u\n", ivarCount);
        for (unsigned int i = 0; i < ivarCount; i++) {
            printf("  ivar[%u]: %s (%s)\n", i,
                   ivar_getName(ivars[i]),
                   ivar_getTypeEncoding(ivars[i]));
        }
        free(ivars);

        printf("OK: dynamic_class\n");
        return 0;
    }
}
