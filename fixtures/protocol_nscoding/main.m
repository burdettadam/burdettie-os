// protocol_nscoding — C1 fixture: NSCoding protocol adoption and introspection
//
// Tests a class adopting NSCoding / NSSecureCoding, implements
// encodeWithCoder: and initWithCoder:, and uses protocol runtime
// introspection: protocol_copyMethodDescriptionList,
// protocol_getMethodDescription, protocol_copyPropertyList,
// class_addProtocol, objc_allocateProtocol / objc_registerProtocol.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>
#include <string.h>

// ── Class adopting NSCoding ────────────────────────────────────────

@interface Bookmark : NSObject <NSCoding>
@property (nonatomic, copy) NSString *title;
@property (nonatomic, assign) NSInteger page;
@end

@implementation Bookmark

- (instancetype)initWithTitle:(NSString *)title page:(NSInteger)page {
    self = [super init];
    if (self) {
        _title = [title copy];
        _page = page;
    }
    return self;
}

- (void)encodeWithCoder:(NSCoder *)coder {
    [coder encodeObject:_title forKey:@"title"];
    [coder encodeInteger:_page forKey:@"page"];
}

- (instancetype)initWithCoder:(NSCoder *)coder {
    self = [super init];
    if (self) {
        _title = [coder decodeObjectForKey:@"title"];
        _page = [coder decodeIntegerForKey:@"page"];
    }
    return self;
}

@end

// ── Plain class (no protocol yet) ─────────────────────────────────

@interface Tag : NSObject
@property (nonatomic, copy) NSString *label;
@end

@implementation Tag
@end

int main() {
    @autoreleasepool {
        // ── NSCoding conformance ───────────────────────────────────
        Protocol *coding = objc_getProtocol("NSCoding");
        printf("NSCoding protocol: %s\n", coding ? "found" : "nil");

        Class bmClass = objc_getClass("Bookmark");
        printf("Bookmark conforms to NSCoding: %s\n",
               class_conformsToProtocol(bmClass, coding) ? "YES" : "NO");

        // ── protocol_copyMethodDescriptionList ─────────────────────
        if (coding) {
            unsigned int count = 0;
            struct objc_method_description *descs =
                protocol_copyMethodDescriptionList(coding, YES, YES, &count);
            printf("NSCoding required instance methods: %u\n", count);
            for (unsigned int i = 0; i < count; i++) {
                printf("  method[%u]: %s\n", i,
                       descs[i].name ? sel_getName(descs[i].name) : "(null)");
            }
            free(descs);
        }

        // ── protocol_getMethodDescription ──────────────────────────
        if (coding) {
            SEL encodeSel = sel_registerName("encodeWithCoder:");
            struct objc_method_description desc =
                protocol_getMethodDescription(coding, encodeSel, YES, YES);
            printf("encodeWithCoder: in NSCoding: %s\n",
                   desc.name ? "found" : "not found");
        }

        // ── class_addProtocol ──────────────────────────────────────
        Class tagClass = objc_getClass("Tag");
        Protocol *greeterProto = objc_getProtocol("NSObject");
        if (greeterProto) {
            BOOL added = class_addProtocol(tagClass, greeterProto);
            printf("class_addProtocol Tag+NSObject: %s\n",
                   added ? "YES" : "NO");
            printf("Tag conforms to NSObject after add: %s\n",
                   class_conformsToProtocol(tagClass, greeterProto) ? "YES" : "NO");
            // Adding again should return NO
            BOOL again = class_addProtocol(tagClass, greeterProto);
            printf("class_addProtocol Tag+NSObject again: %s\n",
                   again ? "YES" : "NO");
        }

        // ── objc_allocateProtocol / objc_registerProtocol ──────────
        Protocol *custom = objc_allocateProtocol("CustomProto");
        printf("objc_allocateProtocol CustomProto: %s\n",
               custom ? "allocated" : "nil");
        if (custom) {
            objc_registerProtocol(custom);
            Protocol *found = objc_getProtocol("CustomProto");
            printf("CustomProto after register: %s\n",
                   found ? "found" : "nil");
            printf("protocol_getName(CustomProto): %s\n",
                   protocol_getName(found));
        }

        // Allocating a duplicate should fail
        Protocol *dup = objc_allocateProtocol("CustomProto");
        printf("duplicate allocateProtocol: %s\n",
               dup ? "unexpected" : "nil (correct)");

        // ── Encode / decode cycle ──────────────────────────────────
        Bookmark *bm = [[Bookmark alloc] initWithTitle:@"Chapter 5" page:42];
        printf("Original: %s p%ld\n", [bm.title UTF8String], (long)bm.page);

        NSData *data = [NSKeyedArchiver archivedDataWithRootObject:bm];
        if (data) {
            printf("Archived %lu bytes\n", (unsigned long)[data length]);
            Bookmark *restored = [NSKeyedUnarchiver unarchiveObjectWithData:data];
            if (restored) {
                printf("Restored: %s p%ld\n",
                       [restored.title UTF8String], (long)restored.page);
            } else {
                printf("FAIL: unarchive returned nil\n");
            }
        } else {
            printf("FAIL: archive returned nil\n");
        }

        printf("OK: protocol_nscoding\n");
        return 0;
    }
}
