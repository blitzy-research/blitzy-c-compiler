/* Area 01 / program 001 - usual arithmetic conversions across mixed
 * signedness and rank, folded and volatile-runtime variants.
 */
int printf(const char *, ...);

/* Result-type probes.  A _Generic controlling expression is never evaluated,
 * so it asserts the type produced by the usual arithmetic conversions without
 * performing any conversion the strict warning gate could object to.
 */
static const char *type_int_uint(void)
{
    volatile int i = -1;
    volatile unsigned int u = 2u;
    return _Generic(i + u, unsigned int: "unsigned int", int: "int", default: "other");
}

static const char *type_uint_llong(void)
{
    volatile unsigned int u = 2u;
    volatile long long l = -1;
    return _Generic(l + u, long long: "long long",
                    unsigned long long: "unsigned long long", default: "other");
}

static const char *type_ushort_int(void)
{
    volatile unsigned short s = 2;
    volatile int i = -1;
    return _Generic(i + s, int: "int", unsigned int: "unsigned int", default: "other");
}

static const char *type_uint_ullong(void)
{
    volatile unsigned int u = 2u;
    volatile unsigned long long q = 3u;
    return _Generic(u + q, unsigned long long: "unsigned long long",
                    unsigned int: "unsigned int", default: "other");
}

int main(void)
{
    /* Masked on use so the operand is provably non-negative: the implicit
     * int -> unsigned int operand conversion of the usual arithmetic
     * conversions is then exercised without a sign-changing conversion. */
    volatile int i_nonneg = 2147483647;
    volatile unsigned int u_two = 2u;
    volatile unsigned short s_two = 2;
    volatile unsigned char c_two = 2;
    volatile long long l_neg = -1;

    /* Folded variant: every operand is a constant expression. */
    printf("fold_uint_common=%u\n", 2147483647 * 2u);
    printf("fold_ushort_promotes=%d\n", -1 * (unsigned short)2);
    printf("fold_uchar_promotes=%d\n", -1 * (unsigned char)2);
    printf("fold_llong_wins=%lld\n", -1LL / 2u);
    printf("fold_ullong_wins=%llu\n", 2u + 3uLL);

    /* Runtime variant: volatile operands force real instruction selection. */
    printf("run_uint_common=%u\n", (i_nonneg & 0x7fffffff) * u_two);
    printf("run_ushort_promotes=%d\n", -1 * s_two);
    printf("run_uchar_promotes=%d\n", -1 * c_two);
    printf("run_llong_wins=%lld\n", l_neg / u_two);
    printf("run_ullong_wins=%llu\n", u_two + 3uLL);

    /* Result types produced by the usual arithmetic conversions. */
    printf("type_int_uint=%s\n", type_int_uint());
    printf("type_ushort_int=%s\n", type_ushort_int());
    printf("type_uint_llong=%s\n", type_uint_llong());
    printf("type_uint_ullong=%s\n", type_uint_ullong());
    return 0;
}
