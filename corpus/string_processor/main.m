// string_processor — text transformation pipeline
// Exercises: NSString, NSMutableString, NSArray, componentsSeparatedByString, blocks
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        NSString *input = @"  Hello, World! This is a TEST of string PROCESSING.  ";

        int checks = 0;

        // Trim whitespace (manual — trim to first/last non-space)
        NSString *trimmed = [input stringByTrimmingCharactersInSet:[NSCharacterSet whitespaceCharacterSet]];
        if (![trimmed hasPrefix:@" "]) { printf("CHECK 1 PASS: trimmed leading space\n"); checks++; }

        // Lowercase
        NSString *lower = [trimmed lowercaseString];
        if ([lower isEqualToString:@"hello, world! this is a test of string processing."]) {
            printf("CHECK 2 PASS: lowercased\n"); checks++;
        }

        // Uppercase
        NSString *upper = [trimmed uppercaseString];
        if ([upper hasPrefix:@"HELLO"]) { printf("CHECK 3 PASS: uppercased\n"); checks++; }

        // Split words
        NSArray *words = [trimmed componentsSeparatedByString:@" "];
        if ([words count] == 9) { printf("CHECK 4 PASS: 9 words\n"); checks++; }

        // String replacement
        NSString *replaced = [trimmed stringByReplacingOccurrencesOfString:@"TEST" withString:@"test"];
        if ([replaced rangeOfString:@"test"].location != NSNotFound) { printf("CHECK 5 PASS: replaced TEST with test\n"); checks++; }

        // Build result with NSMutableString
        NSMutableString *result = [NSMutableString string];
        [result appendString:@"Result: "];
        [result appendString:[NSString stringWithFormat:@"%lu words", (unsigned long)[words count]]];
        if ([result hasPrefix:@"Result:"]) { printf("CHECK 6 PASS: built result string\n"); checks++; }

        // Substring operations
        NSString *sub = [trimmed substringToIndex:5];
        if ([sub isEqualToString:@"Hello"]) { printf("CHECK 7 PASS: substringToIndex\n"); checks++; }

        NSString *sub2 = [trimmed substringFromIndex:[trimmed length] - 1];
        if ([sub2 isEqualToString:@"."]) { printf("CHECK 8 PASS: substringFromIndex\n"); checks++; }

        // Length check
        if ([trimmed length] == 50) { printf("CHECK 9 PASS: length = 50\n"); checks++; }

        // Contains check via rangeOfString
        if ([trimmed rangeOfString:@"World"].location != NSNotFound) { printf("CHECK 10 PASS: contains World\n"); checks++; }

        printf("string_processor: %d/10 checks passed\n", checks);
        return (checks == 10) ? 0 : 1;
    }
}
