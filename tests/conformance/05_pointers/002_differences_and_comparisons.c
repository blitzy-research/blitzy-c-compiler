/* A pointer one past the last element of an array is formed, compared and
 * subtracted here but never dereferenced: forming and comparing it is defined,
 * dereferencing it would not be.  Every loop terminates on a comparison against
 * that sentinel, `p != end` or `p < end`, so it is only ever read as a value.
 *
 * A pointer TWO past the end is not merely undereferenceable: forming it at all
 * is undefined, so both strided walks below are written to make that
 * unreachable, and the folded walk derives the overshoot a further stride would
 * have produced from its last legal difference rather than by forming the
 * out-of-range pointer.
 *
 * Each ptrdiff_t result is cast to long long before printing, so the printed text
 * is identical whether ptrdiff_t is 32 or 64 bits wide.  The runtime walk strides
 * by two over ten elements and so lands exactly on end, which is legal to form.
 * No address or pointer value is printed anywhere. */

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

    begin = i_buf;
    end = i_buf + 10;
    mid = i_buf + 4;
    begin_alias = i_buf;
    end_alias = i_buf + 10;

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
    /* A stride of three over ten elements visits indexes 0, 3, 6 and 9; a fourth
     * stride would land on index 12, two elements past the one-past-end pointer,
     * and FORMING such a pointer is undefined even where it is never
     * dereferenced.  The walk therefore advances only while three elements
     * genuinely remain, and the distance by which the fourth stride would have
     * passed end is computed with integer arithmetic from the last legal
     * difference instead: the last visited element is 9 and end is at 10, so the
     * overshoot is 3 - (end - p) = 3 - 1 = 2. */
    {
        int count = 0;
        int *p = begin;
        long long overshoot = 0;
        while (p < end) {
            ++count;
            if ((end - p) >= 3) {
                p += 3;
            } else {
                overshoot = 3 - (long long)(end - p);
                break;
            }
        }
        printf("folded_stride3_count=%d\n", count);
        printf("folded_stride3_overshoot=%lld\n", overshoot);
    }

    printf("folded_short_span=%lld\n", (long long)((s_buf + 6) - s_buf));
    printf("folded_double_span=%lld\n", (long long)((d_buf + 4) - d_buf));
    printf("folded_struct_span=%lld\n", (long long)((p_buf + 3) - p_buf));
    printf("folded_short_interior=%lld\n", (long long)(&s_buf[5] - &s_buf[1]));
    printf("folded_double_interior=%lld\n", (long long)(&d_buf[3] - &d_buf[1]));
    printf("folded_struct_interior=%lld\n", (long long)(&p_buf[2] - &p_buf[0]));

    printf("folded_short_end_gt=%d\n", (s_buf + 6) > &s_buf[5]);
    printf("folded_double_end_gt=%d\n", (d_buf + 4) > &d_buf[3]);
    printf("folded_struct_end_gt=%d\n", (p_buf + 3) > &p_buf[2]);

    /* a single-element view: &x and &x + 1 form a valid one-element range */
    {
        int solo = 77;
        int *sb = &solo;
        int *se = &solo + 1;
        printf("folded_solo_span=%lld\n", (long long)(se - sb));
        printf("folded_solo_lt=%d\n", sb < se);
        printf("folded_solo_value=%d\n", *sb);
    }

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
        int *rend = i_buf + k;
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
