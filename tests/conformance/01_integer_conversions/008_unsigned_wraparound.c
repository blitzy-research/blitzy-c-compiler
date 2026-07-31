/* Area 01 / program 008 - modular wraparound of unsigned integer types at and
 * across their maximum, folded and volatile-runtime variants.
 *
 * Unsigned arithmetic is defined to wrap modulo 2^N, so every expression here
 * is well defined; no signed type ever overflows.  Limits are spelled as
 * literals because no header is included.
 */
int printf(const char *, ...);

int main(void)
{
    volatile unsigned int u_zero = 0u;
    volatile unsigned int u_max = 4294967295u;
    volatile unsigned int u_half = 65536u;
    volatile unsigned char uc_max = 255;
    volatile unsigned short us_max = 65535;
    volatile unsigned long long q_zero = 0uLL;
    volatile unsigned long long q_max = 18446744073709551615uLL;

    /* Folded variant: constant expressions only. */
    printf("fold_uint_max=%u\n", 4294967295u);
    printf("fold_uint_complement=%u\n", ~0u);
    printf("fold_uint_underflow=%u\n", 0u - 1u);
    printf("fold_uint_overflow=%u\n", 4294967295u + 1u);
    printf("fold_uint_mul_wrap=%u\n", 65536u * 65536u);
    printf("fold_uint_mul_max=%u\n", 4294967295u * 3u);
    printf("fold_uchar_wrap=%d\n", (int)(unsigned char)(255 + 1));
    printf("fold_ushort_wrap=%d\n", (int)(unsigned short)(65535 + 1));
    printf("fold_ullong_underflow=%llu\n", 0uLL - 1uLL);
    printf("fold_ullong_overflow=%llu\n", 18446744073709551615uLL + 1uLL);
    printf("fold_modular_identity=%d\n", (int)((0u - 1u) + 1u == 0u));

    /* Runtime variant: volatile operands force real wrapping arithmetic. */
    printf("run_uint_max=%u\n", u_max);
    printf("run_uint_complement=%u\n", ~u_zero);
    printf("run_uint_underflow=%u\n", u_zero - 1u);
    printf("run_uint_overflow=%u\n", u_max + 1u);
    printf("run_uint_mul_wrap=%u\n", u_half * u_half);
    printf("run_uint_mul_max=%u\n", u_max * 3u);
    printf("run_uchar_wrap=%d\n", (int)(unsigned char)(uc_max + 1));
    printf("run_ushort_wrap=%d\n", (int)(unsigned short)(us_max + 1));
    printf("run_ullong_underflow=%llu\n", q_zero - 1uLL);
    printf("run_ullong_overflow=%llu\n", q_max + 1uLL);
    printf("run_modular_identity=%d\n", (int)((u_zero - 1u) + 1u == 0u));
    return 0;
}
