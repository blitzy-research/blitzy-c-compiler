/*
 * tests/conformance/07_variadics/003_struct_and_pointer_args.c
 *
 * Area:  07_variadics
 * Focus: aggregates and pointers through the variadic path.
 *
 * Sanctioned header exception
 * ---------------------------
 * This program includes <stdarg.h>.  That is one of only two header
 * exceptions in the corpus, and it is sound for both oracles: bcc bundles
 * stdarg.h in its freestanding header set -- docs/technical-specifications.md
 * line 208 records "va_list, va_start, va_arg, va_end, va_copy -- implemented
 * via compiler builtins" -- and the reference compiler supplies stdarg.h as a
 * freestanding header too, so the include compiles identically on both sides
 * and the program stays a single-file reproducer.  No other header is used:
 * printf is hand-declared, because bcc ships no stdio.h (its bundled set is
 * the nine headers listed at docs/technical-specifications.md line 19).
 * Including stdio.h would fail against bcc while succeeding against the
 * reference compiler -- a divergence caused by the test, not the compiler.
 *
 * Why these three aggregate types
 * -------------------------------
 * Every member is an int, and int is 4 bytes on all four supported targets
 * (widths table at docs/technical-specifications.md lines 457-462; measured on
 * all four targets while authoring this program), so each struct has an
 * identical size everywhere and cross-backend comparison stays meaningful:
 *
 *     struct small    8 bytes  -- below the x86-64 by-register threshold
 *     struct mid     16 bytes  -- exactly AT the two-eightbyte threshold
 *     struct large   32 bytes  -- above it, MEMORY class on x86-64
 *
 * Passing an aggregate BY VALUE through an ellipsis is where the four
 * backends' four different ABIs diverge most visibly:
 *   - x86-64 System V AMD64 classifies each argument INTEGER, SSE or MEMORY
 *     and passes an aggregate of at most two eightbytes in rdi, rsi, rdx,
 *     rcx, r8, r9; a larger aggregate becomes MEMORY class and travels on the
 *     stack (docs/technical-specifications.md line 546).
 *   - i686 cdecl passes ALL arguments on the stack, right-to-left, with
 *     caller cleanup (line 553) -- the strongest contrast with the baseline.
 *   - AArch64 AAPCS64 uses x0-x7 and v0-v7 (line 559).
 *   - RISC-V 64 LP64D uses a0-a7 and fa0-fa7 (line 565).
 * The three sizes therefore straddle the by-register/by-memory boundary on
 * purpose, and no target is excluded.
 *
 * No member is a pointer, a long, or a double: pointer and long are 4 bytes
 * on i686 and 8 bytes on the other three targets (lines 457-462), so such a
 * member would give each struct a different size per target and destroy the
 * cross-backend comparison this program exists to make.
 *
 * Pointer discipline
 * ------------------
 * No address and no pointer value is ever printed -- sum_via_pointers prints
 * only the dereferenced value.  int * and const int * are NOT compatible
 * types, and va_arg requires the retrieval type to be compatible with the
 * type actually supplied, so the helper retrieves const int * and EVERY
 * pointer argument at EVERY call site is written with an explicit
 * (const int *) cast.  A bare &object would supply an int * where a
 * const int * is retrieved: undefined behaviour that would silently
 * invalidate the oracle.  The pointees are ordinary int locals of main, which
 * keeps them writable for the runtime variant; the cast is a plain
 * qualification conversion and yields an argument of exactly the retrieved
 * type.
 *
 * Two-variant rule
 * ----------------
 * Each helper is exercised twice.  The folded variant builds its aggregates
 * from literal initializers, exercising the constant folder.  The runtime
 * variant reads every member out of volatile storage, which forces the
 * backend to emit genuine loads so nothing can be folded away and the
 * comparison actually reaches the code generator at -O1 and -O2.
 *
 * Determinism: only %s and %d conversions are used; output is a fixed
 * 28-line sequence, one line per aggregate or pointee retrieved plus one sum
 * line per group; exit status is 0.
 *
 * The expected output, the build and run commands, and the UB-freedom
 * argument are recorded in the sibling record
 * tests/conformance/07_variadics/003_struct_and_pointer_args.expected.
 */

#include <stdarg.h>

int printf(const char *, ...);

/* 8 bytes: one eightbyte, comfortably inside the x86-64 register path. */
struct small {
    int a;
    int b;
};

/* 16 bytes: exactly two eightbytes, the last size still passed in registers. */
struct mid {
    int a;
    int b;
    int c;
    int d;
};

/* 32 bytes: four eightbytes, MEMORY class on x86-64 -- passed on the stack. */
struct large {
    int v[8];
};

/*
 * Retrieve `count` struct small values by value from the variadic list.
 * Aggregates are not subject to the default argument promotions, so the
 * retrieval type is exactly the supplied type.  One line is printed per
 * aggregate; the group sum is returned for main to print.
 */
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

/*
 * Retrieve `count` struct mid values by value.  At 16 bytes these sit exactly
 * on the x86-64 two-eightbyte boundary, the most delicate classification case
 * in the System V ABI.
 */
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

/*
 * Retrieve `count` struct large values by value.  At 32 bytes these are
 * MEMORY class on x86-64 and are passed indirectly or on the stack elsewhere,
 * so every element is printed individually: a single wrong slot then localizes
 * the defect immediately instead of hiding inside an aggregate sum.
 */
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

/*
 * Retrieve `count` pointers from the variadic list and print what each one
 * points AT -- never the pointer itself, because an address is not
 * reproducible across targets or runs.  The retrieval type is const int *,
 * which is why every call site casts explicitly: int * and const int * are
 * not compatible types.
 */
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
 * All locals are declared at the top of the block (C89 placement) and every
 * name in this file is distinct, so nothing shadows anything and the strict
 * warning gate stays clean.  Folded aggregates are fully explicit initializer
 * lists -- every member named, because -Wmissing-field-initializers is part
 * of -Wextra.  Runtime aggregates are left uninitialized here and every
 * member is assigned from volatile storage before the aggregate is used, so
 * there is no read of an indeterminate value.
 */
int main(void)
{
    /* Call 1 -- folded struct small. */
    struct small sm_fold_a = { 1, 2 };
    struct small sm_fold_b = { 30, 40 };
    struct small sm_fold_c = { 500, 600 };

    /* Call 2 -- runtime struct small: volatile sources and destinations. */
    volatile int vsm_a0 = 7;
    volatile int vsm_a1 = 8;
    volatile int vsm_b0 = 90;
    volatile int vsm_b1 = 100;
    volatile int vsm_c0 = 1100;
    volatile int vsm_c1 = 1200;
    struct small sm_run_a;
    struct small sm_run_b;
    struct small sm_run_c;

    /* Call 3 -- folded struct mid. */
    struct mid md_fold_a = { 1, 2, 3, 4 };
    struct mid md_fold_b = { 10, 20, 30, 40 };

    /* Call 4 -- runtime struct mid. */
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

    /* Call 5 -- folded struct large. */
    struct large lg_fold_a = { { 1, 2, 3, 4, 5, 6, 7, 8 } };
    struct large lg_fold_b = { { 10, 20, 30, 40, 50, 60, 70, 80 } };

    /* Call 6 -- runtime struct large, filled from volatile arrays. */
    volatile int vsrc_lg_a[8];
    volatile int vsrc_lg_b[8];
    struct large lg_run_a;
    struct large lg_run_b;
    int k_lg;

    /* Call 7 -- folded pointees.  Ordinary ints; the cast happens at the call. */
    int pt_fold_a = 11;
    int pt_fold_b = 22;
    int pt_fold_c = 33;

    /* Call 8 -- runtime pointees, loaded out of volatile storage. */
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

    /* ---- Call 1: three small aggregates, folded.  Sum 1173. ---- */
    res_fsmall = sum_small("fsmall", 3, sm_fold_a, sm_fold_b, sm_fold_c);
    printf("fsmall_sum=%d\n", res_fsmall);

    /* ---- Call 2: the same shape, every member a genuine volatile load. ---- */
    sm_run_a.a = vsm_a0;
    sm_run_a.b = vsm_a1;
    sm_run_b.a = vsm_b0;
    sm_run_b.b = vsm_b1;
    sm_run_c.a = vsm_c0;
    sm_run_c.b = vsm_c1;
    res_rsmall = sum_small("rsmall", 3, sm_run_a, sm_run_b, sm_run_c);
    printf("rsmall_sum=%d\n", res_rsmall);

    /* ---- Call 3: two 16-byte aggregates, folded.  Sum 110. ---- */
    res_fmid = sum_mid("fmid", 2, md_fold_a, md_fold_b);
    printf("fmid_sum=%d\n", res_fmid);

    /* ---- Call 4: the 16-byte boundary case, built at run time. ---- */
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

    /* ---- Call 5: two MEMORY-class aggregates, folded.  Sum 396. ---- */
    res_flarge = sum_large("flarge", 2, lg_fold_a, lg_fold_b);
    printf("flarge_sum=%d\n", res_flarge);

    /*
     * ---- Call 6: the same MEMORY-class shape at run time.  Each element is
     * written to volatile storage and read back, so the 32-byte copy cannot be
     * folded or elided.  Every element of both arrays is written before any is
     * read, and every index stays within 0..7.
     */
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

    /*
     * ---- Call 7: three pointers, folded pointees.  The explicit
     * (const int *) cast makes the supplied type exactly the type
     * sum_via_pointers retrieves.  Sum 66.
     */
    res_fptr = sum_via_pointers("fptr", 3, (const int *)&pt_fold_a,
                                (const int *)&pt_fold_b,
                                (const int *)&pt_fold_c);
    printf("fptr_sum=%d\n", res_fptr);

    /* ---- Call 8: the same three pointers, pointees loaded at run time. ---- */
    pt_run_a = vpt_a;
    pt_run_b = vpt_b;
    pt_run_c = vpt_c;
    res_rptr = sum_via_pointers("rptr", 3, (const int *)&pt_run_a,
                                (const int *)&pt_run_b,
                                (const int *)&pt_run_c);
    printf("rptr_sum=%d\n", res_rptr);

    return 0;
}
