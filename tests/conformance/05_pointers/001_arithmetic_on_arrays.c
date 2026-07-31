/* Area 05 - Pointer arithmetic and function pointers
 * 001_arithmetic_on_arrays: increment, decrement and indexing scaled by
 * element size, in both a folded (constant-index) and a runtime
 * (volatile-index) variant.
 *
 * No header is included; printf is hand-declared.  No address or pointer
 * value is ever printed: pointer facts appear only as differences and
 * comparisons, and every difference is cast to long long before printing.
 */

int printf(const char *, ...);

struct pair {
    int a;
    int b;
};

static const short s_arr[8] = { 1, 2, 3, 4, 5, 6, 7, 8 };
static const int i_arr[8] = { 10, 20, 30, 40, 50, 60, 70, 80 };
static const double d_arr[4] = { 0.5, 1.5, 2.5, 3.5 };
static const struct pair p_arr[4] = { { 1, 2 }, { 3, 4 }, { 5, 6 }, { 7, 8 } };

int main(void)
{
    const int *ip;
    const short *sp;
    const double *dp;
    const struct pair *pp;
    volatile int vidx;
    int k;
    int step;

    /* ---- folded variant: every index is a compile-time constant ---- */
    ip = i_arr;
    printf("folded_index=%d %d %d %d\n", ip[0], ip[1], ip[4], ip[7]);

    ip = i_arr + 5;
    printf("folded_offset=%d\n", *ip);
    ++ip;
    printf("folded_preinc=%d\n", *ip);
    ip++;
    printf("folded_postinc=%d\n", *ip);
    --ip;
    printf("folded_predec=%d\n", *ip);
    ip--;
    printf("folded_postdec=%d\n", *ip);
    ip += 2;
    printf("folded_plusassign=%d\n", *ip);
    ip -= 4;
    printf("folded_minusassign=%d\n", *ip);
    printf("folded_walked_index=%lld\n", (long long)(ip - i_arr));

    sp = s_arr + 3;
    printf("folded_short=%d %d %d\n", sp[-3], sp[0], sp[4]);
    dp = d_arr + 1;
    printf("folded_double=%.3f %.3f\n", dp[-1], dp[2]);
    pp = p_arr + 2;
    printf("folded_struct=%d %d %d %d\n", pp[-2].a, pp[-2].b, pp[1].a, pp[1].b);

    /* element-size scaling made observable as a byte distance; sizeof short,
     * int, double and this two-int struct are 2, 4, 8 and 8 on all four
     * supported targets, so these values are target independent. */
    printf("folded_scale_short=%lld\n",
           (long long)((const char *)&s_arr[1] - (const char *)&s_arr[0]));
    printf("folded_scale_int=%lld\n",
           (long long)((const char *)&i_arr[1] - (const char *)&i_arr[0]));
    printf("folded_scale_double=%lld\n",
           (long long)((const char *)&d_arr[1] - (const char *)&d_arr[0]));
    printf("folded_scale_struct=%lld\n",
           (long long)((const char *)&p_arr[1] - (const char *)&p_arr[0]));
    printf("folded_scale_int_span=%lld\n",
           (long long)((const char *)&i_arr[6] - (const char *)&i_arr[2]));

    /* ---- runtime variant: volatile operands defeat constant folding ---- */
    vidx = 3;
    k = vidx;
    ip = i_arr;
    printf("runtime_index=%d\n", ip[k]);
    printf("runtime_index_plus=%d\n", *(ip + k));
    printf("runtime_index_commuted=%d\n", k[ip]);

    vidx = 2;
    step = vidx;
    ip = i_arr + 1;
    ip += step;
    printf("runtime_plusassign=%d\n", *ip);
    ip -= step;
    printf("runtime_minusassign=%d\n", *ip);
    printf("runtime_walked_index=%lld\n", (long long)(ip - i_arr));

    vidx = 5;
    k = vidx;
    printf("runtime_short=%d\n", s_arr[k]);
    printf("runtime_double=%.3f\n", d_arr[k - 3]);
    printf("runtime_struct=%d %d\n", p_arr[k - 2].a, p_arr[k - 2].b);

    vidx = 1;
    k = vidx;
    printf("runtime_scale_int=%lld\n",
           (long long)((const char *)&i_arr[k] - (const char *)&i_arr[0]));
    printf("runtime_scale_double=%lld\n",
           (long long)((const char *)&d_arr[k] - (const char *)&d_arr[0]));

    /* a scan driven entirely by a volatile bound */
    {
        int total = 0;
        int n;
        vidx = 8;
        n = vidx;
        for (k = 0; k < n; ++k) {
            total += i_arr[k];
        }
        printf("runtime_scan_total=%d\n", total);
    }

    return 0;
}
