// objc_super — T14 fixture: objc_msgSendSuper / objc_msgSendSuper2
//
// Tests that [super method] calls work correctly. Each class in the
// hierarchy overrides a method and chains to super, exercising the
// objc_msgSendSuper (or objc_msgSendSuper2) trampoline.
//
// Hierarchy: Animal : NSObject
//            Dog    : Animal
//            Puppy  : Dog
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>

// ── Animal (base, inherits from NSObject) ──────────────────────────

@interface Animal : NSObject
- (int)legs;
- (int)sound;
@end

@implementation Animal
- (instancetype)init {
    self = [super init];    // calls NSObject's init via msgSendSuper
    return self;
}
- (int)legs  { return 4; }
- (int)sound { return 0; }
@end

// ── Dog (overrides sound, chains to super for legs) ────────────────

@interface Dog : Animal
- (int)sound;
- (int)bonus;
@end

@implementation Dog
- (instancetype)init {
    self = [super init];    // calls Animal's init via msgSendSuper
    return self;
}
- (int)sound { return 1; }
- (int)bonus {
    // Call super's legs method — should get Animal's legs (4)
    return [super legs] + 10;
}
@end

// ── Puppy (overrides legs, chains to super for sound) ──────────────

@interface Puppy : Dog
- (int)legs;
- (int)chain;
@end

@implementation Puppy
- (instancetype)init {
    self = [super init];    // calls Dog's init via msgSendSuper
    return self;
}
- (int)legs { return 2; }
- (int)chain {
    // [super sound] → Dog's sound → 1
    // [super legs]  → Dog doesn't override legs → Animal's legs → 4
    return [super sound] * 100 + [super legs];
}
@end

int main() {
    // ── 1. Animal: init + methods ──────────────────────────────────
    Animal *a = [[Animal alloc] init];
    if (!a) { printf("FAIL: Animal alloc/init\n"); return 1; }

    if ([a legs] != 4)  { printf("FAIL: Animal legs=%d\n", [a legs]); return 2; }
    if ([a sound] != 0) { printf("FAIL: Animal sound=%d\n", [a sound]); return 3; }

    // ── 2. Dog: init chains to Animal → NSObject ───────────────────
    Dog *d = [[Dog alloc] init];
    if (!d) { printf("FAIL: Dog alloc/init\n"); return 4; }

    if ([d legs] != 4)  { printf("FAIL: Dog legs=%d\n", [d legs]); return 5; }
    if ([d sound] != 1) { printf("FAIL: Dog sound=%d\n", [d sound]); return 6; }

    // ── 3. Dog.bonus uses [super legs] → Animal.legs (4) + 10 = 14
    if ([d bonus] != 14) { printf("FAIL: Dog bonus=%d\n", [d bonus]); return 7; }

    // ── 4. Puppy: init chains Dog → Animal → NSObject ──────────────
    Puppy *p = [[Puppy alloc] init];
    if (!p) { printf("FAIL: Puppy alloc/init\n"); return 8; }

    // Puppy overrides legs to 2, inherits Dog's sound (1)
    if ([p legs] != 2)  { printf("FAIL: Puppy legs=%d\n", [p legs]); return 9; }
    if ([p sound] != 1) { printf("FAIL: Puppy sound=%d\n", [p sound]); return 10; }

    // ── 5. Puppy.chain: [super sound]=1, [super legs]=4 → 104 ─────
    if ([p chain] != 104) { printf("FAIL: Puppy chain=%d\n", [p chain]); return 11; }

    // ── 6. Puppy.bonus inherited from Dog: [super legs]=4+10=14 ────
    if ([p bonus] != 14) { printf("FAIL: Puppy bonus=%d\n", [p bonus]); return 12; }

    printf("T14: all checks passed\n");
    return 0;
}
