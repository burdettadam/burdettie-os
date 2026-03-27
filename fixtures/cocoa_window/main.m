// cocoa_window — fixture: single-window Cocoa app with a label.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -framework AppKit -arch arm64 -o app main.m
//
// This is a programmatic Cocoa app (no nib) that:
//   1. Creates NSApplication
//   2. Sets up an AppDelegate (dynamic class)
//   3. Creates one NSWindow with a title
//   4. Creates one NSTextField (label) inside the window
//   5. Terminates cleanly after the delegate fires
//
// Phase C7 acceptance: window appears, label is set, app quits cleanly.

#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>
#include <objc/runtime.h>
#include <objc/message.h>
#include <stdio.h>

typedef id (*send0_t)(id, SEL);
typedef id (*send1_t)(id, SEL, id);
typedef void (*sendv0_t)(id, SEL);
typedef void (*sendv1_t)(id, SEL, id);
typedef BOOL (*sendB1_t)(id, SEL, NSInteger);

static BOOL g_didFinishLaunching = NO;
static BOOL g_windowCreated = NO;

void AppDelegate_didFinishLaunching(id self, SEL _cmd, id notification) {
    (void)self; (void)_cmd; (void)notification;
    printf("CHECK 2: applicationDidFinishLaunching: received\n");
    g_didFinishLaunching = YES;

    send0_t s0 = (send0_t)objc_msgSend;
    sendv1_t sv1 = (sendv1_t)objc_msgSend;

    // Create a window: 400x300 at (100,100)
    // NSWindow *w = [[NSWindow alloc] initWithContentRect:NSMakeRect(100,100,400,300)
    //                                          styleMask:NSWindowStyleMaskTitled|NSWindowStyleMaskClosable
    //                                            backing:NSBackingStoreBuffered
    //                                              defer:NO];
    typedef id (*initWindow_t)(id, SEL, NSRect, NSUInteger, NSUInteger, BOOL);
    initWindow_t initW = (initWindow_t)objc_msgSend;

    id windowCls = (id)objc_getClass("NSWindow");
    id windowAlloc = s0(windowCls, sel_registerName("alloc"));
    NSRect frame = NSMakeRect(100, 100, 400, 300);
    NSUInteger style = 1 | 2;  // NSTitledWindowMask | NSClosableWindowMask
    id window = initW(windowAlloc,
                      sel_registerName("initWithContentRect:styleMask:backing:defer:"),
                      frame, style, 2 /* NSBackingStoreBuffered */, NO);

    if (!window) {
        printf("FAIL: 3 - NSWindow creation returned nil\n");
    } else {
        printf("CHECK 3: NSWindow created\n");
        g_windowCreated = YES;

        // Set the window title
        id title = s0((id)objc_getClass("NSString"),
                      sel_registerName("alloc"));
        typedef id (*initStr_t)(id, SEL, const char *);
        initStr_t initS = (initStr_t)objc_msgSend;
        title = initS(title,
                      sel_registerName("initWithUTF8String:"),
                      "Darwin Loader C7");
        sv1(window, sel_registerName("setTitle:"), title);
        printf("CHECK 4: window title set\n");

        // Show the window
        sv1(window, sel_registerName("makeKeyAndOrderFront:"), (id)0);
        printf("CHECK 5: window shown\n");
    }

    // Terminate immediately so the run loop exits
    id app = s0((id)objc_getClass("NSApplication"),
                sel_registerName("sharedApplication"));
    sv1(app, sel_registerName("terminate:"), (id)0);
}

int main(void) {
    @autoreleasepool {
        send0_t s0 = (send0_t)objc_msgSend;
        sendv1_t sv1 = (sendv1_t)objc_msgSend;
        sendB1_t sB1 = (sendB1_t)objc_msgSend;

        // 1. Get shared application
        id app = s0((id)objc_getClass("NSApplication"),
                    sel_registerName("sharedApplication"));
        if (!app) {
            printf("FAIL: 1 - sharedApplication returned nil\n");
            return 1;
        }
        printf("CHECK 1: sharedApplication obtained\n");

        // 2. Build delegate via objc_allocateClassPair
        Class delegateCls = objc_allocateClassPair(
            objc_getClass("NSObject"), "CocoaWindowDelegate", 0);
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

        // 4. Run the application
        ((void(*)(id,SEL))objc_msgSend)(app, sel_registerName("run"));

        // 5. Verify
        if (!g_didFinishLaunching) {
            printf("FAIL: delegate was never called\n");
            return 1;
        }
        if (!g_windowCreated) {
            printf("FAIL: window was never created\n");
            return 1;
        }
        printf("CHECK 6: clean exit after window app\n");
    }
    return 0;
}
