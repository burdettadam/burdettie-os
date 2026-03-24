// T-JSON: NSJSONSerialization basics
// Compile: xcrun clang -fno-objc-arc -arch arm64 -framework Foundation -isysroot $(xcrun --show-sdk-path) -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>

int main() {
    // ── Setup: msg_send casts ──────────────────────────────────────
    id (*sendId)(id, SEL)            = (id (*)(id, SEL))objc_msgSend;
    id (*sendObj1)(id, SEL, id)      = (id (*)(id, SEL, id))objc_msgSend;
    BOOL (*sendBool1)(id, SEL, id)   = (BOOL (*)(id, SEL, id))objc_msgSend;
    NSUInteger (*sendLen)(id, SEL)   = (NSUInteger (*)(id, SEL))objc_msgSend;

    // Factory casts
    id (*sendInt)(id, SEL, int)      = (id (*)(id, SEL, int))objc_msgSend;
    id (*sendArrayFactory)(id, SEL, id*, NSUInteger) =
        (id (*)(id, SEL, id*, NSUInteger))objc_msgSend;
    id (*sendDictFactory)(id, SEL, id*, id*, NSUInteger) =
        (id (*)(id, SEL, id*, id*, NSUInteger))objc_msgSend;

    // JSON casts: (id, SEL, id, NSUInteger, id*) for data/object
    id (*sendJSON3)(id, SEL, id, NSUInteger, id*) =
        (id (*)(id, SEL, id, NSUInteger, id*))objc_msgSend;

    // ── 1. Class lookups ───────────────────────────────────────────
    Class jsonCls   = (Class)objc_getClass("NSJSONSerialization");
    Class arrayCls  = (Class)objc_getClass("NSArray");
    Class dictCls   = (Class)objc_getClass("NSDictionary");
    Class numCls    = (Class)objc_getClass("NSNumber");
    if (!jsonCls)  { printf("FAIL: NSJSONSerialization class\n"); return 1; }
    if (!arrayCls) { printf("FAIL: NSArray class\n"); return 1; }

    // ── 2. isValidJSONObject with empty array ──────────────────────
    id noItems[] = {};
    id emptyArr = sendArrayFactory(
        (id)arrayCls,
        sel_registerName("arrayWithObjects:count:"),
        noItems, 0
    );
    BOOL valid = sendBool1((id)jsonCls, sel_registerName("isValidJSONObject:"), emptyArr);
    if (!valid) { printf("FAIL: empty array should be valid JSON\n"); return 2; }

    // ── 3. Build dictionary {"name":"test","count":42} ─────────────
    id nameKey  = @"name";
    id nameVal  = @"test";
    id countKey = @"count";
    id countVal = sendInt((id)numCls, sel_registerName("numberWithInt:"), 42);

    id keys[] = { nameKey, countKey };
    id vals[] = { nameVal, countVal };
    id dict = sendDictFactory(
        (id)dictCls,
        sel_registerName("dictionaryWithObjects:forKeys:count:"),
        vals, keys, 2
    );
    if (!dict) { printf("FAIL: dictionaryWithObjects\n"); return 3; }

    // ── 4. dataWithJSONObject:options:error: ───────────────────────
    id data = sendJSON3(
        (id)jsonCls,
        sel_registerName("dataWithJSONObject:options:error:"),
        dict, 0, NULL
    );
    if (!data) { printf("FAIL: dataWithJSONObject returned nil\n"); return 4; }

    NSUInteger len = sendLen(data, sel_registerName("length"));
    if (len == 0) { printf("FAIL: JSON data length should be > 0\n"); return 5; }

    // ── 5. JSONObjectWithData:options:error: ───────────────────────
    id parsed = sendJSON3(
        (id)jsonCls,
        sel_registerName("JSONObjectWithData:options:error:"),
        data, 0, NULL
    );
    if (!parsed) { printf("FAIL: JSONObjectWithData returned nil\n"); return 6; }

    printf("PASS\n");
    return 0;
}
