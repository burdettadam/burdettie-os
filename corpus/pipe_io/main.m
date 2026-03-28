// pipe_io — pipe creation and URL request building
// Exercises: NSPipe, NSFileHandle, NSURLRequest, NSMutableURLRequest
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // Create pipe
        NSPipe *pipe = [NSPipe pipe];
        if (pipe != nil) { printf("CHECK 1 PASS: pipe created\n"); checks++; }

        // Get file handles
        NSFileHandle *reader = [pipe fileHandleForReading];
        NSFileHandle *writer = [pipe fileHandleForWriting];
        if (reader != nil) { printf("CHECK 2 PASS: reader handle exists\n"); checks++; }
        if (writer != nil) { printf("CHECK 3 PASS: writer handle exists\n"); checks++; }

        // Pipe description
        NSString *pdesc = [pipe description];
        if (pdesc != nil && [pdesc length] > 0) { printf("CHECK 4 PASS: pipe desc = %s\n", [pdesc UTF8String]); checks++; }

        // NSURLRequest
        NSURL *url = [NSURL URLWithString:@"https://example.com/api"];
        NSURLRequest *req = [NSURLRequest requestWithURL:url];
        if ([[req HTTPMethod] isEqualToString:@"GET"]) { printf("CHECK 5 PASS: default method = GET\n"); checks++; }
        if ([req URL] != nil) { printf("CHECK 6 PASS: request URL set\n"); checks++; }

        // NSMutableURLRequest
        NSMutableURLRequest *mreq = [NSMutableURLRequest requestWithURL:url];
        [mreq setHTTPMethod:@"POST"];
        [mreq setValue:@"application/json" forHTTPHeaderField:@"Content-Type"];
        NSString *desc = [req description];
        if (desc != nil && [desc length] > 0) { printf("CHECK 7 PASS: request description = %s\n", [desc UTF8String]); checks++; }

        printf("pipe_io: %d/7 checks passed\n", checks);
        return (checks == 7) ? 0 : 1;
    }
}
