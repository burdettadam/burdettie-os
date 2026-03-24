// nslog_format_specifiers — T23 fixture: expanded format specifier coverage
//
// Tests NSLog with %f, %x, %X, %p, %lld, %llu, %zu, %o, %e
// to verify the format expansion engine handles these specifiers.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <stdio.h>
#include <string.h>
#include <stdint.h>

int main() {
    // 1. %f — floating point
    NSLog(@"T23 float=%f", 3.14);

    // 2. %x — hex lowercase
    NSLog(@"T23 hex=%x", 255);

    // 3. %X — hex uppercase
    NSLog(@"T23 HEX=%X", 255);

    // 4. %p — pointer
    int dummy = 0;
    NSLog(@"T23 ptr=%p", &dummy);

    // 5. %lld — long long signed
    long long big = -123456789LL;
    NSLog(@"T23 lld=%lld", big);

    // 6. %llu — long long unsigned
    unsigned long long ubig = 9999999999ULL;
    NSLog(@"T23 llu=%llu", ubig);

    // 7. %zu — size_t
    size_t sz = 1024;
    NSLog(@"T23 zu=%zu", sz);

    // 8. %o — octal
    NSLog(@"T23 oct=%o", 8);

    // 9. %e — scientific
    NSLog(@"T23 sci=%e", 12345.0);

    // 10. Mixed: %d + %f + %x in one format
    NSLog(@"T23 mixed=%d %f 0x%x", 42, 1.5, 255);

    printf("T23: all checks passed\n");
    return 0;
}
