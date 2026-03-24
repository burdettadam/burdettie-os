// nsobject_methods — T13 fixture: metaclass dispatch + NSObject method stubs
//
// Tests class method dispatch via objc_msgSend on class objects (sends to
// the metaclass), plus inherited NSObject instance methods: init, self,
// class, respondsToSelector:, isKindOfClass:.
//
// Classes inherit from NSObject so these methods exist natively too.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>

// ── NSObject-based hierarchy ───────────────────────────────────────

@interface BaseObj : NSObject
- (int)baseValue;
@end

@implementation BaseObj
- (int)baseValue { return 100; }
@end

@interface Widget : BaseObj
- (int)widgetId;
+ (int)version;
@end

@implementation Widget
- (int)widgetId { return 42; }
+ (int)version { return 3; }
@end

int main() {
    // ── Setup ──────────────────────────────────────────────────────
    Class widgetClass = (Class)objc_getClass("Widget");
    Class baseClass = (Class)objc_getClass("BaseObj");
    if (!widgetClass || !baseClass) {
        printf("FAIL: class lookup\n");
        return 1;
    }

    // ── 1. alloc + init + instance method ──────────────────────────
    id (*sendId)(id, SEL) = (id (*)(id, SEL))objc_msgSend;
    int (*sendInt)(id, SEL) = (int (*)(id, SEL))objc_msgSend;

    SEL allocSel = sel_registerName("alloc");
    SEL initSel  = sel_registerName("init");

    id w = sendId((id)widgetClass, allocSel);
    w = sendId(w, initSel);
    if (!w) { printf("FAIL: alloc/init\n"); return 2; }

    SEL widgetIdSel = sel_registerName("widgetId");
    int wid = sendInt(w, widgetIdSel);
    if (wid != 42) { printf("FAIL: widgetId=%d\n", wid); return 3; }

    // ── 2. Inherited instance method (BaseObj.baseValue) ───────────
    SEL baseSel = sel_registerName("baseValue");
    int bv = sendInt(w, baseSel);
    if (bv != 100) { printf("FAIL: baseValue=%d\n", bv); return 4; }

    // ── 3. Class method via objc_msgSend on class object ───────────
    //    Sending a message to a class object: isa = metaclass
    SEL versionSel = sel_registerName("version");
    int v = sendInt((id)widgetClass, versionSel);
    if (v != 3) { printf("FAIL: version=%d\n", v); return 5; }

    // ── 4. respondsToSelector: YES ─────────────────────────────────
    SEL respondsSel = sel_registerName("respondsToSelector:");
    unsigned char (*sendResponds)(id, SEL, SEL) =
        (unsigned char (*)(id, SEL, SEL))objc_msgSend;

    unsigned char r1 = sendResponds(w, respondsSel, widgetIdSel);
    if (!r1) { printf("FAIL: responds widgetId\n"); return 6; }

    unsigned char r2 = sendResponds(w, respondsSel, baseSel);
    if (!r2) { printf("FAIL: responds baseValue\n"); return 7; }

    // ── 5. respondsToSelector: NO ──────────────────────────────────
    SEL fakeSel = sel_registerName("doesNotExist");
    unsigned char r3 = sendResponds(w, respondsSel, fakeSel);
    if (r3) { printf("FAIL: responds fake\n"); return 8; }

    // ── 6. isKindOfClass: YES (direct class) ──────────────────────
    SEL isKindSel = sel_registerName("isKindOfClass:");
    unsigned char (*sendIsKind)(id, SEL, Class) =
        (unsigned char (*)(id, SEL, Class))objc_msgSend;

    unsigned char k1 = sendIsKind(w, isKindSel, widgetClass);
    if (!k1) { printf("FAIL: isKind Widget\n"); return 9; }

    // ── 7. isKindOfClass: YES (superclass) ─────────────────────────
    unsigned char k2 = sendIsKind(w, isKindSel, baseClass);
    if (!k2) { printf("FAIL: isKind BaseObj\n"); return 10; }

    // ── 8. isKindOfClass: NO (unrelated class) ─────────────────────
    id b = sendId(sendId((id)baseClass, allocSel), initSel);
    unsigned char k3 = sendIsKind(b, isKindSel, widgetClass);
    if (k3) { printf("FAIL: isKind negative\n"); return 11; }

    // ── 9. [obj self] returns self ─────────────────────────────────
    SEL selfSel = sel_registerName("self");
    id s = sendId(w, selfSel);
    if (s != w) { printf("FAIL: self\n"); return 12; }

    // ── 10. [obj class] returns class ──────────────────────────────
    SEL classSel = sel_registerName("class");
    Class (*sendClass)(id, SEL) = (Class (*)(id, SEL))objc_msgSend;
    Class c = sendClass(w, classSel);
    if (c != widgetClass) { printf("FAIL: class\n"); return 13; }

    printf("T13: all checks passed\n");
    return 0;
}
