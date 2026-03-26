// plist_converter — NSPropertyListSerialization round-trip
// Exercises: NSPropertyListSerialization, NSData, NSString, NSDictionary,
//            NSArray, NSNumber, NSDate, plist formats
#import <Foundation/Foundation.h>

int main(void) {
    @autoreleasepool {
        // 1. Build a rich plist-compatible dict
        NSDictionary *config = @{
            @"AppName": @"DarwinLoader",
            @"Version": @(1),
            @"Debug": @NO,
            @"MaxRetries": @(5),
            @"Features": @[@"macho", @"objc", @"foundation"],
            @"Database": @{
                @"Host": @"localhost",
                @"Port": @(5432)
            }
        };

        // 2. Serialize to XML plist
        NSError *err = nil;
        NSData *xmlData = [NSPropertyListSerialization
            dataWithPropertyList:config
                          format:NSPropertyListXMLFormat_v1_0
                         options:0
                           error:&err];
        if (xmlData && !err) {
            printf("CHECK 1 PASS: serialized to XML plist (%lu bytes)\n",
                   (unsigned long)[xmlData length]);
        } else {
            printf("CHECK 1 FAIL: XML serialization error\n");
            return 1;
        }

        // 3. Verify XML contains expected marker
        NSString *xmlStr = [[NSString alloc] initWithData:xmlData
                                                 encoding:NSUTF8StringEncoding];
        if ([xmlStr containsString:@"<plist"]) {
            printf("CHECK 2 PASS: XML contains <plist marker\n");
        } else {
            printf("CHECK 2 FAIL: XML missing plist marker\n");
            return 1;
        }

        // 4. Deserialize XML back
        NSPropertyListFormat fmt = 0;
        id restored = [NSPropertyListSerialization
            propertyListWithData:xmlData
                         options:NSPropertyListImmutable
                          format:&fmt
                           error:&err];
        if ([restored isKindOfClass:[NSDictionary class]] && !err) {
            printf("CHECK 3 PASS: deserialized back to NSDictionary\n");
        } else {
            printf("CHECK 3 FAIL: deserialization error\n");
            return 1;
        }

        // 5. Check format was XML
        if (fmt == NSPropertyListXMLFormat_v1_0) {
            printf("CHECK 4 PASS: detected XML format\n");
        } else {
            printf("CHECK 4 FAIL: format = %lu\n", (unsigned long)fmt);
            return 1;
        }

        // 6. Verify string value round-trip
        NSString *appName = restored[@"AppName"];
        if ([appName isEqualToString:@"DarwinLoader"]) {
            printf("CHECK 5 PASS: AppName round-tripped\n");
        } else {
            printf("CHECK 5 FAIL: AppName = %s\n", [appName UTF8String]);
            return 1;
        }

        // 7. Verify nested dict
        NSDictionary *db = restored[@"Database"];
        NSNumber *port = db[@"Port"];
        if ([port integerValue] == 5432) {
            printf("CHECK 6 PASS: Database.Port = 5432\n");
        } else {
            printf("CHECK 6 FAIL: port = %ld\n", (long)[port integerValue]);
            return 1;
        }

        // 8. Serialize to binary plist
        NSData *binData = [NSPropertyListSerialization
            dataWithPropertyList:config
                          format:NSPropertyListBinaryFormat_v1_0
                         options:0
                           error:&err];
        if (binData && !err && [binData length] > 0) {
            printf("CHECK 7 PASS: serialized to binary plist (%lu bytes)\n",
                   (unsigned long)[binData length]);
        } else {
            printf("CHECK 7 FAIL: binary serialization error\n");
            return 1;
        }

        // 9. Validate plist data
        BOOL valid = [NSPropertyListSerialization
            propertyList:config
            isValidForFormat:NSPropertyListXMLFormat_v1_0];
        if (valid) {
            printf("CHECK 8 PASS: dict is valid for XML format\n");
        } else {
            printf("CHECK 8 FAIL: validation returned NO\n");
            return 1;
        }
    }
    return 0;
}
