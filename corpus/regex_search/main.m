// regex_search — NSRegularExpression pattern matching
// Exercises: NSRegularExpression, NSTextCheckingResult, NSRange,
//            NSString, NSError, NSMutableArray, capture groups
#import <Foundation/Foundation.h>

int main(void) {
    @autoreleasepool {
        // 1. Compile a simple pattern
        NSError *err = nil;
        NSRegularExpression *regex =
            [NSRegularExpression regularExpressionWithPattern:@"\\b[A-Z][a-z]+\\b"
                                                     options:0
                                                       error:&err];
        if (regex && !err) {
            printf("CHECK 1 PASS: compiled regex\n");
        } else {
            printf("CHECK 1 FAIL: regex compilation error\n");
            return 1;
        }

        // 2. Count matches
        NSString *text = @"Hello World from Darwin Loader runtime";
        NSUInteger count = [regex numberOfMatchesInString:text
                                                  options:0
                                                    range:NSMakeRange(0, [text length])];
        // "Hello", "World", "Darwin", "Loader"
        if (count == 4) {
            printf("CHECK 2 PASS: found %lu capitalized words\n", (unsigned long)count);
        } else {
            printf("CHECK 2 FAIL: count = %lu\n", (unsigned long)count);
            return 1;
        }

        // 3. Extract first match
        NSTextCheckingResult *first =
            [regex firstMatchInString:text
                              options:0
                                range:NSMakeRange(0, [text length])];
        if (first) {
            NSString *match = [text substringWithRange:[first range]];
            if ([match isEqualToString:@"Hello"]) {
                printf("CHECK 3 PASS: first match = %s\n", [match UTF8String]);
            } else {
                printf("CHECK 3 FAIL: first match = %s\n", [match UTF8String]);
                return 1;
            }
        } else {
            printf("CHECK 3 FAIL: no match\n");
            return 1;
        }

        // 4. Enumerate all matches
        NSMutableArray *matches = [NSMutableArray array];
        [regex enumerateMatchesInString:text
                                options:0
                                  range:NSMakeRange(0, [text length])
                             usingBlock:^(NSTextCheckingResult *result,
                                          NSMatchingFlags flags,
                                          BOOL *stop) {
            [matches addObject:[text substringWithRange:[result range]]];
        }];
        if ([matches count] == 4) {
            printf("CHECK 4 PASS: enumerated 4 matches\n");
        } else {
            printf("CHECK 4 FAIL: enumerated %lu\n", (unsigned long)[matches count]);
            return 1;
        }

        // 5. Regex with capture groups — parse "key=value" pairs
        NSRegularExpression *kvRegex =
            [NSRegularExpression regularExpressionWithPattern:@"(\\w+)=(\\w+)"
                                                     options:0
                                                       error:&err];
        NSString *kvText = @"host=localhost port=5432 debug=true";
        NSArray *kvMatches = [kvRegex matchesInString:kvText
                                              options:0
                                                range:NSMakeRange(0, [kvText length])];
        if ([kvMatches count] == 3) {
            NSTextCheckingResult *m = kvMatches[0];
            NSString *key = [kvText substringWithRange:[m rangeAtIndex:1]];
            NSString *val = [kvText substringWithRange:[m rangeAtIndex:2]];
            if ([key isEqualToString:@"host"] && [val isEqualToString:@"localhost"]) {
                printf("CHECK 5 PASS: parsed key=%s val=%s\n",
                       [key UTF8String], [val UTF8String]);
            } else {
                printf("CHECK 5 FAIL: key=%s val=%s\n",
                       [key UTF8String], [val UTF8String]);
                return 1;
            }
        } else {
            printf("CHECK 5 FAIL: kvMatches = %lu\n", (unsigned long)[kvMatches count]);
            return 1;
        }

        // 6. String replacement
        NSString *replaced = [regex stringByReplacingMatchesInString:text
                                                             options:0
                                                               range:NSMakeRange(0, [text length])
                                                        withTemplate:@"[$0]"];
        if ([replaced containsString:@"[Hello]"] && [replaced containsString:@"[World]"]) {
            printf("CHECK 6 PASS: replacement worked\n");
        } else {
            printf("CHECK 6 FAIL: replaced = %s\n", [replaced UTF8String]);
            return 1;
        }

        // 7. Case-insensitive matching
        NSRegularExpression *ciRegex =
            [NSRegularExpression regularExpressionWithPattern:@"darwin"
                                                     options:NSRegularExpressionCaseInsensitive
                                                       error:&err];
        NSUInteger ciCount = [ciRegex numberOfMatchesInString:text
                                                      options:0
                                                        range:NSMakeRange(0, [text length])];
        if (ciCount == 1) {
            printf("CHECK 7 PASS: case-insensitive match found\n");
        } else {
            printf("CHECK 7 FAIL: ciCount = %lu\n", (unsigned long)ciCount);
            return 1;
        }

        // 8. Pattern property
        NSString *pattern = [regex pattern];
        if ([pattern isEqualToString:@"\\b[A-Z][a-z]+\\b"]) {
            printf("CHECK 8 PASS: pattern property correct\n");
        } else {
            printf("CHECK 8 FAIL: pattern = %s\n", [pattern UTF8String]);
            return 1;
        }
    }
    return 0;
}
