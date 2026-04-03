// cli_file_ops — C-level file I/O exercising libSystem functions needed by Xcode CLI tools.
// Exercises: open/close/read/write, stat, mmap/munmap, dlopen/dlsym/dlclose,
//            feof/fputc/getc, realpath, rename/unlink, fcntl, qsort, bsearch.
#import <Foundation/Foundation.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/stat.h>
#include <sys/mman.h>
#include <dlfcn.h>

static int compare_ints(const void *a, const void *b) {
    return *(const int *)a - *(const int *)b;
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // ── CHECK 1: open, write, close ──────────────────────────────────────
        char tmppath[] = "/tmp/cli_file_ops_XXXXXX";
        int fd = mkstemp(tmppath);
        if (fd >= 0) {
            const char *msg = "hello cli";
            ssize_t n = write(fd, msg, strlen(msg));
            close(fd);
            if (n == (ssize_t)strlen(msg)) {
                printf("CHECK 1 PASS: write %zd bytes to %s\n", n, tmppath);
                checks++;
            } else {
                printf("CHECK 1 FAIL: write returned %zd\n", n);
            }
        } else {
            printf("CHECK 1 FAIL: mkstemp failed\n");
        }

        // ── CHECK 2: stat ────────────────────────────────────────────────────
        struct stat st;
        if (stat(tmppath, &st) == 0 && st.st_size == 9) {
            printf("CHECK 2 PASS: stat size=%lld\n", (long long)st.st_size);
            checks++;
        } else {
            printf("CHECK 2 FAIL: stat failed or size wrong\n");
        }

        // ── CHECK 3: open + read ─────────────────────────────────────────────
        fd = open(tmppath, O_RDONLY);
        if (fd >= 0) {
            char buf[64] = {0};
            ssize_t n = read(fd, buf, sizeof(buf) - 1);
            close(fd);
            if (n == 9 && strcmp(buf, "hello cli") == 0) {
                printf("CHECK 3 PASS: read '%s'\n", buf);
                checks++;
            } else {
                printf("CHECK 3 FAIL: read %zd bytes, got '%s'\n", n, buf);
            }
        } else {
            printf("CHECK 3 FAIL: open failed\n");
        }

        // ── CHECK 4: mmap + munmap ───────────────────────────────────────────
        fd = open(tmppath, O_RDONLY);
        if (fd >= 0) {
            void *mapped = mmap(NULL, st.st_size, PROT_READ, MAP_PRIVATE, fd, 0);
            close(fd);
            if (mapped != MAP_FAILED && memcmp(mapped, "hello cli", 9) == 0) {
                printf("CHECK 4 PASS: mmap contains 'hello cli'\n");
                checks++;
                munmap(mapped, st.st_size);
            } else {
                printf("CHECK 4 FAIL: mmap failed\n");
            }
        } else {
            printf("CHECK 4 FAIL: open for mmap failed\n");
        }

        // ── CHECK 5: rename + unlink ─────────────────────────────────────────
        char newpath[256];
        snprintf(newpath, sizeof(newpath), "%s.renamed", tmppath);
        if (rename(tmppath, newpath) == 0) {
            // Verify renamed file exists
            struct stat st2;
            if (stat(newpath, &st2) == 0) {
                printf("CHECK 5 PASS: rename OK\n");
                checks++;
            } else {
                printf("CHECK 5 FAIL: renamed file not found\n");
            }
            unlink(newpath);
        } else {
            printf("CHECK 5 FAIL: rename failed\n");
            unlink(tmppath);
        }

        // ── CHECK 6: realpath ────────────────────────────────────────────────
        char resolved[PATH_MAX];
        if (realpath("/tmp", resolved) != NULL && strlen(resolved) > 0) {
            printf("CHECK 6 PASS: realpath /tmp = %s\n", resolved);
            checks++;
        } else {
            printf("CHECK 6 FAIL: realpath failed\n");
        }

        // ── CHECK 7: dlopen + dlsym + dlclose ───────────────────────────────
        void *handle = dlopen("/usr/lib/libSystem.B.dylib", RTLD_LAZY);
        if (handle) {
            void *sym = dlsym(handle, "strlen");
            if (sym != NULL) {
                printf("CHECK 7 PASS: dlsym(strlen) = %p\n", sym);
                checks++;
            } else {
                printf("CHECK 7 FAIL: dlsym returned NULL\n");
            }
            dlclose(handle);
        } else {
            printf("CHECK 7 FAIL: dlopen failed: %s\n", dlerror());
        }

        // ── CHECK 8: qsort ──────────────────────────────────────────────────
        int arr[] = {5, 2, 8, 1, 9, 3};
        qsort(arr, 6, sizeof(int), compare_ints);
        if (arr[0] == 1 && arr[5] == 9) {
            printf("CHECK 8 PASS: qsort [%d..%d]\n", arr[0], arr[5]);
            checks++;
        } else {
            printf("CHECK 8 FAIL: qsort wrong order\n");
        }

        // ── CHECK 9: bsearch ────────────────────────────────────────────────
        int key = 5;
        int *found = bsearch(&key, arr, 6, sizeof(int), compare_ints);
        if (found && *found == 5) {
            printf("CHECK 9 PASS: bsearch found %d\n", *found);
            checks++;
        } else {
            printf("CHECK 9 FAIL: bsearch not found\n");
        }

        // ── CHECK 10: fopen + feof + fputc + getc ───────────────────────────
        char tmppath2[] = "/tmp/cli_file_ops2_XXXXXX";
        int fd2 = mkstemp(tmppath2);
        close(fd2);
        FILE *f = fopen(tmppath2, "w");
        if (f) {
            fputc('A', f);
            fputc('B', f);
            fclose(f);
            f = fopen(tmppath2, "r");
            if (f) {
                int c1 = getc(f);
                int c2 = getc(f);
                int eof = feof(f) == 0; // not yet at EOF
                getc(f); // trigger EOF
                int iseof = feof(f) != 0;
                fclose(f);
                if (c1 == 'A' && c2 == 'B' && eof && iseof) {
                    printf("CHECK 10 PASS: fputc/getc/feof OK\n");
                    checks++;
                } else {
                    printf("CHECK 10 FAIL: c1=%c c2=%c eof=%d iseof=%d\n",
                           c1, c2, eof, iseof);
                }
            }
        }
        unlink(tmppath2);

        // ── Result ───────────────────────────────────────────────────────────
        printf("\ncli_file_ops: %d/10 checks passed\n", checks);
        return (checks == 10) ? 0 : 1;
    }
}
