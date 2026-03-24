// objc_classes — defines custom ObjC classes for metadata testing
#import <Foundation/Foundation.h>

@interface Greeter : NSObject
- (void)greet;
@end

@implementation Greeter
- (void)greet {
    NSLog(@"Hello from Greeter");
}
@end

@interface Calculator : NSObject
- (int)add:(int)a to:(int)b;
@end

@implementation Calculator
- (int)add:(int)a to:(int)b {
    return a + b;
}
@end

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        Greeter *g = [[Greeter alloc] init];
        [g greet];
        Calculator *c = [[Calculator alloc] init];
        NSLog(@"3 + 4 = %d", [c add:3 to:4]);
    }
    return 0;
}
