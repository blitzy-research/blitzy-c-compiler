/*
 * The va_copy lifecycle rules this file obeys throughout: every va_copy
 * destination gets its own va_end; each list created by va_start is ended exactly
 * once; a copy is always taken before its source has been traversed, so source and
 * copy start from the same position; a copy of a copy is legal; a list may be ended
 * without having consumed all of its arguments; no list is ever read past the
 * number of arguments actually supplied; and every va_end sits in the same function
 * as its va_start or va_copy.
 *
 * <stdarg.h> is included because va_copy cannot be reached without it, and it
 * belongs to both compilers' freestanding header sets.  printf is hand-declared,
 * because bcc ships no stdio.h: its bundled set is the nine required freestanding
 * headers plus a bonus stdatomic.h, ten files in all (docs/project-guide.md line
 * 212), and no standard I/O header is among them.
 *
 * Every floating literal and every floating sum here is a dyadic rational exactly
 * representable in IEEE binary64, so %.6f renders the same bytes everywhere, and
 * long is never used because its width differs between the targets under test.
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

    two_passes("fdup", 4, 2, -5, 10, -13);

    /* Each volatile source is read into a plain local of the same type by one
     * simple assignment, and only the plain locals are passed: several volatile
     * reads inside one call expression would make the order of those accesses
     * depend on the unspecified order of argument evaluation. */
    rdup_plain_a0 = rdup_vsrc_a0;
    rdup_plain_a1 = rdup_vsrc_a1;
    rdup_plain_a2 = rdup_vsrc_a2;
    rdup_plain_a3 = rdup_vsrc_a3;
    two_passes("rdup", 4,
        rdup_plain_a0, rdup_plain_a1, rdup_plain_a2, rdup_plain_a3);

    /* The literals carry no f suffix, so they are already double and no default
     * argument promotion is involved in this group. */
    three_passes("ftri", 3, 1.5, 2.25, 4.125);

    rtri_plain_b0 = rtri_vsrc_b0;
    rtri_plain_b1 = rtri_vsrc_b1;
    rtri_plain_b2 = rtri_vsrc_b2;
    three_passes("rtri", 3, rtri_plain_b0, rtri_plain_b1, rtri_plain_b2);

    head_tail("fsplit", 6, 1, 2, 3, 4, 5, 6);

    /* The volatile array is written in one loop and read back into a plain array
     * in a second, separate loop, so no element is available for compile-time
     * substitution at the call site. */
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
