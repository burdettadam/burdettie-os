// error_reporter — error handling and reporting patterns
// Exercises: NSError, NSException, NSMutableArray, NSDictionary, NSString formatting
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // Create errors
        NSDictionary *info1 = @{@"NSLocalizedDescription": @"File not found"};
        NSError *fileError = [NSError errorWithDomain:@"FileErrorDomain" code:404 userInfo:info1];
        if ([fileError code] == 404) { printf("CHECK 1 PASS: error code = 404\n"); checks++; }
        if ([[fileError domain] isEqualToString:@"FileErrorDomain"]) { printf("CHECK 2 PASS: error domain\n"); checks++; }

        NSDictionary *info2 = @{@"NSLocalizedDescription": @"Connection refused"};
        NSError *netError = [NSError errorWithDomain:@"NetworkErrorDomain" code:503 userInfo:info2];

        // Collect errors
        NSMutableArray *errors = [NSMutableArray array];
        [errors addObject:fileError];
        [errors addObject:netError];

        if ([errors count] == 2) { printf("CHECK 3 PASS: 2 errors collected\n"); checks++; }

        // Format error report
        NSMutableString *report = [NSMutableString string];
        [errors enumerateObjectsUsingBlock:^(NSError *err, NSUInteger idx, BOOL *stop) {
            NSString *line = [NSString stringWithFormat:@"Error %lu: [%@] %ld - %@\n",
                (unsigned long)idx,
                [err domain],
                (long)[err code],
                [err localizedDescription]];
            [report appendString:line];
        }];

        if ([report length] > 0) { printf("CHECK 4 PASS: report generated (%lu chars)\n", (unsigned long)[report length]); checks++; }
        if ([report rangeOfString:@"404"].location != NSNotFound) { printf("CHECK 5 PASS: report contains 404\n"); checks++; }

        // Exception creation (not throwing)
        NSException *ex = [NSException exceptionWithName:@"ValidationError"
                                                 reason:@"Invalid input: expected integer"
                                               userInfo:nil];
        if ([[ex name] isEqualToString:@"ValidationError"]) { printf("CHECK 6 PASS: exception name\n"); checks++; }
        if ([[ex reason] hasPrefix:@"Invalid"]) { printf("CHECK 7 PASS: exception reason\n"); checks++; }

        // Error with no userInfo
        NSError *simpleError = [NSError errorWithDomain:@"AppDomain" code:1 userInfo:nil];
        if ([simpleError code] == 1) { printf("CHECK 8 PASS: simple error code\n"); checks++; }

        printf("error_reporter: %d/8 checks passed\n", checks);
        return (checks == 8) ? 0 : 1;
    }
}
