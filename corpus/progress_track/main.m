// progress_track — progress reporting
// Exercises: NSProgress
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // Create progress with total
        NSProgress *p = [NSProgress progressWithTotalUnitCount:100];
        if ([p totalUnitCount] == 100) { printf("CHECK 1 PASS: totalUnitCount = 100\n"); checks++; }

        // Completed count
        [p setCompletedUnitCount:50];
        if ([p completedUnitCount] == 50) { printf("CHECK 2 PASS: completedUnitCount = 50\n"); checks++; }

        // Fraction completed should be 0.5
        double frac = [p fractionCompleted];
        if (frac >= 0.49 && frac <= 0.51) { printf("CHECK 3 PASS: fractionCompleted = %.2f\n", frac); checks++; }

        // Cancellable
        [p setCancellable:YES];
        if ([p isCancellable]) { printf("CHECK 4 PASS: isCancellable = YES\n"); checks++; }

        // Cancel
        [p cancel];
        if ([p isCancelled]) { printf("CHECK 5 PASS: isCancelled after cancel\n"); checks++; }

        // Localized description
        NSString *desc = [p localizedDescription];
        if (desc != nil) { printf("CHECK 6 PASS: localizedDescription = %s\n", [desc UTF8String]); checks++; }

        // Discrete progress
        NSProgress *dp = [NSProgress discreteProgressWithTotalUnitCount:10];
        if ([dp totalUnitCount] == 10) { printf("CHECK 7 PASS: discrete totalUnitCount = 10\n"); checks++; }

        printf("progress_track: %d/7 checks passed\n", checks);
        return (checks == 7) ? 0 : 1;
    }
}
