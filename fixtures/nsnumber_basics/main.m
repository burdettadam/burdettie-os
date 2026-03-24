// nsnumber_basics — T17 fixture: NSNumber creation and value access
//
// Tests [NSNumber numberWithInt:], [NSNumber numberWithDouble:],
// [NSNumber numberWithBool:], intValue, doubleValue, boolValue,
// integerValue, isEqualToNumber:, and NSLog %@ on numbers.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>
#include <math.h>

int main() {
    // ── Setup: msg_send casts ──────────────────────────────────────
    id (*sendId)(id, SEL)            = (id (*)(id, SEL))objc_msgSend;
    int (*sendIntVal)(id, SEL)       = (int (*)(id, SEL))objc_msgSend;
    double (*sendDblVal)(id, SEL)    = (double (*)(id, SEL))objc_msgSend;
    BOOL (*sendBoolVal)(id, SEL)     = (BOOL (*)(id, SEL))objc_msgSend;
    NSInteger (*sendIntegerVal)(id, SEL) = (NSInteger (*)(id, SEL))objc_msgSend;
    BOOL (*sendEqNum)(id, SEL, id)   = (BOOL (*)(id, SEL, id))objc_msgSend;

    // Factory casts
    id (*sendInt)(id, SEL, int)      = (id (*)(id, SEL, int))objc_msgSend;
    id (*sendDbl)(id, SEL, double)   = (id (*)(id, SEL, double))objc_msgSend;
    id (*sendBool)(id, SEL, BOOL)    = (id (*)(id, SEL, BOOL))objc_msgSend;

    // ── 1. NSNumber class lookup ───────────────────────────────────
    Class nsnumberClass = (Class)objc_getClass("NSNumber");
    if (!nsnumberClass) { printf("FAIL: NSNumber class\n"); return 1; }

    // ── 2. [NSNumber numberWithInt:42] ─────────────────────────────
    SEL nwiSel = sel_registerName("numberWithInt:");
    id n42 = sendInt((id)nsnumberClass, nwiSel, 42);
    if (!n42) { printf("FAIL: numberWithInt\n"); return 2; }

    // ── 3. [n42 intValue] == 42 ────────────────────────────────────
    SEL ivSel = sel_registerName("intValue");
    int iv42 = sendIntVal(n42, ivSel);
    if (iv42 != 42) { printf("FAIL: intValue=%d\n", iv42); return 3; }

    // ── 4. [n42 doubleValue] == 42.0 ───────────────────────────────
    SEL dvSel = sel_registerName("doubleValue");
    double dv42 = sendDblVal(n42, dvSel);
    if (dv42 != 42.0) { printf("FAIL: doubleValue=%f\n", dv42); return 4; }

    // ── 5. [n42 boolValue] == YES ──────────────────────────────────
    SEL bvSel = sel_registerName("boolValue");
    BOOL bv42 = sendBoolVal(n42, bvSel);
    if (!bv42) { printf("FAIL: boolValue\n"); return 5; }

    // ── 6. [NSNumber numberWithDouble:3.14] ────────────────────────
    SEL nwdSel = sel_registerName("numberWithDouble:");
    id nPi = sendDbl((id)nsnumberClass, nwdSel, 3.14);
    if (!nPi) { printf("FAIL: numberWithDouble\n"); return 6; }

    // ── 7. [nPi doubleValue] == 3.14 ───────────────────────────────
    double dvPi = sendDblVal(nPi, dvSel);
    if (fabs(dvPi - 3.14) > 0.001) { printf("FAIL: pi doubleValue=%f\n", dvPi); return 7; }

    // ── 8. [nPi intValue] == 3 (truncation) ────────────────────────
    int ivPi = sendIntVal(nPi, ivSel);
    if (ivPi != 3) { printf("FAIL: pi intValue=%d\n", ivPi); return 8; }

    // ── 9. [NSNumber numberWithBool:YES] ───────────────────────────
    SEL nwbSel = sel_registerName("numberWithBool:");
    id nYes = sendBool((id)nsnumberClass, nwbSel, YES);
    if (!nYes) { printf("FAIL: numberWithBool YES\n"); return 9; }

    // ── 10. [nYes boolValue] == YES ────────────────────────────────
    BOOL bvYes = sendBoolVal(nYes, bvSel);
    if (!bvYes) { printf("FAIL: nYes boolValue\n"); return 10; }

    // ── 11. [nYes intValue] == 1 ───────────────────────────────────
    int ivYes = sendIntVal(nYes, ivSel);
    if (ivYes != 1) { printf("FAIL: nYes intValue=%d\n", ivYes); return 11; }

    // ── 12. [NSNumber numberWithBool:NO] ───────────────────────────
    id nNo = sendBool((id)nsnumberClass, nwbSel, NO);
    if (!nNo) { printf("FAIL: numberWithBool NO\n"); return 12; }

    // ── 13. [nNo boolValue] == NO ──────────────────────────────────
    BOOL bvNo = sendBoolVal(nNo, bvSel);
    if (bvNo) { printf("FAIL: nNo boolValue\n"); return 13; }

    // ── 14. [nNo intValue] == 0 ────────────────────────────────────
    int ivNo = sendIntVal(nNo, ivSel);
    if (ivNo != 0) { printf("FAIL: nNo intValue=%d\n", ivNo); return 14; }

    // ── 15. numberWithInt:0 → boolValue == NO ──────────────────────
    id n0 = sendInt((id)nsnumberClass, nwiSel, 0);
    BOOL bv0 = sendBoolVal(n0, bvSel);
    if (bv0) { printf("FAIL: n0 boolValue\n"); return 15; }

    // ── 16. isEqualToNumber: same value ────────────────────────────
    SEL eqSel = sel_registerName("isEqualToNumber:");
    id n42b = sendInt((id)nsnumberClass, nwiSel, 42);
    BOOL eq = sendEqNum(n42, eqSel, n42b);
    if (!eq) { printf("FAIL: isEqualToNumber same\n"); return 16; }

    // ── 17. isEqualToNumber: different value ───────────────────────
    id n99 = sendInt((id)nsnumberClass, nwiSel, 99);
    BOOL neq = sendEqNum(n42, eqSel, n99);
    if (neq) { printf("FAIL: isEqualToNumber diff\n"); return 17; }

    // ── 18. integerValue ───────────────────────────────────────────
    SEL itvSel = sel_registerName("integerValue");
    NSInteger itv42 = sendIntegerVal(n42, itvSel);
    if (itv42 != 42) { printf("FAIL: integerValue=%ld\n", (long)itv42); return 18; }

    // ── 19. NSLog with number description ──────────────────────────
    NSLog(@"T17 n42=%@", n42);

    printf("T17: all checks passed\n");
    return 0;
}
