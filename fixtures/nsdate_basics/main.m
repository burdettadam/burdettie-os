// nsdate_basics — NSDate creation, comparison, arithmetic.
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>

typedef id (*send0_t)(id, SEL);
typedef id (*sendD_t)(id, SEL, double);
typedef double (*dsend0_t)(id, SEL);
typedef double (*dsend1_t)(id, SEL, id);
typedef BOOL (*bsend1_t)(id, SEL, id);
typedef NSComparisonResult (*cmpsend_t)(id, SEL, id);
typedef const char *(*csend0_t)(id, SEL);

int main() {
    @autoreleasepool {
        send0_t send0 = (send0_t)objc_msgSend;
        sendD_t sendD = (sendD_t)objc_msgSend;
        dsend0_t dsend0 = (dsend0_t)objc_msgSend;
        dsend1_t dsend1 = (dsend1_t)objc_msgSend;
        bsend1_t bsend1 = (bsend1_t)objc_msgSend;
        cmpsend_t cmpsend = (cmpsend_t)objc_msgSend;
        csend0_t csend0 = (csend0_t)objc_msgSend;

        Class dateCls = objc_getClass("NSDate");
        SEL utf8 = sel_registerName("UTF8String");

        // 1. Create a date from a known Unix timestamp (2001-01-01T00:00:00Z = 978307200)
        id refDate = sendD((id)dateCls, sel_registerName("dateWithTimeIntervalSince1970:"), 978307200.0);
        if (!refDate) { printf("FAIL: 1 - dateWithTimeIntervalSince1970 nil\n"); return 1; }
        printf("CHECK 1: date created from unix timestamp\n");

        // 2. timeIntervalSince1970 roundtrip
        double ti1970 = dsend0(refDate, sel_registerName("timeIntervalSince1970"));
        double diff = ti1970 - 978307200.0;
        if (diff < -0.001 || diff > 0.001) {
            printf("FAIL: 2 - ti1970 = %f\n", ti1970); return 2;
        }
        printf("CHECK 2: timeIntervalSince1970 = %.0f\n", ti1970);

        // 3. timeIntervalSinceReferenceDate (should be ~0 for the reference date)
        double tiRef = dsend0(refDate, sel_registerName("timeIntervalSinceReferenceDate"));
        if (tiRef < -0.001 || tiRef > 0.001) {
            printf("FAIL: 3 - tiRef = %f\n", tiRef); return 3;
        }
        printf("CHECK 3: timeIntervalSinceReferenceDate = %.1f\n", tiRef);

        // 4. dateByAddingTimeInterval:
        id laterDate = sendD(refDate, sel_registerName("dateByAddingTimeInterval:"), 3600.0);
        double laterTi = dsend0(laterDate, sel_registerName("timeIntervalSince1970"));
        diff = laterTi - 978310800.0; // 978307200 + 3600
        if (diff < -0.001 || diff > 0.001) {
            printf("FAIL: 4 - laterTi = %f\n", laterTi); return 4;
        }
        printf("CHECK 4: dateByAddingTimeInterval +3600s = %.0f\n", laterTi);

        // 5. isEqualToDate:
        id sameDate = sendD((id)dateCls, sel_registerName("dateWithTimeIntervalSince1970:"), 978307200.0);
        BOOL eq = bsend1(refDate, sel_registerName("isEqualToDate:"), sameDate);
        if (!eq) { printf("FAIL: 5 - dates not equal\n"); return 5; }
        BOOL neq = bsend1(refDate, sel_registerName("isEqualToDate:"), laterDate);
        if (neq) { printf("FAIL: 5 - different dates reported equal\n"); return 5; }
        printf("CHECK 5: isEqualToDate works correctly\n");

        // 6. compare:
        NSComparisonResult cmp = cmpsend(refDate, sel_registerName("compare:"), laterDate);
        if (cmp != NSOrderedAscending) { printf("FAIL: 6 - compare result %ld\n", (long)cmp); return 6; }
        NSComparisonResult cmp2 = cmpsend(laterDate, sel_registerName("compare:"), refDate);
        if (cmp2 != NSOrderedDescending) { printf("FAIL: 6 - compare2 result %ld\n", (long)cmp2); return 6; }
        NSComparisonResult cmp3 = cmpsend(refDate, sel_registerName("compare:"), sameDate);
        if (cmp3 != NSOrderedSame) { printf("FAIL: 6 - compare3 result %ld\n", (long)cmp3); return 6; }
        printf("CHECK 6: compare works correctly\n");

        // 7. timeIntervalSinceDate:
        double delta = dsend1(laterDate, sel_registerName("timeIntervalSinceDate:"), refDate);
        if (delta < 3599.9 || delta > 3600.1) {
            printf("FAIL: 7 - delta = %f\n", delta); return 7;
        }
        printf("CHECK 7: timeIntervalSinceDate = %.0f\n", delta);

        // 8. description
        id desc = send0(refDate, sel_registerName("description"));
        if (!desc) { printf("FAIL: 8 - description nil\n"); return 8; }
        const char *descStr = csend0(desc, utf8);
        printf("CHECK 8: description = %s\n", descStr);

        // 9. [NSDate date] should return current time (just verify non-nil and reasonable)
        id now = send0((id)dateCls, sel_registerName("date"));
        if (!now) { printf("FAIL: 9 - [NSDate date] nil\n"); return 9; }
        double nowTi = dsend0(now, sel_registerName("timeIntervalSince1970"));
        // Should be after 2024 (1704067200)
        if (nowTi < 1704067200.0) { printf("FAIL: 9 - now too old: %f\n", nowTi); return 9; }
        printf("CHECK 9: [NSDate date] timeIntervalSince1970 = %.0f\n", nowTi);

        // 10. timeIntervalSinceNow should be negative for a past date
        double sinceNow = dsend0(refDate, sel_registerName("timeIntervalSinceNow"));
        if (sinceNow >= 0) { printf("FAIL: 10 - sinceNow >= 0: %f\n", sinceNow); return 10; }
        printf("CHECK 10: timeIntervalSinceNow = %.0f (negative = past)\n", sinceNow);

        printf("OK: nsdate_basics passed\n");
        return 0;
    }
}
