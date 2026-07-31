/*
 * tests/conformance/07_variadics/004_va_copy_multiple_passes.c
 *
 * Differential conformance corpus, area 07_variadics, program 004.
 * Focus: an argument list copied and traversed more than once.
 *
 * Sanctioned header exception. This area may include <stdarg.h>. It is one of
 * the nine freestanding headers bcc bundles, and line 208 of
 * docs/technical-specifications.md documents va_list, va_start, va_arg, va_end
 * and va_copy as compiler builtins -- va_copy included, which is precisely the
 * facility exercised here.
 * The reference compiler supplies the same freestanding header, so both oracles
 * see identical declarations and the program stays a single-file reproducer.
 * No other header is included: printf is hand-declared because bcc ships no
 * <stdio.h>, so including one would fail under bcc while succeeding under the
 * reference compiler, producing a divergence caused by the test rather than by
 * the compiler.
 *
 * va_copy lifecycle rules obeyed throughout (C11 7.16.1):
 *   - every va_copy destination gets its own va_end;
 *   - each list created by va_start is va_end'ed exactly once;
 *   - a copy is always taken before its source list has been traversed, so
 *     source and copy start from the same position;
 *   - a copy of a copy is legal: three_passes takes cp2_th from cp1_th;
 *   - a list may be va_end'ed without having consumed all of its arguments,
 *     which head_tail relies on by ending its copy after reading only the
 *     first three of six arguments;
 *   - no list is ever read past the number of arguments actually supplied;
 *   - every va_end sits in the same function as its va_start or va_copy.
 * Macro budget for this file: 3 va_start, 4 va_copy, 7 va_end.
 *
 * Two-variant rule. Every helper is exercised once with literal arguments,
 * which the constant folder is free to evaluate at compile time, and once with
 * arguments that originate in volatile storage and are copied into plain
 * locals of the same type, which forces the backend to emit real loads and
 * real argument marshalling. Without the second variant, optimization could
 * substitute the folder's answer for the backend's and a code generation
 * defect would escape detection.
 *
 * Determinism. Only %s, %d and %.6f are used. No address, pointer value,
 * sizeof result or va_list-derived value is ever printed; only the values
 * retrieved from a list. The type long is never used, because sizeof(long)
 * and sizeof(void *) are 4 bytes on i686 and 8 bytes on the other three
 * targets. Every floating literal (1.5, 2.25, 4.125, 0.5, 3.25, 8.125) and
 * every floating sum (7.875, 11.875) is a dyadic rational exactly
 * representable in IEEE binary64, so %.6f renders byte-identically on all
 * four backends. There is no timestamp, randomness, uninitialized read,
 * locale-dependent formatting or variable iteration order. Exit status is 0.
 *
 * Expected output: 66 lines, recorded verbatim in the sibling expectation
 * record 004_va_copy_multiple_passes.expected, which also carries the target
 * and optimization matrix, the shared flags, the literal command templates and
 * the undefined-behaviour freedom argument for this program.
 */

#include <stdarg.h>

int printf(const char *, ...);

/*
 * Helper 1. Copy the list, traverse the copy in full, then traverse the
 * original in full. A va_copy must reproduce its source exactly, so the two
 * passes print identical values at identical positions and the two reported
 * sums must agree; the trailing equality line asserts that agreement.
 */
static void two_passes(const char *tag, int count, ...)
{
    va_list ap_tp;
    va_list dup_tp;
    int idx_tp;
    int val_copy_tp;
    int val_orig_tp;
    int acc_copy_tp = 0;
    int acc_orig_tp = 0;

    va_start(ap_tp, count);
    va_copy(dup_tp, ap_tp);

    for (idx_tp = 0; idx_tp < count; idx_tp++) {
        val_copy_tp = va_arg(dup_tp, int);
        printf("%s_copy[%d]=%d\n", tag, idx_tp, val_copy_tp);
        acc_copy_tp += val_copy_tp;
    }
    va_end(dup_tp);

    for (idx_tp = 0; idx_tp < count; idx_tp++) {
        val_orig_tp = va_arg(ap_tp, int);
        printf("%s_orig[%d]=%d\n", tag, idx_tp, val_orig_tp);
        acc_orig_tp += val_orig_tp;
    }
    va_end(ap_tp);

    printf("%s_copy_sum=%d\n", tag, acc_copy_tp);
    printf("%s_orig_sum=%d\n", tag, acc_orig_tp);
    printf("%s_equal=%d\n", tag, acc_copy_tp == acc_orig_tp);
}

/*
 * Helper 2. Take a copy of the original, then a copy of that copy, both
 * before either list has been traversed, so all three lists start at the same
 * position. Traverse the second copy, then the first copy, then the original,
 * each in full and each ended exactly once. All three traversals must yield
 * the same values in the same order, so the three reported sums are identical.
 */
static void three_passes(const char *tag, int count, ...)
{
    va_list ap_th;
    va_list cp1_th;
    va_list cp2_th;
    int idx_th;
    double val2_th;
    double val1_th;
    double val0_th;
    double acc2_th = 0.0;
    double acc1_th = 0.0;
    double acc0_th = 0.0;

    va_start(ap_th, count);
    va_copy(cp1_th, ap_th);
    va_copy(cp2_th, cp1_th);

    for (idx_th = 0; idx_th < count; idx_th++) {
        val2_th = va_arg(cp2_th, double);
        printf("%s_c2[%d]=%.6f\n", tag, idx_th, val2_th);
        acc2_th += val2_th;
    }
    va_end(cp2_th);

    for (idx_th = 0; idx_th < count; idx_th++) {
        val1_th = va_arg(cp1_th, double);
        printf("%s_c1[%d]=%.6f\n", tag, idx_th, val1_th);
        acc1_th += val1_th;
    }
    va_end(cp1_th);

    for (idx_th = 0; idx_th < count; idx_th++) {
        val0_th = va_arg(ap_th, double);
        printf("%s_ap[%d]=%.6f\n", tag, idx_th, val0_th);
        acc0_th += val0_th;
    }
    va_end(ap_th);

    printf("%s_sums=%.6f,%.6f,%.6f\n", tag, acc2_th, acc1_th, acc0_th);
}

/*
 * Helper 3. Copy the list, read only the first half_ht arguments from the
 * copy, then end the copy without consuming the rest -- legal, and the point
 * of this helper. Afterwards traverse the original in full, accumulating a
 * running total for every argument and a tail total for the arguments the
 * copy never saw. The head and tail sums must add up to the total.
 */
static void head_tail(const char *tag, int count, ...)
{
    va_list ap_ht;
    va_list dup_ht;
    int idx_ht;
    int val_head_ht;
    int val_all_ht;
    int acc_head_ht = 0;
    int acc_tail_ht = 0;
    int acc_total_ht = 0;
    int half_ht;

    half_ht = 3;

    va_start(ap_ht, count);
    va_copy(dup_ht, ap_ht);

    for (idx_ht = 0; idx_ht < half_ht; idx_ht++) {
        val_head_ht = va_arg(dup_ht, int);
        printf("%s_head[%d]=%d\n", tag, idx_ht, val_head_ht);
        acc_head_ht += val_head_ht;
    }
    va_end(dup_ht);

    for (idx_ht = 0; idx_ht < count; idx_ht++) {
        val_all_ht = va_arg(ap_ht, int);
        printf("%s_all[%d]=%d\n", tag, idx_ht, val_all_ht);
        acc_total_ht += val_all_ht;
        if (idx_ht >= half_ht) {
            acc_tail_ht += val_all_ht;
        }
    }
    va_end(ap_ht);

    printf("%s_head_sum=%d\n", tag, acc_head_ht);
    printf("%s_tail_sum=%d\n", tag, acc_tail_ht);
    printf("%s_total=%d\n", tag, acc_total_ht);
}

int main(void)
{
    volatile int rdup_vsrc_a0 = 3;
    volatile int rdup_vsrc_a1 = -7;
    volatile int rdup_vsrc_a2 = 12;
    volatile int rdup_vsrc_a3 = -18;
    int rdup_plain_a0;
    int rdup_plain_a1;
    int rdup_plain_a2;
    int rdup_plain_a3;
    volatile double rtri_vsrc_b0 = 0.5;
    volatile double rtri_vsrc_b1 = 3.25;
    volatile double rtri_vsrc_b2 = 8.125;
    double rtri_plain_b0;
    double rtri_plain_b1;
    double rtri_plain_b2;
    volatile int rsplit_vsrc_c[6];
    int rsplit_plain_c[6];
    int rsplit_idx_c;

    /* 1. Folded variant: literal integer arguments, four of them. */
    two_passes("fdup", 4, 2, -5, 10, -13);

    /* 2. Runtime variant: each volatile source is read into a plain local of
     * the same type by one simple assignment, and the plain locals are what
     * gets passed. Several volatile reads inside a single call expression are
     * deliberately avoided. */
    rdup_plain_a0 = rdup_vsrc_a0;
    rdup_plain_a1 = rdup_vsrc_a1;
    rdup_plain_a2 = rdup_vsrc_a2;
    rdup_plain_a3 = rdup_vsrc_a3;
    two_passes("rdup", 4,
        rdup_plain_a0, rdup_plain_a1, rdup_plain_a2, rdup_plain_a3);

    /* 3. Folded variant: literal double arguments with no f suffix, so the
     * retrieval type is unambiguously double and no default argument
     * promotion is involved. */
    three_passes("ftri", 3, 1.5, 2.25, 4.125);

    /* 4. Runtime variant: volatile double sources copied into plain doubles. */
    rtri_plain_b0 = rtri_vsrc_b0;
    rtri_plain_b1 = rtri_vsrc_b1;
    rtri_plain_b2 = rtri_vsrc_b2;
    three_passes("rtri", 3, rtri_plain_b0, rtri_plain_b1, rtri_plain_b2);

    /* 5. Folded variant: six literal integer arguments, split three and
     * three by the helper. */
    head_tail("fsplit", 6, 1, 2, 3, 4, 5, 6);

    /* 6. Runtime variant: a volatile array is written in one loop and read
     * back into a plain array in a second, separate loop, so the values
     * cannot be propagated to the call site at compile time. */
    for (rsplit_idx_c = 0; rsplit_idx_c < 6; rsplit_idx_c++) {
        rsplit_vsrc_c[rsplit_idx_c] = (rsplit_idx_c + 1) * 10;
    }
    for (rsplit_idx_c = 0; rsplit_idx_c < 6; rsplit_idx_c++) {
        rsplit_plain_c[rsplit_idx_c] = rsplit_vsrc_c[rsplit_idx_c];
    }
    head_tail("rsplit", 6,
        rsplit_plain_c[0], rsplit_plain_c[1], rsplit_plain_c[2],
        rsplit_plain_c[3], rsplit_plain_c[4], rsplit_plain_c[5]);

    return 0;
}
