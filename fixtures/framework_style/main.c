// Test app linking against a .framework bundle via @rpath.
#include <stdio.h>

extern void my_framework_hello(void);

int main() {
    my_framework_hello();
    printf("[framework_app] done\n");
    return 0;
}
