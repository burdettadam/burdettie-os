// Characterize native block-token removal separately from the compatibility gate.
// Each case owns fresh centers, senders and a retained token; no captured stack state.
#import <Foundation/Foundation.h>
#include <stdio.h>

static unsigned calls;

struct RemovalCase {
    const char *label;
    unsigned receiver; // 0: owner; 1: foreign; 2: default center.
    unsigned operation; // 0: plain; 1: exact; 2: wrong name; 3: wrong sender; 4: nil filters.
    unsigned wildcard_registration;
};

static const struct RemovalCase cases[] = {
    {"owner_plain", 0, 0, 0},
    {"foreign_plain", 1, 0, 0},
    {"default_plain", 2, 0, 0},
    {"owner_exact", 0, 1, 0},
    {"foreign_exact", 1, 1, 0},
    {"owner_wrong_name", 0, 2, 0},
    {"foreign_wrong_name", 1, 2, 0},
    {"owner_wrong_sender", 0, 3, 0},
    {"foreign_wrong_sender", 1, 3, 0},
    {"owner_nil_filters", 0, 4, 0},
    {"foreign_nil_filters", 1, 4, 0},
    {"owner_wildcard_registration", 0, 1, 1},
    {"foreign_wildcard_registration", 1, 1, 1},
};

int main(void) {
    unsigned failures = 0;
    for (unsigned index = 0; index < sizeof(cases) / sizeof(cases[0]); ++index) {
        const struct RemovalCase *test = &cases[index];
        NSNotificationCenter *owner = [[NSNotificationCenter alloc] init];
        NSNotificationCenter *foreign = [[NSNotificationCenter alloc] init];
        NSObject *sender = [[NSObject alloc] init];
        NSObject *other = [[NSObject alloc] init];
        NSString *name = @"M1IndependentBlockRemoval";
        NSString *wrong = @"M1IndependentWrongName";
        calls = 0;
        id token = [[owner addObserverForName:test->wildcard_registration ? nil : name
                                      object:test->wildcard_registration ? nil : sender
                                       queue:nil usingBlock:^(NSNotification *value) {
            (void)value;
            ++calls;
        }] retain];
        unsigned case_failures = !owner || !foreign || owner == foreign || !sender || !other || !token;
        [foreign postNotificationName:name object:sender];
        case_failures += calls != 0;
        if (!test->wildcard_registration) {
            [owner postNotificationName:wrong object:sender];
            [owner postNotificationName:name object:other];
            case_failures += calls != 0;
        }
        [owner postNotificationName:name object:sender];
        unsigned before = calls;
        case_failures += before != 1;
        NSNotificationCenter *receiver = test->receiver == 0 ? owner :
            test->receiver == 1 ? foreign : [NSNotificationCenter defaultCenter];
        if (test->operation == 0) {
            [receiver removeObserver:token];
        } else {
            NSString *remove_name = test->operation == 4 ? nil :
                test->operation == 2 ? wrong : name;
            id remove_sender = test->operation == 4 ? nil :
                test->operation == 3 ? other : sender;
            [receiver removeObserver:token name:remove_name object:remove_sender];
        }
        calls = 0;
        [owner postNotificationName:name object:sender];
        unsigned after = calls;
        case_failures += after > 1;
        [owner removeObserver:token];
        calls = 0;
        [owner postNotificationName:name object:sender];
        unsigned after_cleanup = calls;
        case_failures += after_cleanup != 0;
        failures += case_failures;
        printf("{\"scope\":\"m1_block_removal_observation\",\"case\":\"%s\","
               "\"before\":%u,\"after\":%u,\"after_cleanup\":%u,\"control_failures\":%u}\n",
               test->label, before, after, after_cleanup, case_failures);
        [token release];
        [owner release];
        [foreign release];
        [sender release];
        [other release];
    }
    fflush(stdout);
    return failures ? 1 : 0;
}
