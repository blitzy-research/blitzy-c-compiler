/*
 * tests/conformance/07_variadics/002_mixed_int_double_promotions.c
 *
 * Area:  07_variadics (differential conformance corpus)
 * Focus: default argument promotions across the variadic boundary --
 *        float -> double, and narrow integers -> int.
 *
 * WHAT THIS PROGRAM ESTABLISHES
 *   An argument passed through the "..." of a variadic function undergoes the
 *   default argument promotions: a float argument is converted to double, and
 *   signed char, unsigned char, short, unsigned short and _Bool arguments are
 *   converted to int.  Arguments are therefore supplied here in the
 *   unpromoted types and retrieved in the promoted types.  Every retrieved
 *   value is printed on its own line, so a single divergent line localises a
 *   defect to one promotion rather than to "something in the program".
 *
 * PROMOTED RETRIEVAL TYPE -- THE CENTRAL RULE OF THIS PROGRAM
 *   va_arg must name the PROMOTED type, never the type written at the call
 *   site:
 *     - a supplied float is retrieved as va_arg(ap, double);
 *     - a supplied signed char, unsigned char, short, unsigned short or
 *       _Bool is retrieved as va_arg(ap, int).
 *   Naming an unpromoted type instead -- float, or any of the narrow integer
 *   types -- is undefined behaviour, because no argument of that type is ever
 *   present in the list, and this file therefore never does so.  Undefined
 *   behaviour would also invalidate the oracle this program serves: a
 *   divergence between two compilers on a program whose behaviour is
 *   undefined proves nothing about either compiler.  int is 32 bits on all
 *   four supported targets, so unsigned char (maximum 255) and unsigned short
 *   (maximum 65535) are fully representable in int and consequently promote
 *   to int, not to unsigned int.
 *
 * HEADER POLICY -- ONE SANCTIONED EXCEPTION
 *   <stdarg.h> is included because it is a freestanding header present both
 *   in bcc's own bundled set (see the include/stdarg.h row of
 *   docs/technical-specifications.md) and in the reference compiler's, so it
 *   compiles identically under both compilers and this file remains a
 *   single-file reproducer.  No other header is included: <stdio.h> is not,
 *   because bcc ships none and including it would manufacture a divergence
 *   caused by the test rather than by a compiler; <stdbool.h> is not, because
 *   _Bool is a C11 keyword that needs no header.  printf is hand-declared.
 *
 * TWO-VARIANT RULE
 *   Each helper is exercised twice: once with literal arguments, which lets
 *   the constant folder see through the call, and once with arguments read
 *   from volatile storage, which forces the backend to emit real loads and
 *   real argument set-up.  Without the second variant an optimizer could
 *   substitute the folder's answer for the backend's and a code-generation
 *   defect in the promotion path would escape detection entirely.
 *
 * DETERMINISM
 *   Output is 32 fixed lines; the exit status is 0.  No address, pointer,
 *   sizeof result, plain-char value, long value, timestamp or random value is
 *   printed, and every floating literal is a dyadic rational exactly
 *   representable in IEEE binary32 and binary64, so %.6f renders byte for
 *   byte identically on every target.
 *
 * The target list, optimization levels, shared flags, literal command
 * templates, undefined-behaviour argument and byte-exact expected stdout for
 * this program are recorded in the sibling record
 * 002_mixed_int_double_promotions.expected.
 */

#include <stdarg.h>

int printf(const char *, ...);

/*
 * Sum arguments that were supplied as float and are therefore present in the
 * argument list as double.  One line is printed per retrieved value; the
 * single derived value is returned so that main prints it.
 */
static double sum_doubles(const char *tag, int count, ...)
{
    va_list ap_d;
    int idx_d;
    double val_d;
    double acc_d;

    acc_d = 0.0;
    va_start(ap_d, count);
    for (idx_d = 0; idx_d < count; idx_d++) {
        /* Promoted type: the caller wrote float, the list holds double. */
        val_d = va_arg(ap_d, double);
        printf("%s[%d]=%.6f\n", tag, idx_d, val_d);
        acc_d += val_d;
    }
    va_end(ap_d);
    return acc_d;
}

/*
 * Sum arguments that were supplied as narrow integer types and are therefore
 * present in the argument list as int.  The largest total produced by this
 * file is 57196, well inside the range of the 32-bit int every target uses.
 */
static int sum_promoted_ints(const char *tag, int count, ...)
{
    va_list ap_n;
    int idx_n;
    int val_n;
    int acc_n;

    acc_n = 0;
    va_start(ap_n, count);
    for (idx_n = 0; idx_n < count; idx_n++) {
        /* Promoted type: signed char, unsigned char, short, unsigned short
           and _Bool all arrive as int. */
        val_n = va_arg(ap_n, int);
        printf("%s[%d]=%d\n", tag, idx_n, val_n);
        acc_n += val_n;
    }
    va_end(ap_n);
    return acc_n;
}

/*
 * Read n_int int arguments and then n_flt promoted-float arguments from one
 * argument list, exercising both promotion classes in a single call frame.
 * This helper has three named parameters, so its argument list is started from
 * the last of them, n_flt.  Because it derives two values rather than one it
 * prints both itself and returns void.
 */
static void mixed_args(const char *tag, int n_int, int n_flt, ...)
{
    va_list ap_m;
    int idx_mi;
    int idx_mf;
    int val_mi;
    double val_mf;
    int acc_mi;
    double acc_mf;

    acc_mi = 0;
    acc_mf = 0.0;
    va_start(ap_m, n_flt);
    for (idx_mi = 0; idx_mi < n_int; idx_mi++) {
        val_mi = va_arg(ap_m, int);
        printf("%s_int[%d]=%d\n", tag, idx_mi, val_mi);
        acc_mi += val_mi;
    }
    for (idx_mf = 0; idx_mf < n_flt; idx_mf++) {
        val_mf = va_arg(ap_m, double);
        printf("%s_flt[%d]=%.6f\n", tag, idx_mf, val_mf);
        acc_mf += val_mf;
    }
    printf("%s_int_sum=%d\n", tag, acc_mi);
    printf("%s_flt_sum=%.6f\n", tag, acc_mf);
    va_end(ap_m);
}

int main(void)
{
    /* Runtime-variant sources for call 2: plain float values in volatile
       storage, so the loads cannot be folded away. */
    volatile float vsrc_rflt_a = 0.25f;
    volatile float vsrc_rflt_b = 8.5f;
    volatile float vsrc_rflt_c = 3.0625f;
    /* Runtime-variant sources for call 4: one volatile object per narrow
       integer type, each holding the extreme value of that type. */
    volatile signed char vsrc_rnarrow_sc = (signed char)-128;
    volatile unsigned char vsrc_rnarrow_uc = (unsigned char)255;
    volatile short vsrc_rnarrow_sh = (short)-32768;
    volatile unsigned short vsrc_rnarrow_us = (unsigned short)65535;
    volatile _Bool vsrc_rnarrow_bo = (_Bool)1;
    /* Runtime-variant sources for call 6: two ints and two floats. */
    volatile int vsrc_rmix_i0 = 33;
    volatile int vsrc_rmix_i1 = 44;
    volatile float vsrc_rmix_f0 = 0.75f;
    volatile float vsrc_rmix_f1 = 1.5f;

    /* Each volatile object is read exactly once, into a plain local of the
       identical type, so the assignment itself performs no conversion and the
       promotion still happens at the call -- which is what is under test. */
    float plain_rflt_a;
    float plain_rflt_b;
    float plain_rflt_c;
    signed char plain_rnarrow_sc;
    unsigned char plain_rnarrow_uc;
    short plain_rnarrow_sh;
    unsigned short plain_rnarrow_us;
    _Bool plain_rnarrow_bo;
    int plain_rmix_i0;
    int plain_rmix_i1;
    float plain_rmix_f0;
    float plain_rmix_f1;

    /* Named temporaries for the helper return values: a helper call is never
       nested inside a printf argument list. */
    double total_fflt;
    double total_rflt;
    int total_fnarrow;
    int total_rnarrow;

    /* Call 1 -- folded: three float literals promoted to double at the call. */
    total_fflt = sum_doubles("fflt", 3, 1.5f, 2.25f, 4.125f);
    printf("fflt_sum=%.6f\n", total_fflt);

    /* Call 2 -- runtime: the same shape, every argument read from volatile
       storage. */
    plain_rflt_a = vsrc_rflt_a;
    plain_rflt_b = vsrc_rflt_b;
    plain_rflt_c = vsrc_rflt_c;
    total_rflt = sum_doubles("rflt", 3, plain_rflt_a, plain_rflt_b,
                             plain_rflt_c);
    printf("rflt_sum=%.6f\n", total_rflt);

    /* Call 3 -- folded: one argument of each narrow integer type, every
       literal explicitly cast so that no implicit narrowing occurs. */
    total_fnarrow = sum_promoted_ints("fnarrow", 5,
                                      (signed char)-5,
                                      (unsigned char)200,
                                      (short)-3000,
                                      (unsigned short)60000,
                                      (_Bool)1);
    printf("fnarrow_sum=%d\n", total_fnarrow);

    /* Call 4 -- runtime, at the boundary of each narrow type: -128 is the
       minimum signed char, 255 the maximum unsigned char, -32768 the minimum
       short and 65535 the maximum unsigned short.  These are the values at
       which a sign- or width-extension defect in the promotion path shows. */
    plain_rnarrow_sc = vsrc_rnarrow_sc;
    plain_rnarrow_uc = vsrc_rnarrow_uc;
    plain_rnarrow_sh = vsrc_rnarrow_sh;
    plain_rnarrow_us = vsrc_rnarrow_us;
    plain_rnarrow_bo = vsrc_rnarrow_bo;
    total_rnarrow = sum_promoted_ints("rnarrow", 5,
                                      plain_rnarrow_sc,
                                      plain_rnarrow_uc,
                                      plain_rnarrow_sh,
                                      plain_rnarrow_us,
                                      plain_rnarrow_bo);
    printf("rnarrow_sum=%d\n", total_rnarrow);

    /* Call 5 -- folded: both promotion classes in one argument list. */
    mixed_args("fmix", 2, 2, 11, 22, 1.25f, 2.5f);

    /* Call 6 -- runtime: the same mixed list read from volatile storage. */
    plain_rmix_i0 = vsrc_rmix_i0;
    plain_rmix_i1 = vsrc_rmix_i1;
    plain_rmix_f0 = vsrc_rmix_f0;
    plain_rmix_f1 = vsrc_rmix_f1;
    mixed_args("rmix", 2, 2, plain_rmix_i0, plain_rmix_i1, plain_rmix_f0,
               plain_rmix_f1);

    return 0;
}
