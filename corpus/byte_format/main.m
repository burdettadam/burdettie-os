// byte_format — byte count formatting
// Exercises: NSByteCountFormatter
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // Class method: decimal style (default)
        NSString *s1 = [NSByteCountFormatter stringFromByteCount:1500 countStyle:NSByteCountFormatterCountStyleDecimal];
        if (s1 != nil && [s1 length] > 0) { printf("CHECK 1 PASS: 1500 bytes = %s\n", [s1 UTF8String]); checks++; }

        // Large value
        NSString *s2 = [NSByteCountFormatter stringFromByteCount:1073741824 countStyle:NSByteCountFormatterCountStyleDecimal];
        if (s2 != nil && [s2 length] > 0) { printf("CHECK 2 PASS: 1GB = %s\n", [s2 UTF8String]); checks++; }

        // Instance with binary style
        NSByteCountFormatter *fmt = [[NSByteCountFormatter alloc] init];
        [fmt setCountStyle:NSByteCountFormatterCountStyleBinary];
        NSString *s3 = [fmt stringFromByteCount:1048576];
        if (s3 != nil && [s3 length] > 0) { printf("CHECK 3 PASS: 1MiB = %s\n", [s3 UTF8String]); checks++; }

        // Zero bytes
        NSString *s4 = [NSByteCountFormatter stringFromByteCount:0 countStyle:NSByteCountFormatterCountStyleDecimal];
        if (s4 != nil) { printf("CHECK 4 PASS: 0 bytes = %s\n", [s4 UTF8String]); checks++; }

        // countStyle getter
        if ([fmt countStyle] == NSByteCountFormatterCountStyleBinary) { printf("CHECK 5 PASS: countStyle = binary\n"); checks++; }

        // Description
        NSString *desc = [fmt description];
        if (desc != nil && [desc length] > 0) { printf("CHECK 6 PASS: description = %s\n", [desc UTF8String]); checks++; }

        printf("byte_format: %d/6 checks passed\n", checks);
        return (checks == 6) ? 0 : 1;
    }
}
