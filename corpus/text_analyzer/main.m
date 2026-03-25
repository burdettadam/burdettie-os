// text_analyzer — text analysis with character sets and string operations
// Exercises: NSString, NSCharacterSet, NSMutableDictionary, NSNumber, NSArray, blocks
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        NSString *text = @"Hello, World! 123 testing 456. Special chars: @#$%";

        // Count character categories
        NSCharacterSet *letters = [NSCharacterSet letterCharacterSet];
        NSCharacterSet *digits = [NSCharacterSet decimalDigitCharacterSet];
        NSCharacterSet *whitespace = [NSCharacterSet whitespaceCharacterSet];
        NSCharacterSet *punctuation = [NSCharacterSet punctuationCharacterSet];

        int letterCount = 0, digitCount = 0, spaceCount = 0, punctCount = 0;
        // Use NSString substring checks per character position
        NSString *textLower = [text lowercaseString];
        for (NSUInteger i = 0; i < [text length]; i++) {
            NSString *ch = [text substringFromIndex:i];
            if (i + 1 < [text length]) {
                ch = [ch substringToIndex:1];
            }
            // Simple ASCII classification
            const char *c = [ch UTF8String];
            if (c && c[0]) {
                char cc = c[0];
                if ((cc >= 'A' && cc <= 'Z') || (cc >= 'a' && cc <= 'z')) letterCount++;
                else if (cc >= '0' && cc <= '9') digitCount++;
                else if (cc == ' ' || cc == '\t') spaceCount++;
                else if (cc == '.' || cc == ',' || cc == '!' || cc == ':' || cc == '@' || cc == '#' || cc == '$' || cc == '%') punctCount++;
            }
        }

        if (letterCount > 0) { printf("CHECK 1 PASS: %d letters\n", letterCount); checks++; }
        if (digitCount == 6) { printf("CHECK 2 PASS: 6 digits\n"); checks++; }
        if (spaceCount > 0) { printf("CHECK 3 PASS: %d spaces\n", spaceCount); checks++; }

        // Word frequency
        NSArray *words = [[text lowercaseString] componentsSeparatedByString:@" "];
        NSMutableDictionary *freq = [NSMutableDictionary dictionary];
        [words enumerateObjectsUsingBlock:^(NSString *word, NSUInteger idx, BOOL *stop) {
            // Strip punctuation from word (simple: just track as-is)
            NSNumber *existing = [freq objectForKey:word];
            int count = existing ? [existing intValue] + 1 : 1;
            [freq setObject:[NSNumber numberWithInt:count] forKey:word];
        }];

        if ([freq count] > 0) { printf("CHECK 4 PASS: %lu unique words\n", (unsigned long)[freq count]); checks++; }

        // Check specific word
        if ([[freq objectForKey:@"hello,"] intValue] == 1) { printf("CHECK 5 PASS: 'hello,' appears once\n"); checks++; }

        // Trim and split
        NSString *csv = @"  apple, banana, cherry, date  ";
        NSString *trimmed = [csv stringByTrimmingCharactersInSet:[NSCharacterSet whitespaceCharacterSet]];
        NSArray *items = [trimmed componentsSeparatedByString:@", "];
        if ([items count] == 4) { printf("CHECK 6 PASS: 4 CSV items\n"); checks++; }
        if ([[items firstObject] isEqualToString:@"apple"]) { printf("CHECK 7 PASS: first = apple\n"); checks++; }
        if ([[items lastObject] isEqualToString:@"date"]) { printf("CHECK 8 PASS: last = date\n"); checks++; }

        // Case conversion
        NSString *mixed = @"camelCaseIdentifier";
        NSString *upper = [mixed uppercaseString];
        if ([upper isEqualToString:@"CAMELCASEIDENTIFIER"]) { printf("CHECK 9 PASS: uppercase\n"); checks++; }

        // Length stats
        __block NSInteger totalLen = 0;
        [items enumerateObjectsUsingBlock:^(NSString *item, NSUInteger idx, BOOL *stop) {
            totalLen += [item length];
        }];
        if (totalLen > 0) { printf("CHECK 10 PASS: total item length = %ld\n", (long)totalLen); checks++; }

        printf("text_analyzer: %d/10 checks passed\n", checks);
        return (checks == 10) ? 0 : 1;
    }
}
