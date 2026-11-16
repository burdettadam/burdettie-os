/* Runnable with a real C library. The freestanding core is a separate object. */
#include <stdio.h>
#include <fenv.h>
#include "probe.h"
#if defined(__APPLE__) && defined(__MACH__)
#define M1_HOST "darwin"
#elif defined(__linux__)
#define M1_HOST "linux"
#else
#define M1_HOST "other"
#endif
#if defined(__x86_64__)
#define M1_ARCH "x86_64"
#else
#define M1_ARCH "other"
#endif
int main(void) {
    static const char *names[M1_RT_CATEGORIES] = {"signed_div", "signed_mod", "unsigned_div",
        "unsigned_mod", "double_to_signed", "signed_to_double"};
    struct m1_rt_result r;
    unsigned int i, failures = 0;
    /* Integer-to-double expected values use round-to-nearest, ties-to-even. */
    if (fegetround() != FE_TONEAREST) {
        fputs("unsupported floating-point rounding mode\n", stderr);
        return 2;
    }
    m1_compiler_rt_probe(&r);
    printf("{\"schema_version\":1,\"scope\":\"m1_compiler_rt_arithmetic_execution\","
           "\"platform\":\"%s\",\"architecture\":\"%s\",\"rounding\":\"nearest_even\",\"cases\":{", M1_HOST, M1_ARCH);
    for (i = 0; i < M1_RT_CATEGORIES; i++) {
        printf("%s\"%s\":{\"checked\":%u,\"failed\":%u,\"first_failure\":%u}",
               i ? "," : "", names[i], r.checked[i], r.failed[i], r.first_failure[i]);
        failures += r.failed[i];
    }
    puts("}}");
    if (ferror(stdout) || fflush(stdout) != 0) return 3;
    return failures ? 1 : 0;
}
