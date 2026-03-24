// kvc_basics — Key-Value Coding fixture
//
// Tests KVC: valueForKey:, setValue:forKey:, and auto-boxing
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <stdio.h>

// ── Vehicle: a class with KVC-accessible properties ──────────────

@interface Vehicle : NSObject {
    NSString *_make;
    int _year;
    double _horsepower;
}
@property (nonatomic, retain) NSString *make;
@property (nonatomic, assign) int year;
@property (nonatomic, assign) double horsepower;
@end

@implementation Vehicle
@synthesize make = _make;
@synthesize year = _year;
@synthesize horsepower = _horsepower;
@end

// ── Engine: for key-path traversal ─────────────────────────────────

@interface Engine : NSObject {
    int _cylinders;
}
@property (nonatomic, assign) int cylinders;
@end

@implementation Engine
@synthesize cylinders = _cylinders;
@end

// ── Car: has an engine for key-path test ───────────────────────────

@interface Car : NSObject {
    NSString *_model;
    Engine *_engine;
}
@property (nonatomic, retain) NSString *model;
@property (nonatomic, retain) Engine *engine;
@end

@implementation Car
@synthesize model = _model;
@synthesize engine = _engine;
@end

// ── Main ───────────────────────────────────────────────────────────

int main() {
    printf("kvc_basics test\n");
    int failures = 0;

    // 1. valueForKey: with getter method
    printf("\n--- Test 1: valueForKey: via ivar ---\n");
    Vehicle *v = [[Vehicle alloc] init];
    [v setMake:@"Toyota"];
    [v setYear:2024];
    [v setHorsepower:301.5];

    NSString *make = [v valueForKey:@"make"];
    if (make && [make isEqualToString:@"Toyota"]) {
        printf("  PASS: valueForKey:@\"make\" = %s\n", [make UTF8String]);
    } else {
        printf("  FAIL: valueForKey:@\"make\" expected Toyota, got %s\n",
               make ? [make UTF8String] : "(nil)");
        failures++;
    }

    // 2. valueForKey: with scalar auto-boxing
    printf("\n--- Test 2: valueForKey: scalar auto-boxing ---\n");
    NSNumber *yearNum = [v valueForKey:@"year"];
    if (yearNum && [yearNum intValue] == 2024) {
        printf("  PASS: valueForKey:@\"year\" = %d\n", [yearNum intValue]);
    } else {
        printf("  FAIL: valueForKey:@\"year\" expected 2024, got %d\n",
               yearNum ? [yearNum intValue] : -1);
        failures++;
    }

    // 3. valueForKey: with double auto-boxing
    printf("\n--- Test 3: valueForKey: double auto-boxing ---\n");
    NSNumber *hpNum = [v valueForKey:@"horsepower"];
    if (hpNum && [hpNum doubleValue] > 301.0 && [hpNum doubleValue] < 302.0) {
        printf("  PASS: valueForKey:@\"horsepower\" = %.1f\n", [hpNum doubleValue]);
    } else {
        printf("  FAIL: valueForKey:@\"horsepower\" expected ~301.5, got %.1f\n",
               hpNum ? [hpNum doubleValue] : -1.0);
        failures++;
    }

    // 4. setValue:forKey: with object
    printf("\n--- Test 4: setValue:forKey: with object ---\n");
    [v setValue:@"Honda" forKey:@"make"];
    NSString *newMake = [v valueForKey:@"make"];
    if (newMake && [newMake isEqualToString:@"Honda"]) {
        printf("  PASS: setValue:forKey: make = %s\n", [newMake UTF8String]);
    } else {
        printf("  FAIL: setValue:forKey: expected Honda, got %s\n",
               newMake ? [newMake UTF8String] : "(nil)");
        failures++;
    }

    // 5. setValue:forKey: with scalar (NSNumber auto-unboxing)
    printf("\n--- Test 5: setValue:forKey: with scalar ---\n");
    [v setValue:@(2025) forKey:@"year"];
    NSNumber *yearNum2 = [v valueForKey:@"year"];
    if (yearNum2 && [yearNum2 intValue] == 2025) {
        printf("  PASS: setValue:forKey: year = %d\n", [yearNum2 intValue]);
    } else {
        printf("  FAIL: setValue:forKey: expected 2025, got %d\n",
               yearNum2 ? [yearNum2 intValue] : -1);
        failures++;
    }

    // 6. valueForKeyPath: traversal
    printf("\n--- Test 6: valueForKeyPath: traversal ---\n");
    Car *car = [[Car alloc] init];
    Engine *eng = [[Engine alloc] init];
    [eng setCylinders:8];
    [car setEngine:eng];
    [car setModel:@"Mustang"];

    NSNumber *cyl = [car valueForKeyPath:@"engine.cylinders"];
    if (cyl && [cyl intValue] == 8) {
        printf("  PASS: valueForKeyPath:@\"engine.cylinders\" = %d\n", [cyl intValue]);
    } else {
        printf("  FAIL: valueForKeyPath:@\"engine.cylinders\" expected 8, got %d\n",
               cyl ? [cyl intValue] : -1);
        failures++;
    }

    // 7. setValue:forKeyPath: traversal
    printf("\n--- Test 7: setValue:forKeyPath: traversal ---\n");
    [car setValue:@(6) forKeyPath:@"engine.cylinders"];
    NSNumber *cyl2 = [car valueForKeyPath:@"engine.cylinders"];
    if (cyl2 && [cyl2 intValue] == 6) {
        printf("  PASS: setValue:forKeyPath: engine.cylinders = %d\n", [cyl2 intValue]);
    } else {
        printf("  FAIL: setValue:forKeyPath: expected 6, got %d\n",
               cyl2 ? [cyl2 intValue] : -1);
        failures++;
    }

    // Summary
    printf("\nkvc_basics: %d failures\n", failures);
    if (failures == 0) {
        printf("kvc_basics: PASS\n");
    } else {
        printf("kvc_basics: FAIL\n");
    }
    return failures;
}
