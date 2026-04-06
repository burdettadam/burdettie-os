// cocoa_controls: Tests NSTextField labels, NSButton, CG text drawing, and NSColor.
//
// Exercises the full GUI rendering pipeline with standard AppKit controls
// alongside a custom drawRect: view that draws CG text.

#import <Cocoa/Cocoa.h>
#import <objc/runtime.h>

// ── Custom view with text drawing ─────────────────────────────

static void CustomTextView_drawRect(id self_, SEL _cmd, CGRect rect) {
    (void)self_; (void)_cmd;
    fprintf(stderr, "CHECK 4: drawRect: called (%d x %d)\n",
            (int)rect.size.width, (int)rect.size.height);

    NSGraphicsContext *nsCtx = [NSGraphicsContext currentContext];
    if (nsCtx) {
        fprintf(stderr, "CHECK 5: NSGraphicsContext obtained\n");
    }
    CGContextRef ctx = [nsCtx CGContext];
    if (ctx) {
        fprintf(stderr, "CHECK 6: CGContext obtained\n");
    }

    // Draw a green background rectangle
    CGContextSetRGBFillColor(ctx, 0.2, 0.8, 0.2, 1.0);
    CGContextFillRect(ctx, CGRectMake(0, 0, rect.size.width, rect.size.height));
    fprintf(stderr, "CHECK 7: drew green background\n");

    // Draw text using CGContextShowTextAtPoint
    CGContextSetRGBFillColor(ctx, 0.0, 0.0, 0.0, 1.0);
    CGContextSetFontSize(ctx, 16.0);
    const char *msg = "Hello from CG Text!";
    CGContextShowTextAtPoint(ctx, 10.0, 30.0, msg, strlen(msg));
    fprintf(stderr, "CHECK 8: drew CG text\n");

    // Use NSColor to set fill color
    NSColor *blueColor = [NSColor blueColor];
    [blueColor setFill];
    CGContextFillRect(ctx, CGRectMake(10, 50, 80, 20));
    fprintf(stderr, "CHECK 9: drew blue rect via NSColor setFill\n");
}

// ── App delegate ──────────────────────────────────────────────

@interface AppDelegate : NSObject <NSApplicationDelegate>
@end

@implementation AppDelegate
- (void)applicationDidFinishLaunching:(NSNotification *)note {
    (void)note;
    fprintf(stderr, "CHECK 2: applicationDidFinishLaunching\n");

    // Create the window
    NSWindow *window = [[NSWindow alloc] initWithContentRect:NSMakeRect(100, 100, 500, 400)
                                                   styleMask:(NSWindowStyleMaskTitled |
                                                              NSWindowStyleMaskClosable |
                                                              NSWindowStyleMaskResizable)
                                                     backing:NSBackingStoreBuffered
                                                       defer:NO];
    [window setTitle:@"Cocoa Controls Test"];
    fprintf(stderr, "CHECK 3: NSWindow created\n");

    // Create content view
    NSView *contentView = [[NSView alloc] initWithFrame:NSMakeRect(0, 0, 500, 400)];

    // Add a label (NSTextField)
    NSTextField *label = [NSTextField labelWithString:@"Status: Ready"];
    [label setFrame:NSMakeRect(20, 360, 200, 24)];
    [contentView addSubview:label];
    fprintf(stderr, "CHECK 10: NSTextField label created\n");

    // Add a button
    NSButton *button = [NSButton buttonWithTitle:@"Click Me"
                                          target:nil
                                          action:nil];
    [button setFrame:NSMakeRect(20, 320, 120, 32)];
    [contentView addSubview:button];
    fprintf(stderr, "CHECK 11: NSButton created\n");

    // Add the custom text view
    Class CustomTextViewClass = objc_allocateClassPair([NSView class], "CustomTextView", 0);
    class_addMethod(CustomTextViewClass, @selector(drawRect:),
                    (IMP)CustomTextView_drawRect, "v@:{CGRect=dddd}");
    objc_registerClassPair(CustomTextViewClass);

    id textView = [[CustomTextViewClass alloc] initWithFrame:NSMakeRect(20, 20, 460, 280)];
    [contentView addSubview:textView];
    fprintf(stderr, "CHECK 12: CustomTextView added\n");

    [window setContentView:contentView];
    [window makeKeyAndOrderFront:nil];

    // Force display
    [textView display];
    fprintf(stderr, "CHECK 13: display pipeline complete\n");

    [NSApp terminate:nil];
}
@end

int main(int argc, const char *argv[]) {
    (void)argc; (void)argv;
    @autoreleasepool {
        NSApplication *app = [NSApplication sharedApplication];
        fprintf(stderr, "CHECK 1: sharedApplication obtained\n");

        AppDelegate *delegate = [[AppDelegate alloc] init];
        [app setDelegate:delegate];
        [app run];

        fprintf(stderr, "CHECK 14: clean exit\n");
    }
    return 0;
}
