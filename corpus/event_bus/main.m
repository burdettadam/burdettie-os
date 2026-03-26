// event_bus — NSNotificationCenter post/observe pattern
// Exercises: NSNotificationCenter, NSNotification, NSDictionary,
//            NSMutableArray, blocks, addObserverForName:object:queue:usingBlock:
#import <Foundation/Foundation.h>

static NSMutableArray *receivedNames = nil;
static NSMutableArray *receivedInfos = nil;

// Traditional observer
@interface EventLogger : NSObject
@end

@implementation EventLogger
- (void)handleEvent:(NSNotification *)note {
    [receivedNames addObject:[note name]];
    if ([note userInfo]) {
        [receivedInfos addObject:[note userInfo]];
    }
}
@end

int main(void) {
    @autoreleasepool {
        receivedNames = [NSMutableArray array];
        receivedInfos = [NSMutableArray array];

        NSNotificationCenter *center = [NSNotificationCenter defaultCenter];

        // 1. Basic post + observe with selector
        EventLogger *logger = [[EventLogger alloc] init];
        [center addObserver:logger
                   selector:@selector(handleEvent:)
                       name:@"TestEvent"
                     object:nil];

        [center postNotificationName:@"TestEvent"
                              object:nil
                            userInfo:@{@"seq": @(1)}];
        if ([receivedNames count] == 1 &&
            [[receivedNames lastObject] isEqualToString:@"TestEvent"]) {
            printf("CHECK 1 PASS: received TestEvent via selector\n");
        } else {
            printf("CHECK 1 FAIL: names=%lu\n", (unsigned long)[receivedNames count]);
            return 1;
        }

        // 2. userInfo delivered
        NSDictionary *info = [receivedInfos lastObject];
        if ([info[@"seq"] integerValue] == 1) {
            printf("CHECK 2 PASS: userInfo.seq = 1\n");
        } else {
            printf("CHECK 2 FAIL: userInfo missing\n");
            return 1;
        }

        // 3. Block-based observer
        __block NSInteger blockSeq = -1;
        id token = [center addObserverForName:@"BlockEvent"
                                       object:nil
                                        queue:nil
                                   usingBlock:^(NSNotification *note) {
            blockSeq = [note.userInfo[@"seq"] integerValue];
        }];
        [center postNotificationName:@"BlockEvent"
                              object:nil
                            userInfo:@{@"seq": @(42)}];
        if (blockSeq == 42) {
            printf("CHECK 3 PASS: block observer received seq=42\n");
        } else {
            printf("CHECK 3 FAIL: blockSeq=%ld\n", (long)blockSeq);
            return 1;
        }

        // 4. Remove block observer, verify no more delivery
        [center removeObserver:token];
        [center postNotificationName:@"BlockEvent"
                              object:nil
                            userInfo:@{@"seq": @(99)}];
        if (blockSeq == 42) {
            printf("CHECK 4 PASS: removed observer, no delivery\n");
        } else {
            printf("CHECK 4 FAIL: blockSeq changed to %ld\n", (long)blockSeq);
            return 1;
        }

        // 5. Multiple observers for same notification
        __block NSInteger countA = 0;
        __block NSInteger countB = 0;
        [center addObserverForName:@"Multi" object:nil queue:nil
                        usingBlock:^(NSNotification *n) { countA++; }];
        [center addObserverForName:@"Multi" object:nil queue:nil
                        usingBlock:^(NSNotification *n) { countB++; }];
        [center postNotificationName:@"Multi" object:nil];
        [center postNotificationName:@"Multi" object:nil];
        if (countA == 2 && countB == 2) {
            printf("CHECK 5 PASS: two observers each received 2 events\n");
        } else {
            printf("CHECK 5 FAIL: countA=%ld countB=%ld\n", (long)countA, (long)countB);
            return 1;
        }

        // 6. Name filtering — unrelated notification not delivered
        __block BOOL wrongDelivered = NO;
        [center addObserverForName:@"Specific" object:nil queue:nil
                        usingBlock:^(NSNotification *n) { wrongDelivered = YES; }];
        [center postNotificationName:@"Unrelated" object:nil];
        if (!wrongDelivered) {
            printf("CHECK 6 PASS: name filtering works\n");
        } else {
            printf("CHECK 6 FAIL: wrong notification delivered\n");
            return 1;
        }

        // 7. Remove selector-based observer
        [center removeObserver:logger name:@"TestEvent" object:nil];
        NSUInteger beforeCount = [receivedNames count];
        [center postNotificationName:@"TestEvent" object:nil];
        if ([receivedNames count] == beforeCount) {
            printf("CHECK 7 PASS: selector observer removed\n");
        } else {
            printf("CHECK 7 FAIL: still receiving after remove\n");
            return 1;
        }

        // 8. Post with NSNotification object
        NSNotification *note = [NSNotification notificationWithName:@"Direct"
                                                             object:nil
                                                           userInfo:@{@"x": @(7)}];
        __block NSInteger directX = -1;
        [center addObserverForName:@"Direct" object:nil queue:nil
                        usingBlock:^(NSNotification *n) {
            directX = [n.userInfo[@"x"] integerValue];
        }];
        [center postNotification:note];
        if (directX == 7) {
            printf("CHECK 8 PASS: postNotification: with object works\n");
        } else {
            printf("CHECK 8 FAIL: directX=%ld\n", (long)directX);
            return 1;
        }
    }
    return 0;
}
