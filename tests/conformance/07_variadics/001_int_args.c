/*
 * tests/conformance/07_variadics/001_int_args.c
 *
 * Area:  07_variadics
 * Focus: integer variadic arguments retrieved in order.
 *
 * Header exception: this area is one of only two places in the whole corpus that
 * includes a header. <stdarg.h> is unavoidable here because a variadic function
 * cannot be written at all without va_list, va_start, va_arg and va_end, and
 * stdarg.h is part of bcc's bundled freestanding header set as well as the
 * reference compiler's, so it resolves identically under both oracles and the
 * program stays a single-file reproducer. No other header is included: printf is
 * hand-declared below because bcc ships no stdio.h, and including one would
 * manufacture a divergence caused by the test rather than by the compiler.
 *
 * Two-variant rule: each of the three retrieval types is exercised twice. Once
 * with compile-time literal arguments (folded variant, tags "fint"/"fuint"/
 * "fllong"), which exercises the constant folder and the compile-time argument
 * setup path; and once with arguments that originate in volatile storage
 * (runtime variant, tags "rint"/"ruint"/"rllong"), which forces the backend to
 * emit real argument-setup instructions. Without the runtime variant a folded
 * answer could silently stand in for the backend's answer.
 *
 * Every value is a literal in this source: no input, no file, no clock, no
 * randomness. Output is one line per retrieved argument plus one line per
 * computed sum, so a single divergent line localizes the defect to exactly one
 * retrieved argument.
 *
 * The reproduction commands, the target and optimization matrix, and the golden
 * stdout are recorded in the sibling expectation record
 * tests/conformance/07_variadics/001_int_args.expected.
 */

#include <stdarg.h>

int printf(const char *, ...);

/*
 * Retrieve and sum `count` variadic arguments of type int, printing one line per
 * retrieved argument.
 *
 * `int` is not subject to the default argument promotions, so va_arg(_, int)
 * retrieves exactly the type the caller supplied. The accumulator is long long
 * so no partial sum can overflow the int range; int -> long long is a widening,
 * same-signedness conversion, which needs no cast and raises no -Wconversion or
 * -Wsign-conversion diagnostic.
 */
static long long sum_ints(const char *tag, int count, ...)
{
    va_list ap_ints;
    long long acc_ints;
    int idx_ints;
    int val_ints;

    acc_ints = 0;
    va_start(ap_ints, count);
    for (idx_ints = 0; idx_ints < count; idx_ints++) {
        val_ints = va_arg(ap_ints, int);
        printf("%s[%d]=%d\n", tag, idx_ints, val_ints);
        acc_ints += val_ints;
    }
    va_end(ap_ints);

    return acc_ints;
}

/*
 * Retrieve and sum `count` variadic arguments of type unsigned int.
 *
 * unsigned int shares int's rank, so it too escapes the default argument
 * promotions and va_arg(_, unsigned int) matches the supplied type exactly.
 * Accumulating into unsigned long long keeps the running total exact for the
 * near-UINT_MAX operands used below and is again a widening, same-signedness
 * conversion.
 */
static unsigned long long sum_uints(const char *tag, int count, ...)
{
    va_list ap_uints;
    unsigned long long acc_uints;
    int idx_uints;
    unsigned int val_uints;

    acc_uints = 0u;
    va_start(ap_uints, count);
    for (idx_uints = 0; idx_uints < count; idx_uints++) {
        val_uints = va_arg(ap_uints, unsigned int);
        printf("%s[%d]=%u\n", tag, idx_uints, val_uints);
        acc_uints += val_uints;
    }
    va_end(ap_uints);

    return acc_uints;
}

/*
 * Retrieve and sum `count` variadic arguments of type long long.
 *
 * This is the widest integer retrieval the suite performs and the one where the
 * four targets differ most in how the argument is placed: on i686 a long long
 * variadic argument occupies two stack words, while the three 64-bit targets
 * pass it in a single register or slot. The operands below are chosen so every
 * partial sum stays far inside the 64-bit signed range, so no signed overflow is
 * possible.
 */
static long long sum_llongs(const char *tag, int count, ...)
{
    va_list ap_llongs;
    long long acc_llongs;
    int idx_llongs;
    long long val_llongs;

    acc_llongs = 0;
    va_start(ap_llongs, count);
    for (idx_llongs = 0; idx_llongs < count; idx_llongs++) {
        val_llongs = va_arg(ap_llongs, long long);
        printf("%s[%d]=%lld\n", tag, idx_llongs, val_llongs);
        acc_llongs += val_llongs;
    }
    va_end(ap_llongs);

    return acc_llongs;
}

int main(void)
{
    /*
     * Runtime-variant sources. Because these objects are volatile the compiler
     * may not assume they still hold their initializers, so the calls below
     * cannot be constant folded.
     */
    volatile int vsrc_rint_a = 5;
    volatile int vsrc_rint_b = 9;
    volatile int vsrc_rint_c = -20;
    volatile int vsrc_rint_d = 400;
    volatile int vsrc_rint_e = 606;
    volatile unsigned int vsrc_ruint_a = 2u;
    volatile unsigned int vsrc_ruint_b = 3000000000u;
    volatile unsigned int vsrc_ruint_c = 4u;
    volatile unsigned int vsrc_ruint_d = 65536u;
    volatile long long vsrc_rllong_a = 2000000000000LL;
    volatile long long vsrc_rllong_b = 500000000000LL;
    volatile long long vsrc_rllong_c = -7LL;

    /*
     * Plain copies of the volatile sources. Each volatile object is read exactly
     * once, in a statement of its own, and only these plain locals are passed as
     * arguments. Reading the volatiles inside the call expression instead would
     * make the order of the volatile accesses depend on the unspecified order of
     * argument evaluation; copying first keeps that order unobservable while the
     * values remain unfoldable.
     */
    int plain_rint_a;
    int plain_rint_b;
    int plain_rint_c;
    int plain_rint_d;
    int plain_rint_e;
    unsigned int plain_ruint_a;
    unsigned int plain_ruint_b;
    unsigned int plain_ruint_c;
    unsigned int plain_ruint_d;
    long long plain_rllong_a;
    long long plain_rllong_b;
    long long plain_rllong_c;

    /*
     * Each helper prints its own per-argument lines and returns the sum; the sum
     * is held in a named temporary and printed by a separate statement, so no
     * call is ever nested inside a printf argument list.
     */
    long long total_fint;
    long long total_rint;
    unsigned long long total_fuint;
    unsigned long long total_ruint;
    long long total_fllong;
    long long total_rllong;

    /* 1. int, folded: 3 + -7 + 11 + 250 + -1000 == -743 */
    total_fint = sum_ints("fint", 5, 3, -7, 11, 250, -1000);
    printf("fint_sum=%lld\n", total_fint);

    /* 2. int, runtime: 5 + 9 + -20 + 400 + 606 == 1000 */
    plain_rint_a = vsrc_rint_a;
    plain_rint_b = vsrc_rint_b;
    plain_rint_c = vsrc_rint_c;
    plain_rint_d = vsrc_rint_d;
    plain_rint_e = vsrc_rint_e;
    total_rint = sum_ints("rint", 5, plain_rint_a, plain_rint_b, plain_rint_c,
                          plain_rint_d, plain_rint_e);
    printf("rint_sum=%lld\n", total_rint);

    /* 3. unsigned int, folded: 1 + 4000000000 + 7 + 65535 == 4000065543 */
    total_fuint = sum_uints("fuint", 4, 1u, 4000000000u, 7u, 65535u);
    printf("fuint_sum=%llu\n", total_fuint);

    /* 4. unsigned int, runtime: 2 + 3000000000 + 4 + 65536 == 3000065542 */
    plain_ruint_a = vsrc_ruint_a;
    plain_ruint_b = vsrc_ruint_b;
    plain_ruint_c = vsrc_ruint_c;
    plain_ruint_d = vsrc_ruint_d;
    total_ruint = sum_uints("ruint", 4, plain_ruint_a, plain_ruint_b,
                            plain_ruint_c, plain_ruint_d);
    printf("ruint_sum=%llu\n", total_ruint);

    /* 5. long long, folded: 1000000000000 + -250000000000 + 3 == 750000000003 */
    total_fllong = sum_llongs("fllong", 3, 1000000000000LL, -250000000000LL, 3LL);
    printf("fllong_sum=%lld\n", total_fllong);

    /* 6. long long, runtime: 2000000000000 + 500000000000 + -7 == 2499999999993 */
    plain_rllong_a = vsrc_rllong_a;
    plain_rllong_b = vsrc_rllong_b;
    plain_rllong_c = vsrc_rllong_c;
    total_rllong = sum_llongs("rllong", 3, plain_rllong_a, plain_rllong_b,
                              plain_rllong_c);
    printf("rllong_sum=%lld\n", total_rllong);

    return 0;
}
