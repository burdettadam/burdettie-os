// libb.c — bottom of the dependency chain
#include <stdio.h>

__attribute__((constructor))
static void libb_init(void) {
    printf("[libb] constructor fired\n");
}

void libb_greet(void) {
    printf("[libb] hello from libb\n");
}
