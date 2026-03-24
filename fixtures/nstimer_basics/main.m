// nstimer_basics — Phase 12 fixture: NSTimer
#import <Foundation/Foundation.h>
#include <stdio.h>

int main() {
    @autoreleasepool {
        // 1. Create a single-fire timer with a block
        __block int fired = 0;
        NSTimer *t = [NSTimer timerWithTimeInterval:1.0 repeats:NO block:^(NSTimer *timer) {
            fired = 1;
        }];
        if (!t) { printf("FAIL: timer nil\n"); return 1; }

        // 2. isValid (should be YES before firing)
        if (![t isValid]) { printf("FAIL: not valid\n"); return 2; }

        // 3. fire manually
        [t fire];
        if (fired != 1) { printf("FAIL: block not called\n"); return 3; }

        // 4. Single-fire timer is invalidated after fire
        if ([t isValid]) { printf("FAIL: still valid after fire\n"); return 4; }

        // 5. Repeating timer stays valid after fire
        __block int count = 0;
        NSTimer *rep = [NSTimer timerWithTimeInterval:0.5 repeats:YES block:^(NSTimer *timer) {
            count++;
        }];
        [rep fire];
        if (count != 1) { printf("FAIL: rep count=%d\n", count); return 5; }
        if (![rep isValid]) { printf("FAIL: rep invalid after fire\n"); return 6; }

        // 6. timeInterval — only valid on repeating timers
        NSTimeInterval ti = [rep timeInterval];
        if (ti < 0.4 || ti > 0.6) { printf("FAIL: interval %f\n", ti); return 7; }

        // 7. invalidate
        [rep invalidate];
        if ([rep isValid]) { printf("FAIL: rep still valid after invalidate\n"); return 8; }

        // 8. description
        NSTimer *t2 = [NSTimer timerWithTimeInterval:2.0 repeats:NO block:^(NSTimer *timer) {}];
        NSString *desc = [t2 description];
        if (!desc) { printf("FAIL: desc nil\n"); return 9; }

        printf("OK\n");
    }
    return 0;
}
