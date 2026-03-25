// path_resolver — file path manipulation utility
// Exercises: NSString path methods, NSURL, NSFileManager, NSMutableArray
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // Path manipulation
        NSString *path = @"/Users/alice/Documents/project/src/main.swift";
        if ([[path lastPathComponent] isEqualToString:@"main.swift"]) { printf("CHECK 1 PASS: lastPathComponent\n"); checks++; }
        if ([[path pathExtension] isEqualToString:@"swift"]) { printf("CHECK 2 PASS: pathExtension\n"); checks++; }
        if ([[path stringByDeletingPathExtension] hasSuffix:@"main"]) { printf("CHECK 3 PASS: stringByDeletingPathExtension\n"); checks++; }
        if ([[path stringByDeletingLastPathComponent] hasSuffix:@"src"]) { printf("CHECK 4 PASS: stringByDeletingLastPathComponent\n"); checks++; }

        // Build paths
        NSString *base = @"/usr/local";
        NSString *full = [base stringByAppendingPathComponent:@"bin"];
        if ([full isEqualToString:@"/usr/local/bin"]) { printf("CHECK 5 PASS: appendPathComponent\n"); checks++; }

        // NSURL
        NSURL *fileUrl = [NSURL fileURLWithPath:@"/tmp/test.txt"];
        if ([fileUrl isFileURL]) { printf("CHECK 6 PASS: isFileURL\n"); checks++; }
        if ([[fileUrl lastPathComponent] isEqualToString:@"test.txt"]) { printf("CHECK 7 PASS: URL lastPathComponent\n"); checks++; }
        if ([[fileUrl pathExtension] isEqualToString:@"txt"]) { printf("CHECK 8 PASS: URL pathExtension\n"); checks++; }

        NSURL *webUrl = [NSURL URLWithString:@"https://example.com/api/v1"];
        if ([[webUrl scheme] isEqualToString:@"https"]) { printf("CHECK 9 PASS: URL scheme\n"); checks++; }

        // Path is absolute
        if ([path isAbsolutePath]) { printf("CHECK 10 PASS: isAbsolutePath\n"); checks++; }

        // Relative path is not absolute
        NSString *rel = @"src/main.swift";
        if (![rel isAbsolutePath]) { printf("CHECK 11 PASS: relative not absolute\n"); checks++; }

        // Collect path components manually
        NSArray *parts = [path componentsSeparatedByString:@"/"];
        // parts: ["", "Users", "alice", "Documents", "project", "src", "main.swift"]
        if ([parts count] == 7) { printf("CHECK 12 PASS: 7 path parts\n"); checks++; }

        printf("path_resolver: %d/12 checks passed\n", checks);
        return (checks == 12) ? 0 : 1;
    }
}
