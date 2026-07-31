/* 004_common_subexpression_invariance.c -- Area 09, conformance suite.
 *
 * Claim under test: repeated subexpressions produce IDENTICAL results whether
 * or not the compiler eliminates them.  CSE is one of the two passes that -O2
 * adds over -O1, so this program is the area's sharpest -O1 versus -O2
 * discriminator.  Seeds are read once out of volatile storage so the values are
 * opaque to the constant folder, while the arithmetic itself is pure and is
 * therefore a genuine elimination candidate.
 */

int printf(const char *, ...);

int main(void)
{
    /* Seeds are read once out of volatile storage into ordinary locals.  The
       expressions below are therefore pure and are genuine common
       subexpression elimination candidates, while remaining opaque to the
       constant folder because the seed values are unknown at compile time. */
    volatile int v_a = 13;
    volatile int v_b = 7;
    volatile int v_c = 5;
    volatile unsigned int v_u = 3000000000u;

    int a = v_a;
    int b = v_b;
    int c = v_c;
    unsigned int u = v_u;

    int e1 = a * b + c;
    int e2 = a * b + c;           /* textually identical to e1 */
    int e3 = c + a * b;           /* same value, different spelling */
    int product = (a * b + c) * (a * b + c);
    int chain = (a + b) * (a + b) - (a + b);
    int nested = ((a * b) + (a * b)) / 2;
    unsigned int uexp1 = u / 7u + u % 7u;
    unsigned int uexp2 = u / 7u + u % 7u;

    /* Repeated subexpression across a call boundary and a branch. */
    int branchy;
    if (a * b + c > 0) {
        branchy = a * b + c;
    } else {
        branchy = -(a * b + c);
    }

    printf("e1=%d\n", e1);
    printf("e2=%d\n", e2);
    printf("e1_eq_e2=%d\n", e1 == e2);
    printf("e3=%d e1_eq_e3=%d\n", e3, e1 == e3);
    printf("product=%d\n", product);
    printf("chain=%d\n", chain);
    printf("nested=%d\n", nested);
    printf("uexp1=%u uexp2=%u equal=%d\n", uexp1, uexp2, uexp1 == uexp2);
    printf("branchy=%d\n", branchy);
    return 0;
}
