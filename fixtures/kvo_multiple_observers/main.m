// kvo_multiple_observers — Multiple observers on the same key path
//
// Tests that multiple observers all receive KVO callbacks, and that
// removing one observer does not affect the other.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <stdio.h>

// ── Observable ─────────────────────────────────────────────────────

@interface Counter : NSObject {
    int _value;
}
@property (nonatomic) int value;
@end

@implementation Counter

+ (BOOL)automaticallyNotifiesObserversForKey:(NSString *)key {
    if ([key isEqualToString:@"value"]) {
        return NO;
    }
    return [super automaticallyNotifiesObserversForKey:key];
}

- (void)setValue:(int)value {
    [self willChangeValueForKey:@"value"];
    _value = value;
    [self didChangeValueForKey:@"value"];
}

- (int)value {
    return _value;
}

@end

// ── Observers ──────────────────────────────────────────────────────

static int gObserver1Count = 0;
static int gObserver2Count = 0;

@interface Observer1 : NSObject
@end

@implementation Observer1
- (void)observeValueForKeyPath:(NSString *)keyPath
                      ofObject:(id)object
                        change:(NSDictionary *)change
                       context:(void *)context {
    gObserver1Count++;
    printf("  Observer1 callback #%d\n", gObserver1Count);
}
@end

@interface Observer2 : NSObject
@end

@implementation Observer2
- (void)observeValueForKeyPath:(NSString *)keyPath
                      ofObject:(id)object
                        change:(NSDictionary *)change
                       context:(void *)context {
    gObserver2Count++;
    printf("  Observer2 callback #%d\n", gObserver2Count);
}
@end

// ── Main ───────────────────────────────────────────────────────────

int main() {
    printf("kvo_multiple_observers test\n");

    Counter *counter = [[Counter alloc] init];
    Observer1 *obs1 = [[Observer1 alloc] init];
    Observer2 *obs2 = [[Observer2 alloc] init];

    // 1. Register both observers
    [counter addObserver:obs1 forKeyPath:@"value" options:0 context:NULL];
    [counter addObserver:obs2 forKeyPath:@"value" options:0 context:NULL];
    printf("  Both observers registered\n");

    // 2. Change — both should fire
    printf("  Setting value to 10...\n");
    [counter setValue:10];

    if (gObserver1Count != 1 || gObserver2Count != 1) {
        printf("FAIL: expected both at 1, got obs1=%d obs2=%d\n",
               gObserver1Count, gObserver2Count);
        return 1;
    }
    printf("  ✓ Both observers fired (obs1=%d, obs2=%d)\n",
           gObserver1Count, gObserver2Count);

    // 3. Remove observer1 only
    [counter removeObserver:obs1 forKeyPath:@"value"];
    printf("  Observer1 removed\n");

    // 4. Change — only observer2 should fire
    printf("  Setting value to 20...\n");
    [counter setValue:20];

    if (gObserver1Count != 1) {
        printf("FAIL: obs1 should still be 1, got %d\n", gObserver1Count);
        return 1;
    }
    if (gObserver2Count != 2) {
        printf("FAIL: obs2 should be 2, got %d\n", gObserver2Count);
        return 1;
    }
    printf("  ✓ Only Observer2 fired (obs1=%d, obs2=%d)\n",
           gObserver1Count, gObserver2Count);

    // 5. Remove observer2
    [counter removeObserver:obs2 forKeyPath:@"value"];

    // 6. Change — no observers should fire
    printf("  Setting value to 30 (no observers)...\n");
    [counter setValue:30];

    if (gObserver1Count != 1 || gObserver2Count != 2) {
        printf("FAIL: counts changed after removing all: obs1=%d obs2=%d\n",
               gObserver1Count, gObserver2Count);
        return 1;
    }
    printf("  ✓ No callbacks after removing all (obs1=%d, obs2=%d)\n",
           gObserver1Count, gObserver2Count);

    printf("kvo_multiple_observers: PASS\n");
    return 0;
}
