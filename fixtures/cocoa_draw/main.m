// cocoa_draw — fixture: Cocoa app with custom drawing in a view subclass.
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -framework AppKit -arch arm64 -o app main.m
//
// This tests the full drawing pipeline:
//   1. Creates NSApplication + delegate
//   2. Creates NSWindow
//   3. Creates a custom NSView subclass with drawRect:
//   4. drawRect: uses NSGraphicsContext + CGContext to draw colored rectangles
//   5. Terminates after a brief delay

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

static BOOL g_drawRectCalled = NO;

// Custom view's drawRect: — draws colored rectangles.
void CustomView_drawRect(id self, SEL _cmd, NSRect dirtyRect) {
    (void)self; (void)_cmd;
    printf("CHECK 4: drawRect: called (%.0f x %.0f)\n",
           dirtyRect.size.width, dirtyRect.size.height);
    g_drawRectCalled = YES;

    // Get the current graphics context
    id contextClass = (id)objc_getClass("NSGraphicsContext");
    send0_t s0 = (send0_t)objc_msgSend;
    id context = s0(contextClass, sel_registerName("currentContext"));

    if (context) {
        printf("CHECK 5: NSGraphicsContext obtained\n");

        // Get CGContext
        id cgCtx = s0(context, sel_registerName("CGContext"));
        if (cgCtx) {
            printf("CHECK 6: CGContext obtained\n");

            // Set fill color to red and fill a rectangle
            CGContextRef cgContextRef = (CGContextRef)cgCtx;
            CGContextSetRGBFillColor(cgContextRef, 1.0, 0.0, 0.0, 1.0);  // Red
            CGContextFillRect(cgContextRef, CGRectMake(10, 10, 100, 80));
            printf("CHECK 7: drew red rectangle\n");

            CGContextSetRGBFillColor(cgContextRef, 0.0, 0.0, 1.0, 1.0);  // Blue
            CGContextFillRect(cgContextRef, CGRectMake(150, 50, 80, 120));
            printf("CHECK 8: drew blue rectangle\n");
        } else {
            printf("WARN: CGContext is nil\n");
        }
    } else {
        printf("WARN: NSGraphicsContext is nil\n");
    }
}

void AppDelegate_didFinishLaunching(id self, SEL _cmd, id notification) {
    (void)self; (void)_cmd; (void)notification;
    printf("CHECK 2: applicationDidFinishLaunching\n");

    send0_t s0 = (send0_t)objc_msgSend;
    sendv1_t sv1 = (sendv1_t)objc_msgSend;

    // Create window
    typedef id (*initWindow_t)(id, SEL, NSRect, NSUInteger, NSUInteger, BOOL);
    initWindow_t initW = (initWindow_t)objc_msgSend;

    id windowCls = (id)objc_getClass("NSWindow");
    id windowAlloc = s0(windowCls, sel_registerName("alloc"));
    NSRect frame = NSMakeRect(100, 100, 400, 300);
    id window = initW(windowAlloc,
                      sel_registerName("initWithContentRect:styleMask:backing:defer:"),
                      frame, 1|2, 2, NO);

    if (!window) {
        printf("FAIL: window creation\n");
        return;
    }
    printf("CHECK 3: NSWindow created\n");

    // Create custom view class
    Class customViewCls = objc_allocateClassPair(
        objc_getClass("NSView"), "CustomDrawView", 0);
    class_addMethod(customViewCls,
                    sel_registerName("drawRect:"),
                    (IMP)CustomView_drawRect,
                    "v@:{CGRect=dddd}");
    objc_registerClassPair(customViewCls);

    // Create instance
    typedef id (*initView_t)(id, SEL, NSRect);
    initView_t initV = (initView_t)objc_msgSend;
    id customView = initV(s0((id)customViewCls, sel_registerName("alloc")),
                          sel_registerName("initWithFrame:"),
                          NSMakeRect(0, 0, 400, 300));

    // Set as content view
    sv1(window, sel_registerName("setContentView:"), customView);

    // Show window
    sv1(window, sel_registerName("makeKeyAndOrderFront:"), (id)0);

    // Call display to trigger drawRect:
    ((void(*)(id,SEL))objc_msgSend)(customView, sel_registerName("display"));

    // Terminate
    id app = s0((id)objc_getClass("NSApplication"),
                sel_registerName("sharedApplication"));
    sv1(app, sel_registerName("terminate:"), (id)0);
}

int main(void) {
    @autoreleasepool {
        send0_t s0 = (send0_t)objc_msgSend;
        sendv1_t sv1 = (sendv1_t)objc_msgSend;
        sendB1_t sB1 = (sendB1_t)objc_msgSend;

        id app = s0((id)objc_getClass("NSApplication"),
                    sel_registerName("sharedApplication"));
        if (!app) {
            printf("FAIL: sharedApplication\n");
            return 1;
        }
        printf("CHECK 1: sharedApplication obtained\n");

        // Build delegate
        Class delegateCls = objc_allocateClassPair(
            objc_getClass("NSObject"), "DrawAppDelegate", 0);
        class_addMethod(delegateCls,
                        sel_registerName("applicationDidFinishLaunching:"),
                        (IMP)AppDelegate_didFinishLaunching,
                        "v@:@");
        objc_registerClassPair(delegateCls);

        id delegate = s0(s0((id)delegateCls, sel_registerName("alloc")),
                         sel_registerName("init"));
        sv1(app, sel_registerName("setDelegate:"), delegate);
        sB1(app, sel_registerName("setActivationPolicy:"), 0);

        ((void(*)(id,SEL))objc_msgSend)(app, sel_registerName("run"));

        if (g_drawRectCalled) {
            printf("CHECK 9: drawing pipeline complete\n");
        } else {
            printf("FAIL: drawRect: was never called\n");
        }
    }
    return 0;
}
