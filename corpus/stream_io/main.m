// stream_io — stream-based I/O operations
// Exercises: NSStream, NSInputStream, NSOutputStream
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // Write to memory output stream
        NSOutputStream *os = [NSOutputStream outputStreamToMemory];
        if (os != nil) { printf("CHECK 1 PASS: output stream created\n"); checks++; }

        [os open];
        NSUInteger status = [os streamStatus];
        if (status == NSStreamStatusOpen) { printf("CHECK 2 PASS: output stream opened (status=%lu)\n", (unsigned long)status); checks++; }

        const uint8_t bytes[] = "Hello, Streams!";
        NSInteger written = [os write:bytes maxLength:15];
        if (written == 15) { printf("CHECK 3 PASS: wrote %ld bytes\n", (long)written); checks++; }

        if ([os hasSpaceAvailable]) { printf("CHECK 4 PASS: has space available\n"); checks++; }

        // Read memory contents
        NSData *outputData = [os propertyForKey:@"NSStreamDataWrittenToMemoryStreamKey"];
        if (outputData != nil && [outputData length] == 15) {
            printf("CHECK 5 PASS: output data length = %lu\n", (unsigned long)[outputData length]);
            checks++;
        }

        [os close];

        // Read from input stream backed by NSData
        NSData *inputData = [@"Stream test data" dataUsingEncoding:NSUTF8StringEncoding];
        NSInputStream *is = [NSInputStream inputStreamWithData:inputData];
        if (is != nil) { printf("CHECK 6 PASS: input stream created\n"); checks++; }

        [is open];
        if ([is hasBytesAvailable]) { printf("CHECK 7 PASS: has bytes available\n"); checks++; }

        uint8_t buf[64];
        NSInteger nread = [is read:buf maxLength:64];
        if (nread > 0) { printf("CHECK 8 PASS: read %ld bytes\n", (long)nread); checks++; }

        [is close];

        // Stream description
        NSString *desc = [os description];
        if (desc != nil && [desc length] > 0) { printf("CHECK 9 PASS: description = %s\n", [desc UTF8String]); checks++; }

        printf("stream_io: %d/9 checks passed\n", checks);
        return (checks == 9) ? 0 : 1;
    }
}
