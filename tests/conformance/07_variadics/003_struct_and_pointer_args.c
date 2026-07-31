/*
 * Aggregates are not subject to the default argument promotions, so va_arg names
 * exactly the type the caller supplied.  For the pointer group the retrieval type
 * is const int *, and int * and const int * are not compatible types, so every
 * pointer argument is written with an explicit (const int *) cast: a bare &object
 * would supply an int * where a const int * is retrieved, which is undefined and
 * would invalidate the oracle.  Only the pointee value is ever printed, never a
 * pointer.
 *
 * Every member of the three aggregates is an int, so their sizes agree across the
 * targets under test and cross-backend comparison stays meaningful; a pointer,
 * long or double member would size differently per target.  The three sizes are
 * chosen to sit below, exactly on, and above the by-register/by-memory boundary
 * that the ABIs under test use for aggregates, which is where those ABIs differ
 * most visibly - but which values travel where is the ABI's decision, not this
 * program's, and only the retrieved values are compared.
 *
 * <stdarg.h> is included because a variadic function cannot be written without it,
 * and it belongs to both compilers' freestanding header sets.  printf is
 * hand-declared, because bcc ships no stdio.h: its bundled set is the nine
 * required freestanding headers plus a bonus stdatomic.h, ten files in all
 * (docs/project-guide.md line 212), and no standard I/O header is among them.
 */

#include <stdarg.h>

int printf(const char *, ...);

struct small {
    int a;
    int b;
};

struct mid {
    int a;
    int b;
    int c;
    int d;
};

struct large {
    int v[8];
};

static int sum_small(const char *tag, int count, ...)
{
    va_list ap_sm;
    struct small val_sm;
    int acc_sm;
    int idx_sm;

    acc_sm = 0;
    va_start(ap_sm, count);
    for (idx_sm = 0; idx_sm < count; idx_sm++) {
        val_sm = va_arg(ap_sm, struct small);
        printf("%s[%d]=%d,%d\n", tag, idx_sm, val_sm.a, val_sm.b);
        acc_sm += val_sm.a + val_sm.b;
    }
    va_end(ap_sm);

    return acc_sm;
}

static int sum_mid(const char *tag, int count, ...)
{
    va_list ap_md;
    struct mid val_md;
    int acc_md;
    int idx_md;

    acc_md = 0;
    va_start(ap_md, count);
    for (idx_md = 0; idx_md < count; idx_md++) {
        val_md = va_arg(ap_md, struct mid);
        printf("%s[%d]=%d,%d,%d,%d\n", tag, idx_md,
               val_md.a, val_md.b, val_md.c, val_md.d);
        acc_md += val_md.a + val_md.b + val_md.c + val_md.d;
    }
    va_end(ap_md);

    return acc_md;
}

static int sum_large(const char *tag, int count, ...)
{
    va_list ap_lg;
    struct large val_lg;
    int acc_lg;
    int idx_lg;
    int elem_lg;

    acc_lg = 0;
    va_start(ap_lg, count);
    for (idx_lg = 0; idx_lg < count; idx_lg++) {
        val_lg = va_arg(ap_lg, struct large);
        printf("%s[%d]=%d,%d,%d,%d,%d,%d,%d,%d\n", tag, idx_lg,
               val_lg.v[0], val_lg.v[1], val_lg.v[2], val_lg.v[3],
               val_lg.v[4], val_lg.v[5], val_lg.v[6], val_lg.v[7]);
        for (elem_lg = 0; elem_lg < 8; elem_lg++) {
            acc_lg += val_lg.v[elem_lg];
        }
    }
    va_end(ap_lg);

    return acc_lg;
}

static int sum_via_pointers(const char *tag, int count, ...)
{
    va_list ap_pt;
    const int *val_pt;
    int acc_pt;
    int idx_pt;

    acc_pt = 0;
    va_start(ap_pt, count);
    for (idx_pt = 0; idx_pt < count; idx_pt++) {
        val_pt = va_arg(ap_pt, const int *);
        printf("%s[%d]=%d\n", tag, idx_pt, *val_pt);
        acc_pt += *val_pt;
    }
    va_end(ap_pt);

    return acc_pt;
}

/*
 * Folded aggregates name every member, because -Wmissing-field-initializers is
 * part of -Wextra.  Runtime aggregates are left uninitialized here and every
 * member is assigned from volatile storage before the aggregate is used, so no
 * indeterminate value is ever read.
 */
int main(void)
{
    struct small sm_fold_a = { 1, 2 };
    struct small sm_fold_b = { 30, 40 };
    struct small sm_fold_c = { 500, 600 };

    volatile int vsm_a0 = 7;
    volatile int vsm_a1 = 8;
    volatile int vsm_b0 = 90;
    volatile int vsm_b1 = 100;
    volatile int vsm_c0 = 1100;
    volatile int vsm_c1 = 1200;
    struct small sm_run_a;
    struct small sm_run_b;
    struct small sm_run_c;

    struct mid md_fold_a = { 1, 2, 3, 4 };
    struct mid md_fold_b = { 10, 20, 30, 40 };

    volatile int vmd_a0 = 5;
    volatile int vmd_a1 = 6;
    volatile int vmd_a2 = 7;
    volatile int vmd_a3 = 8;
    volatile int vmd_b0 = 50;
    volatile int vmd_b1 = 60;
    volatile int vmd_b2 = 70;
    volatile int vmd_b3 = 80;
    struct mid md_run_a;
    struct mid md_run_b;

    struct large lg_fold_a = { { 1, 2, 3, 4, 5, 6, 7, 8 } };
    struct large lg_fold_b = { { 10, 20, 30, 40, 50, 60, 70, 80 } };

    volatile int vsrc_lg_a[8];
    volatile int vsrc_lg_b[8];
    struct large lg_run_a;
    struct large lg_run_b;
    int k_lg;

    int pt_fold_a = 11;
    int pt_fold_b = 22;
    int pt_fold_c = 33;

    volatile int vpt_a = 111;
    volatile int vpt_b = 222;
    volatile int vpt_c = 333;
    int pt_run_a;
    int pt_run_b;
    int pt_run_c;

    /*
     * One named temporary per group.  A helper call is never nested inside a
     * printf argument list: at most one side-effecting argument per call, and
     * the print order stays fixed regardless of evaluation order.
     */
    int res_fsmall;
    int res_rsmall;
    int res_fmid;
    int res_rmid;
    int res_flarge;
    int res_rlarge;
    int res_fptr;
    int res_rptr;

    res_fsmall = sum_small("fsmall", 3, sm_fold_a, sm_fold_b, sm_fold_c);
    printf("fsmall_sum=%d\n", res_fsmall);

    sm_run_a.a = vsm_a0;
    sm_run_a.b = vsm_a1;
    sm_run_b.a = vsm_b0;
    sm_run_b.b = vsm_b1;
    sm_run_c.a = vsm_c0;
    sm_run_c.b = vsm_c1;
    res_rsmall = sum_small("rsmall", 3, sm_run_a, sm_run_b, sm_run_c);
    printf("rsmall_sum=%d\n", res_rsmall);

    res_fmid = sum_mid("fmid", 2, md_fold_a, md_fold_b);
    printf("fmid_sum=%d\n", res_fmid);

    md_run_a.a = vmd_a0;
    md_run_a.b = vmd_a1;
    md_run_a.c = vmd_a2;
    md_run_a.d = vmd_a3;
    md_run_b.a = vmd_b0;
    md_run_b.b = vmd_b1;
    md_run_b.c = vmd_b2;
    md_run_b.d = vmd_b3;
    res_rmid = sum_mid("rmid", 2, md_run_a, md_run_b);
    printf("rmid_sum=%d\n", res_rmid);

    res_flarge = sum_large("flarge", 2, lg_fold_a, lg_fold_b);
    printf("flarge_sum=%d\n", res_flarge);

    /* Every element of both arrays is written before any is read, and every index
     * stays within 0..7. */
    for (k_lg = 0; k_lg < 8; k_lg++) {
        vsrc_lg_a[k_lg] = (k_lg + 1) * 2;
    }
    for (k_lg = 0; k_lg < 8; k_lg++) {
        vsrc_lg_b[k_lg] = (k_lg + 1) * 100;
    }
    for (k_lg = 0; k_lg < 8; k_lg++) {
        lg_run_a.v[k_lg] = vsrc_lg_a[k_lg];
    }
    for (k_lg = 0; k_lg < 8; k_lg++) {
        lg_run_b.v[k_lg] = vsrc_lg_b[k_lg];
    }
    res_rlarge = sum_large("rlarge", 2, lg_run_a, lg_run_b);
    printf("rlarge_sum=%d\n", res_rlarge);

    res_fptr = sum_via_pointers("fptr", 3, (const int *)&pt_fold_a,
                                (const int *)&pt_fold_b,
                                (const int *)&pt_fold_c);
    printf("fptr_sum=%d\n", res_fptr);

    pt_run_a = vpt_a;
    pt_run_b = vpt_b;
    pt_run_c = vpt_c;
    res_rptr = sum_via_pointers("rptr", 3, (const int *)&pt_run_a,
                                (const int *)&pt_run_b,
                                (const int *)&pt_run_c);
    printf("rptr_sum=%d\n", res_rptr);

    return 0;
}
