// main.c — weak-links against liboptional (may be absent at runtime)
#include <stdio.h>

extern void optional_greet(void) __attribute__((weak_import));

int main() {
    printf("[main] starting\n");
    if (optional_greet) {
        optional_greet();
    } else {
        printf("[main] optional_greet is NULL (weak import absent)\n");
    }
    printf("[main] done\n");
    return 0;
}
