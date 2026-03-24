// Process identity fixture — exercises getprogname() and getenv().
// Expected: exit 0, prints process name and env value.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// getprogname is a BSD extension available on macOS
extern const char *getprogname(void);

int main(int argc, char **argv) {
    const char *name = getprogname();
    if (name) {
        printf("progname=%s\n", name);
    } else {
        printf("progname=(null)\n");
    }

    const char *home = getenv("HOME");
    if (home) {
        printf("HOME=%s\n", home);
    } else {
        printf("HOME=(null)\n");
    }

    printf("argc=%d\n", argc);
    return 0;
}
