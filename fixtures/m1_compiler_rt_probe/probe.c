/* Exercise real helper calls, not the compiler's constant-folded expressions. */
#include "probe.h"
#include "vectors.h"
_Static_assert(sizeof(m1_u64) == 8 && sizeof(m1_i128) == 16, "128-bit ABI required");
_Static_assert(sizeof(double) == 8 && __DBL_MANT_DIG__ == 53 && __DBL_MAX_EXP__ == 1024,
               "IEEE binary64 required");
extern m1_i128 __divti3(m1_i128, m1_i128);
extern m1_i128 __modti3(m1_i128, m1_i128);
extern m1_u128 __udivti3(m1_u128, m1_u128);
extern m1_u128 __umodti3(m1_u128, m1_u128);
extern m1_i128 __fixdfti(double);
extern double __floattidf(m1_i128);
/* Volatile pointers keep the external calls observable even at -O2. */
static m1_i128 (*volatile call_sdiv)(m1_i128, m1_i128) = __divti3;
static m1_i128 (*volatile call_smod)(m1_i128, m1_i128) = __modti3;
static m1_u128 (*volatile call_udiv)(m1_u128, m1_u128) = __udivti3;
static m1_u128 (*volatile call_umod)(m1_u128, m1_u128) = __umodti3;
static m1_i128 (*volatile call_from_double)(double) = __fixdfti;
static double (*volatile call_to_double)(m1_i128) = __floattidf;
#define COUNT(a) ((unsigned int)(sizeof(a) / sizeof((a)[0])))

static m1_u128 unsigned_value(struct m1_words p) {
    return ((m1_u128)p.high << 64) | p.low;
}
static m1_i128 signed_value(struct m1_words p) {
    m1_u128 value = unsigned_value(p);
    /* All unsigned-to-signed conversions here are representable, including MIN. */
    return p.high >> 63 ? -1 - (m1_i128)(~value) : (m1_i128)value;
}
static void check(struct m1_rt_result *r, unsigned int kind, unsigned int index, int ok) {
    r->checked[kind]++;
    if (!ok) {
        if (!r->failed[kind]) r->first_failure[kind] = index;
        r->failed[kind]++;
    }
}
void m1_compiler_rt_probe(struct m1_rt_result *r) {
    unsigned int i;
    /* Avoid a libc memset dependency in the freestanding core. */
    for (i = 0; i < M1_RT_CATEGORIES; i++) {
        r->checked[i] = 0; r->failed[i] = 0; r->first_failure[i] = 0;
    }
    for (i = 0; i < COUNT(m1_signed_cases); i++) {
        const struct m1_div_case *c = &m1_signed_cases[i];
        m1_i128 a = signed_value(c->a), b = signed_value(c->b);
        check(r, 0, i, call_sdiv(a, b) == signed_value(c->q));
        check(r, 1, i, call_smod(a, b) == signed_value(c->r));
    }
    for (i = 0; i < COUNT(m1_unsigned_cases); i++) {
        const struct m1_div_case *c = &m1_unsigned_cases[i];
        m1_u128 a = unsigned_value(c->a), b = unsigned_value(c->b);
        check(r, 2, i, call_udiv(a, b) == unsigned_value(c->q));
        check(r, 3, i, call_umod(a, b) == unsigned_value(c->r));
    }
    for (i = 0; i < COUNT(m1_from_double_cases); i++) {
        union { m1_u64 bits; double value; } input;
        input.bits = m1_from_double_cases[i].bits;
        check(r, 4, i, call_from_double(input.value) == signed_value(m1_from_double_cases[i].expected));
    }
    for (i = 0; i < COUNT(m1_to_double_cases); i++) {
        union { m1_u64 bits; double value; } output;
        output.value = call_to_double(signed_value(m1_to_double_cases[i].input));
        check(r, 5, i, output.bits == m1_to_double_cases[i].bits);
    }
}
