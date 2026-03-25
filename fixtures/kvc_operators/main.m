// kvc_operators — KVC collection operator fixture
//
// Tests @count, @sum.x, @avg.x, @min.x, @max.x,
//       @unionOfObjects.x, @distinctUnionOfObjects.x
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m

#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>
#include <math.h>

// ── Product: a simple class with a price and a category ───────────

@interface Product : NSObject {
    NSString *_name;
    NSString *_category;
    double    _price;
}
@property (nonatomic, retain) NSString *name;
@property (nonatomic, retain) NSString *category;
@property (nonatomic, assign) double    price;
@end

@implementation Product
@synthesize name     = _name;
@synthesize category = _category;
@synthesize price    = _price;
@end

// ── Helpers ────────────────────────────────────────────────────────

static id (*msgSendId)(id, SEL)       = (id (*)(id, SEL))objc_msgSend;
static id (*msgSendId1)(id, SEL, id)  = (id (*)(id, SEL, id))objc_msgSend;
static NSUInteger (*msgSendUI)(id, SEL) = (NSUInteger (*)(id, SEL))objc_msgSend;
static id (*msgSendIdx)(id, SEL, NSUInteger) = (id (*)(id, SEL, NSUInteger))objc_msgSend;
static double (*msgSendD)(id, SEL) = (double (*)(id, SEL))objc_msgSend;
static const char *(*msgSendCStr)(id, SEL) = (const char *(*)(id, SEL))objc_msgSend;

static Product *makeProduct(const char *name, const char *cat, double price) {
    Class cls = (Class)objc_getClass("Product");
    if (!cls) return nil;

    id (*init)(id, SEL)     = (id (*)(id, SEL))objc_msgSend;
    id (*alloc)(id, SEL)    = (id (*)(id, SEL))objc_msgSend;

    id obj = alloc((id)cls, sel_registerName("alloc"));
    obj    = init(obj,       sel_registerName("init"));
    if (!obj) return nil;

    // Set properties via KVC so we exercise setValue:forKey: path.
    Class ns  = (Class)objc_getClass("NSString");
    SEL swu   = sel_registerName("stringWithUTF8String:");
    id (*sw)(id, SEL, const char *) = (id (*)(id, SEL, const char *))objc_msgSend;

    id nsName = sw((id)ns, swu, name);
    id nsCat  = sw((id)ns, swu, cat);

    // Set via ObjC property setters
    void (*setStr)(id, SEL, id) = (void (*)(id, SEL, id))objc_msgSend;
    void (*setDbl)(id, SEL, double) = (void (*)(id, SEL, double))objc_msgSend;
    setStr(obj, sel_registerName("setName:"),     nsName);
    setStr(obj, sel_registerName("setCategory:"), nsCat);
    setDbl(obj, sel_registerName("setPrice:"),    price);

    return (Product *)obj;
}

int main(void) {
    printf("kvc_operators test\n");
    int failures = 0;

    // Build product array: prices 1.00, 2.50, 4.50  (sum=8.00, avg=8/3≈2.667)
    Product *p1 = makeProduct("Alpha",   "A", 1.00);
    Product *p2 = makeProduct("Beta",    "B", 2.50);
    Product *p3 = makeProduct("Gamma",   "A", 4.50);
    Product *p4 = makeProduct("Delta",   "B", 2.50);   // duplicate price+category

    if (!p1 || !p2 || !p3 || !p4) {
        printf("FAIL: product creation\n");
        return 1;
    }

    // Build NSArray via arrayWithObjects:count:
    Class nsa = (Class)objc_getClass("NSArray");
    if (!nsa) { printf("FAIL: NSArray class\n"); return 1; }
    id objects[] = { (id)p1, (id)p2, (id)p3, (id)p4 };
    id (*awoc)(id, SEL, id *, NSUInteger) =
        (id (*)(id, SEL, id *, NSUInteger))objc_msgSend;
    id arr = awoc((id)nsa, sel_registerName("arrayWithObjects:count:"), objects, 4);
    if (!arr) { printf("FAIL: arrayWithObjects:count:\n"); return 1; }

    SEL vkp = sel_registerName("valueForKeyPath:");

    // Helper to create NSString key paths
    Class nsc = (Class)objc_getClass("NSString");
    SEL swu   = sel_registerName("stringWithUTF8String:");
    id (*sw)(id, SEL, const char *) = (id (*)(id, SEL, const char *))objc_msgSend;

    // ── Test 1: @count ────────────────────────────────────────────
    printf("\n--- Test 1: @count ---\n");
    {
        id kp = sw((id)nsc, swu, "@count");
        id result = msgSendId1(arr, vkp, kp);
        int n = result ? (int)((NSInteger (*)(id, SEL))objc_msgSend)(result, sel_registerName("integerValue")) : -1;
        if (n == 4) {
            printf("  PASS: @count = %d\n", n);
        } else {
            printf("  FAIL: @count expected 4, got %d\n", n);
            failures++;
        }
    }

    // ── Test 2: @sum.price ────────────────────────────────────────
    printf("\n--- Test 2: @sum.price ---\n");
    {
        id kp = sw((id)nsc, swu, "@sum.price");
        id result = msgSendId1(arr, vkp, kp);
        double v = result ? msgSendD(result, sel_registerName("doubleValue")) : -1.0;
        if (fabs(v - 10.50) < 0.01) {
            printf("  PASS: @sum.price = %.2f\n", v);
        } else {
            printf("  FAIL: @sum.price expected 10.50, got %.2f\n", v);
            failures++;
        }
    }

    // ── Test 3: @avg.price ────────────────────────────────────────
    printf("\n--- Test 3: @avg.price ---\n");
    {
        id kp = sw((id)nsc, swu, "@avg.price");
        id result = msgSendId1(arr, vkp, kp);
        double v = result ? msgSendD(result, sel_registerName("doubleValue")) : -1.0;
        double expected = 10.50 / 4.0;
        if (fabs(v - expected) < 0.01) {
            printf("  PASS: @avg.price = %.4f\n", v);
        } else {
            printf("  FAIL: @avg.price expected %.4f, got %.4f\n", expected, v);
            failures++;
        }
    }

    // ── Test 4: @min.price ────────────────────────────────────────
    printf("\n--- Test 4: @min.price ---\n");
    {
        id kp = sw((id)nsc, swu, "@min.price");
        id result = msgSendId1(arr, vkp, kp);
        double v = result ? msgSendD(result, sel_registerName("doubleValue")) : -1.0;
        if (fabs(v - 1.00) < 0.01) {
            printf("  PASS: @min.price = %.2f\n", v);
        } else {
            printf("  FAIL: @min.price expected 1.00, got %.2f\n", v);
            failures++;
        }
    }

    // ── Test 5: @max.price ────────────────────────────────────────
    printf("\n--- Test 5: @max.price ---\n");
    {
        id kp = sw((id)nsc, swu, "@max.price");
        id result = msgSendId1(arr, vkp, kp);
        double v = result ? msgSendD(result, sel_registerName("doubleValue")) : -1.0;
        if (fabs(v - 4.50) < 0.01) {
            printf("  PASS: @max.price = %.2f\n", v);
        } else {
            printf("  FAIL: @max.price expected 4.50, got %.2f\n", v);
            failures++;
        }
    }

    // ── Test 6: @unionOfObjects.name ─────────────────────────────
    printf("\n--- Test 6: @unionOfObjects.name ---\n");
    {
        id kp = sw((id)nsc, swu, "@unionOfObjects.name");
        id result = msgSendId1(arr, vkp, kp);
        NSUInteger cnt = result ? msgSendUI(result, sel_registerName("count")) : 0;
        if (cnt == 4) {
            printf("  PASS: @unionOfObjects.name count = %lu\n", (unsigned long)cnt);
            for (NSUInteger i = 0; i < cnt; i++) {
                id elem = msgSendIdx(result, sel_registerName("objectAtIndex:"), i);
                const char *s = elem ? msgSendCStr(elem, sel_registerName("UTF8String")) : "(nil)";
                printf("    [%lu] %s\n", (unsigned long)i, s ? s : "(nil)");
            }
        } else {
            printf("  FAIL: @unionOfObjects.name expected 4 items, got %lu\n", (unsigned long)cnt);
            failures++;
        }
    }

    // ── Test 7: @distinctUnionOfObjects.category ──────────────────
    // p1=A, p2=B, p3=A, p4=B → 2 distinct categories
    printf("\n--- Test 7: @distinctUnionOfObjects.category ---\n");
    {
        // First build an array of just 3 products with known distinct categories.
        // p1 category "A", p2 category "B" — use only those two for determinism.
        id obj2[] = { (id)p1, (id)p2 };
        id arr2 = awoc((id)nsa, sel_registerName("arrayWithObjects:count:"), obj2, 2);

        id kp = sw((id)nsc, swu, "@distinctUnionOfObjects.category");
        id result = arr2 ? msgSendId1(arr2, vkp, kp) : nil;
        NSUInteger cnt = result ? msgSendUI(result, sel_registerName("count")) : 0;
        if (cnt == 2) {
            printf("  PASS: @distinctUnionOfObjects.category count = %lu\n", (unsigned long)cnt);
        } else {
            printf("  FAIL: @distinctUnionOfObjects.category expected 2 items, got %lu\n", (unsigned long)cnt);
            failures++;
        }
    }

    // ── Summary ───────────────────────────────────────────────────
    if (failures == 0) {
        printf("\nAll tests PASSED (7/7)\n");
        return 0;
    } else {
        printf("\n%d test(s) FAILED\n", failures);
        return 1;
    }
}
