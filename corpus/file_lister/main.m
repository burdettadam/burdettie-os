// file_lister — lists directory contents with metadata
// Exercises: NSFileManager, NSString path utilities, NSArray, NSError, NSMutableArray, sorting blocks
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        NSFileManager *fm = [NSFileManager defaultManager];
        NSString *cwd = [fm currentDirectoryPath];

        int checks = 0;

        // Check current directory exists
        if ([cwd length] > 0) { printf("CHECK 1 PASS: cwd = %s\n", [cwd UTF8String]); checks++; }

        // Check cwd is absolute
        if ([cwd hasPrefix:@"/"]) { printf("CHECK 2 PASS: cwd is absolute\n"); checks++; }

        // List directory
        NSError *error = nil;
        NSArray *contents = [fm contentsOfDirectoryAtPath:cwd error:&error];
        if (contents != nil && error == nil) { printf("CHECK 3 PASS: listed %lu entries\n", (unsigned long)[contents count]); checks++; }

        // Path component checks
        NSString *testPath = @"/usr/local/bin/test.txt";
        if ([[testPath lastPathComponent] isEqualToString:@"test.txt"]) { printf("CHECK 4 PASS: lastPathComponent\n"); checks++; }
        if ([[testPath pathExtension] isEqualToString:@"txt"]) { printf("CHECK 5 PASS: pathExtension\n"); checks++; }
        if ([[testPath stringByDeletingLastPathComponent] isEqualToString:@"/usr/local/bin"]) { printf("CHECK 6 PASS: stringByDeletingLastPathComponent\n"); checks++; }

        // Build a filtered list of items starting with '.'
        NSMutableArray *hidden = [NSMutableArray array];
        for (NSUInteger i = 0; i < [contents count] && i < 100; i++) {
            NSString *item = [contents objectAtIndex:i];
            if ([item hasPrefix:@"."]) {
                [hidden addObject:item];
            }
        }

        // Sort alphabetically
        NSArray *sorted = [hidden sortedArrayUsingComparator:^NSComparisonResult(NSString *a, NSString *b) {
            return [a compare:b];
        }];

        if ([sorted count] <= [contents count]) { printf("CHECK 7 PASS: hidden files filtered (%lu items)\n", (unsigned long)[sorted count]); checks++; }

        // File existence
        if ([fm fileExistsAtPath:cwd]) { printf("CHECK 8 PASS: cwd exists\n"); checks++; }

        printf("file_lister: %d/8 checks passed\n", checks);
        return (checks == 8) ? 0 : 1;
    }
}
