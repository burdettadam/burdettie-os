// main.c — top of the dependency chain
#include <stdio.h>

extern void liba_greet(void);

int main() {
    printf("[main] starting\n");
    liba_greet();
    printf("[main] done\n");
    return 0;
}
