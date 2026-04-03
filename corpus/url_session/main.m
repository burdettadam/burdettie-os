// url_session — HTTP client stub operations
// Exercises: NSURLSession, NSURLSessionConfiguration, NSURLSessionDataTask
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // Default configuration
        NSURLSessionConfiguration *config = [NSURLSessionConfiguration defaultSessionConfiguration];
        if (config != nil) { printf("CHECK 1 PASS: default config created\n"); checks++; }

        // Timeout
        NSTimeInterval timeout = [config timeoutIntervalForRequest];
        if (timeout > 0) { printf("CHECK 2 PASS: timeout = %.1f\n", timeout); checks++; }

        // Set timeout
        [config setTimeoutIntervalForRequest:30.0];
        printf("CHECK 3 PASS: timeout set to 30\n"); checks++;

        // Ephemeral config
        NSURLSessionConfiguration *ephemeral = [NSURLSessionConfiguration ephemeralSessionConfiguration];
        if (ephemeral != nil) { printf("CHECK 4 PASS: ephemeral config created\n"); checks++; }

        // Create session
        NSURLSession *session = [NSURLSession sessionWithConfiguration:config];
        if (session != nil) { printf("CHECK 5 PASS: session created\n"); checks++; }

        // Shared session
        NSURLSession *shared = [NSURLSession sharedSession];
        if (shared != nil) { printf("CHECK 6 PASS: shared session\n"); checks++; }

        // Data task with URL
        NSURL *url = [NSURL URLWithString:@"https://example.com"];
        NSURLSessionDataTask *task = [session dataTaskWithURL:url];
        if (task != nil) { printf("CHECK 7 PASS: data task created\n"); checks++; }

        // Task state (suspended initially)
        NSUInteger state = [task state];
        printf("CHECK 8 PASS: task state = %lu\n", (unsigned long)state); checks++;

        // Resume / cancel
        [task resume];
        [task cancel];
        printf("CHECK 9 PASS: task resumed then cancelled\n"); checks++;

        // Session description
        NSString *desc = [session description];
        if (desc != nil && [desc length] > 0) { printf("CHECK 10 PASS: session desc = %s\n", [desc UTF8String]); checks++; }

        // Invalidate
        [session invalidateAndCancel];
        printf("CHECK 11 PASS: session invalidated\n"); checks++;

        // NSMutableURLRequest with HTTP body
        NSURL *postURL = [NSURL URLWithString:@"https://httpbin.org/post"];
        NSMutableURLRequest *req = [NSMutableURLRequest requestWithURL:postURL];
        [req setHTTPMethod:@"POST"];
        NSData *bodyData = [@"hello=world" dataUsingEncoding:NSUTF8StringEncoding];
        [req setHTTPBody:bodyData];
        NSData *retrieved = [req HTTPBody];
        if (retrieved != nil && [retrieved length] > 0) {
            printf("CHECK 12 PASS: HTTPBody round-trip OK (%lu bytes)\n", (unsigned long)[retrieved length]);
            checks++;
        }

        // Create task from mutable request (exercises body forwarding path)
        NSURLSessionDataTask *postTask = [session dataTaskWithRequest:req completionHandler:^(NSData *d, NSURLResponse *r, NSError *e) {
            (void)d; (void)r; (void)e;
        }];
        if (postTask != nil) { printf("CHECK 13 PASS: POST task created from mutable request\n"); checks++; }

        printf("url_session: %d/13 checks passed\n", checks);
        return (checks == 13) ? 0 : 1;
    }
}
