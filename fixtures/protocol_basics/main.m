// protocol_basics — C1 fixture: ObjC protocol definitions and conformance
//
// Tests protocol declaration, adoption, conformance checking,
// and protocol introspection via the ObjC runtime.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>
#include <string.h>

// ── Protocol declarations ──────────────────────────────────────────

@protocol Greeter
- (void)greet;
@end

@protocol Farewell
- (void)farewell;
@end

@protocol Polite <Greeter, Farewell>
- (void)bow;
@optional
- (void)curtsy;
@end

// ── Class adopting protocols ───────────────────────────────────────

@interface Person : NSObject <Polite>
@property (nonatomic, copy) NSString *name;
@end

@implementation Person

- (void)greet {
    printf("Hello from %s\n", [self.name UTF8String]);
}

- (void)farewell {
    printf("Goodbye from %s\n", [self.name UTF8String]);
}

- (void)bow {
    printf("%s bows\n", [self.name UTF8String]);
}

@end

// ── Class adopting only Greeter ────────────────────────────────────

@interface Robot : NSObject <Greeter>
@end

@implementation Robot

- (void)greet {
    printf("BEEP BOOP\n");
}

@end

int main() {
    @autoreleasepool {
        // ── Protocol lookup ────────────────────────────────────────
        Protocol *greeter = objc_getProtocol("Greeter");
        Protocol *farewell = objc_getProtocol("Farewell");
        Protocol *polite = objc_getProtocol("Polite");
        Protocol *nonexist = objc_getProtocol("Nonexistent");

        printf("Greeter protocol: %s\n", greeter ? "found" : "nil");
        printf("Farewell protocol: %s\n", farewell ? "found" : "nil");
        printf("Polite protocol: %s\n", polite ? "found" : "nil");
        printf("Nonexistent protocol: %s\n", nonexist ? "nil" : "nil");

        // ── Protocol name ──────────────────────────────────────────
        if (greeter) {
            printf("protocol_getName(Greeter): %s\n", protocol_getName(greeter));
        }

        // ── Class conformance ──────────────────────────────────────
        Class personClass = objc_getClass("Person");
        Class robotClass = objc_getClass("Robot");

        printf("Person conforms to Greeter: %s\n",
               class_conformsToProtocol(personClass, greeter) ? "YES" : "NO");
        printf("Person conforms to Farewell: %s\n",
               class_conformsToProtocol(personClass, farewell) ? "YES" : "NO");
        printf("Person conforms to Polite: %s\n",
               class_conformsToProtocol(personClass, polite) ? "YES" : "NO");
        printf("Robot conforms to Greeter: %s\n",
               class_conformsToProtocol(robotClass, greeter) ? "YES" : "NO");
        printf("Robot conforms to Polite: %s\n",
               class_conformsToProtocol(robotClass, polite) ? "YES" : "NO");

        // ── Protocol conformance (protocol inherits protocol) ──────
        if (polite && greeter) {
            printf("Polite conforms to Greeter: %s\n",
                   protocol_conformsToProtocol(polite, greeter) ? "YES" : "NO");
        }
        if (greeter && polite) {
            printf("Greeter conforms to Polite: %s\n",
                   protocol_conformsToProtocol(greeter, polite) ? "YES" : "NO");
        }

        // ── Instance conformsToProtocol: ───────────────────────────
        Person *p = [[Person alloc] init];
        p.name = @"Alice";
        BOOL (*sendConforms)(id, SEL, Protocol *) = (BOOL (*)(id, SEL, Protocol *))objc_msgSend;
        SEL confSel = sel_registerName("conformsToProtocol:");

        printf("person conformsToProtocol Greeter: %s\n",
               sendConforms(p, confSel, greeter) ? "YES" : "NO");
        printf("person conformsToProtocol Polite: %s\n",
               sendConforms(p, confSel, polite) ? "YES" : "NO");

        // ── Protocol list on class ─────────────────────────────────
        unsigned int protoCount = 0;
        Protocol * __unsafe_unretained *protos = class_copyProtocolList(personClass, &protoCount);
        printf("Person protocol count: %u\n", protoCount);
        for (unsigned int i = 0; i < protoCount; i++) {
            printf("  protocol[%u]: %s\n", i, protocol_getName(protos[i]));
        }
        free(protos);

        // ── Method invocation through protocol ─────────────────────
        [p greet];
        [p farewell];
        [p bow];

        printf("OK: protocol_basics\n");
    }
    return 0;
}
