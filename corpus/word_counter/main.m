// word_counter — counts words, lines, and characters in hardcoded text
// Exercises: NSString, NSArray, NSNumber, NSMutableDictionary, for-loop iteration
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        NSArray *lines = @[
            @"The quick brown fox jumps over the lazy dog",
            @"Pack my box with five dozen liquor jugs",
            @"How vexingly quick daft zebras jump",
            @"The five boxing wizards jump quickly",
            @"Sphinx of black quartz judge my vow"
        ];

        NSMutableDictionary *stats = [NSMutableDictionary dictionary];
        NSInteger totalWords = 0;
        NSInteger totalChars = 0;

        for (NSUInteger i = 0; i < [lines count]; i++) {
            NSString *line = [lines objectAtIndex:i];
            NSArray *words = [line componentsSeparatedByString:@" "];
            NSInteger wordCount = [words count];
            totalWords += wordCount;
            totalChars += [line length];

            NSString *key = [NSString stringWithFormat:@"line_%lu", (unsigned long)i];
            [stats setObject:[NSNumber numberWithInt:(int)wordCount] forKey:key];
        }

        NSInteger lineCount = [lines count];

        // Verify results
        int checks = 0;
        if (lineCount == 5) { printf("CHECK 1 PASS: line count = 5\n"); checks++; }
        if (totalWords == 36) { printf("CHECK 2 PASS: total words = 36\n"); checks++; }
        if (totalChars > 0) { printf("CHECK 3 PASS: total chars = %ld\n", (long)totalChars); checks++; }
        if ([[stats objectForKey:@"line_0"] intValue] == 9) { printf("CHECK 4 PASS: line 0 has 9 words\n"); checks++; }
        if ([stats count] == 5) { printf("CHECK 5 PASS: stats has 5 entries\n"); checks++; }

        // Average words per line
        double avg = (double)totalWords / (double)lineCount;
        if (avg > 7.0 && avg < 7.5) { printf("CHECK 6 PASS: avg words/line = %.1f\n", avg); checks++; }

        printf("word_counter: %d/6 checks passed\n", checks);
        return (checks == 6) ? 0 : 1;
    }
}
