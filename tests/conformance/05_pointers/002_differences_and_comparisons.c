/* Area 05 - Pointer arithmetic and function pointers
 * 002_differences_and_comparisons: pointer differences and relational
 * comparisons, in a folded and a volatile-index runtime variant.
 *
 * UB-CRITICAL PROGRAM.  A one-past-the-end pointer is FORMED and COMPARED
 * but NEVER DEREFERENCED.  C permits forming a pointer one past the last
 * element of an array and permits comparing and subtracting it; only
 * dereferencing it would be undefined.  Every loop below terminates on
 * "p != end" so the sentinel is read as a value and never as an object.
 *
 * Every ptrdiff_t result is cast to long long before printing, so the
 * printed text is identical whether ptrdiff_t is 32 or 64 bits wide.
 * No address or pointer value is printed anywhere.
 */

int printf(const char *, ...);

struct pair {
    int a;
    int b;
};

static int i_buf[10] = { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 };
static short s_buf[6] = { 10, 11, 12, 13, 14, 15 };
static double d_buf[4] = { 1.25, 2.25, 3.25, 4.25 };
static struct pair p_buf[3] = { { 1, 2 }, { 3, 4 }, { 5, 6 } };

int main(void)
{
    int *begin;
    int *end;
    int *mid;
    int *begin_alias;
    int *end_alias;
    volatile int vidx;
    int k;

    /* ---- folded variant ---- */
    begin = i_buf;
    end = i_buf + 10;   /* one past the end: legal to form */
    mid = i_buf + 4;
    begin_alias = i_buf;
    end_alias = i_buf + 10;   /* a second one-past-end pointer, same value */

    printf("folded_span=%lld\n", (long long)(end - begin));
    printf("folded_head=%lld\n", (long long)(mid - begin));
    printf("folded_tail=%lld\n", (long long)(end - mid));
    printf("folded_negative=%lld\n", (long long)(begin - mid));
    printf("folded_self=%lld\n", (long long)(begin - begin_alias));
    printf("folded_end_self=%lld\n", (long long)(end - end_alias));
    printf("folded_alias_eq=%d\n", begin == begin_alias);
    printf("folded_end_alias_eq=%d\n", end == end_alias);

    printf("folded_lt=%d\n", begin < end);
    printf("folded_le=%d\n", begin <= begin_alias);
    printf("folded_gt=%d\n", end > mid);
    printf("folded_ge=%d\n", end >= end_alias);
    printf("folded_eq=%d\n", begin == i_buf);
    printf("folded_ne=%d\n", begin != end);
    printf("folded_mid_between=%d\n", (begin < mid) && (mid < end));
    printf("folded_end_not_begin=%d\n", end != begin);

    /* the one-past-end pointer used purely as a loop sentinel */
    {
        int sum = 0;
        int *p;
        for (p = begin; p != end; ++p) {
            sum += *p;
        }
        printf("folded_forward_sum=%d\n", sum);
        printf("folded_forward_stopped_at_end=%d\n", p == end);
    }
    {
        int sum = 0;
        int *p;
        for (p = end; p != begin;) {
            --p;                 /* decrement first: never dereference end */
            sum += *p * 2;
        }
        printf("folded_reverse_sum=%d\n", sum);
        printf("folded_reverse_stopped_at_begin=%d\n", p == begin);
    }
    {
        int count = 0;
        int *p;
        for (p = begin; p < end; p += 3) {
            ++count;
        }
        printf("folded_stride3_count=%d\n", count);
        printf("folded_stride3_overshoot=%lld\n", (long long)(p - end));
    }

    /* differences over other element types: element count, not byte count */
    printf("folded_short_span=%lld\n", (long long)((s_buf + 6) - s_buf));
    printf("folded_double_span=%lld\n", (long long)((d_buf + 4) - d_buf));
    printf("folded_struct_span=%lld\n", (long long)((p_buf + 3) - p_buf));
    printf("folded_short_interior=%lld\n", (long long)(&s_buf[5] - &s_buf[1]));
    printf("folded_double_interior=%lld\n", (long long)(&d_buf[3] - &d_buf[1]));
    printf("folded_struct_interior=%lld\n", (long long)(&p_buf[2] - &p_buf[0]));

    /* one-past-end for each of those arrays, formed and compared only */
    printf("folded_short_end_gt=%d\n", (s_buf + 6) > &s_buf[5]);
    printf("folded_double_end_gt=%d\n", (d_buf + 4) > &d_buf[3]);
    printf("folded_struct_end_gt=%d\n", (p_buf + 3) > &p_buf[2]);

    /* a single-element view: &x and &x + 1 form a valid one-element range */
    {
        int solo = 77;
        int *sb = &solo;
        int *se = &solo + 1;      /* one past a single object: legal to form */
        printf("folded_solo_span=%lld\n", (long long)(se - sb));
        printf("folded_solo_lt=%d\n", sb < se);
        printf("folded_solo_value=%d\n", *sb);
    }

    /* ---- runtime variant: volatile indices defeat constant folding ---- */
    vidx = 4;
    k = vidx;
    mid = i_buf + k;
    printf("runtime_head=%lld\n", (long long)(mid - begin));
    printf("runtime_tail=%lld\n", (long long)(end - mid));
    printf("runtime_negative=%lld\n", (long long)(begin - mid));
    printf("runtime_lt=%d\n", mid < end);
    printf("runtime_gt=%d\n", mid > begin);
    printf("runtime_ne_end=%d\n", mid != end);

    vidx = 10;
    k = vidx;
    {
        int *rend = i_buf + k;    /* one past the end, computed at run time */
        int sum = 0;
        int *p;
        printf("runtime_end_matches=%d\n", rend == end);
        printf("runtime_end_span=%lld\n", (long long)(rend - begin));
        for (p = begin; p != rend; ++p) {
            sum += *p + 1;
        }
        printf("runtime_forward_sum=%d\n", sum);
        printf("runtime_stopped_at_end=%d\n", p == rend);
    }

    vidx = 2;
    k = vidx;
    {
        int count = 0;
        int *p;
        for (p = begin; p < end; p += k) {
            ++count;
        }
        printf("runtime_stride_count=%d\n", count);
        printf("runtime_stride_overshoot=%lld\n", (long long)(p - end));
    }

    vidx = 5;
    k = vidx;
    printf("runtime_short_interior=%lld\n", (long long)(&s_buf[k] - &s_buf[1]));
    printf("runtime_double_interior=%lld\n",
           (long long)(&d_buf[k - 2] - &d_buf[0]));
    printf("runtime_struct_interior=%lld\n",
           (long long)(&p_buf[k - 3] - &p_buf[0]));

    return 0;
}
