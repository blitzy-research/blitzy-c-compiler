/* Area 02 - constant expressions and folding, program 001: constant arithmetic
 * folded at compile time, paired with a runtime twin.
 *
 * Every property below is printed twice.  The `fold_' line computes it from a
 * constant expression, so the answer comes from the constant evaluator: from the
 * front end at -O0, and from `constant_fold' and `simplify' at -O1 and -O2
 * (docs/technical-specifications.md documents the pipeline as -O0 running no
 * passes, -O1 running mem2reg, constant_fold and dce, and -O2 adding cse and
 * simplify iterated to a fixed point).  The `runtime_' line computes the same
 * value from volatile-qualified operands, which no compiler may read at
 * translation time, so that answer comes from the code generator instead - at
 * every optimization level, on every target.
 *
 * The pairing is the point of this program, and it is load-bearing rather than
 * decorative: `volatile int x = 7; return x * 6;' emits a genuine runtime
 * multiply at -O2, whereas the plain form folds to a single immediate move.
 * Without the twin, optimization substitutes the folder's answer for the
 * backend's and a code-generation defect escapes detection entirely, while the
 * suite still reports a pass.  With it, a divergent `fold_' line accuses the
 * constant evaluator, a divergent `runtime_' line accuses the backend, and a
 * pair that disagrees with itself accuses one of the two - which is more than
 * either line could say alone.
 *
 * Determinism.  Only %d and %lld are used, so no printed value depends on a type
 * whose width or signedness varies between targets.  Every wide value is a
 * `long long' built from LL-suffixed literals, because plain `long' is 4 bytes on
 * i686 and 8 bytes on the other three targets and would diverge for that reason
 * alone.  No address, no pointer value and no plain-`char' value is printed;
 * nothing reads the clock, the environment, a file or a socket; every input is a
 * literal in this file; and the one external symbol is printf, hand-declared
 * below because bcc ships no <stdio.h>, so a directive naming that header would
 * fail against bcc while succeeding against the reference compiler - a spurious
 * divergence caused by the test rather than by the compiler.  No header of any
 * kind is named anywhere in this file.
 *
 * Freedom from undefined behaviour, which is what makes a divergence here mean
 * anything at all.  Every operand is a small literal, or a volatile object
 * holding one, so no signed addition, subtraction or multiplication can
 * overflow; no divisor is zero and the most negative value is never divided by
 * -1; every shift count is a non-negative constant strictly below the width of
 * its promoted left operand; only non-negative values are left-shifted, and the
 * single negative operand is right-shifted, which is implementation-defined
 * rather than undefined and was measured to be arithmetic on all four targets;
 * no object is read before it is initialized; no object is modified anywhere in
 * the program, so nothing is modified twice between sequence points; and no call
 * receives more than one argument that could have an effect.  The sibling
 * .expected record holds the recorded command lines, the golden output and this
 * argument in the form the audit gate reads.
 */

int printf(const char *, ...);

/* Named constants, so that a folded operand can reach an expression through a
 * name rather than as a literal.  The two spellings are deliberately different
 * in kind: an enumeration constant is itself a constant expression, whereas a
 * const-qualified object is not one in C, so a name of the second kind is folded
 * only by constant propagation. */
enum { BASE = 7, SCALE = 6 };

static const int LIMIT = 1000;

int main(void)
{
    /* The runtime operands.  Every one is volatile, so each read is an
     * observable side effect the implementation is obliged to perform and none
     * of these values may be substituted into the expressions below at
     * translation time.  Two objects hold the value 9 because a property that
     * needs two equal operands must compare distinct objects: comparing one
     * object with itself is a question a compiler can answer without consulting
     * either side. */
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
     * toward zero and the remainder takes the sign of the dividend; both were
     * measured identical on all four targets, so no target restriction applies
     * and every operand here is positive in any case. */
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

    /* Group B - precedence and associativity.  The unparenthesized form and its
     * fully parenthesized counterpart must agree, which is what proves the parse
     * rather than merely the arithmetic.  The two associativity lines
     * discriminate on their own: right association would print 90 and 32 instead
     * of 50 and 8. */
    printf("fold_precedence=%d\n", 2 + 3 * 4 - 6 / 3);
    printf("runtime_precedence=%d\n", v2 + v3 * v4 - v6 / v3);
    printf("fold_precedence_paren=%d\n", 2 + (3 * 4) - (6 / 3));
    printf("runtime_precedence_paren=%d\n", v2 + (v3 * v4) - (v6 / v3));
    printf("fold_assoc_sub=%d\n", 100 - 30 - 20);
    printf("runtime_assoc_sub=%d\n", v100 - v30 - v20);
    printf("fold_assoc_div=%d\n", 64 / 4 / 2);
    printf("runtime_assoc_div=%d\n", v64 / v4 / v2);

    /* Group C - the unary and bitwise operators, and the shifts.  Each bitwise
     * operand is non-negative and each shift count is well inside range.  The one
     * negative value appears only as the left operand of a RIGHT shift, which the
     * standard makes implementation-defined rather than undefined (C11 6.5.7p5);
     * a left shift of a negative value would be undefined and is therefore never
     * formed anywhere in this program.  The implementation-defined choice is
     * accounted for rather than assumed: the shift was measured to be arithmetic
     * on all four targets, so the line below is cross-comparable and needs no
     * target restriction. */
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

    /* Group D - `long long' arithmetic, written with LL-suffixed literals and
     * printed with %lld, so every value here is 64 bits wide on every target and
     * the property is width-normalized rather than target-dependent. */
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

    /* Group E - the algebraic identities the simplifier is documented to
     * rewrite: identity removal, strength reduction of a multiply by two into a
     * shift, and division by one.  The volatile twin is what proves each rewrite
     * value-preserving.  The self-subtraction is the sharpest of the five: the
     * constant form may be folded straight to zero, while for a volatile object
     * both reads must be performed and only then subtracted. */
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

    /* Group F - the relational, equality and logical operators, every one of
     * which yields exactly 0 or 1.  All six relational and equality spellings
     * appear, in both a true and a false instance, and the logical operators are
     * folded from them.  Where a property needs two equal operands, the folded
     * spelling writes one of them as a sum, so the evaluator must reduce a nested
     * constant expression before it can answer the comparison, and the runtime
     * spelling compares two distinct objects that happen to hold the same value.
     * Neither variant compares one thing with itself, which a compiler could
     * answer without consulting either side. */
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

    /* Group G - constant propagation through a name.  BASE and SCALE are
     * enumeration constants, and an enumeration constant has type `int' and is a
     * constant expression; LIMIT is a const-qualified object, which is not a
     * constant expression in C, so its line depends on propagation rather than on
     * constant-expression evaluation.  Both must reach the same answer as their
     * volatile twins. */
    printf("fold_enum_constant=%d\n", BASE * SCALE);
    printf("runtime_enum_constant=%d\n", v_base * v_scale);
    printf("fold_named_constant=%d\n", LIMIT + 337);
    printf("runtime_named_constant=%d\n", v_limit + v337);

    return 0;
}
