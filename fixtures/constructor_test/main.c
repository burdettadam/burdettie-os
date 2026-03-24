// constructor_test — verify initializer ordering
#include <stdio.h>

__attribute__((constructor))
static void init_first(void) {
    printf("[constructor_test] first constructor\n");
}

__attribute__((constructor))
static void init_second(void) {
    printf("[constructor_test] second constructor\n");
}

int main() {
    printf("[constructor_test] main\n");
    return 0;
}
