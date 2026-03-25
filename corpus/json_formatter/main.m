// json_formatter — builds and formats nested dictionary/array structures
// Exercises: NSDictionary, NSArray, NSMutableDictionary, NSMutableArray, NSNumber, NSNull, NSString
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        // Build a JSON-like structure
        NSMutableDictionary *person = [NSMutableDictionary dictionary];
        [person setObject:@"Alice" forKey:@"name"];
        [person setObject:[NSNumber numberWithInt:30] forKey:@"age"];
        [person setObject:[NSNumber numberWithBool:YES] forKey:@"active"];

        NSMutableArray *hobbies = [NSMutableArray array];
        [hobbies addObject:@"reading"];
        [hobbies addObject:@"coding"];
        [hobbies addObject:@"hiking"];
        [person setObject:hobbies forKey:@"hobbies"];

        NSMutableDictionary *address = [NSMutableDictionary dictionary];
        [address setObject:@"123 Main St" forKey:@"street"];
        [address setObject:@"Springfield" forKey:@"city"];
        [address setObject:[NSNull null] forKey:@"apt"];
        [person setObject:address forKey:@"address"];

        // Query the structure
        int checks = 0;
        NSString *name = [person objectForKey:@"name"];
        if ([name isEqualToString:@"Alice"]) { printf("CHECK 1 PASS: name = Alice\n"); checks++; }

        NSNumber *age = [person objectForKey:@"age"];
        if ([age intValue] == 30) { printf("CHECK 2 PASS: age = 30\n"); checks++; }

        NSArray *h = [person objectForKey:@"hobbies"];
        if ([h count] == 3) { printf("CHECK 3 PASS: 3 hobbies\n"); checks++; }
        if ([[h objectAtIndex:1] isEqualToString:@"coding"]) { printf("CHECK 4 PASS: hobby[1] = coding\n"); checks++; }

        NSDictionary *addr = [person objectForKey:@"address"];
        if ([[addr objectForKey:@"city"] isEqualToString:@"Springfield"]) { printf("CHECK 5 PASS: city = Springfield\n"); checks++; }

        id apt = [addr objectForKey:@"apt"];
        if (apt == [NSNull null]) { printf("CHECK 6 PASS: apt is NSNull\n"); checks++; }

        if ([person count] == 5) { printf("CHECK 7 PASS: person has 5 keys\n"); checks++; }

        NSNumber *active = [person objectForKey:@"active"];
        if ([active boolValue] == YES) { printf("CHECK 8 PASS: active = YES\n"); checks++; }

        printf("json_formatter: %d/8 checks passed\n", checks);
        return (checks == 8) ? 0 : 1;
    }
}
