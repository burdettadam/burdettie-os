/*
 * test_darwin_runtime.c — Minimal C client for libdarwin_runtime_ffi
 *
 * Build with pkg-config:
 *   cc -o test_darwin_runtime test_darwin_runtime.c \
 *      $(pkg-config --cflags --libs darwin-runtime)
 *
 * Or build directly:
 *   cc -o test_darwin_runtime test_darwin_runtime.c \
 *      -I/usr/local/include -L/usr/local/lib -ldarwin_runtime_ffi
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "darwin_runtime.h"

static int tests_passed = 0;
static int tests_failed = 0;

#define CHECK(cond, msg) do { \
    if (cond) { \
        printf("  ✓ %s\n", msg); \
        tests_passed++; \
    } else { \
        printf("  ✗ FAIL: %s\n", msg); \
        tests_failed++; \
    } \
} while(0)

int main(int argc, char *argv[]) {
    printf("darwin-runtime C client test\n");
    printf("============================\n\n");

    /* 1. Version */
    printf("[version]\n");
    DarwinVersion v = darwin_runtime_version();
    CHECK(v.major == 0 && v.minor == 1 && v.patch == 0,
          "version is 0.1.0");
    printf("  version: %u.%u.%u\n\n", v.major, v.minor, v.patch);

    /* 2. Error handling — null path */
    printf("[error handling]\n");
    DarwinRuntimeResult r = darwin_runtime_run(NULL);
    CHECK(r.error != 0, "null path returns error");
    CHECK(r.exit_code == -1, "null path exit_code is -1");

    const char *err = darwin_runtime_last_error();
    CHECK(err != NULL, "last_error is non-null after error");
    if (err) {
        CHECK(strstr(err, "null") != NULL, "error mentions 'null'");
        printf("  error: %s\n", err);
    }

    /* 3. Error handling — nonexistent file */
    r = darwin_runtime_run("/nonexistent/binary");
    CHECK(r.error != 0, "nonexistent path returns error");
    printf("\n");

    /* 4. Run a fixture if path provided */
    if (argc > 1) {
        printf("[runtime execution]\n");
        printf("  fixture: %s\n", argv[1]);
        r = darwin_runtime_run(argv[1]);
        if (r.error == 0) {
            CHECK(1, "fixture executed successfully");
            printf("  exit_code:        %d\n", r.exit_code);
            printf("  images_loaded:    %zu\n", r.images_loaded);
            printf("  shims_installed:  %zu\n", r.shims_installed);
            printf("  initializers_run: %zu\n", r.initializers_run);
        } else {
            err = darwin_runtime_last_error();
            printf("  ✗ FAIL: fixture execution failed: %s\n",
                   err ? err : "(no error message)");
            tests_failed++;
        }
        printf("\n");

        /* 5. ObjC runtime introspection (only meaningful after running a fixture) */
        printf("[objc introspection]\n");
        size_t class_count = darwin_objc_class_count();
        printf("  class_count:    %zu\n", class_count);
        CHECK(class_count > 0, "class count > 0 after fixture run");

        size_t sel_count = darwin_objc_selector_count();
        printf("  selector_count: %zu\n", sel_count);

        size_t found_count = darwin_foundation_class_count();
        printf("  foundation_classes: %zu\n", found_count);

        /* 6. List classes */
        DarwinClassList list = darwin_objc_list_classes();
        CHECK(list.count > 0, "list_classes returns non-empty list");
        printf("  listed %zu classes", list.count);
        if (list.count > 0 && list.names[0]) {
            printf(" (first: %s)", list.names[0]);
        }
        printf("\n");
        darwin_class_list_free(list);

        /* 7. Get a specific class */
        DarwinClassInfo info = darwin_objc_get_class("NSObject");
        if (info.name) {
            CHECK(strcmp(info.name, "NSObject") == 0, "get_class NSObject");
            printf("  NSObject: %zu methods, %zu class methods, size=%u\n",
                   info.method_count, info.class_method_count, info.instance_size);
            darwin_string_free(info.name);
        } else {
            printf("  (NSObject not found — might need Foundation fixture)\n");
        }

        /* 8. List methods */
        size_t method_count = 0;
        DarwinMethodInfo *methods = darwin_objc_list_methods("NSObject", &method_count);
        if (methods && method_count > 0) {
            CHECK(method_count > 0, "NSObject has methods");
            printf("  NSObject methods (%zu):", method_count);
            for (size_t i = 0; i < method_count && i < 5; i++) {
                if (methods[i].selector)
                    printf(" %s", methods[i].selector);
            }
            if (method_count > 5) printf(" ...");
            printf("\n");
            darwin_method_list_free(methods, method_count);
        }
        printf("\n");
    }

    /* Summary */
    printf("============================\n");
    printf("Results: %d passed, %d failed\n", tests_passed, tests_failed);
    return tests_failed > 0 ? 1 : 0;
}
