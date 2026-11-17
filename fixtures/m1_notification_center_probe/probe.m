// Source-owned synchronous notification fixture. No queue, IPC or lifetime claim.
#import <Foundation/Foundation.h>
#include <stdio.h>
#include <string.h>

static unsigned checks, failures, shared_calls, default_calls, filtered_calls;
static unsigned reentrant_calls, foreign_calls, block_calls, field_checks;
static NSNotificationCenter *reentrant_center;
static NSString *filtered_name;
static NSObject *expected_sender;
static NSDictionary *expected_info;

#define CHECK(condition) do { \
    ++checks; \
    if (!(condition)) { \
        ++failures; \
        fprintf(stderr, "notification check failed at line %d\n", __LINE__); \
    } \
} while (0)

@interface M1CenterObserver : NSObject
- (void)shared:(NSNotification *)notification;
- (void)defaultNotification:(NSNotification *)notification;
- (void)filtered:(NSNotification *)notification;
- (void)reentrant:(NSNotification *)notification;
- (void)foreign:(NSNotification *)notification;
@end

@implementation M1CenterObserver
- (void)shared:(NSNotification *)notification {
    (void)notification;
    ++shared_calls;
}
- (void)defaultNotification:(NSNotification *)notification {
    (void)notification;
    ++default_calls;
}
- (void)filtered:(NSNotification *)notification {
    ++filtered_calls;
    field_checks += 3;
    if ([notification name] != filtered_name) ++failures;
    if ([notification object] != expected_sender) ++failures;
    if ([notification userInfo] != expected_info) ++failures;
}
- (void)reentrant:(NSNotification *)notification {
    ++reentrant_calls;
    // Bound the source-owned callback even when removal is broken.
    if (reentrant_calls == 1) {
        [reentrant_center removeObserver:self name:[notification name] object:nil];
        [reentrant_center postNotification:notification];
    }
}
- (void)foreign:(NSNotification *)notification {
    (void)notification;
    ++foreign_calls;
}
@end

int main(int argc, const char *argv[]) {
    NSNotificationCenter *first = [[NSNotificationCenter alloc] init];
    NSNotificationCenter *second = [[NSNotificationCenter alloc] init];
    NSNotificationCenter *standard = [NSNotificationCenter defaultCenter];
    M1CenterObserver *observer = [[M1CenterObserver alloc] init];
    NSObject *sender = [[NSObject alloc] init];
    NSObject *other = [[NSObject alloc] init];
    NSDictionary *info = [NSDictionary dictionary];
    // A real negative control collapses center identities; it does not forge a report.
    if (argc == 2 && strcmp(argv[1], "--collapse-centers") == 0) first = second;
    CHECK(first && second && standard && observer && sender && other && info);
    CHECK(first != second && first != standard && second != standard);
    CHECK([NSNotificationCenter defaultCenter] == standard);

    NSString *name = @"M1SharedNotification";
    [first addObserver:observer selector:@selector(shared:) name:name object:nil];
    [second addObserver:observer selector:@selector(shared:) name:name object:nil];
    [standard addObserver:observer selector:@selector(defaultNotification:) name:name object:nil];
    [first postNotificationName:name object:nil];
    CHECK(shared_calls == 1 && default_calls == 0);
    [second postNotificationName:name object:nil];
    CHECK(shared_calls == 2 && default_calls == 0);
    [standard postNotificationName:name object:nil];
    CHECK(shared_calls == 2 && default_calls == 1);
    [first removeObserver:observer];
    [first postNotificationName:name object:nil];
    CHECK(shared_calls == 2);
    [second postNotificationName:name object:nil];
    CHECK(shared_calls == 3);
    [first removeObserver:observer name:name object:nil];
    [second postNotificationName:name object:nil];
    CHECK(shared_calls == 4);
    [second removeObserver:observer name:@"WrongName" object:nil];
    [second postNotificationName:name object:nil];
    CHECK(shared_calls == 5);
    [second removeObserver:observer name:name object:nil];
    [second postNotificationName:name object:nil];
    CHECK(shared_calls == 5);
    [standard removeObserver:observer];
    [standard postNotificationName:name object:nil];
    CHECK(default_calls == 1);

    filtered_name = @"M1FilteredNotification";
    expected_sender = sender;
    expected_info = info;
    [first addObserver:observer selector:@selector(filtered:) name:filtered_name object:sender];
    [second postNotificationName:filtered_name object:sender userInfo:info];
    CHECK(filtered_calls == 0);
    [first postNotificationName:filtered_name object:other userInfo:info];
    CHECK(filtered_calls == 0);
    [first postNotificationName:@"WrongName" object:sender userInfo:info];
    CHECK(filtered_calls == 0);
    [first postNotificationName:filtered_name object:sender userInfo:info];
    CHECK(filtered_calls == 1 && field_checks == 3);
    NSNotification *notification = [NSNotification notificationWithName:filtered_name
                                                               object:sender userInfo:info];
    CHECK(notification != nil);
    [second postNotification:notification];
    CHECK(filtered_calls == 1);
    [first postNotification:notification];
    CHECK(filtered_calls == 2 && field_checks == 6);
    [first removeObserver:observer name:filtered_name object:other];
    [first postNotification:notification];
    CHECK(filtered_calls == 3 && field_checks == 9);
    [first removeObserver:observer name:filtered_name object:sender];
    [first postNotification:notification];
    CHECK(filtered_calls == 3);

    reentrant_center = first;
    NSString *reentrant_name = @"M1ReentrantNotification";
    [first addObserver:observer selector:@selector(reentrant:) name:reentrant_name object:nil];
    [second addObserver:observer selector:@selector(foreign:) name:reentrant_name object:nil];
    [first postNotificationName:reentrant_name object:nil];
    CHECK(reentrant_calls == 1 && foreign_calls == 0);
    [second postNotificationName:reentrant_name object:nil];
    CHECK(foreign_calls == 1);
    [first removeObserver:observer];
    [second postNotificationName:reentrant_name object:nil];
    CHECK(foreign_calls == 2);
    [second removeObserver:observer];

    NSString *block_name = @"M1BlockNotification";
    // No captured stack state: the block remains valid through all synchronous posts.
    id token = [first addObserverForName:block_name object:sender queue:nil
                             usingBlock:^(NSNotification *value) {
        (void)value;
        ++block_calls;
    }];
    CHECK(token != nil);
    [second postNotificationName:block_name object:sender];
    CHECK(block_calls == 0);
    [second removeObserver:token];
    [first postNotificationName:block_name object:sender];
    CHECK(block_calls == 1);
    [second removeObserver:token name:block_name object:sender];
    [first postNotificationName:block_name object:sender];
    CHECK(block_calls == 2);
    [first postNotificationName:block_name object:other];
    CHECK(block_calls == 2);
    [first removeObserver:token name:block_name object:sender];
    [first postNotificationName:block_name object:sender];
    CHECK(block_calls == 2);
    [first removeObserver:observer];
    [second removeObserver:observer];
    [standard removeObserver:observer];
    CHECK(field_checks == 9);

#if defined(__arm64__) || defined(__aarch64__)
    const char *architecture = "arm64";
#elif defined(__x86_64__)
    const char *architecture = "x86_64";
#else
#error Unsupported native notification reference architecture
#endif
    printf("{\"schema_version\":1,\"scope\":\"m1_notification_center_execution\","
           "\"status\":\"%s\",\"platform\":\"darwin\",\"architecture\":\"%s\","
           "\"checks\":%u,\"failures\":%u,\"shared_calls\":%u,\"default_calls\":%u,"
           "\"filtered_calls\":%u,\"reentrant_calls\":%u,\"foreign_calls\":%u,"
           "\"block_calls\":%u,\"field_checks\":%u}\n",
           failures ? "fail" : "pass", architecture, checks, failures,
           shared_calls, default_calls, filtered_calls, reentrant_calls,
           foreign_calls, block_calls, field_checks);
    fflush(stdout);
    return failures ? 1 : 0;
}
