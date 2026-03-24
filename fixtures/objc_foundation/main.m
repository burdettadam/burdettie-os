// objc_foundation — minimal ObjC program using Foundation
#import <Foundation/Foundation.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        NSString *msg = @"hello from Foundation";
        NSLog(@"%@", msg);
    }
    return 0;
}
