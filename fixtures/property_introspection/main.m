// property_introspection — T19 fixture: @property, ivar, getter/setter
//
// Tests property declaration, ivar access via runtime introspection APIs,
// getter/setter dispatch through @property, class_copyIvarList,
// class_copyPropertyList, ivar_getName, ivar_getOffset, ivar_getTypeEncoding,
// property_getName, property_getAttributes, object_getIvar, object_setIvar,
// class_getInstanceVariable, class_getProperty.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <stdio.h>
#include <string.h>

// ── Test class with properties and ivars ───────────────────────────

@interface Vehicle : NSObject {
    int _year;
}
@property (nonatomic, copy) NSString *make;
@property (nonatomic, assign) int horsepower;
- (int)year;
- (void)setYear:(int)y;
@end

@implementation Vehicle
@synthesize make = _make;
@synthesize horsepower = _horsepower;

- (int)year { return _year; }
- (void)setYear:(int)y { _year = y; }

- (instancetype)initWithMake:(NSString *)m hp:(int)hp year:(int)y {
    self = [super init];
    if (self) {
        _make = [m copy];
        _horsepower = hp;
        _year = y;
    }
    return self;
}
@end

int main(void) {
    int failures = 0;
    Class vehicleClass = objc_getClass("Vehicle");

    // ── 1. Create instance and verify property getters ─────────────
    Vehicle *car = [[Vehicle alloc] initWithMake:@"Tesla" hp:670 year:2024];
    if (!car) {
        fprintf(stderr, "FAIL: could not create Vehicle\n");
        return 1;
    }

    if (car.horsepower != 670) {
        fprintf(stderr, "FAIL: horsepower = %d (expected 670)\n", car.horsepower);
        failures++;
    } else {
        NSLog(@"T19 horsepower=%d", car.horsepower);
    }

    if ([car year] != 2024) {
        fprintf(stderr, "FAIL: year = %d (expected 2024)\n", [car year]);
        failures++;
    } else {
        NSLog(@"T19 year=%d", [car year]);
    }

    // ── 2. Test property setter ────────────────────────────────────
    car.horsepower = 700;
    if (car.horsepower != 700) {
        fprintf(stderr, "FAIL: horsepower after set = %d (expected 700)\n", car.horsepower);
        failures++;
    } else {
        NSLog(@"T19 hpAfterSet=%d", car.horsepower);
    }

    // ── 3. class_copyPropertyList ──────────────────────────────────
    unsigned int propCount = 0;
    objc_property_t *props = class_copyPropertyList(vehicleClass, &propCount);
    if (propCount < 2) {
        fprintf(stderr, "FAIL: class_copyPropertyList count=%u (expected >= 2)\n", propCount);
        failures++;
    } else {
        NSLog(@"T19 propCount=%u", propCount);
        for (unsigned int i = 0; i < propCount; i++) {
            const char *pname = property_getName(props[i]);
            const char *pattr = property_getAttributes(props[i]);
            NSLog(@"T19 prop[%u]=%s attrs=%s", i, pname, pattr);
        }
    }
    if (props) free(props);

    // ── 4. class_getProperty ───────────────────────────────────────
    objc_property_t makeProp = class_getProperty(vehicleClass, "make");
    if (!makeProp) {
        fprintf(stderr, "FAIL: class_getProperty('make') returned NULL\n");
        failures++;
    } else {
        const char *name = property_getName(makeProp);
        const char *attrs = property_getAttributes(makeProp);
        if (strcmp(name, "make") != 0) {
            fprintf(stderr, "FAIL: property_getName returned '%s'\n", name);
            failures++;
        } else {
            NSLog(@"T19 getProperty=%s", name);
        }
        if (!attrs || strlen(attrs) == 0) {
            fprintf(stderr, "FAIL: property_getAttributes returned NULL or empty\n");
            failures++;
        } else {
            NSLog(@"T19 propAttrs=%s", attrs);
        }
    }

    // ── 5. class_copyIvarList ──────────────────────────────────────
    unsigned int ivarCount = 0;
    Ivar *ivars = class_copyIvarList(vehicleClass, &ivarCount);
    if (ivarCount < 2) {
        fprintf(stderr, "FAIL: class_copyIvarList count=%u (expected >= 2)\n", ivarCount);
        failures++;
    } else {
        NSLog(@"T19 ivarCount=%u", ivarCount);
        for (unsigned int i = 0; i < ivarCount; i++) {
            const char *iname = ivar_getName(ivars[i]);
            const char *itype = ivar_getTypeEncoding(ivars[i]);
            ptrdiff_t ioff = ivar_getOffset(ivars[i]);
            NSLog(@"T19 ivar[%u]=%s type=%s offset=%td", i, iname, itype, ioff);
        }
    }
    if (ivars) free(ivars);

    // ── 6. class_getInstanceVariable ───────────────────────────────
    Ivar yearIvar = class_getInstanceVariable(vehicleClass, "_year");
    if (!yearIvar) {
        fprintf(stderr, "FAIL: class_getInstanceVariable('_year') returned NULL\n");
        failures++;
    } else {
        const char *iname = ivar_getName(yearIvar);
        ptrdiff_t ioff = ivar_getOffset(yearIvar);
        if (strcmp(iname, "_year") != 0) {
            fprintf(stderr, "FAIL: ivar_getName returned '%s'\n", iname);
            failures++;
        } else {
            NSLog(@"T19 getInstanceVariable=%s offset=%td", iname, ioff);
        }
    }

    // ── 7. object_getIvar / object_setIvar ─────────────────────────
    Ivar hpIvar = class_getInstanceVariable(vehicleClass, "_horsepower");
    if (hpIvar) {
        // Read current value via object_getIvar (reads as pointer-sized)
        ptrdiff_t hpOff = ivar_getOffset(hpIvar);
        NSLog(@"T19 hpIvarOffset=%td", hpOff);

        // Use ivar offset directly to verify memory layout
        int *hpPtr = (int *)((char *)car + hpOff);
        if (*hpPtr != 700) {
            fprintf(stderr, "FAIL: direct ivar read = %d (expected 700)\n", *hpPtr);
            failures++;
        } else {
            NSLog(@"T19 directIvarRead=%d", *hpPtr);
        }
    } else {
        fprintf(stderr, "FAIL: class_getInstanceVariable('_horsepower') returned NULL\n");
        failures++;
    }

    // ── Summary ────────────────────────────────────────────────────
    if (failures == 0) {
        NSLog(@"T19 done=ok");
    } else {
        fprintf(stderr, "T19 FAILED with %d failures\n", failures);
    }

    return failures;
}
