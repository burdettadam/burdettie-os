// attr_string — attributed text construction and inspection
// Mimics: textutil(1) rich text processing, terminal colorizers, Markdown renderers
// Exercises: NSAttributedString, NSMutableAttributedString, NSDictionary,
//   NSString, NSNumber, NSValue (rangeValue), addAttribute:value:range:
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // CHECK 1: Create attributed string with attributes
        NSDictionary *boldAttrs = @{
            @"NSFont": @"Helvetica-Bold",
            @"NSFontSize": [NSNumber numberWithInteger:16],
        };
        NSAttributedString *bold = [[NSAttributedString alloc] initWithString:@"Hello" attributes:boldAttrs];
        if ([[bold string] isEqualToString:@"Hello"] && [bold length] == 5) {
            printf("CHECK 1 PASS: attributed string 'Hello' len=5\n"); checks++;
        } else {
            printf("CHECK 1 FAIL: string=%s len=%lu\n",
                   [[bold string] UTF8String], (unsigned long)[bold length]);
        }

        // CHECK 2: Build mutable attributed string
        NSMutableAttributedString *doc = [[NSMutableAttributedString alloc]
            initWithString:@"The quick brown fox"];
        if ([[doc string] isEqualToString:@"The quick brown fox"] && [doc length] == 19) {
            printf("CHECK 2 PASS: mutable attrstring len=19\n"); checks++;
        } else {
            printf("CHECK 2 FAIL: len=%lu\n", (unsigned long)[doc length]);
        }

        // CHECK 3: Add attributes to ranges — simulate syntax highlighting
        // "The" = keyword, "quick" = identifier, "brown" = literal, "fox" = keyword
        [doc addAttribute:@"SyntaxType" value:@"keyword" range:NSMakeRange(0, 3)];
        [doc addAttribute:@"SyntaxType" value:@"identifier" range:NSMakeRange(4, 5)];
        [doc addAttribute:@"SyntaxType" value:@"literal" range:NSMakeRange(10, 5)];
        [doc addAttribute:@"SyntaxType" value:@"keyword" range:NSMakeRange(16, 3)];

        // Read back attribute at index 4 ("quick")
        NSRange effectiveRange;
        NSDictionary *attrsAt4 = [doc attributesAtIndex:4 effectiveRange:&effectiveRange];
        NSString *syntaxAt4 = [attrsAt4 objectForKey:@"SyntaxType"];
        if ([syntaxAt4 isEqualToString:@"identifier"]) {
            printf("CHECK 3 PASS: attr at idx 4 = identifier\n"); checks++;
        } else {
            printf("CHECK 3 FAIL: attr = %s\n", syntaxAt4 ? [syntaxAt4 UTF8String] : "(nil)");
        }

        // CHECK 4: Read attribute at different position
        NSDictionary *attrsAt0 = [doc attributesAtIndex:0 effectiveRange:&effectiveRange];
        NSString *syntaxAt0 = [attrsAt0 objectForKey:@"SyntaxType"];
        if ([syntaxAt0 isEqualToString:@"keyword"]) {
            printf("CHECK 4 PASS: attr at idx 0 = keyword\n"); checks++;
        } else {
            printf("CHECK 4 FAIL: attr = %s\n", syntaxAt0 ? [syntaxAt0 UTF8String] : "(nil)");
        }

        // CHECK 5: Replace text within attributed string
        [doc replaceCharactersInRange:NSMakeRange(10, 5) withString:@"red"];
        // "The quick red fox" — length 17
        NSString *replaced = [doc string];
        if ([replaced isEqualToString:@"The quick red fox"] && [doc length] == 17) {
            printf("CHECK 5 PASS: replaced 'brown' → 'red', len=17\n"); checks++;
        } else {
            printf("CHECK 5 FAIL: '%s' len=%lu\n", [replaced UTF8String], (unsigned long)[doc length]);
        }

        // CHECK 6: Plain attributed string (no attributes) — verify empty attrs
        NSAttributedString *plain = [[NSAttributedString alloc] initWithString:@"plain text"];
        NSDictionary *plainAttrs = [plain attributesAtIndex:0 effectiveRange:&effectiveRange];
        if ([plain length] == 10 && [plainAttrs count] == 0) {
            printf("CHECK 6 PASS: plain string, 0 attrs\n"); checks++;
        } else {
            printf("CHECK 6 FAIL: len=%lu attrs=%lu\n",
                   (unsigned long)[plain length], (unsigned long)[plainAttrs count]);
        }

        printf("attr_string: %d/6 checks passed\n", checks);
        return (checks == 6) ? 0 : 1;
    }
}
