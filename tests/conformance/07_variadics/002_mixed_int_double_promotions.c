/*
 * An argument passed through the "..." of a variadic function undergoes the
 * default argument promotions, so a float arrives as double and signed char,
 * unsigned char, short, unsigned short and _Bool all arrive as int.  Arguments
 * are therefore supplied here in the unpromoted types and retrieved in the
 * promoted ones: va_arg must name the promoted type, because naming the type
 * written at the call site would be undefined behaviour - no argument of that
 * type is present in the list - and a divergence on an undefined program would
 * prove nothing about either compiler.
 *
 * <stdarg.h> is included because a variadic function cannot be written without
 * it, and it belongs to both compilers' freestanding header sets.  printf is
 * hand-declared, because bcc ships no stdio.h.
 *
 * Every floating literal is a dyadic rational exactly representable in IEEE
 * binary32 and binary64, so %.6f renders the same bytes everywhere.
 */

#include <stdarg.h>

int printf(const char *, ...);

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
 * file is 57196, well inside the range of int on every target under test.
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

/* va_start names the last named parameter, which here is n_flt. */
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
    volatile float vsrc_rflt_a = 0.25f;
    volatile float vsrc_rflt_b = 8.5f;
    volatile float vsrc_rflt_c = 3.0625f;
    volatile signed char vsrc_rnarrow_sc = (signed char)-128;
    volatile unsigned char vsrc_rnarrow_uc = (unsigned char)255;
    volatile short vsrc_rnarrow_sh = (short)-32768;
    volatile unsigned short vsrc_rnarrow_us = (unsigned short)65535;
    volatile _Bool vsrc_rnarrow_bo = (_Bool)1;
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

    total_fflt = sum_doubles("fflt", 3, 1.5f, 2.25f, 4.125f);
    printf("fflt_sum=%.6f\n", total_fflt);

    plain_rflt_a = vsrc_rflt_a;
    plain_rflt_b = vsrc_rflt_b;
    plain_rflt_c = vsrc_rflt_c;
    total_rflt = sum_doubles("rflt", 3, plain_rflt_a, plain_rflt_b,
                             plain_rflt_c);
    printf("rflt_sum=%.6f\n", total_rflt);

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

    mixed_args("fmix", 2, 2, 11, 22, 1.25f, 2.5f);

    plain_rmix_i0 = vsrc_rmix_i0;
    plain_rmix_i1 = vsrc_rmix_i1;
    plain_rmix_f0 = vsrc_rmix_f0;
    plain_rmix_f1 = vsrc_rmix_f1;
    mixed_args("rmix", 2, 2, plain_rmix_i0, plain_rmix_i1, plain_rmix_f0,
               plain_rmix_f1);

    return 0;
}
