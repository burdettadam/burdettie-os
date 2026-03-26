// appkit_bootstrap — fixture: NSApplication launch + delegate callback.
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -framework AppKit -arch arm64 -o app main.m
#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>
#include <objc/runtime.h>
#include <objc/message.h>

// ── Minimal AppDelegate ────────────────────────────────────────

typedef id (*send0_t)(id, SEL);
typedef id (*send1_t)(id, SEL, id);
typedef void (*sendv0_t)(id, SEL);
typedef void (*sendv1_t)(id, SEL, id);
typedef BOOL (*sendB1_t)(id, SEL, NSInteger);

static BOOL g_didFinishLaunching = NO;

void AppDelegate_didFinishLaunching(id self, SEL _cmd, id notification) {
    (void)self; (void)_cmd;
    printf("CHECK 2: applicationDidFinishLaunching: received\n");
    g_didFinishLaunching = YES;

    // Terminate immediately so the run loop exits
    send0_t s0 = (send0_t)objc_msgSend;
    sendv1_t sv1 = (sendv1_t)objc_msgSend;
    id app = s0((id)objc_getClass("NSApplication"), sel_registerName("sharedApplication"));
    sv1(app, sel_registerName("terminate:"), (id)0);
}

int main(void) {
    @autoreleasepool {
        send0_t s0 = (send0_t)objc_msgSend;
        sendv1_t sv1 = (sendv1_t)objc_msgSend;
        sendB1_t sB1 = (sendB1_t)objc_msgSend;

        // 1. Get shared application
        id app = s0((id)objc_getClass("NSApplication"), sel_registerName("sharedApplication"));
        if (!app) { printf("FAIL: 1 - sharedApplication returned nil\n"); return 1; }
        printf("CHECK 1: sharedApplication obtained\n");

        // 2. Build a tiny delegate via objc_allocateClassPair
        Class delegateCls = objc_allocateClassPair(objc_getClass("NSObject"), "AppDelegate", 0);
        class_addMethod(delegateCls,
                        sel_registerName("applicationDidFinishLaunching:"),
                        (IMP)AppDelegate_didFinishLaunching,
                        "v@:@");
        objc_registerClassPair(delegateCls);

        id delegate = s0(s0((id)delegateCls, sel_registerName("alloc")),
                         sel_registerName("init"));
        sv1(app, sel_registerName("setDelegate:"), delegate);

        // 3. Set activation policy (regular app)
        sB1(app, sel_registerName("setActivationPolicy:"), 0);

        // 4. Run the application — will call delegate, then terminate
        ((void(*)(id,SEL))objc_msgSend)(app, sel_registerName("run"));

        // 5. Verify delegate was called
        if (!g_didFinishLaunching) {
            printf("FAIL: 5 - delegate was never called\n");
            return 1;
        }
        printf("CHECK 3: clean exit after delegate callback\n");
    }
    return 0;
}
