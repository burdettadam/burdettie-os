// main.c — trivial binary for fat/universal container
#include <stdio.h>

int main() {
    printf("[fat_binary] hello from %s\n",
#if defined(__x86_64__)
        "x86_64"
#elif defined(__aarch64__)
        "arm64"
#else
        "unknown"
#endif
    );
    return 0;
}
