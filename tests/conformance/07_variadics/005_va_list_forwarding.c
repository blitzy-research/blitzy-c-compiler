/*
 * Forwarding rules this file obeys.  Once the callee that received the list has
 * invoked va_arg, the list is indeterminate in the forwarding frame, so nothing
 * here reads it again after a forward - the three wrappers below contain no va_arg
 * at all, and the only thing the owning frame still does with its list is the
 * required va_end.  A "v"-style consumer never starts, copies or ends the list it
 * receives, never assigns to that parameter and never takes its address, because
 * va_end must be invoked by the frame that invoked va_start.  The middle forwarder
 * passes the list on without consuming it.  Forwarding is chained once on purpose:
 * fwd_doubles owns the list, vmid_doubles forwards it unread, vsum_doubles reads
 * it.  There is no va_copy here; multi-pass traversal is 004's subject.
 *
 * The retrieval type always matches what the caller supplied: int arguments are
 * retrieved as int and double arguments as double, never as float.
 *
 * <stdarg.h> is included because the va_list parameter type of a "v"-style callee
 * cannot be spelled without it, and it belongs to both compilers' freestanding
 * header sets.  printf is hand-declared, because bcc ships no stdio.h: its bundled
 * set is the nine required freestanding headers plus a bonus stdatomic.h, ten
 * files in all (docs/project-guide.md line 212), and no standard I/O header is
 * among them.
 *
 * Every floating literal here is a dyadic rational exactly representable in IEEE
 * binary64, so %.6f renders the same bytes everywhere.
 */

#include <stdarg.h>

int printf(const char *, ...);

static int vsum_ints(const char *tag, int count, va_list ap_in);
static double vsum_doubles(const char *tag, int count, va_list ap_in);
static double vmid_doubles(const char *tag, int count, va_list ap_in);
static int vsum_mixed(const char *tag, int n_int, int n_dbl, va_list ap_in);

static int fwd_ints(const char *tag, int count, ...);
static double fwd_doubles(const char *tag, int count, ...);
static int fwd_mixed(const char *tag, int n_int, int n_dbl, ...);

/*
 * int is not subject to the default argument promotions, so retrieving as int
 * yields precisely what the caller supplied.
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
 * The supplied arguments are double literals or plain double objects, so double
 * is the correct retrieval type; retrieving as float would be undefined.
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
 * The middle forwarder passes its va_list parameter straight on, unread: no
 * va_arg, no va_start, no va_copy and no va_end.
 */
static double vmid_doubles(const char *tag, int count, va_list ap_in)
{
    return vsum_doubles(tag, count, ap_in);
}

/*
 * The two retrieval groups are read in the order they were supplied, and the
 * integer accumulator is only ever combined with int values and the floating one
 * only with double values, so no implicit conversion arises.
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
 * va_start names the last named parameter, count.  This frame's list is
 * indeterminate from the moment vsum_ints invokes va_arg, so nothing here reads
 * it afterwards - only va_end remains.
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
 * The list is indeterminate in this frame and in the middle frame once the
 * innermost consumer has read it; only this frame, which called va_start, calls
 * va_end.
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

/* va_start names the last named parameter, which here is n_dbl. */
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
 * Each volatile source is read once into a plain local of the same type, and only
 * the plain locals are passed, so no call expression contains more than one
 * volatile access and the order of those reads does not depend on the unspecified
 * order of argument evaluation.  Every wrapper result is stored in a named object
 * before it is printed, so no wrapper call is nested inside a printf argument
 * list.
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

    res_ffwd_int = fwd_ints("ffwd_int", 4, 6, -4, 15, -8);
    printf("ffwd_int_sum=%d\n", res_ffwd_int);

    plain_fi0 = vsrc_fi0;
    plain_fi1 = vsrc_fi1;
    plain_fi2 = vsrc_fi2;
    plain_fi3 = vsrc_fi3;
    res_rfwd_int = fwd_ints("rfwd_int", 4, plain_fi0, plain_fi1, plain_fi2,
                            plain_fi3);
    printf("rfwd_int_sum=%d\n", res_rfwd_int);

    res_ffwd_dbl = fwd_doubles("ffwd_dbl", 3, 1.5, 2.25, 4.125);
    printf("ffwd_dbl_sum=%.6f\n", res_ffwd_dbl);

    plain_fd0 = vsrc_fd0;
    plain_fd1 = vsrc_fd1;
    plain_fd2 = vsrc_fd2;
    res_rfwd_dbl = fwd_doubles("rfwd_dbl", 3, plain_fd0, plain_fd1, plain_fd2);
    printf("rfwd_dbl_sum=%.6f\n", res_rfwd_dbl);

    res_fmix = fwd_mixed("fmix", 2, 2, 3, -4, 1.5, 0.25);
    printf("fmix_isum=%d\n", res_fmix);

    plain_fm_i0 = vsrc_fm_i0;
    plain_fm_i1 = vsrc_fm_i1;
    plain_fm_d0 = vsrc_fm_d0;
    plain_fm_d1 = vsrc_fm_d1;
    res_rmix = fwd_mixed("rmix", 2, 2, plain_fm_i0, plain_fm_i1, plain_fm_d0,
                         plain_fm_d1);
    printf("rmix_isum=%d\n", res_rmix);

    return 0;
}
