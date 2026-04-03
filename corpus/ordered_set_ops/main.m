// ordered_set_ops — NSMutableOrderedSet, NSMutableSet, NSRegularExpression usage
// matching genstrings requirements. Also exercises NSMutableIndexSet extended API,
// NSScanner, CFStringConvertEncodingToNSStringEncoding patterns.
#import <Foundation/Foundation.h>
#include <stdio.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // ── CHECK 1: NSMutableOrderedSet basic ops ──────────────────────────
        NSMutableOrderedSet *oset = [NSMutableOrderedSet orderedSetWithCapacity:4];
        [oset addObject:@"alpha"];
        [oset addObject:@"beta"];
        [oset addObject:@"alpha"]; // duplicate — should not add
        if ([oset count] == 2 && [[oset objectAtIndex:0] isEqualToString:@"alpha"]) {
            printf("CHECK 1 PASS: NSMutableOrderedSet count=%lu first=%s\n",
                   (unsigned long)[oset count], [[oset objectAtIndex:0] UTF8String]);
            checks++;
        } else {
            printf("CHECK 1 FAIL: count=%lu\n", (unsigned long)[oset count]);
        }

        // ── CHECK 2: NSMutableOrderedSet containsObject ─────────────────────
        if ([oset containsObject:@"beta"] && ![oset containsObject:@"gamma"]) {
            printf("CHECK 2 PASS: containsObject works\n");
            checks++;
        } else {
            printf("CHECK 2 FAIL: containsObject\n");
        }

        // ── CHECK 3: NSMutableSet ───────────────────────────────────────────
        NSMutableSet *mset = [NSMutableSet setWithCapacity:4];
        [mset addObject:@"x"];
        [mset addObject:@"y"];
        [mset addObject:@"x"]; // duplicate
        if ([mset count] == 2 && [mset containsObject:@"y"]) {
            printf("CHECK 3 PASS: NSMutableSet count=%lu\n", (unsigned long)[mset count]);
            checks++;
        } else {
            printf("CHECK 3 FAIL: NSMutableSet count=%lu\n", (unsigned long)[mset count]);
        }

        // ── CHECK 4: NSRegularExpression ────────────────────────────────────
        NSError *error = nil;
        NSRegularExpression *regex = [NSRegularExpression
            regularExpressionWithPattern:@"(\\d+)"
            options:0
            error:&error];
        if (regex && error == nil) {
            NSString *input = @"abc 123 def 456 ghi";
            NSArray *matches = [regex matchesInString:input
                                             options:0
                                               range:NSMakeRange(0, [input length])];
            if ([matches count] == 2) {
                printf("CHECK 4 PASS: regex found %lu matches\n",
                       (unsigned long)[matches count]);
                checks++;
            } else {
                printf("CHECK 4 FAIL: regex found %lu matches\n",
                       (unsigned long)[matches count]);
            }
        } else {
            printf("CHECK 4 FAIL: regex creation failed\n");
        }

        // ── CHECK 5: NSRegularExpression firstMatch + rangeAtIndex ──────────
        if (regex) {
            NSString *input = @"key=value123";
            NSTextCheckingResult *match = [regex firstMatchInString:input
                                                           options:0
                                                             range:NSMakeRange(0, [input length])];
            if (match && [match numberOfRanges] >= 2) {
                NSRange r = [match rangeAtIndex:1];
                NSString *captured = [input substringWithRange:r];
                if ([captured isEqualToString:@"123"]) {
                    printf("CHECK 5 PASS: captured group = '%s'\n",
                           [captured UTF8String]);
                    checks++;
                } else {
                    printf("CHECK 5 FAIL: captured = '%s'\n", [captured UTF8String]);
                }
            } else {
                printf("CHECK 5 FAIL: no match\n");
            }
        }

        // ── CHECK 6: NSMutableIndexSet extended ─────────────────────────────
        NSMutableIndexSet *idxset = [NSMutableIndexSet indexSet];
        [idxset addIndexesInRange:NSMakeRange(10, 5)]; // 10..14
        [idxset addIndex:20];
        if ([idxset count] == 6 && [idxset containsIndex:12] && [idxset containsIndex:20]) {
            printf("CHECK 6 PASS: NSMutableIndexSet count=%lu\n",
                   (unsigned long)[idxset count]);
            checks++;
        } else {
            printf("CHECK 6 FAIL: count=%lu\n", (unsigned long)[idxset count]);
        }

        // ── CHECK 7: NSScanner ──────────────────────────────────────────────
        NSScanner *scanner = [NSScanner scannerWithString:@"key=42 rest"];
        NSString *key = nil;
        [scanner scanUpToString:@"=" intoString:&key];
        [scanner scanString:@"=" intoString:nil];
        int value = 0;
        [scanner scanInt:&value];
        if ([key isEqualToString:@"key"] && value == 42) {
            printf("CHECK 7 PASS: scanner key='%s' value=%d\n",
                   [key UTF8String], value);
            checks++;
        } else {
            printf("CHECK 7 FAIL: key='%s' value=%d\n",
                   key ? [key UTF8String] : "(nil)", value);
        }

        // ── CHECK 8: NSMutableOrderedSet from objects ────────────────────────
        NSMutableOrderedSet *nums = [NSMutableOrderedSet orderedSetWithCapacity:5];
        [nums addObject:@"five"];
        [nums addObject:@"one"];
        [nums addObject:@"three"];
        if ([nums count] == 3 && [[nums objectAtIndex:0] isEqualToString:@"five"]) {
            printf("CHECK 8 PASS: orderedSetWithCapacity count=%lu first=%s\n",
                   (unsigned long)[nums count], [[nums objectAtIndex:0] UTF8String]);
            checks++;
        } else {
            printf("CHECK 8 FAIL: count=%lu\n", (unsigned long)[nums count]);
        }

        // ── Result ──────────────────────────────────────────────────────────
        printf("\nordered_set_ops: %d/8 checks passed\n", checks);
        return (checks == 8) ? 0 : 1;
    }
}
