// json_query — NSJSONSerialization round-trip + nested access
// Exercises: NSJSONSerialization, NSData, NSDictionary, NSArray, NSNumber,
//            NSString, NSError, valueForKeyPath:
#import <Foundation/Foundation.h>

int main(void) {
    @autoreleasepool {
        printf("json_query: starting\n");

        // 1. Build a simple dict first
        NSDictionary *simple = @{@"name": @"test", @"version": @(2)};
        printf("json_query: dict created with %lu keys\n",
               (unsigned long)[simple count]);

        // 2. Serialize to JSON
        NSError *err = nil;
        NSData *jsonData = [NSJSONSerialization dataWithJSONObject:simple
                                                          options:0
                                                            error:&err];
        if (jsonData && !err) {
            printf("CHECK 1 PASS: serialized to JSON (%lu bytes)\n",
                   (unsigned long)[jsonData length]);
        } else {
            printf("CHECK 1 FAIL: serialization error\n");
            return 1;
        }

        // 3. Deserialize back
        id parsed = [NSJSONSerialization JSONObjectWithData:jsonData
                                                    options:0
                                                      error:&err];
        if ([parsed isKindOfClass:[NSDictionary class]] && !err) {
            printf("CHECK 2 PASS: deserialized back to NSDictionary\n");
        } else {
            printf("CHECK 2 FAIL: deserialization error\n");
            return 1;
        }

        // 4. Access top-level string
        NSString *name = parsed[@"name"];
        if ([name isEqualToString:@"test"]) {
            printf("CHECK 3 PASS: name = %s\n", [name UTF8String]);
        } else {
            printf("CHECK 3 FAIL: name mismatch\n");
            return 1;
        }

        // 5. Access number
        NSNumber *version = parsed[@"version"];
        if ([version integerValue] == 2) {
            printf("CHECK 4 PASS: version = 2\n");
        } else {
            printf("CHECK 4 FAIL: version = %ld\n", (long)[version integerValue]);
            return 1;
        }

        // 6. Nested structure
        NSDictionary *nested = @{
            @"outer": @{@"inner": @"value"},
            @"list": @[@"a", @"b", @"c"]
        };
        NSData *nestedJson = [NSJSONSerialization dataWithJSONObject:nested
                                                            options:0
                                                              error:&err];
        if (nestedJson && !err) {
            printf("CHECK 5 PASS: nested serialized (%lu bytes)\n",
                   (unsigned long)[nestedJson length]);
        } else {
            printf("CHECK 5 FAIL: nested serialization error\n");
            return 1;
        }

        // 7. valueForKeyPath on nested dict
        NSString *innerVal = [nested valueForKeyPath:@"outer.inner"];
        if ([innerVal isEqualToString:@"value"]) {
            printf("CHECK 6 PASS: keypath access works\n");
        } else {
            printf("CHECK 6 FAIL: keypath value wrong\n");
            return 1;
        }

        // 8. Boolean round-trip
        NSDictionary *boolDict = @{@"flag": @YES};
        NSData *boolJson = [NSJSONSerialization dataWithJSONObject:boolDict
                                                           options:0
                                                             error:&err];
        id boolParsed = [NSJSONSerialization JSONObjectWithData:boolJson
                                                        options:0
                                                          error:&err];
        if ([boolParsed[@"flag"] boolValue] == YES) {
            printf("CHECK 7 PASS: boolean round-trip\n");
        } else {
            printf("CHECK 7 FAIL: boolean round-trip failed\n");
            return 1;
        }

        // 9. Pretty print produces larger output
        NSData *pretty = [NSJSONSerialization dataWithJSONObject:nested
                                                        options:NSJSONWritingPrettyPrinted
                                                          error:&err];
        if (pretty && [pretty length] > [nestedJson length]) {
            printf("CHECK 8 PASS: pretty JSON larger (%lu > %lu)\n",
                   (unsigned long)[pretty length],
                   (unsigned long)[nestedJson length]);
        } else {
            printf("CHECK 8 FAIL: pretty print failed\n");
            return 1;
        }
    }
    return 0;
}
