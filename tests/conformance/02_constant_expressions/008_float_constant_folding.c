/* Floating constant folding at fixed print precision.
 *
 * Area 02 program 8 of 8, and the floating-point counterpart of
 * 001_arithmetic_constant_folding.c.  It drives the constant folder over
 * floating-point operands and, through the runtime twin of every line, the four
 * backends' floating-point instruction selection.
 *
 * THE AREA RULE, which organises the whole file: print at fixed precision with
 * margin, and make no NaN or infinity identity assumption.  Every floating value
 * printed here is a DYADIC RATIONAL - a finite sum of powers of two such as 0.5,
 * 0.25, 0.125, 0.75, 1.5, 2.25, 3.5, 3.75 - so it is exactly representable in
 * both binary32 and binary64, and it is printed with an explicit precision short
 * enough that no rounding decision is ever at stake.  That is the "margin": the
 * printed digits are exact, with room to spare.  It is also what makes the twelve
 * cells of this program's matrix comparable byte for byte, because i686 evaluates
 * floating expressions in x87 extended precision (FLT_EVAL_METHOD 2) while the
 * other three targets do not.  Excess precision cannot perturb a value that is
 * already exact in every format involved, so no target restriction is needed and
 * this program carries no expected-divergence marker.
 *
 * THE TWO-VARIANT RULE, mandatory for an arithmetic program: every computation is
 * written ONCE as a macro and instantiated TWICE - over literal constants, which
 * the folder evaluates at compile time, and over volatile-qualified operands,
 * which must be read at run time and so cannot be folded away.  The two results
 * are printed on adjacent lines as fold_X / rt_X.  Without the runtime twin,
 * optimization would silently substitute the folder's answer for the backend's
 * and a floating-point code-generation defect would escape detection entirely;
 * with it, a divergence is attributable to the folder or to the backend rather
 * than to "one of the two".  Every macro parameter appears exactly once in its
 * body, so no operand is evaluated twice and no volatile object is read twice
 * within a single expression.  Where a computation needs the same value in two
 * places, two distinct volatile objects supply it (v_two / v_two_alt) rather than
 * one object read twice.  Every result is stored in its own named variable and
 * only those variables are passed to printf, so no call receives a
 * side-effecting argument and no argument-evaluation order is ever observable.
 *
 * DETERMINISM AND PORTABILITY.  This program names no header at all and
 * hand-declares the one libc prototype it needs: bcc ships no hosted stdio
 * header, so a directive pulling one in would fail on the bcc side of oracle (a)
 * while succeeding on the reference side - a spurious divergence caused by the
 * test rather than by the compiler.  No math function is called, so the program
 * links against libc alone under -static.  The format set is deliberately
 * minimal: %d for integers, %.6f and %.9f for floating values.  Nothing
 * width-dependent or representation-dependent is ever printed - no value of the
 * widest floating type (measured at 16, 12, 16 and 16 bytes across the four
 * targets, x87 80-bit against IEEE binary128), no plain long, no size_t, no
 * pointer, no address and no plain-char value.  Every float
 * passed to the variadic printf is written with an explicit (double) cast so the
 * default argument promotion is stated rather than implied, and every float ->
 * int conversion is written with an explicit (int) cast, which is what keeps the
 * program clean under the audit gate's -Wconversion and -Wsign-conversion with no
 * sanctioned deviation available to area 02.
 *
 * UNDEFINED-BEHAVIOUR FREEDOM, the precondition that makes both oracles sound.
 * No divisor is zero.  No operation overflows to infinity and none produces a
 * NaN: every operand is a small dyadic rational and every result is exact.  Each
 * float -> int conversion is well inside int's range by construction (3, -3 and
 * 7), which forecloses the one undefined case specific to this program.  No
 * signed integer overflow is possible - the only integers are conversion results
 * and comparison results.  No floating value is type-punned through a pointer or
 * a union, so nothing depends on representation.  Every volatile object is
 * initialized at its declaration, so no uninitialized storage is read.  No object
 * is modified twice between sequence points, no pointer arithmetic is performed
 * and no padding byte is observed.  The record's ub_notes carries this argument
 * in prose; the warning and sanitizer gates are its machine half.
 *
 * DELIBERATELY ELSEWHERE, so this program does not duplicate it: the widest
 * floating type's value comparison (13_floating_point/004, which owns the
 * target-varying representation and the recorded reason for restricting its
 * cross-backend comparison), general float <-> int conversion boundaries
 * (13_floating_point/002), finite ordering breadth (13_floating_point/003),
 * sizeof and _Alignof facts (004_sizeof_alignof.c), and the f suffix's typing
 * effect observed through _Generic (005_integer_constant_suffixes.c) - here the
 * f suffix appears only as part of a computed value.  NaN and infinity behaviour
 * is owned by nobody: the area rule forecloses it deliberately, so neither is
 * ever produced.
 *
 * Expected exit status 0, well inside the 0-125 range.
 */

int printf(const char *, ...);

/* Each computation is written once here and instantiated twice below.  Every
   parameter appears exactly once in every body. */
#define FP_ADD(A, B)              ((A) + (B))
#define FP_SUB(A, B)              ((A) - (B))
#define FP_MUL(A, B)              ((A) * (B))
#define FP_DIV(A, B)              ((A) / (B))
#define FP_SUM3(A, B, C)          ((A) + (B) + (C))
#define FP_MULADD(A, B, C)        (((A) * (B)) + (C))
#define FP_NEG_SUM(A, B)          (-((A) + (B)))
#define FP_GROUP_LEFT(A, B, C)    (((A) + (B)) * (C))
#define FP_GROUP_RIGHT(A, B, C)   ((A) + ((B) * (C)))
#define FP_CHAIN(A, B, C, D, E)   ((((A) + (B)) * (C)) - ((D) / (E)))
#define FP_EQ_SUM(A, B, C)        (((A) + (B)) == (C))
#define FP_LE_SUM(A, B, C)        (((A) + (B)) <= (C))
#define FP_LT(A, B)               ((A) < (B))
#define FP_GT(A, B)               ((A) > (B))
#define FP_LE(A, B)               ((A) <= (B))
#define FP_TRUNC(A)               ((int)(A))
#define FP_ROUNDTRIP(A, B, C)     ((int)((double)(A) / (B) * (C)))
#define FP_INT_TO_DOUBLE(A, B)    ((double)(A) / (B))
#define FP_INT_TO_FLOAT(A, B)     ((float)(A) / (B))

/* A static-duration object whose initializer is a floating constant expression.
   The folder evaluates 0.5 + 0.25 + 0.125 at compile time and the value reaches
   the program through the emitted data image rather than through an instruction,
   which is a second, independent path to the same answer. */
static const double k_sum3 = 0.5 + 0.25 + 0.125;

int main(void)
{
    /* Runtime operands.  Volatile, and each initialized at its declaration, so
       every read is a genuine load that the optimizer may not elide, fold or
       reorder across another volatile access. */
    volatile double v_half = 0.5;
    volatile double v_quart = 0.25;
    volatile double v_three_quarters = 0.75;
    volatile double v_three = 3.0;
    volatile double v_three_halves = 1.5;
    volatile double v_two = 2.0;
    volatile double v_two_alt = 2.0;
    volatile double v_one = 1.0;
    volatile double v_eight = 8.0;
    volatile double v_seven = 7.0;
    volatile double v_nine_quarters = 2.25;
    volatile double v_eighth = 0.125;
    volatile double v_four = 4.0;
    volatile double v_frac_pos = 3.75;
    volatile double v_frac_neg = -3.75;
    volatile float vf_half = 0.5f;
    volatile float vf_quart = 0.25f;
    volatile float vf_three_halves = 1.5f;
    volatile float vf_nine_quarters = 2.25f;
    volatile float vf_eighth = 0.125f;
    volatile float vf_four = 4.0f;
    volatile int vi_five = 5;
    volatile int vi_seven = 7;
    volatile int vi_nine = 9;

    /* Folded variant: every operand is a floating constant, so each initializer
       is a constant expression the folder can evaluate outright. */
    double f_add = FP_ADD(0.5, 0.25);                          /* 0.75  */
    double f_sub = FP_SUB(3.0, 0.75);                          /* 2.25  */
    double f_mul = FP_MUL(1.5, 2.0);                           /* 3.0   */
    double f_div_eighth = FP_DIV(1.0, 8.0);                    /* 0.125 */
    double f_div_half = FP_DIV(7.0, 2.0);                      /* 3.5   */
    float f_addf = FP_ADD(0.5f, 0.25f);                        /* 0.75  */
    float f_mixf = FP_MULADD(1.5f, 2.25f, 0.125f);             /* 3.5   */
    double f_mixd = FP_MULADD(1.5, 2.25, 0.125);               /* 3.5   */
    int f_mix_agree = ((double)f_mixf == f_mixd);              /* 1     */
    int f_cmp_eq = FP_EQ_SUM(0.5, 0.25, 0.75);                 /* 1     */
    int f_cmp_lt = FP_LT(0.5, 0.75);                           /* 1     */
    int f_cmp_gt = FP_GT(2.25, 1.5);                           /* 1     */
    int f_cmp_le = FP_LE_SUM(0.5, 0.25, 0.75);                 /* 1     */
    int f_cmp_le_strict = FP_LE(0.125, 0.25);                  /* 1     */
    int f_cmp_false = FP_LT(0.75, 0.5);                        /* 0     */
    int f_trunc_pos = FP_TRUNC(3.75);                          /* 3     */
    int f_trunc_neg = FP_TRUNC(-3.75);                         /* -3    */
    int f_roundtrip = FP_ROUNDTRIP(7, 2.0, 2.0);               /* 7     */
    double f_int_to_double = FP_INT_TO_DOUBLE(5, 2.0);         /* 2.5   */
    float f_int_to_float = FP_INT_TO_FLOAT(9, 4.0f);           /* 2.25  */
    double f_unary_minus = FP_NEG_SUM(0.5, 0.25);              /* -0.75 */
    double f_group_left = FP_GROUP_LEFT(1.5, 0.25, 4.0);       /* 7.0   */
    double f_group_right = FP_GROUP_RIGHT(1.5, 0.25, 4.0);     /* 2.5   */
    double f_sum3 = FP_SUM3(0.5, 0.25, 0.125);                 /* 0.875 */
    double f_static_scaled = FP_MUL(k_sum3, 2.0);              /* 1.75  */
    double f_chain = FP_CHAIN(0.5, 0.25, 8.0, 2.0, 4.0);       /* 5.5   */

    /* Runtime variant: the same macros over volatile operands.  Each value must
       be computed from loads performed at run time, so these lines exercise the
       backend's floating-point instruction selection rather than the folder. */
    double r_add = FP_ADD(v_half, v_quart);
    double r_sub = FP_SUB(v_three, v_three_quarters);
    double r_mul = FP_MUL(v_three_halves, v_two);
    double r_div_eighth = FP_DIV(v_one, v_eight);
    double r_div_half = FP_DIV(v_seven, v_two);
    float r_addf = FP_ADD(vf_half, vf_quart);
    float r_mixf = FP_MULADD(vf_three_halves, vf_nine_quarters, vf_eighth);
    double r_mixd = FP_MULADD(v_three_halves, v_nine_quarters, v_eighth);
    int r_mix_agree = ((double)r_mixf == r_mixd);
    int r_cmp_eq = FP_EQ_SUM(v_half, v_quart, v_three_quarters);
    int r_cmp_lt = FP_LT(v_half, v_three_quarters);
    int r_cmp_gt = FP_GT(v_nine_quarters, v_three_halves);
    int r_cmp_le = FP_LE_SUM(v_half, v_quart, v_three_quarters);
    int r_cmp_le_strict = FP_LE(v_eighth, v_quart);
    int r_cmp_false = FP_LT(v_three_quarters, v_half);
    int r_trunc_pos = FP_TRUNC(v_frac_pos);
    int r_trunc_neg = FP_TRUNC(v_frac_neg);
    int r_roundtrip = FP_ROUNDTRIP(vi_seven, v_two, v_two_alt);
    double r_int_to_double = FP_INT_TO_DOUBLE(vi_five, v_two);
    float r_int_to_float = FP_INT_TO_FLOAT(vi_nine, vf_four);
    double r_unary_minus = FP_NEG_SUM(v_half, v_quart);
    double r_group_left = FP_GROUP_LEFT(v_three_halves, v_quart, v_four);
    double r_group_right = FP_GROUP_RIGHT(v_three_halves, v_quart, v_four);
    double r_sum3 = FP_SUM3(v_half, v_quart, v_eighth);
    double r_static_scaled = FP_MUL(k_sum3, v_two);
    double r_chain = FP_CHAIN(v_half, v_quart, v_eight, v_two, v_four);

    /* The folded and runtime spellings of every computation must agree.  Each
       pair is also printed individually below, so this line is a summary on top
       of the per-property lines rather than a substitute for them. */
    int all_pairs_agree =
        (f_add == r_add) && (f_sub == r_sub) && (f_mul == r_mul)
        && (f_div_eighth == r_div_eighth) && (f_div_half == r_div_half)
        && (f_addf == r_addf) && (f_mixf == r_mixf) && (f_mixd == r_mixd)
        && (f_mix_agree == r_mix_agree) && (f_cmp_eq == r_cmp_eq)
        && (f_cmp_lt == r_cmp_lt) && (f_cmp_gt == r_cmp_gt)
        && (f_cmp_le == r_cmp_le) && (f_cmp_le_strict == r_cmp_le_strict)
        && (f_cmp_false == r_cmp_false)
        && (f_trunc_pos == r_trunc_pos) && (f_trunc_neg == r_trunc_neg)
        && (f_roundtrip == r_roundtrip)
        && (f_int_to_double == r_int_to_double)
        && (f_int_to_float == r_int_to_float)
        && (f_unary_minus == r_unary_minus)
        && (f_group_left == r_group_left)
        && (f_group_right == r_group_right) && (f_sum3 == r_sum3)
        && (f_static_scaled == r_static_scaled) && (f_chain == r_chain);

    /* Additive, subtractive and multiplicative folding over exact operands. */
    printf("fold_add=%.6f\n", f_add);
    printf("rt_add=%.6f\n", r_add);
    printf("fold_sub=%.6f\n", f_sub);
    printf("rt_sub=%.6f\n", r_sub);
    printf("fold_mul=%.6f\n", f_mul);
    printf("rt_mul=%.6f\n", r_mul);

    /* Division by an exact power of two is itself exact, which is why division
       appears here at all, and %.9f shows the extra digits are exact too. */
    printf("fold_div_eighth=%.9f\n", f_div_eighth);
    printf("rt_div_eighth=%.9f\n", r_div_eighth);
    printf("fold_div_half=%.9f\n", f_div_half);
    printf("rt_div_half=%.9f\n", r_div_half);

    /* Computed in float, then promoted to double explicitly at the variadic
       call, which is the promotion printf would perform implicitly. */
    printf("fold_addf=%.6f\n", (double)f_addf);
    printf("rt_addf=%.6f\n", (double)r_addf);

    /* The same dyadic computation in float and in double must agree, and the
       agreement is asserted as its own property so a precision-handling defect
       in either type shows up on its own line. */
    printf("fold_mixf=%.6f\n", (double)f_mixf);
    printf("rt_mixf=%.6f\n", (double)r_mixf);
    printf("fold_mixd=%.6f\n", f_mixd);
    printf("rt_mixd=%.6f\n", r_mixd);
    printf("fold_mix_agree=%d\n", f_mix_agree);
    printf("rt_mix_agree=%d\n", r_mix_agree);

    /* Floating comparison yielding an integer.  Equality is deterministic here
       because both sides are exactly representable.  <= is exercised twice, at its
       equality boundary and at a strictly-less pair, since those are separate
       lowerings; and the final pair is false on purpose, because a comparison that
       only ever answered 1 would not discriminate at all. */
    printf("fold_cmp_eq=%d\n", f_cmp_eq);
    printf("rt_cmp_eq=%d\n", r_cmp_eq);
    printf("fold_cmp_lt=%d\n", f_cmp_lt);
    printf("rt_cmp_lt=%d\n", r_cmp_lt);
    printf("fold_cmp_gt=%d\n", f_cmp_gt);
    printf("rt_cmp_gt=%d\n", r_cmp_gt);
    printf("fold_cmp_le=%d\n", f_cmp_le);
    printf("rt_cmp_le=%d\n", r_cmp_le);
    printf("fold_cmp_le_strict=%d\n", f_cmp_le_strict);
    printf("rt_cmp_le_strict=%d\n", r_cmp_le_strict);
    printf("fold_cmp_false=%d\n", f_cmp_false);
    printf("rt_cmp_false=%d\n", r_cmp_false);

    /* Conversions in both directions, every one well inside the destination
       range.  Truncation is toward zero, which the negative case pins down. */
    printf("fold_trunc_pos=%d\n", f_trunc_pos);
    printf("rt_trunc_pos=%d\n", r_trunc_pos);
    printf("fold_trunc_neg=%d\n", f_trunc_neg);
    printf("rt_trunc_neg=%d\n", r_trunc_neg);
    printf("fold_int_roundtrip=%d\n", f_roundtrip);
    printf("rt_int_roundtrip=%d\n", r_roundtrip);
    printf("fold_int_to_double=%.6f\n", f_int_to_double);
    printf("rt_int_to_double=%.6f\n", r_int_to_double);
    printf("fold_int_to_float=%.6f\n", (double)f_int_to_float);
    printf("rt_int_to_float=%.6f\n", (double)r_int_to_float);

    /* Unary minus, and grouping fixed by explicit parentheses: the two groupings
       of the same three operands must differ, which is what proves the
       parenthesization is honoured rather than reassociated. */
    printf("fold_unary_minus=%.6f\n", f_unary_minus);
    printf("rt_unary_minus=%.6f\n", r_unary_minus);
    printf("fold_group_left=%.6f\n", f_group_left);
    printf("rt_group_left=%.6f\n", r_group_left);
    printf("fold_group_right=%.6f\n", f_group_right);
    printf("rt_group_right=%.6f\n", r_group_right);

    /* A three-term sum, then the same value reached through a static const
       initializer, then that constant used in further arithmetic. */
    printf("fold_sum3=%.6f\n", f_sum3);
    printf("rt_sum3=%.6f\n", r_sum3);
    printf("static_const_init=%.6f\n", k_sum3);
    printf("static_const_matches_fold=%d\n", k_sum3 == f_sum3);
    printf("fold_static_scaled=%.6f\n", f_static_scaled);
    printf("rt_static_scaled=%.6f\n", r_static_scaled);

    /* Several folded subexpressions in one chain: add, multiply, divide and
       subtract, with a result that is still exact. */
    printf("fold_chain=%.6f\n", f_chain);
    printf("rt_chain=%.6f\n", r_chain);

    printf("all_pairs_agree=%d\n", all_pairs_agree);
    return 0;
}
