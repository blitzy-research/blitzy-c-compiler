/*
 * tests/conformance/07_variadics/005_va_list_forwarding.c
 *
 * Area:  07_variadics
 * Focus: argument list forwarded to a second variadic-consuming function.
 *
 * SANCTIONED HEADER EXCEPTION
 * ---------------------------
 * This program includes <stdarg.h>, one of only two header exceptions in the
 * conformance corpus. The exception is required rather than convenient: the
 * va_list parameter type of a "v"-style callee cannot be spelled at all
 * without the header. <stdarg.h> is in bcc's own bundled freestanding set
 * (docs/technical-specifications.md line 208: va_list, va_start, va_arg,
 * va_end and va_copy, implemented via compiler builtins) and is equally a
 * freestanding header of the reference compiler, so it compiles identically
 * under both oracles and this file remains a single-file reproducer.
 *
 * No other header is included. printf is hand-declared because bcc ships no
 * standard I/O header at all: the bundled set is exactly nine headers
 * (docs/technical-specifications.md lines 19 and 636) and the standard I/O
 * header is not among them, so including it would fail against bcc while
 * succeeding against the reference compiler -- a divergence caused by the test
 * rather than by the compiler.
 *
 * FORWARDING SEQUENCING RULES OBEYED HERE (C11 7.16.1)
 * ----------------------------------------------------
 * 1. A va_list is passed BY VALUE to a callee whose parameter type is
 *    va_list. That callee reads the list with va_arg and does nothing else
 *    to it.
 * 2. Once the callee has invoked va_arg, the caller's list is INDETERMINATE.
 *    va_end on it is still legal and is still required; va_arg on it is not
 *    legal and appears nowhere in this file after a forward. The three
 *    variadic wrappers below therefore contain zero va_arg invocations.
 * 3. A "v"-style callee never calls va_end. va_end must be invoked by the
 *    same function that invoked va_start for that list, and the callee
 *    neither creates nor destroys the list.
 * 4. A "v"-style callee never calls va_start or va_copy on its parameter,
 *    never assigns to that parameter, and never takes its address.
 * 5. Forwarding is chained here: fwd_doubles (variadic, owns the list) ->
 *    vmid_doubles (forwards it on, unread) -> vsum_doubles (reads it). After
 *    the innermost read the list is indeterminate in both outer frames, and
 *    only the frame that called va_start calls va_end.
 * This file contains no va_copy at all; multi-pass traversal of one argument
 * list is the subject of 004_va_copy_multiple_passes.c.
 *
 * WHY THIS PROGRAM IS ABI-REVEALING
 * ---------------------------------
 * On x86-64 System V, va_list is an array type -- a one-element array of a
 * register-save-area descriptor -- so passing it "by value" decays to passing
 * a pointer, and the callee's va_arg mutates the caller's object. On i686
 * cdecl, va_list is effectively a plain char * passed genuinely by value, so
 * the callee's va_arg does not mutate the caller's object. That difference in
 * representation is precisely why the standard declares the caller's list
 * indeterminate after a forward, and precisely why this file never re-reads a
 * forwarded list. Written this way the program is conforming on all four
 * supported ABIs -- x86-64 System V, i686 cdecl, AArch64 AAPCS64 and RISC-V 64
 * LP64D (docs/technical-specifications.md lines 546, 553, 559 and 565) -- and
 * must print identical bytes on every one of them, so any cross-backend
 * difference observed here is a genuine finding rather than an
 * implementation-defined difference.
 *
 * TWO-VARIANT RULE
 * ----------------
 * Every forwarding case is exercised twice: once with literal arguments,
 * which the constant folder may evaluate at translation time, and once with
 * arguments read out of volatile storage, which forces the backend to marshal
 * them at run time. Without the second variant the folder's answer would be
 * substituted for the backend's and a code-generation defect in variadic
 * argument marshalling would escape detection entirely.
 *
 * DETERMINISM
 * -----------
 * Only the %s, %d and %.6f conversions are used. No plain char, no long, no
 * pointer value and nothing derived from a va_list object itself is ever
 * printed. Every floating literal here (1.5, 2.25, 4.125, 0.25, 8.5, 3.0625,
 * 2.5 and 0.125) is a dyadic rational exactly representable in IEEE binary64,
 * so %.6f renders the same bytes on every target. All input is literal in
 * this file: no network access, no file access, no command-line arguments, no
 * environment, no clock and no randomness. The program prints 30 lines and
 * exits 0.
 *
 * The target list, the optimization levels, the shared flag set, the literal
 * command templates, the undefined-behaviour argument and the byte-exact
 * expected stdout are recorded together in the sibling expectation record
 * 005_va_list_forwarding.expected.
 */

#include <stdarg.h>

int printf(const char *, ...);

/*
 * The four "v"-style consumers. Each takes an already-started argument list
 * as a va_list parameter. None of them starts, copies or ends that list.
 */
static int vsum_ints(const char *tag, int count, va_list ap_in);
static double vsum_doubles(const char *tag, int count, va_list ap_in);
static double vmid_doubles(const char *tag, int count, va_list ap_in);
static int vsum_mixed(const char *tag, int n_int, int n_dbl, va_list ap_in);

/*
 * The three variadic wrappers. Each one starts a list, forwards it by value
 * to a consumer, then ends it. None of them reads the list with va_arg.
 */
static int fwd_ints(const char *tag, int count, ...);
static double fwd_doubles(const char *tag, int count, ...);
static int fwd_mixed(const char *tag, int n_int, int n_dbl, ...);

/*
 * Innermost consumer for the integer cases. Retrieves exactly count int
 * arguments -- int is not subject to the default argument promotions, so
 * retrieving as int yields precisely what the caller supplied -- prints one
 * line per argument, and returns their sum for the wrapper to hand back.
 */
static int vsum_ints(const char *tag, int count, va_list ap_in)
{
    int idx_vi;
    int val_vi;
    int acc_vi;

    acc_vi = 0;
    for (idx_vi = 0; idx_vi < count; idx_vi++) {
        val_vi = va_arg(ap_in, int);
        printf("%s[%d]=%d\n", tag, idx_vi, val_vi);
        acc_vi = acc_vi + val_vi;
    }
    return acc_vi;
}

/*
 * Innermost consumer for the floating cases. Retrieves exactly count double
 * arguments. The supplied arguments are double literals or plain double
 * objects, so double is the correct retrieval type; retrieving as float would
 * be wrong and is never done anywhere in this file.
 */
static double vsum_doubles(const char *tag, int count, va_list ap_in)
{
    int idx_vd;
    double val_vd;
    double acc_vd;

    acc_vd = 0.0;
    for (idx_vd = 0; idx_vd < count; idx_vd++) {
        val_vd = va_arg(ap_in, double);
        printf("%s[%d]=%.6f\n", tag, idx_vd, val_vd);
        acc_vd = acc_vd + val_vd;
    }
    return acc_vd;
}

/*
 * The middle forwarder, and the reason this program tests two-level
 * forwarding rather than one. It receives the list by value and passes it
 * straight on, unread: it invokes no va_arg, no va_start, no va_copy and no
 * va_end, and it declares no local objects. All three of its parameters are
 * consumed by the single call it makes.
 */
static double vmid_doubles(const char *tag, int count, va_list ap_in)
{
    return vsum_doubles(tag, count, ap_in);
}

/*
 * Innermost consumer for the mixed cases. Retrieves n_int int arguments and
 * then n_dbl double arguments from one list, in that order, printing one line
 * per argument. It derives two values rather than one, so it prints the
 * floating sum itself and returns the integer sum for main to print; the
 * resulting line order is every _i line, then every _d line, then _dsum,
 * then _isum. The integer accumulator is only ever combined with int values
 * and the floating accumulator only ever with double values, so no implicit
 * conversion arises.
 */
static int vsum_mixed(const char *tag, int n_int, int n_dbl, va_list ap_in)
{
    int idx_mi;
    int idx_md;
    int val_mi;
    double val_md;
    int acc_mi;
    double acc_md;

    acc_mi = 0;
    for (idx_mi = 0; idx_mi < n_int; idx_mi++) {
        val_mi = va_arg(ap_in, int);
        printf("%s_i[%d]=%d\n", tag, idx_mi, val_mi);
        acc_mi = acc_mi + val_mi;
    }
    acc_md = 0.0;
    for (idx_md = 0; idx_md < n_dbl; idx_md++) {
        val_md = va_arg(ap_in, double);
        printf("%s_d[%d]=%.6f\n", tag, idx_md, val_md);
        acc_md = acc_md + val_md;
    }
    printf("%s_dsum=%.6f\n", tag, acc_md);
    return acc_mi;
}

/*
 * Variadic wrapper for the integer cases. It starts the list on its last
 * named parameter, count, forwards the list by value to vsum_ints, and only
 * then ends it. Its own list is indeterminate from the moment vsum_ints
 * invokes va_arg, so nothing here reads it afterwards.
 */
static int fwd_ints(const char *tag, int count, ...)
{
    va_list ap_fi;
    int res_fi;

    va_start(ap_fi, count);
    res_fi = vsum_ints(tag, count, ap_fi);
    va_end(ap_fi);
    return res_fi;
}

/*
 * Variadic wrapper for the floating cases, and the one that builds the
 * two-level chain: it forwards to vmid_doubles, which forwards again to
 * vsum_doubles. The list is indeterminate in this frame and in the middle
 * frame once the innermost consumer has read it; only this frame, which
 * called va_start, calls va_end.
 */
static double fwd_doubles(const char *tag, int count, ...)
{
    va_list ap_fd;
    double res_fd;

    va_start(ap_fd, count);
    res_fd = vmid_doubles(tag, count, ap_fd);
    va_end(ap_fd);
    return res_fd;
}

/*
 * Variadic wrapper for the mixed cases. This wrapper has three named
 * parameters, so the list is started on n_dbl -- the last named one -- and
 * not on tag or n_int.
 */
static int fwd_mixed(const char *tag, int n_int, int n_dbl, ...)
{
    va_list ap_fm;
    int res_fm;

    va_start(ap_fm, n_dbl);
    res_fm = vsum_mixed(tag, n_int, n_dbl, ap_fm);
    va_end(ap_fm);
    return res_fm;
}

/*
 * Six cases in a fixed order: integer, floating and mixed forwarding, each in
 * a folded variant whose arguments are literals and a runtime variant whose
 * arguments originate in volatile storage. Each volatile source is read once
 * into a plain local of the same type by a single simple assignment, and the
 * plain locals are what the wrapper is called with, so no call expression
 * contains more than one volatile access and the order of the volatile reads
 * is fully determined. Every declaration sits at the top of the block, and
 * every wrapper result is stored in a named object before it is printed, so
 * no wrapper call is nested inside a printf argument list.
 *
 * Verified sums: 6-4+15-8 = 9; 12-5+30-16 = 21; 1.5+2.25+4.125 = 7.875;
 * 0.25+8.5+3.0625 = 11.8125; 3-4 = -1 with 1.5+0.25 = 1.75; and
 * 9-10 = -1 with 2.5+0.125 = 2.625.
 */
int main(void)
{
    volatile int vsrc_fi0 = 12;
    volatile int vsrc_fi1 = -5;
    volatile int vsrc_fi2 = 30;
    volatile int vsrc_fi3 = -16;
    volatile double vsrc_fd0 = 0.25;
    volatile double vsrc_fd1 = 8.5;
    volatile double vsrc_fd2 = 3.0625;
    volatile int vsrc_fm_i0 = 9;
    volatile int vsrc_fm_i1 = -10;
    volatile double vsrc_fm_d0 = 2.5;
    volatile double vsrc_fm_d1 = 0.125;
    int plain_fi0;
    int plain_fi1;
    int plain_fi2;
    int plain_fi3;
    double plain_fd0;
    double plain_fd1;
    double plain_fd2;
    int plain_fm_i0;
    int plain_fm_i1;
    double plain_fm_d0;
    double plain_fm_d1;
    int res_ffwd_int;
    int res_rfwd_int;
    double res_ffwd_dbl;
    double res_rfwd_dbl;
    int res_fmix;
    int res_rmix;

    /* Case 1 -- folded integer forwarding. */
    res_ffwd_int = fwd_ints("ffwd_int", 4, 6, -4, 15, -8);
    printf("ffwd_int_sum=%d\n", res_ffwd_int);

    /* Case 2 -- runtime integer forwarding. */
    plain_fi0 = vsrc_fi0;
    plain_fi1 = vsrc_fi1;
    plain_fi2 = vsrc_fi2;
    plain_fi3 = vsrc_fi3;
    res_rfwd_int = fwd_ints("rfwd_int", 4, plain_fi0, plain_fi1, plain_fi2,
                            plain_fi3);
    printf("rfwd_int_sum=%d\n", res_rfwd_int);

    /* Case 3 -- folded floating forwarding, through the middle forwarder. */
    res_ffwd_dbl = fwd_doubles("ffwd_dbl", 3, 1.5, 2.25, 4.125);
    printf("ffwd_dbl_sum=%.6f\n", res_ffwd_dbl);

    /* Case 4 -- runtime floating forwarding, through the middle forwarder. */
    plain_fd0 = vsrc_fd0;
    plain_fd1 = vsrc_fd1;
    plain_fd2 = vsrc_fd2;
    res_rfwd_dbl = fwd_doubles("rfwd_dbl", 3, plain_fd0, plain_fd1, plain_fd2);
    printf("rfwd_dbl_sum=%.6f\n", res_rfwd_dbl);

    /* Case 5 -- folded mixed forwarding; the consumer prints fmix_dsum. */
    res_fmix = fwd_mixed("fmix", 2, 2, 3, -4, 1.5, 0.25);
    printf("fmix_isum=%d\n", res_fmix);

    /* Case 6 -- runtime mixed forwarding; the consumer prints rmix_dsum. */
    plain_fm_i0 = vsrc_fm_i0;
    plain_fm_i1 = vsrc_fm_i1;
    plain_fm_d0 = vsrc_fm_d0;
    plain_fm_d1 = vsrc_fm_d1;
    res_rmix = fwd_mixed("rmix", 2, 2, plain_fm_i0, plain_fm_i1, plain_fm_d0,
                         plain_fm_d1);
    printf("rmix_isum=%d\n", res_rmix);

    return 0;
}
