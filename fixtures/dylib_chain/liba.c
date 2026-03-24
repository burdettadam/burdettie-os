// liba.c — middle of the chain, depends on libb
#include <stdio.h>

extern void libb_greet(void);

__attribute__((constructor))
static void liba_init(void) {
    printf("[liba] constructor fired\n");
}

void liba_greet(void) {
    libb_greet();
    printf("[liba] hello from liba\n");
}
