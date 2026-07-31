/* Area 01 / program 009 - shift operators with counts strictly inside range,
 * including arithmetic right shift of negative signed values.
 *
 * Every shift count is at least 0 and strictly less than the width of the
 * promoted left operand, so no shift is undefined.  No negative value is ever
 * shifted left.  No signed left shift produces a value outside the range of
 * its type.
 */
int printf(const char *, ...);

int main(void)
{
    volatile int i_one = 1;
    volatile int i_neg8 = -8;
    volatile int i_neg1 = -1;
    volatile int i_neg1024 = -1024;
    volatile int i_five = 5;
    volatile unsigned int u_one = 1u;
    volatile unsigned int u_high = 2147483648u;
    volatile unsigned char uc_high = 128;
    volatile long long l_one = 1;
    volatile long long l_neg8 = -8;
    volatile unsigned long long q_one = 1u;
    volatile int sh_zero = 0;
    volatile int sh_five = 5;
    volatile int sh_31 = 31;
    volatile int sh_62 = 62;

    /* Folded variant: constant expressions only. */
    printf("fold_ushl_max=%u\n", 1u << 31);
    printf("fold_ushr_max=%u\n", 2147483648u >> 31);
    printf("fold_sshl_safe=%d\n", 1 << 30);
    printf("fold_sshr_neg1=%d\n", -8 >> 1);
    printf("fold_sshr_neg2=%d\n", -1024 >> 5);
    printf("fold_sshr_all=%d\n", -1 >> 31);
    printf("fold_shift_zero=%d %d\n", 5 << 0, 5 >> 0);
    printf("fold_uchar_promoted=%d\n", (unsigned char)128 >> 3);
    printf("fold_count_type=%d\n", 1 << (unsigned char)4);
    printf("fold_llong_shl=%lld\n", 1LL << 62);
    printf("fold_llong_shr=%lld\n", -8LL >> 1);
    printf("fold_ullong_shl=%llu\n", 1uLL << 63);

    /* Runtime variant: volatile operands and volatile counts force real shifts. */
    printf("run_ushl_max=%u\n", u_one << sh_31);
    printf("run_ushr_max=%u\n", u_high >> sh_31);
    printf("run_sshl_safe=%d\n", i_one << 30);
    printf("run_sshr_neg1=%d\n", i_neg8 >> 1);
    printf("run_sshr_neg2=%d\n", i_neg1024 >> sh_five);
    printf("run_sshr_all=%d\n", i_neg1 >> sh_31);
    printf("run_shift_zero=%d %d\n", i_five << sh_zero, i_five >> sh_zero);
    printf("run_uchar_promoted=%d\n", uc_high >> 3);
    printf("run_count_type=%d\n", i_one << (unsigned char)4);
    printf("run_llong_shl=%lld\n", l_one << sh_62);
    printf("run_llong_shr=%lld\n", l_neg8 >> 1);
    printf("run_ullong_shl=%llu\n", q_one << 63);
    return 0;
}
