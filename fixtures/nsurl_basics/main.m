#import <Foundation/Foundation.h>
#include <stdio.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;
        int passed = 0;

        // Check 1: fileURLWithPath
        NSURL *fileURL = [NSURL fileURLWithPath:@"/tmp/test.txt"];
        checks++; if (fileURL != nil) { passed++; printf("CHECK 1 PASSED: fileURLWithPath non-nil\n"); }
        else printf("CHECK 1 FAILED\n");

        // Check 2: isFileURL
        checks++; if ([fileURL isFileURL]) { passed++; printf("CHECK 2 PASSED: isFileURL\n"); }
        else printf("CHECK 2 FAILED\n");

        // Check 3: path
        NSString *path = [fileURL path];
        checks++; if ([path isEqualToString:@"/tmp/test.txt"]) { passed++; printf("CHECK 3 PASSED: path == /tmp/test.txt\n"); }
        else printf("CHECK 3 FAILED: path=%s\n", [path UTF8String]);

        // Check 4: lastPathComponent
        NSString *last = [fileURL lastPathComponent];
        checks++; if ([last isEqualToString:@"test.txt"]) { passed++; printf("CHECK 4 PASSED: lastPathComponent == test.txt\n"); }
        else printf("CHECK 4 FAILED: last=%s\n", [last UTF8String]);

        // Check 5: pathExtension
        NSString *ext = [fileURL pathExtension];
        checks++; if ([ext isEqualToString:@"txt"]) { passed++; printf("CHECK 5 PASSED: pathExtension == txt\n"); }
        else printf("CHECK 5 FAILED: ext=%s\n", [ext UTF8String]);

        // Check 6: URLByAppendingPathComponent
        NSURL *dir = [NSURL fileURLWithPath:@"/tmp"];
        NSURL *appended = [dir URLByAppendingPathComponent:@"foo.log"];
        NSString *appendedPath = [appended path];
        checks++; if ([appendedPath isEqualToString:@"/tmp/foo.log"]) { passed++; printf("CHECK 6 PASSED: appended path == /tmp/foo.log\n"); }
        else printf("CHECK 6 FAILED: path=%s\n", [appendedPath UTF8String]);

        // Check 7: URLWithString
        NSURL *webURL = [NSURL URLWithString:@"https://example.com/page"];
        checks++; if (webURL != nil) { passed++; printf("CHECK 7 PASSED: URLWithString non-nil\n"); }
        else printf("CHECK 7 FAILED\n");

        // Check 8: scheme for file URL
        NSString *scheme = [fileURL scheme];
        checks++; if ([scheme isEqualToString:@"file"]) { passed++; printf("CHECK 8 PASSED: scheme == file\n"); }
        else printf("CHECK 8 FAILED: scheme=%s\n", [scheme UTF8String]);

        // Check 9: scheme for https URL
        NSString *webScheme = [webURL scheme];
        checks++; if ([webScheme isEqualToString:@"https"]) { passed++; printf("CHECK 9 PASSED: scheme == https\n"); }
        else printf("CHECK 9 FAILED: scheme=%s\n", [webScheme UTF8String]);

        // Check 10: absoluteString
        NSString *absStr = [fileURL absoluteString];
        checks++; if ([absStr isEqualToString:@"file:///tmp/test.txt"]) { passed++; printf("CHECK 10 PASSED: absoluteString correct\n"); }
        else printf("CHECK 10 FAILED: abs=%s\n", [absStr UTF8String]);

        printf("\n=== %d/%d checks passed ===\n", passed, checks);
    }
    return 0;
}
