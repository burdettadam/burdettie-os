// dispatch_once fixture — exercises dispatch_once_f (function pointer variant).
// Expected: exit 0, prints "initialized" exactly once.
#include <stdio.h>
#include <dispatch/dispatch.h>

static dispatch_once_t init_predicate;
static int init_count = 0;

static void do_init(void *ctx) {
    (void)ctx;
    init_count++;
    printf("initialized\n");
}

int main(void) {
    // Call dispatch_once_f three times — should only run once
    dispatch_once_f(&init_predicate, NULL, do_init);
    dispatch_once_f(&init_predicate, NULL, do_init);
    dispatch_once_f(&init_predicate, NULL, do_init);

    if (init_count == 1) {
        printf("count=1\n");
        return 0;
    } else {
        printf("count=%d (wrong!)\n", init_count);
        return 1;
    }
}
