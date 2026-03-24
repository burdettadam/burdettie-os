// objc_inherit — T12 fixture: superclass method dispatch + bracket syntax
//
// Tests inheritance chain: Shape (root) → Circle → FilledCircle
// Circle.area calls its own IMP; FilledCircle.color has its own method;
// FilledCircle inherits area from Circle, and name from Shape.
//
// Uses explicit objc_msgSend casts (safe for all clang versions).
// A second section tests [obj method] bracket syntax routed through
// __objc_stubs when compiled with modern clang.
//
// Compiled with: clang -fno-objc-arc -arch arm64 -o app main.m
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>
#include <string.h>

// ── Root class: Shape ──────────────────────────────────────────────

__attribute__((objc_root_class))
@interface Shape
- (const char *)name;
- (int)sides;
@end

@implementation Shape
- (const char *)name { return "shape"; }
- (int)sides { return 0; }
@end

// ── Subclass: Circle : Shape ───────────────────────────────────────

@interface Circle : Shape
- (int)area;
@end

@implementation Circle
- (const char *)name { return "circle"; }
- (int)area { return 314; }
@end

// ── Sub-subclass: FilledCircle : Circle ────────────────────────────

@interface FilledCircle : Circle
- (const char *)color;
@end

@implementation FilledCircle
- (const char *)color { return "red"; }
@end

int main() {
    // Create instances via objc_getClass + class_createInstance
    Class shapeClass = (Class)objc_getClass("Shape");
    Class circleClass = (Class)objc_getClass("Circle");
    Class filledClass = (Class)objc_getClass("FilledCircle");

    if (!shapeClass || !circleClass || !filledClass) {
        printf("FAIL: class lookup\n");
        return 1;
    }

    id shape = class_createInstance(shapeClass, 0);
    id circle = class_createInstance(circleClass, 0);
    id filled = class_createInstance(filledClass, 0);

    // ── 1. Direct method on Shape ──────────────────────────────────
    SEL nameSel = sel_registerName("name");
    const char *(*sendName)(id, SEL) = (const char *(*)(id, SEL))objc_msgSend;
    const char *n1 = sendName(shape, nameSel);
    if (!n1 || strcmp(n1, "shape") != 0) { printf("FAIL: shape.name=%s\n", n1); return 2; }

    // ── 2. Overridden method on Circle ─────────────────────────────
    const char *n2 = sendName(circle, nameSel);
    if (!n2 || strcmp(n2, "circle") != 0) { printf("FAIL: circle.name=%s\n", n2); return 3; }

    // ── 3. Circle's own method ─────────────────────────────────────
    SEL areaSel = sel_registerName("area");
    int (*sendArea)(id, SEL) = (int (*)(id, SEL))objc_msgSend;
    int a1 = sendArea(circle, areaSel);
    if (a1 != 314) { printf("FAIL: circle.area=%d\n", a1); return 4; }

    // ── 4. FilledCircle inherits area from Circle ──────────────────
    int a2 = sendArea(filled, areaSel);
    if (a2 != 314) { printf("FAIL: filled.area=%d\n", a2); return 5; }

    // ── 5. FilledCircle inherits name from Circle (which overrides Shape)
    const char *n3 = sendName(filled, nameSel);
    if (!n3 || strcmp(n3, "circle") != 0) { printf("FAIL: filled.name=%s\n", n3); return 6; }

    // ── 6. FilledCircle's own method ───────────────────────────────
    SEL colorSel = sel_registerName("color");
    const char *(*sendColor)(id, SEL) = (const char *(*)(id, SEL))objc_msgSend;
    const char *c = sendColor(filled, colorSel);
    if (!c || strcmp(c, "red") != 0) { printf("FAIL: filled.color=%s\n", c); return 7; }

    // ── 7. Shape.sides (inherited by Circle and FilledCircle) ──────
    SEL sidesSel = sel_registerName("sides");
    int (*sendSides)(id, SEL) = (int (*)(id, SEL))objc_msgSend;
    int s1 = sendSides(shape, sidesSel);
    int s2 = sendSides(circle, sidesSel);
    int s3 = sendSides(filled, sidesSel);
    if (s1 != 0 || s2 != 0 || s3 != 0) {
        printf("FAIL: sides=%d,%d,%d\n", s1, s2, s3);
        return 8;
    }

    // ── 8. Bracket syntax (routes through __objc_stubs if modern clang)
    const char *bn = [circle name];
    if (!bn || strcmp(bn, "circle") != 0) { printf("FAIL: bracket.name=%s\n", bn); return 9; }

    int ba = [circle area];
    if (ba != 314) { printf("FAIL: bracket.area=%d\n", ba); return 10; }

    const char *bc = [filled color];
    if (!bc || strcmp(bc, "red") != 0) { printf("FAIL: bracket.color=%s\n", bc); return 11; }

    // Bracket syntax on inherited method
    int bs = [filled sides];
    if (bs != 0) { printf("FAIL: bracket.sides=%d\n", bs); return 12; }

    printf("T12: all checks passed\n");
    return 0;
}
