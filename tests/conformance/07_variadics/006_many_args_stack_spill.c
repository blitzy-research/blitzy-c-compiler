/*
 * Each group passes far more arguments of one class than any of the ABIs under test
 * documents argument registers for - twenty ints, twelve long longs, sixteen
 * doubles, and ten (int, double) pairs in a single call - so the paths those ABIs
 * use once their registers are exhausted are exercised rather than left unvisited.
 * How many arguments actually travel where is the ABI's decision; what this program
 * compares is only the values that come back out, one printed line per retrieved
 * argument, so a single divergent line identifies the exact argument that was
 * mishandled.  The interleaved group matters most, because both the integer and the
 * floating class run past their documented capacity within one call.
 *
 * Every group is exercised twice, once with literal arguments and once with
 * arguments that originate in volatile storage and so are not available for
 * compile-time substitution.
 *
 * <stdarg.h> is included because a variadic function cannot be written without it,
 * and it belongs to both compilers' freestanding header sets.  printf is
 * hand-declared, because bcc ships no stdio.h: its bundled set is the nine required
 * freestanding headers plus a bonus stdatomic.h, ten files in all
 * (docs/project-guide.md, "9 bundled freestanding headers"), and no standard I/O
 * header is among them.
 *
 * long is never used, because its width differs between the targets under test;
 * long long is 64 bits on all of them.  Every floating value is a dyadic rational
 * exactly representable in IEEE binary64, so %.6f renders the same bytes
 * everywhere, and no address, type size or plain-char value is printed.
 */

#include <stdarg.h>

int printf(const char *, ...);

/*
 * Retrieve and report `count` int arguments.  int is not subject to the default
 * argument promotions, so va_arg with type int retrieves exactly what was
 * supplied.  The running total is accumulated in a long long: widening an int to
 * long long is value-preserving and signedness-preserving, so it needs no cast
 * and cannot overflow for the argument counts used here.
 */
static long long sum_many_ints(const char *tag, int count, ...)
{
    va_list ap_mi;
    long long acc_mi;
    int idx_mi;
    int val_mi;

    acc_mi = 0;
    va_start(ap_mi, count);
    for (idx_mi = 0; idx_mi < count; idx_mi++) {
        val_mi = va_arg(ap_mi, int);
        printf("%s[%d]=%d\n", tag, idx_mi, val_mi);
        acc_mi += val_mi;
    }
    va_end(ap_mi);

    return acc_mi;
}

/*
 * long long is 64 bits on every target under test, which is why it is used here
 * in place of long, whose width differs between them.
 */
static long long sum_many_llongs(const char *tag, int count, ...)
{
    va_list ap_ml;
    long long acc_ml;
    long long val_ml;
    int idx_ml;

    acc_ml = 0;
    va_start(ap_ml, count);
    for (idx_ml = 0; idx_ml < count; idx_ml++) {
        val_ml = va_arg(ap_ml, long long);
        printf("%s[%d]=%lld\n", tag, idx_ml, val_ml);
        acc_ml += val_ml;
    }
    va_end(ap_ml);

    return acc_ml;
}

/*
 * Retrieve and report `count` double arguments.  The retrieval type is double,
 * never float: a float argument to a variadic function is promoted to double by
 * the default argument promotions, so double is the only correct retrieval type.
 */
static double sum_many_doubles(const char *tag, int count, ...)
{
    va_list ap_md;
    double acc_md;
    double val_md;
    int idx_md;

    acc_md = 0.0;
    va_start(ap_md, count);
    for (idx_md = 0; idx_md < count; idx_md++) {
        val_md = va_arg(ap_md, double);
        printf("%s[%d]=%.6f\n", tag, idx_md, val_md);
        acc_md += val_md;
    }
    va_end(ap_md);

    return acc_md;
}

/*
 * Retrieve and report `pairs` (int, double) pairs.  Within each iteration the
 * int is retrieved first and the double second, matching the order in which the
 * caller supplies them; retrieving them in any other order would be undefined
 * behaviour.  This helper produces two derived values rather than one, so it
 * prints them itself and returns void, keeping the corpus rule that a caller
 * never nests a side-effecting call inside a printf argument list.
 */
static void sum_interleaved(const char *tag, int pairs, ...)
{
    va_list ap_iv;
    double acc_dbl_iv;
    double val_dbl_iv;
    int acc_int_iv;
    int idx_iv;
    int val_int_iv;

    acc_int_iv = 0;
    acc_dbl_iv = 0.0;
    va_start(ap_iv, pairs);
    for (idx_iv = 0; idx_iv < pairs; idx_iv++) {
        val_int_iv = va_arg(ap_iv, int);
        val_dbl_iv = va_arg(ap_iv, double);
        printf("%s[%d]=%d,%.6f\n", tag, idx_iv, val_int_iv, val_dbl_iv);
        acc_int_iv += val_int_iv;
        acc_dbl_iv += val_dbl_iv;
    }
    va_end(ap_iv);

    printf("%s_isum=%d\n", tag, acc_int_iv);
    printf("%s_dsum=%.6f\n", tag, acc_dbl_iv);
}

int main(void)
{
    /*
     * Each group owns a volatile array that is written in one loop and read back
     * into a plain array in a second, so the values are not available for
     * compile-time substitution while each call site contains nothing but plain
     * array reads - which keeps the unspecified order of argument evaluation
     * unobservable.  Every element of every volatile array is written before it is
     * read.
     */
    volatile int vsrc_ri[20];
    volatile long long vsrc_rl[12];
    volatile double vsrc_rd[16];
    volatile int vsrc_rmi[10];
    volatile double vsrc_rmd[10];
    int plain_ri[20];
    long long plain_rl[12];
    double plain_rd[16];
    int plain_rmi[10];
    double plain_rmd[10];
    long long res_fi;
    long long res_ri;
    long long res_fl;
    long long res_rl;
    double res_fd;
    double res_rd;
    int k_ri;
    int k_rl;
    int k_rd;
    int k_rm;

    /* Twenty int arguments - more than any of these ABIs documents integer
     * argument registers for, so the beyond-registers path is reached. */
    res_fi = sum_many_ints("fint", 20,
                           1000, 1001, 1002, 1003,
                           1004, 1005, 1006, 1007,
                           1008, 1009, 1010, 1011,
                           1012, 1013, 1014, 1015,
                           1016, 1017, 1018, 1019);
    printf("fint_sum=%lld\n", res_fi);

    for (k_ri = 0; k_ri < 20; k_ri++) {
        vsrc_ri[k_ri] = 2000 + k_ri;
    }
    for (k_ri = 0; k_ri < 20; k_ri++) {
        plain_ri[k_ri] = vsrc_ri[k_ri];
    }
    res_ri = sum_many_ints("rint", 20,
                           plain_ri[0],  plain_ri[1],  plain_ri[2],  plain_ri[3],
                           plain_ri[4],  plain_ri[5],  plain_ri[6],  plain_ri[7],
                           plain_ri[8],  plain_ri[9],  plain_ri[10], plain_ri[11],
                           plain_ri[12], plain_ri[13], plain_ri[14], plain_ri[15],
                           plain_ri[16], plain_ri[17], plain_ri[18], plain_ri[19]);
    printf("rint_sum=%lld\n", res_ri);

    /* Twelve long long arguments, each needing a full 64-bit slot however the ABI
     * chooses to provide one. */
    res_fl = sum_many_llongs("fll", 12,
                             1000000000000LL, 1000000000001LL,
                             1000000000002LL, 1000000000003LL,
                             1000000000004LL, 1000000000005LL,
                             1000000000006LL, 1000000000007LL,
                             1000000000008LL, 1000000000009LL,
                             1000000000010LL, 1000000000011LL);
    printf("fll_sum=%lld\n", res_fl);

    for (k_rl = 0; k_rl < 12; k_rl++) {
        vsrc_rl[k_rl] = 3000000000000LL + (long long)k_rl;
    }
    for (k_rl = 0; k_rl < 12; k_rl++) {
        plain_rl[k_rl] = vsrc_rl[k_rl];
    }
    res_rl = sum_many_llongs("rll", 12,
                             plain_rl[0],  plain_rl[1],  plain_rl[2],
                             plain_rl[3],  plain_rl[4],  plain_rl[5],
                             plain_rl[6],  plain_rl[7],  plain_rl[8],
                             plain_rl[9],  plain_rl[10], plain_rl[11]);
    printf("rll_sum=%lld\n", res_rl);

    /* Sixteen double arguments - twice the floating-point argument registers any
     * of these ABIs documents. */
    res_fd = sum_many_doubles("fdbl", 16,
                              1.0, 1.5, 2.0, 2.5,
                              3.0, 3.5, 4.0, 4.5,
                              5.0, 5.5, 6.0, 6.5,
                              7.0, 7.5, 8.0, 8.5);
    printf("fdbl_sum=%.6f\n", res_fd);

    for (k_rd = 0; k_rd < 16; k_rd++) {
        vsrc_rd[k_rd] = 100.0 + 0.5 * (double)k_rd;
    }
    for (k_rd = 0; k_rd < 16; k_rd++) {
        plain_rd[k_rd] = vsrc_rd[k_rd];
    }
    res_rd = sum_many_doubles("rdbl", 16,
                              plain_rd[0],  plain_rd[1],  plain_rd[2],  plain_rd[3],
                              plain_rd[4],  plain_rd[5],  plain_rd[6],  plain_rd[7],
                              plain_rd[8],  plain_rd[9],  plain_rd[10], plain_rd[11],
                              plain_rd[12], plain_rd[13], plain_rd[14], plain_rd[15]);
    printf("rdbl_sum=%.6f\n", res_rd);

    /* Ten (int, double) pairs in one call, so both argument classes run past their
     * documented capacity at the same time.  The helper prints its own two derived
     * lines, so main prints nothing for it. */
    sum_interleaved("fmix", 10,
                    10, 0.25, 11, 0.5,
                    12, 0.75, 13, 1.0,
                    14, 1.25, 15, 1.5,
                    16, 1.75, 17, 2.0,
                    18, 2.25, 19, 2.5);

    for (k_rm = 0; k_rm < 10; k_rm++) {
        vsrc_rmi[k_rm] = 50 + k_rm;
        vsrc_rmd[k_rm] = 0.5 * (double)(k_rm + 1);
    }
    for (k_rm = 0; k_rm < 10; k_rm++) {
        plain_rmi[k_rm] = vsrc_rmi[k_rm];
        plain_rmd[k_rm] = vsrc_rmd[k_rm];
    }
    sum_interleaved("rmix", 10,
                    plain_rmi[0], plain_rmd[0], plain_rmi[1], plain_rmd[1],
                    plain_rmi[2], plain_rmd[2], plain_rmi[3], plain_rmd[3],
                    plain_rmi[4], plain_rmd[4], plain_rmi[5], plain_rmd[5],
                    plain_rmi[6], plain_rmd[6], plain_rmi[7], plain_rmd[7],
                    plain_rmi[8], plain_rmd[8], plain_rmi[9], plain_rmd[9]);

    return 0;
}
