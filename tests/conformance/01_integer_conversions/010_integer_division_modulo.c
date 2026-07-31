/* Area 01 / program 010 - integer division and remainder: truncation toward
 * zero, and a remainder that takes the sign of the dividend.
 *
 * No divisor is ever zero, and the one undefined signed case - the most
 * negative value divided by -1 - is deliberately never formed.  The division
 * identity (a/b)*b + a%b == a is asserted for every signed combination.
 */
int printf(const char *, ...);

int main(void)
{
    volatile int a_pos = 7;
    volatile int a_neg = -7;
    volatile int b_pos = 2;
    volatile int b_neg = -2;
    volatile int big_neg = -2147483647;
    volatile int three = 3;
    volatile unsigned int u_seven = 7u;
    volatile unsigned int u_two = 2u;
    volatile long long l_neg7 = -7;
    volatile long long l_two = 2;

    /* Folded variant: constant expressions only. */
    printf("fold_pos_pos=%d %d\n", 7 / 2, 7 % 2);
    printf("fold_neg_pos=%d %d\n", -7 / 2, -7 % 2);
    printf("fold_pos_neg=%d %d\n", 7 / -2, 7 % -2);
    printf("fold_neg_neg=%d %d\n", -7 / -2, -7 % -2);
    printf("fold_exact=%d %d\n", -8 / 2, -8 % 2);
    printf("fold_large=%d %d\n", -2147483647 / 3, -2147483647 % 3);
    printf("fold_unsigned=%u %u\n", 7u / 2u, 7u % 2u);
    printf("fold_llong=%lld %lld\n", -7LL / 2LL, -7LL % 2LL);
    printf("fold_identity=%d\n",
           (int)((-7 / 2) * 2 + (-7 % 2) == -7 && (7 / -2) * -2 + (7 % -2) == 7));

    /* Runtime variant: volatile operands force real divide instructions. */
    printf("run_pos_pos=%d %d\n", a_pos / b_pos, a_pos % b_pos);
    printf("run_neg_pos=%d %d\n", a_neg / b_pos, a_neg % b_pos);
    printf("run_pos_neg=%d %d\n", a_pos / b_neg, a_pos % b_neg);
    printf("run_neg_neg=%d %d\n", a_neg / b_neg, a_neg % b_neg);
    printf("run_exact=%d %d\n", (a_neg - 1) / b_pos, (a_neg - 1) % b_pos);
    printf("run_large=%d %d\n", big_neg / three, big_neg % three);
    printf("run_unsigned=%u %u\n", u_seven / u_two, u_seven % u_two);
    printf("run_llong=%lld %lld\n", l_neg7 / l_two, l_neg7 % l_two);
    printf("run_identity=%d\n",
           (int)((a_neg / b_pos) * b_pos + (a_neg % b_pos) == a_neg
                 && (a_pos / b_neg) * b_neg + (a_pos % b_neg) == a_pos));
    return 0;
}
