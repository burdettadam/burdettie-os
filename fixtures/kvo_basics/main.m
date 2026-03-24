// kvo_basics — Key-Value Observing fixture
//
// Tests manual KVO: addObserver / willChangeValueForKey / didChangeValueForKey
// → observeValueForKeyPath:ofObject:change:context: callback dispatch.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <stdio.h>

// ── Observable: a class with a property we'll observe ──────────────

@interface Person : NSObject {
    NSString *_name;
}
@property (nonatomic, copy) NSString *name;
@end

@implementation Person

// Manual KVO — we call willChange/didChange explicitly,
// and disable automatic KVO notification for this key.
+ (BOOL)automaticallyNotifiesObserversForKey:(NSString *)key {
    if ([key isEqualToString:@"name"]) {
        return NO;
    }
    return [super automaticallyNotifiesObserversForKey:key];
}

- (void)setName:(NSString *)name {
    [self willChangeValueForKey:@"name"];
    _name = name;
    [self didChangeValueForKey:@"name"];
}

- (NSString *)name {
    return _name;
}

@end

// ── Observer: tracks KVO callbacks ─────────────────────────────────

static int gCallbackCount = 0;
static const char *gLastKeyPath = NULL;

@interface Watcher : NSObject
@end

@implementation Watcher

- (void)observeValueForKeyPath:(NSString *)keyPath
                      ofObject:(id)object
                        change:(NSDictionary *)change
                       context:(void *)context {
    gCallbackCount++;
    gLastKeyPath = [keyPath UTF8String];
    printf("  KVO callback #%d: keyPath=%s\n", gCallbackCount, gLastKeyPath);
}

@end

// ── Main ───────────────────────────────────────────────────────────

int main() {
    printf("kvo_basics test\n");

    Person *person = [[Person alloc] init];
    Watcher *watcher = [[Watcher alloc] init];

    // 1. Register observer
    [person addObserver:watcher
             forKeyPath:@"name"
                options:0
                context:NULL];
    printf("  Observer registered\n");

    // 2. Change property — should trigger callback
    printf("  Setting name to Alice...\n");
    [person setName:@"Alice"];

    if (gCallbackCount != 1) {
        printf("FAIL: expected 1 callback, got %d\n", gCallbackCount);
        return 1;
    }
    printf("  ✓ Got %d callback(s) after first change\n", gCallbackCount);

    // 3. Change again — should trigger another callback
    printf("  Setting name to Bob...\n");
    [person setName:@"Bob"];

    if (gCallbackCount != 2) {
        printf("FAIL: expected 2 callbacks, got %d\n", gCallbackCount);
        return 1;
    }
    printf("  ✓ Got %d callback(s) after second change\n", gCallbackCount);

    // 4. Remove observer
    [person removeObserver:watcher forKeyPath:@"name"];
    printf("  Observer removed\n");

    // 5. Change after removal — should NOT trigger callback
    printf("  Setting name to Charlie (after removal)...\n");
    [person setName:@"Charlie"];

    if (gCallbackCount != 2) {
        printf("FAIL: expected 2 callbacks after removal, got %d\n", gCallbackCount);
        return 1;
    }
    printf("  ✓ No callback after removal (still %d)\n", gCallbackCount);

    printf("kvo_basics: PASS\n");
    return 0;
}
