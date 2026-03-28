// url_builder — URL construction and component manipulation
// Exercises: NSURLComponents, NSURLQueryItem, NSURL
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // Parse a URL
        NSURLComponents *comp = [NSURLComponents componentsWithString:@"https://example.com:8080/path?key=value#frag"];
        if ([[comp scheme] isEqualToString:@"https"]) { printf("CHECK 1 PASS: scheme = https\n"); checks++; }
        if ([[comp host] isEqualToString:@"example.com"]) { printf("CHECK 2 PASS: host = example.com\n"); checks++; }
        if ([[comp path] isEqualToString:@"/path"]) { printf("CHECK 3 PASS: path = /path\n"); checks++; }
        if ([[comp query] isEqualToString:@"key=value"]) { printf("CHECK 4 PASS: query = key=value\n"); checks++; }
        if ([[comp fragment] isEqualToString:@"frag"]) { printf("CHECK 5 PASS: fragment = frag\n"); checks++; }

        // Build a URL from scratch
        NSURLComponents *builder = [NSURLComponents new];
        [builder setScheme:@"http"];
        [builder setHost:@"api.test.io"];
        [builder setPath:@"/v1/items"];
        NSURLQueryItem *q1 = [NSURLQueryItem queryItemWithName:@"page" value:@"1"];
        NSURLQueryItem *q2 = [NSURLQueryItem queryItemWithName:@"limit" value:@"10"];
        [builder setQueryItems:@[q1, q2]];
        NSString *built = [builder string];
        if (built != nil && [built length] > 0) { printf("CHECK 6 PASS: built URL = %s\n", [built UTF8String]); checks++; }

        // Query item accessors
        if ([[q1 name] isEqualToString:@"page"]) { printf("CHECK 7 PASS: q1.name = page\n"); checks++; }
        if ([[q1 value] isEqualToString:@"1"]) { printf("CHECK 8 PASS: q1.value = 1\n"); checks++; }

        printf("url_builder: %d/8 checks passed\n", checks);
        return (checks == 8) ? 0 : 1;
    }
}
