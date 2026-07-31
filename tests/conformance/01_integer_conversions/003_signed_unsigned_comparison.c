/* Area 01 / program 003 - comparison across signedness, where the common type
 * chosen by the usual arithmetic conversions changes the result.
 *
 * Authoring note: a bare implicit signed/unsigned comparison such as
 * `i < u` is rejected by -Wsign-compare, which -Wextra supplies and which
 * this program's gate keeps.  Every property below is therefore expressed
 * either through operands whose promotion removes the mismatch, through a
 * signed operand that is provably non-negative, through an explicit
 * conversion, or inside an unevaluated _Generic controlling expression.
 */
int printf(const char *, ...);

static const char *common_int_uint(void)
{
    volatile int i = -1;
    volatile unsigned int u = 1u;
    return _Generic(i < u ? i + u : u, unsigned int: "unsigned int",
                    int: "int", default: "other");
}

static const char *common_llong_uint(void)
{
    volatile long long l = -1;
    volatile unsigned int u = 1u;
    return _Generic(l < u ? l + u : l, long long: "long long",
                    unsigned long long: "unsigned long long", default: "other");
}

static const char *common_llong_ullong(void)
{
    volatile long long l = -1;
    volatile unsigned long long q = 1u;
    return _Generic(l < q ? l + q : q, unsigned long long: "unsigned long long",
                    long long: "long long", default: "other");
}

int main(void)
{
    volatile int i_neg = -1;
    volatile int i_max = 2147483647;
    volatile unsigned int u_one = 1u;
    volatile unsigned int u_big = 4000000000u;
    volatile unsigned short us_one = 1;
    volatile unsigned char uc_one = 1;
    volatile long long l_neg = -1;
    volatile unsigned long long q_one = 1u;

    /* Folded variant. */
    /* Narrow unsigned operands promote to int, so the negative stays negative. */
    printf("fold_lt_ushort=%d\n", -1 < (unsigned short)1);
    printf("fold_lt_uchar=%d\n", -1 < (unsigned char)1);
    /* long long can represent every unsigned int value, so it is the common type. */
    printf("fold_lt_llong_uint=%d\n", -1LL < 1u);
    /* unsigned long long outranks long long, so the negative converts upward. */
    printf("fold_gt_ullong=%d\n", (unsigned long long)(-1LL) > 1uLL);
    /* int against unsigned int: the signed operand converts to unsigned. */
    printf("fold_lt_int_uint=%d\n", (unsigned int)(-1) < 1u);
    printf("fold_eq_int_uint=%d\n", (unsigned int)(-1) == 4294967295u);
    /* Provably non-negative signed operand against a large unsigned value:
     * the result is 1 only if the comparison is performed as unsigned. */
    printf("fold_uac_sensitive=%d\n", 2147483647 < 4000000000u);

    /* Runtime variant: volatile operands force a real comparison. */
    printf("run_lt_ushort=%d\n", i_neg < us_one);
    printf("run_lt_uchar=%d\n", i_neg < uc_one);
    printf("run_lt_llong_uint=%d\n", l_neg < u_one);
    printf("run_gt_ullong=%d\n", (unsigned long long)l_neg > q_one);
    printf("run_lt_int_uint=%d\n", (unsigned int)i_neg < u_one);
    printf("run_eq_int_uint=%d\n", (unsigned int)i_neg == 4294967295u);
    printf("run_uac_sensitive=%d\n", (i_max & 0x7fffffff) < u_big);

    /* Common type selected for each mixed-signedness comparison. */
    printf("common_int_uint=%s\n", common_int_uint());
    printf("common_llong_uint=%s\n", common_llong_uint());
    printf("common_llong_ullong=%s\n", common_llong_ullong());
    return 0;
}
