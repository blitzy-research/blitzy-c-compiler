/* Area 02 - constant expressions and folding, program 001: constant arithmetic
 * folded at compile time, paired with a runtime twin.
 *
 * Every property is printed twice: the `fold_' line computes it from a constant
 * expression, and the `runtime_' line computes the same value from volatile
 * operands, which no compiler may read at translation time.  That pairing is what
 * makes a divergence attributable -- `fold_' accuses the constant evaluator,
 * `runtime_' accuses the backend, and a self-disagreeing pair accuses one of the
 * two.  Without the twin a folded answer could stand in for the backend's and a
 * code-generation defect would escape unseen.
 *
 * Freedom from undefined behaviour is what makes a divergence here mean anything.
 * Every operand is a small literal or a volatile object holding one, so no signed
 * arithmetic can overflow; no divisor is zero; every shift count is a constant
 * strictly inside the width of its promoted left operand and only non-negative
 * values are left-shifted; and no object is modified anywhere, so nothing is
 * modified twice between sequence points.
 */

int printf(const char *, ...);

/* Two deliberately different kinds of name: an enumeration constant is itself a
 * constant expression, whereas a const-qualified object is not one in C, so a
 * name of the second kind is folded only by constant propagation. */
enum { BASE = 7, SCALE = 6 };

static const int LIMIT = 1000;

int main(void)
{
    volatile int v0 = 0;
    volatile int v1 = 1;
    volatile int v2 = 2;
    volatile int v3 = 3;
    volatile int v4 = 4;
    volatile int v6 = 6;
    volatile int v7 = 7;
    volatile int v9 = 9;
    volatile int v9_copy = 9;
    volatile int v10 = 10;
    volatile int v20 = 20;
    volatile int v30 = 30;
    volatile int v40 = 40;
    volatile int v42 = 42;
    volatile int v64 = 64;
    volatile int v100 = 100;
    volatile int v337 = 337;
    volatile int v1023 = 1023;
    volatile int v1024 = 1024;
    volatile int v1337 = 1337;
    volatile int v_mask_high = 0x0FF0;
    volatile int v_mask_low = 0x00FF;
    volatile int v_neg1024 = -1024;
    volatile int v_base = BASE;
    volatile int v_scale = SCALE;
    volatile int v_limit = LIMIT;
    volatile long long w100000 = 100000LL;
    volatile long long w_a = 1234567890123LL;
    volatile long long w_b = 876543210987LL;
    volatile long long w_one = 1LL;
    volatile long long w_big = 10000000000LL;
    volatile long long w_seven = 7LL;

    /* Group A - the additive and multiplicative operators.  Division truncates
     * toward zero and the remainder takes the sign of the dividend, both of
     * which C11 6.5.5p6 fixes rather than leaving to the implementation, so
     * the four-target measurement confirms conformance rather than
     * establishing agreement; no target restriction applies, and every operand
     * here is positive in any case. */
    printf("fold_add=%d\n", 7 + 6);
    printf("runtime_add=%d\n", v7 + v6);
    printf("fold_sub=%d\n", 9 - 4);
    printf("runtime_sub=%d\n", v9 - v4);
    printf("fold_mul=%d\n", 7 * 6);
    printf("runtime_mul=%d\n", v7 * v6);
    printf("fold_div=%d\n", 100 / 7);
    printf("runtime_div=%d\n", v100 / v7);
    printf("fold_mod=%d\n", 100 % 7);
    printf("runtime_mod=%d\n", v100 % v7);
    printf("fold_nested=%d\n", (9 - 4) * (3 + 2));
    printf("runtime_nested=%d\n", (v9 - v4) * (v3 + v2));

    /* Group B - precedence and associativity.  Agreement with the fully
     * parenthesized form proves the parse, not merely the arithmetic; right
     * association would print 90 and 32 instead of 50 and 8. */
    printf("fold_precedence=%d\n", 2 + 3 * 4 - 6 / 3);
    printf("runtime_precedence=%d\n", v2 + v3 * v4 - v6 / v3);
    printf("fold_precedence_paren=%d\n", 2 + (3 * 4) - (6 / 3));
    printf("runtime_precedence_paren=%d\n", v2 + (v3 * v4) - (v6 / v3));
    printf("fold_assoc_sub=%d\n", 100 - 30 - 20);
    printf("runtime_assoc_sub=%d\n", v100 - v30 - v20);
    printf("fold_assoc_div=%d\n", 64 / 4 / 2);
    printf("runtime_assoc_div=%d\n", v64 / v4 / v2);

    /* Group C - unary, bitwise and shift operators.  The one negative value
     * appears only as the left operand of a RIGHT shift, which C11 6.5.7p5 makes
     * implementation-defined rather than undefined; all four targets were
     * measured to shift arithmetically, so the value is comparable across them. */
    printf("fold_neg=%d\n", -(7 * 6));
    printf("runtime_neg=%d\n", -(v7 * v6));
    printf("fold_complement=%d\n", ~1023);
    printf("runtime_complement=%d\n", ~v1023);
    printf("fold_and=%d\n", 0x0FF0 & 0x00FF);
    printf("runtime_and=%d\n", v_mask_high & v_mask_low);
    printf("fold_or=%d\n", 0x0FF0 | 0x00FF);
    printf("runtime_or=%d\n", v_mask_high | v_mask_low);
    printf("fold_xor=%d\n", 0x0FF0 ^ 0x00FF);
    printf("runtime_xor=%d\n", v_mask_high ^ v_mask_low);
    printf("fold_shl=%d\n", 1 << 10);
    printf("runtime_shl=%d\n", v1 << v10);
    printf("fold_shr=%d\n", 1024 >> 3);
    printf("runtime_shr=%d\n", v1024 >> v3);
    printf("fold_shift=%d\n", (1 << 10) + (1024 >> 3));
    printf("runtime_shift=%d\n", (v1 << v10) + (v1024 >> v3));
    printf("fold_shr_negative=%d\n", -1024 >> 4);
    printf("runtime_shr_negative=%d\n", v_neg1024 >> v4);

    /* Group D - `long long' arithmetic.  Plain `long' is deliberately unused: it
     * is 4 bytes on i686 and 8 on the other three targets. */
    printf("fold_ll=%lld\n", 100000LL * 100000LL);
    printf("runtime_ll=%lld\n", w100000 * w100000);
    printf("fold_ll_add=%lld\n", 1234567890123LL + 876543210987LL);
    printf("runtime_ll_add=%lld\n", w_a + w_b);
    printf("fold_ll_shift=%lld\n", 1LL << 40);
    printf("runtime_ll_shift=%lld\n", w_one << v40);
    printf("fold_ll_div=%lld\n", 10000000000LL / 7LL);
    printf("runtime_ll_div=%lld\n", w_big / w_seven);
    printf("fold_ll_mod=%lld\n", 10000000000LL % 7LL);
    printf("runtime_ll_mod=%lld\n", w_big % w_seven);

    /* Group E - algebraic identities, where the twin is what proves a rewrite
     * value-preserving.  The self-subtraction is the sharpest of the five: the
     * constant form may be folded straight to zero, whereas for a volatile
     * object both reads must be performed and only then subtracted. */
    printf("fold_identity_add_zero=%d\n", 1337 + 0);
    printf("runtime_identity_add_zero=%d\n", v1337 + v0);
    printf("fold_identity_mul_one=%d\n", 1337 * 1);
    printf("runtime_identity_mul_one=%d\n", v1337 * v1);
    printf("fold_identity_mul_two=%d\n", 1337 * 2);
    printf("runtime_identity_mul_two=%d\n", v1337 * v2);
    printf("fold_identity_self_sub=%d\n", 1337 - 1337);
    printf("runtime_identity_self_sub=%d\n", v1337 - v1337);
    printf("fold_identity_div_one=%d\n", 1337 / 1);
    printf("runtime_identity_div_one=%d\n", v1337 / v1);

    /* Group F - relational, equality and logical operators.  Where two equal
     * operands are needed the folded spelling writes one as a sum, forcing a
     * nested reduction, and the runtime spelling uses v9_copy: neither variant
     * compares a thing with itself, which a compiler could answer unaided. */
    printf("fold_cmp_lt=%d\n", 3 < 9);
    printf("runtime_cmp_lt=%d\n", v3 < v9);
    printf("fold_cmp_gt=%d\n", 3 > 9);
    printf("runtime_cmp_gt=%d\n", v3 > v9);
    printf("fold_cmp_le=%d\n", 3 + 6 <= 9);
    printf("runtime_cmp_le=%d\n", v9 <= v9_copy);
    printf("fold_cmp_ge=%d\n", 9 >= 10);
    printf("runtime_cmp_ge=%d\n", v9 >= v10);
    printf("fold_cmp_eq=%d\n", 7 * 6 == 42);
    printf("runtime_cmp_eq=%d\n", v7 * v6 == v42);
    printf("fold_cmp_ne=%d\n", 7 * 6 != 42);
    printf("runtime_cmp_ne=%d\n", v7 * v6 != v42);
    printf("fold_logic_and=%d\n", (3 < 9) && (3 + 6 <= 9));
    printf("runtime_logic_and=%d\n", (v3 < v9) && (v9 <= v9_copy));
    printf("fold_logic_or=%d\n", (3 > 9) || (9 >= 3 + 6));
    printf("runtime_logic_or=%d\n", (v3 > v9) || (v9 >= v9_copy));
    printf("fold_logic_not=%d\n", !(3 > 9));
    printf("runtime_logic_not=%d\n", !(v3 > v9));
    printf("fold_logic_mixed=%d\n", ((3 < 9) && !(9 < 3)) || (7 * 6 != 42));
    printf("runtime_logic_mixed=%d\n", ((v3 < v9) && !(v9 < v3)) || (v7 * v6 != v42));

    /* Group G - the two kinds of name declared above, each against its twin. */
    printf("fold_enum_constant=%d\n", BASE * SCALE);
    printf("runtime_enum_constant=%d\n", v_base * v_scale);
    printf("fold_named_constant=%d\n", LIMIT + 337);
    printf("runtime_named_constant=%d\n", v_limit + v337);

    return 0;
}
