#ifndef BURDETTIE_M1_COMPILER_RT_PROBE_H
#define BURDETTIE_M1_COMPILER_RT_PROBE_H
/* No SDK headers: this core can be cross-compiled before libSystem is ready. */
typedef unsigned long long m1_u64;
typedef unsigned __int128 m1_u128;
typedef __int128 m1_i128;
#define M1_RT_CATEGORIES 6
struct m1_rt_result {
    unsigned int checked[M1_RT_CATEGORIES];
    unsigned int failed[M1_RT_CATEGORIES];
    unsigned int first_failure[M1_RT_CATEGORIES];
};
void m1_compiler_rt_probe(struct m1_rt_result *result);
#endif
