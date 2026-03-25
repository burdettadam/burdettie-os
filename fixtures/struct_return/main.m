#import <Foundation/Foundation.h>

// A class with methods that return various struct types.
// On ARM64: structs <= 16 bytes return in registers (x0/x1 or d0/d1),
// structs > 16 bytes use x8 indirect return.

@interface GeometryHelper : NSObject
+ (NSRange)sampleRange;        // 16 bytes — x0/x1
+ (CGPoint)samplePoint;        // 16 bytes — d0/d1
+ (CGSize)sampleSize;          // 16 bytes — d0/d1
+ (CGRect)sampleRect;          // 32 bytes — x8 indirect
+ (CGRect)rectFromPoint:(CGPoint)p size:(CGSize)s;
@end

@implementation GeometryHelper

+ (NSRange)sampleRange {
    return NSMakeRange(10, 42);
}

+ (CGPoint)samplePoint {
    return CGPointMake(3.14, 2.71);
}

+ (CGSize)sampleSize {
    return CGSizeMake(100.0, 200.0);
}

+ (CGRect)sampleRect {
    return CGRectMake(10.0, 20.0, 300.0, 400.0);
}

+ (CGRect)rectFromPoint:(CGPoint)p size:(CGSize)s {
    return CGRectMake(p.x, p.y, s.width, s.height);
}

@end

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        // Test NSRange (16 bytes — returned in x0/x1)
        NSRange range = [GeometryHelper sampleRange];
        if (range.location != 10 || range.length != 42) {
            fprintf(stderr, "FAIL: range={%lu, %lu}\n", range.location, range.length);
            return 1;
        }
        printf("range={%lu, %lu}\n", range.location, range.length);

        // Test CGPoint (16 bytes — returned in d0/d1)
        CGPoint point = [GeometryHelper samplePoint];
        if (point.x < 3.13 || point.x > 3.15 || point.y < 2.70 || point.y > 2.72) {
            fprintf(stderr, "FAIL: point={%.2f, %.2f}\n", point.x, point.y);
            return 1;
        }
        printf("point={%.2f, %.2f}\n", point.x, point.y);

        // Test CGSize (16 bytes — returned in d0/d1)
        CGSize size = [GeometryHelper sampleSize];
        if (size.width != 100.0 || size.height != 200.0) {
            fprintf(stderr, "FAIL: size={%.0f, %.0f}\n", size.width, size.height);
            return 1;
        }
        printf("size={%.0f, %.0f}\n", size.width, size.height);

        // Test CGRect (32 bytes — uses x8 indirect return on ARM64)
        CGRect rect = [GeometryHelper sampleRect];
        if (rect.origin.x != 10.0 || rect.origin.y != 20.0 ||
            rect.size.width != 300.0 || rect.size.height != 400.0) {
            fprintf(stderr, "FAIL: rect={{%.0f, %.0f}, {%.0f, %.0f}}\n",
                    rect.origin.x, rect.origin.y, rect.size.width, rect.size.height);
            return 1;
        }
        printf("rect={{%.0f, %.0f}, {%.0f, %.0f}}\n",
               rect.origin.x, rect.origin.y, rect.size.width, rect.size.height);

        // Test CGRect with struct arguments
        CGRect composed = [GeometryHelper rectFromPoint:point size:size];
        if (composed.origin.x < 3.13 || composed.origin.x > 3.15 ||
            composed.size.width != 100.0) {
            fprintf(stderr, "FAIL: composed={{%.2f, %.2f}, {%.0f, %.0f}}\n",
                    composed.origin.x, composed.origin.y,
                    composed.size.width, composed.size.height);
            return 1;
        }
        printf("composed={{%.2f, %.2f}, {%.0f, %.0f}}\n",
               composed.origin.x, composed.origin.y,
               composed.size.width, composed.size.height);

        // Test NSStringFromRange
        NSString *rangeStr = NSStringFromRange(range);
        printf("NSStringFromRange=%s\n", [rangeStr UTF8String]);

        // Test NSStringFromRect
        NSString *rectStr = NSStringFromRect(rect);
        printf("NSStringFromRect=%s\n", [rectStr UTF8String]);

        printf("all checks passed\n");
    }
    return 0;
}
