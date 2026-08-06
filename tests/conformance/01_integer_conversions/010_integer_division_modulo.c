/* No divisor is ever zero, and the one undefined signed case - the most negative
 * value divided by -1 - is deliberately never formed.
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

    /* Runtime variant: the operands are volatile, so each quotient and remainder
     * is computed from values read at run time rather than folded.
     *
     * STAGING, AND WHY EVERY VOLATILE READ GETS A STATEMENT OF ITS OWN.  An
     * access to a volatile object is an observable side effect, and C11
     * 6.5.2.2p10 leaves the order of evaluation of a call's arguments
     * unspecified, so a call that divided one volatile object by another twice
     * over would sequence four side effects in an order the standard does not
     * fix.  Each operand is therefore read once, in its own full expression,
     * into a plain object; the quotients, the remainders and the identity
     * results are then computed from those plain objects in statements of their
     * own, and the calls below pass plain objects only and contain no side
     * effect at all.  The suite's authoring rule is at most one side-effecting
     * argument per call, and this satisfies it structurally.  It costs the test
     * nothing: a value that arrived through a volatile load stays opaque to the
     * optimizer, so every division and every remainder below is still emitted
     * as a run-time operation rather than folded. */
    int r_a_pos = a_pos;
    int r_a_neg = a_neg;
    int r_b_pos = b_pos;
    int r_b_neg = b_neg;
    int r_big_neg = big_neg;
    int r_three = three;
    unsigned int r_u_seven = u_seven;
    unsigned int r_u_two = u_two;
    long long r_l_neg7 = l_neg7;
    long long r_l_two = l_two;

    int q_pos_pos = r_a_pos / r_b_pos;
    int m_pos_pos = r_a_pos % r_b_pos;
    int q_neg_pos = r_a_neg / r_b_pos;
    int m_neg_pos = r_a_neg % r_b_pos;
    int q_pos_neg = r_a_pos / r_b_neg;
    int m_pos_neg = r_a_pos % r_b_neg;
    int q_neg_neg = r_a_neg / r_b_neg;
    int m_neg_neg = r_a_neg % r_b_neg;
    int r_exact_dividend = r_a_neg - 1;
    int q_exact = r_exact_dividend / r_b_pos;
    int m_exact = r_exact_dividend % r_b_pos;
    int q_large = r_big_neg / r_three;
    int m_large = r_big_neg % r_three;
    unsigned int q_unsigned = r_u_seven / r_u_two;
    unsigned int m_unsigned = r_u_seven % r_u_two;
    long long q_llong = r_l_neg7 / r_l_two;
    long long m_llong = r_l_neg7 % r_l_two;
    int identity_neg_pos = q_neg_pos * r_b_pos + m_neg_pos == r_a_neg;
    int identity_pos_neg = q_pos_neg * r_b_neg + m_pos_neg == r_a_pos;
    int run_identity = identity_neg_pos && identity_pos_neg;

    printf("run_pos_pos=%d %d\n", q_pos_pos, m_pos_pos);
    printf("run_neg_pos=%d %d\n", q_neg_pos, m_neg_pos);
    printf("run_pos_neg=%d %d\n", q_pos_neg, m_pos_neg);
    printf("run_neg_neg=%d %d\n", q_neg_neg, m_neg_neg);
    printf("run_exact=%d %d\n", q_exact, m_exact);
    printf("run_large=%d %d\n", q_large, m_large);
    printf("run_unsigned=%u %u\n", q_unsigned, m_unsigned);
    printf("run_llong=%lld %lld\n", q_llong, m_llong);
    printf("run_identity=%d\n", run_identity);
    return 0;
}
