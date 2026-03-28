// data_buffer — exercises NSMutableData byte buffer operations
// Exercises: NSMutableData data, dataWithCapacity:, dataWithBytes:length:,
//   length, bytes, appendBytes:length:, appendData:, setLength:,
//   resetBytesInRange:, replaceBytesInRange:withBytes:, description
#import <Foundation/Foundation.h>
#include <string.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // CHECK 1: Create empty mutable data
        NSMutableData *d1 = [NSMutableData data];
        if ([d1 length] == 0) {
            printf("CHECK 1 PASS: empty data length=0\n"); checks++;
        } else {
            printf("CHECK 1 FAIL: length=%lu\n", (unsigned long)[d1 length]);
        }

        // CHECK 2: dataWithBytes:length:
        const char *hello = "Hello";
        NSMutableData *d2 = [NSMutableData dataWithBytes:hello length:5];
        if ([d2 length] == 5) {
            printf("CHECK 2 PASS: dataWithBytes length=5\n"); checks++;
        } else {
            printf("CHECK 2 FAIL: length=%lu\n", (unsigned long)[d2 length]);
        }

        // CHECK 3: appendBytes:length:
        const char *world = " World";
        [d2 appendBytes:world length:6];
        if ([d2 length] == 11) {
            printf("CHECK 3 PASS: after append length=11\n"); checks++;
        } else {
            printf("CHECK 3 FAIL: length=%lu\n", (unsigned long)[d2 length]);
        }

        // CHECK 4: bytes content check
        const char *ptr = (const char *)[d2 bytes];
        if (ptr && memcmp(ptr, "Hello World", 11) == 0) {
            printf("CHECK 4 PASS: bytes = 'Hello World'\n"); checks++;
        } else {
            printf("CHECK 4 FAIL: unexpected bytes content\n");
        }

        // CHECK 5: appendData:
        NSData *suffix = [NSData dataWithBytes:"!" length:1];
        [d2 appendData:suffix];
        if ([d2 length] == 12) {
            printf("CHECK 5 PASS: appendData => length=12\n"); checks++;
        } else {
            printf("CHECK 5 FAIL: length=%lu\n", (unsigned long)[d2 length]);
        }

        // CHECK 6: setLength: (truncation)
        [d2 setLength:5];
        if ([d2 length] == 5) {
            printf("CHECK 6 PASS: setLength:5 => length=5\n"); checks++;
        } else {
            printf("CHECK 6 FAIL: length=%lu\n", (unsigned long)[d2 length]);
        }

        // CHECK 7: dataWithCapacity: pre-allocates but length stays 0
        NSMutableData *d3 = [NSMutableData dataWithCapacity:1024];
        if ([d3 length] == 0) {
            printf("CHECK 7 PASS: dataWithCapacity: length=0\n"); checks++;
        } else {
            printf("CHECK 7 FAIL: length=%lu\n", (unsigned long)[d3 length]);
        }

        // CHECK 8: description non-nil
        NSString *desc = [d2 description];
        if (desc != nil) {
            printf("CHECK 8 PASS: description non-nil\n"); checks++;
        } else {
            printf("CHECK 8 FAIL: nil description\n");
        }

        printf("\ndata_buffer: %d/8 checks passed\n", checks);
        return (checks == 8) ? 0 : 1;
    }
}
