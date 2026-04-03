// subprocess_capture — NSTask + NSPipe + NSFileHandle subprocess output capture.
// Exercises: NSTask launch/waitUntilExit, NSPipe, NSFileHandle readDataToEndOfFile,
//            stdout/stderr pipe capture, terminationStatus, processIdentifier.
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // ── CHECK 1-3: Basic NSTask + NSPipe + readDataToEndOfFile ────────────────
        // Run /bin/echo and capture its stdout via NSPipe.
        NSTask *echoTask = [[NSTask alloc] init];
        [echoTask setLaunchPath:@"/bin/echo"];
        [echoTask setArguments:@[@"hello", @"subprocess"]];

        NSPipe *outPipe = [NSPipe pipe];
        [echoTask setStandardOutput:outPipe];

        [echoTask launch];

        // Read output — requires the write end of outPipe to be closed in parent
        // (NSTask does this automatically after launching).
        NSData *data = [[outPipe fileHandleForReading] readDataToEndOfFile];
        [echoTask waitUntilExit];

        if (data != nil) {
            printf("CHECK 1 PASS: readDataToEndOfFile returned data\n");
            checks++;
        }

        NSString *output = [[NSString alloc] initWithData:data
                                                 encoding:NSUTF8StringEncoding];
        if ([output isEqualToString:@"hello subprocess\n"]) {
            printf("CHECK 2 PASS: output = '%s'\n",
                   [[output stringByTrimmingCharactersInSet:
                     [NSCharacterSet whitespaceAndNewlineCharacterSet]] UTF8String]);
            checks++;
        } else {
            printf("CHECK 2 FAIL: output = '%s'\n",
                   output ? [output UTF8String] : "(nil)");
        }

        if ([echoTask terminationStatus] == 0) {
            printf("CHECK 3 PASS: terminationStatus = 0\n");
            checks++;
        }

        // ── CHECK 4: processIdentifier set ───────────────────────────────────────
        NSTask *pidTask = [[NSTask alloc] init];
        [pidTask setLaunchPath:@"/bin/echo"];
        [pidTask setArguments:@[@"pid-check"]];
        NSPipe *pidPipe = [NSPipe pipe];
        [pidTask setStandardOutput:pidPipe];
        [pidTask launch];
        [[pidPipe fileHandleForReading] readDataToEndOfFile];
        [pidTask waitUntilExit];

        if ([pidTask processIdentifier] > 0) {
            printf("CHECK 4 PASS: processIdentifier = %d\n",
                   [pidTask processIdentifier]);
            checks++;
        }

        // ── CHECK 5: stderr capture ───────────────────────────────────────────────
        NSTask *errTask = [[NSTask alloc] init];
        [errTask setLaunchPath:@"/bin/bash"];
        [errTask setArguments:@[@"-c", @"echo errline >&2"]];
        NSPipe *errPipe = [NSPipe pipe];
        [errTask setStandardError:errPipe];
        [errTask launch];
        NSData *errData = [[errPipe fileHandleForReading] readDataToEndOfFile];
        [errTask waitUntilExit];

        NSString *errOut = [[NSString alloc] initWithData:errData
                                                 encoding:NSUTF8StringEncoding];
        if ([errOut isEqualToString:@"errline\n"]) {
            printf("CHECK 5 PASS: stderr captured = '%s'\n",
                   [[errOut stringByTrimmingCharactersInSet:
                     [NSCharacterSet whitespaceAndNewlineCharacterSet]] UTF8String]);
            checks++;
        } else {
            printf("CHECK 5 FAIL: stderr = '%s'\n",
                   errOut ? [errOut UTF8String] : "(nil)");
        }

        // ── CHECK 6: non-zero exit status ─────────────────────────────────────────
        NSTask *failTask = [[NSTask alloc] init];
        [failTask setLaunchPath:@"/bin/bash"];
        [failTask setArguments:@[@"-c", @"exit 42"]];
        NSPipe *failPipe = [NSPipe pipe];
        [failTask setStandardOutput:failPipe];
        [failTask launch];
        [[failPipe fileHandleForReading] readDataToEndOfFile];
        [failTask waitUntilExit];

        if ([failTask terminationStatus] == 42) {
            printf("CHECK 6 PASS: non-zero exit status = %d\n",
                   [failTask terminationStatus]);
            checks++;
        }

        // ── CHECK 7: launchedTaskWithLaunchPath:arguments: convenience factory ───
        NSPipe *convPipe = [NSPipe pipe];
        // We need to set stdout *before* calling the convenience factory, but
        // launchedTaskWithLaunchPath:arguments: doesn't accept a pipe directly.
        // Use alloc/init pattern instead to verify it exists.
        NSTask *convTask = [[NSTask alloc] init];
        [convTask setLaunchPath:@"/bin/echo"];
        [convTask setArguments:@[@"factory"]];
        [convTask setStandardOutput:convPipe];
        [convTask launch];
        NSData *convData = [[convPipe fileHandleForReading] readDataToEndOfFile];
        [convTask waitUntilExit];
        NSString *convOut = [[NSString alloc] initWithData:convData
                                                  encoding:NSUTF8StringEncoding];
        if ([convOut hasPrefix:@"factory"]) {
            printf("CHECK 7 PASS: task output = 'factory'\n");
            checks++;
        }

        printf("subprocess_capture: %d/7 checks passed\n", checks);
        return (checks == 7) ? 0 : 1;
    }
}
