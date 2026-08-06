/* Single- and double-precision arithmetic at fixed print precision with margin.
 *
 * Area 13 program 1 of 4, and the area's happy path.  It drives the same set of
 * arithmetic operators through binary32 (float) and binary64 (double) and, through
 * the runtime twin of every line, the four backends' floating-point instruction
 * selection and their four different floating-point calling conventions: System V
 * AMD64 passes floating arguments in xmm0-xmm7, i686 cdecl passes them on the stack
 * and returns in the x87 stack, AAPCS64 uses v0-v7 and RISC-V LP64D uses fa0-fa7
 * (docs/technical-specifications.md lines 162, 166, 457-462, 546, 565).  Two
 * spellings of one computation that agree on x86-64 but disagree on one of the
 * other three localize a defect to that backend, which is the whole point of
 * comparing them.
 *
 * THE AREA RULE, which organises the whole file: print at fixed precision with
 * margin.  Every floating value here is a DYADIC RATIONAL - a finite sum of powers
 * of two, built only from 0.125, 0.25, 0.5, 0.75, 1.25, 1.5, 2.0, 2.5, 3.0, 4.0,
 * 8.0 and 10.0 - and every intermediate and final result is one too.  The widest
 * result needs eight significand bits (48.75 is 110000.11 in binary); the rest need
 * fewer.  Eight bits fits binary32's twenty-four, binary64's fifty-three and x87
 * extended's sixty-four, so NO operation in this program ever makes a rounding
 * decision at all.
 *
 * That is what makes this program comparable byte for byte across targets rather
 * than merely comparable in principle.  i686 evaluates floating expressions in x87
 * extended precision - FLT_EVAL_METHOD 2 - while x86-64 uses SSE, AArch64 uses its
 * FP/SIMD unit and RISC-V 64 uses the F and D extensions.  Excess precision changes
 * a result only where a rounding decision was taken, and here none ever is: an exact
 * value is exact in every one of those formats, and converting it between them is
 * lossless in both directions.  A single operand that needed rounding - 0.1, or a
 * quotient by anything other than a power of two - would let i686 double-round and
 * would manufacture a divergence that says nothing about any compiler.
 *
 * That exactness argument is a property of the program above, and it holds whether or
 * not anything runs.  The sibling record 001_float_double_arithmetic.expected acts on it:
 * because the printed digits are exact on every target, the record restricts no target,
 * disables no oracle and carries no expected-divergence marker - four targets, three
 * optimization levels, all three oracles - and its single golden stdout is byte-identical
 * across all twelve cells, which is what lets oracle (b) compare the four backends with no
 * narrowing and oracle (c) hold those bytes as the record.
 *
 * Every value is printed with an explicit .6 precision.  Each needs at most three
 * fractional digits, so the printed digits are exact with three digits to spare -
 * that spare room is the "margin", and it is why no printed line can turn on a
 * last-digit rounding decision either.
 *
 * THE TWO-VARIANT RULE.  Each computation is written ONCE as a macro and
 * instantiated TWICE: over literal constants, which the constant folder evaluates
 * at translation time, and over volatile-qualified operands, which must be re-read
 * at run time and so cannot be folded away.  The two results are printed on
 * adjacent lines as fold_X / rt_X.  Without the runtime spelling, -O1 and -O2 would
 * answer every line with the folder's value and the floating-point backend would
 * never be asked to compute anything, so a code-generation defect would escape
 * while the suite reported PASS.  Using one macro for both spellings is what makes
 * the two the same computation rather than two computations that resemble each
 * other, and a final all_pairs_agree line asserts the two spellings agreed on every
 * property at once.
 *
 * Evaluation order.  Every macro parameter appears exactly once in every macro
 * body, so no operand is evaluated twice; where a computation needs one value in two
 * places, two DISTINCT volatile objects supply it (vf_b / vf_b_alt, vd_b / vd_b_alt)
 * rather than one object read twice.  Each volatile object is read at most once per
 * expression, and the objects read within one expression are distinct and unrelated,
 * so no printed value can depend on the unspecified order in which those reads
 * happen.  The two helper-function calls take no volatile operand at all: each
 * argument is read into a plain local in a statement of its own first, so no call
 * anywhere in this program - helper or printf - receives an argument with a side
 * effect.  Every result is stored in its own named variable before it is printed.
 *
 * Freedom from undefined behaviour.  No divisor is zero, and every divisor is a
 * power of two.  No operation overflows to infinity and none produces a NaN: every
 * operand is a small dyadic rational, the largest magnitude reached anywhere is
 * 50.0, and every result is exact.  No NaN, infinity or negative zero is written,
 * produced or assumed about.  No floating value is type-punned through a pointer or
 * a union, so nothing depends on representation, and there is no aliasing violation.
 * Every volatile object is initialized at its declaration, so no uninitialized
 * storage is read.  No object is modified twice between sequence points: each
 * compound assignment is a full expression of its own.  Nothing is converted to an
 * integer type, so the one undefined case of a floating-to-integer conversion cannot
 * arise.  Every float -> double widening and double -> float narrowing carries an
 * explicit cast, which is also what keeps the program clean under -Wconversion and
 * -Wsign-conversion.  Area 13 sanctions no deviation from the default warning gate,
 * so the program is written to satisfy that gate in full - -Wall -Wextra -pedantic
 * -Wconversion -Wsign-conversion -Wshadow -Werror - and it does: the reference
 * compiler accepts it with no diagnostic under exactly those flags.  The gate the
 * harness applies is selected by the program's own record, which declares no
 * ub_audit_flags deviation, so the flags above are precisely the ones the audit uses.
 *
 * No header is named.  bcc ships no stdio.h - its bundled set is the nine required
 * freestanding headers plus a bonus stdatomic.h (docs/technical-specifications.md
 * line 19 and lines 202-214) - so naming one would fail against bcc while
 * succeeding against the reference compiler, which is a divergence caused by the
 * test rather than by the compiler.  float.h is bundled but is deliberately not
 * named either: it belongs to 12_preprocessor/003_bundled_header_inclusion.c, and
 * nothing here needs it, because no limit or epsilon is consulted.  printf is
 * hand-declared and is the only library function used, so the program links against
 * libc alone under -static.  The format set is %d and %.6f.
 *
 * Deliberately elsewhere, so this program does not duplicate a sibling: general
 * float <-> int conversion boundaries belong to 002_float_int_conversions.c;
 * comparison and ordering breadth over finite values to 003_finite_comparisons.c;
 * the widest floating type, whose representation differs per target, to
 * 004_long_double_target_restricted.c, and no long double appears here at all;
 * argument counts large enough to exhaust a target's floating argument registers and
 * force a stack spill to 14_abi_calling_convention/002_many_float_parameters.c, so
 * the two helper calls here take two arguments each and probe only that a floating
 * argument and a floating return value cross a call boundary intact; and the
 * constant folder's own behaviour to 02_constant_expressions/008_float_constant_folding.c,
 * whose folded spellings this program's folded spellings deliberately resemble
 * because both halves of the two-variant rule are needed wherever arithmetic is
 * under test.
 */

int printf(const char *, ...);

/* Each computation is written once here and instantiated twice below.  Every
   parameter appears exactly once in every body, so no operand is evaluated more
   than once and no volatile object is read twice within a single expression. */
#define FP_ADD(A, B)                  ((A) + (B))
#define FP_SUB(A, B)                  ((A) - (B))
#define FP_MUL(A, B)                  ((A) * (B))
#define FP_DIV(A, B)                  ((A) / (B))
#define FP_NEG_SUM(A, B)              (-((A) + (B)))
#define FP_CHAIN(A, B, C, D, E)       ((((A) + (B)) * (C)) - ((D) / (E)))

/* Compound assignment applied to a floating lvalue: a read-modify-write for each
   of the four arithmetic operators, each in a full expression of its own, so no
   object is modified twice between sequence points.  The macro is a do/while(0)
   statement so that an instantiation followed by a semicolon is exactly one
   statement, and DST appears once per assignment. */
#define FP_COMPOUND(DST, A, B, C, D)                                          \
    do {                                                                      \
        (DST) += (A);                                                         \
        (DST) *= (B);                                                         \
        (DST) -= (C);                                                         \
        (DST) /= (D);                                                         \
    } while (0)

/* The two cross-precision conversions, each with the cast written out.  WIDEN
   promotes a float operand to double and continues in double; NARROW evaluates in
   double and converts the result to float. */
#define FP_WIDEN(A, B)                ((double)(A) + (B))
#define FP_NARROW(A, B)               ((float)((A) + (B)))

/* One arithmetic step behind a call boundary, once per precision.  These exist so
   that a floating argument and a floating return value have to survive the target's
   calling convention: xmm0-xmm7 on x86-64, the stack and the x87 return slot on
   i686, v0-v7 on AArch64, fa0-fa7 on RISC-V 64.  Both are static, so at -O1 and -O2
   they are candidates for inlining and at -O0 they are genuine calls; the printed
   value must not depend on which happened. */
static float fp_scale_flt(float lhs, float rhs)
{
    return (lhs + rhs) * 0.5f;
}

static double fp_scale_dbl(double lhs, double rhs)
{
    return (lhs + rhs) * 0.5;
}

int main(void)
{
    /* Runtime operands.  Volatile, and each initialized at its declaration, so
       every read is a genuine load the optimizer may not elide, fold or reorder
       across another volatile access.  The _alt objects exist so that a
       computation needing one value twice reads two different objects once each
       rather than one object twice. */
    volatile float vf_a = 2.5f;
    volatile float vf_b = 1.25f;
    volatile float vf_b_alt = 1.25f;
    volatile float vf_two = 2.0f;
    volatile float vf_four = 4.0f;
    volatile float vf_half = 0.5f;
    volatile float vf_three_halves = 1.5f;
    volatile float vf_quarter = 0.25f;
    volatile float vf_eight = 8.0f;
    volatile float vf_three = 3.0f;
    volatile double vd_a = 10.0;
    volatile double vd_b = 2.5;
    volatile double vd_b_alt = 2.5;
    volatile double vd_two = 2.0;
    volatile double vd_four = 4.0;
    volatile double vd_half = 0.5;
    volatile double vd_three_halves = 1.5;
    volatile double vd_quarter = 0.25;
    volatile double vd_eight = 8.0;
    volatile double vd_three = 3.0;
    volatile double vd_eighth = 0.125;
    volatile double vd_three_quarters = 0.75;

    /* ---- Folded variant: every operand is a floating constant, so each
       initializer is a constant expression the folder can evaluate outright. ---- */

    /* binary32.  2.5 and 1.25 are the operands throughout; the divisor is a power
       of two so the quotient is exact. */
    float fold_flt_add = FP_ADD(2.5f, 1.25f);                          /*  3.75   */
    float fold_flt_sub = FP_SUB(2.5f, 1.25f);                          /*  1.25   */
    float fold_flt_mul = FP_MUL(2.5f, 1.25f);                          /*  3.125  */
    float fold_flt_div = FP_DIV(2.5f, 2.0f);                           /*  1.25   */
    float fold_flt_neg = FP_NEG_SUM(2.5f, 1.25f);                      /* -3.75   */
    float fold_flt_chain = FP_CHAIN(2.5f, 1.25f, 4.0f, 1.25f, 2.0f);   /* 14.375  */
    float fold_flt_compound = 2.5f;
    float fold_flt_call = fp_scale_flt(2.5f, 1.25f);                   /*  1.875  */

    /* binary64.  10.0 and 2.5 are the operands throughout, so a value that is
       wrong only in the wider type shows up on its own line rather than being
       masked by agreement with the narrower one. */
    double fold_dbl_add = FP_ADD(10.0, 2.5);                           /* 12.5    */
    double fold_dbl_sub = FP_SUB(10.0, 2.5);                           /*  7.5    */
    double fold_dbl_mul = FP_MUL(10.0, 2.5);                           /* 25.0    */
    double fold_dbl_div = FP_DIV(10.0, 4.0);                           /*  2.5    */
    double fold_dbl_neg = FP_NEG_SUM(10.0, 2.5);                       /* -12.5   */
    double fold_dbl_chain = FP_CHAIN(10.0, 2.5, 4.0, 2.5, 2.0);        /* 48.75   */
    double fold_dbl_compound = 10.0;
    double fold_dbl_call = fp_scale_dbl(10.0, 2.5);                    /*  6.25   */

    /* Cross-precision.  The same dyadic computation is spelled once in float and
       once in double; because every value is exact in both formats the two must
       agree after the float result is widened, and that agreement is asserted on a
       line of its own. */
    double fold_widen = FP_WIDEN(2.5f, 0.125);                         /*  2.625  */
    float fold_narrow = FP_NARROW(0.75, 1.5);                          /*  2.25   */
    float fold_xprec_flt = FP_CHAIN(1.5f, 0.25f, 8.0f, 3.0f, 4.0f);    /* 13.25   */
    double fold_xprec_dbl = FP_CHAIN(1.5, 0.25, 8.0, 3.0, 4.0);        /* 13.25   */
    int fold_xprec_agree;

    /* ---- Runtime variant: the same macros over volatile operands.  Each value
       must be computed from loads performed at run time, so these lines exercise
       the backend's floating-point instruction selection rather than the folder. -- */

    float rt_flt_add = FP_ADD(vf_a, vf_b);
    float rt_flt_sub = FP_SUB(vf_a, vf_b);
    float rt_flt_mul = FP_MUL(vf_a, vf_b);
    float rt_flt_div = FP_DIV(vf_a, vf_two);
    float rt_flt_neg = FP_NEG_SUM(vf_a, vf_b);
    float rt_flt_chain = FP_CHAIN(vf_a, vf_b, vf_four, vf_b_alt, vf_two);
    float rt_flt_compound = vf_a;
    float rt_flt_call;

    double rt_dbl_add = FP_ADD(vd_a, vd_b);
    double rt_dbl_sub = FP_SUB(vd_a, vd_b);
    double rt_dbl_mul = FP_MUL(vd_a, vd_b);
    double rt_dbl_div = FP_DIV(vd_a, vd_four);
    double rt_dbl_neg = FP_NEG_SUM(vd_a, vd_b);
    double rt_dbl_chain = FP_CHAIN(vd_a, vd_b, vd_four, vd_b_alt, vd_two);
    double rt_dbl_compound = vd_a;
    double rt_dbl_call;

    double rt_widen = FP_WIDEN(vf_a, vd_eighth);
    float rt_narrow = FP_NARROW(vd_three_quarters, vd_three_halves);
    float rt_xprec_flt =
        FP_CHAIN(vf_three_halves, vf_quarter, vf_eight, vf_three, vf_four);
    double rt_xprec_dbl =
        FP_CHAIN(vd_three_halves, vd_quarter, vd_eight, vd_three, vd_four);
    int rt_xprec_agree;

    /* Call arguments are read into plain locals in statements of their own, so
       neither helper call receives an argument with a side effect.  The values
       still originate in volatile loads, so nothing about them is available to the
       folder. */
    float rt_call_flt_lhs = vf_a;
    float rt_call_flt_rhs = vf_b;
    double rt_call_dbl_lhs = vd_a;
    double rt_call_dbl_rhs = vd_b;

    int all_pairs_agree;

    /* Compound assignment, folded then runtime.  The folded sequence starts from a
       literal and applies literal operands, so the folder can propagate it; the
       runtime sequence starts from a volatile load and applies volatile operands,
       so every step must be emitted.  Both walk 2.5 -> 3.75 -> 7.5 -> 7.0 -> 1.75
       in float and 10.0 -> 12.5 -> 25.0 -> 24.5 -> 6.125 in double. */
    FP_COMPOUND(fold_flt_compound, 1.25f, 2.0f, 0.5f, 4.0f);
    FP_COMPOUND(rt_flt_compound, vf_b, vf_two, vf_half, vf_four);
    FP_COMPOUND(fold_dbl_compound, 2.5, 2.0, 0.5, 4.0);
    FP_COMPOUND(rt_dbl_compound, vd_b, vd_two, vd_half, vd_four);

    rt_flt_call = fp_scale_flt(rt_call_flt_lhs, rt_call_flt_rhs);
    rt_dbl_call = fp_scale_dbl(rt_call_dbl_lhs, rt_call_dbl_rhs);

    /* Equality on floating values is deterministic here precisely because every
       value is exactly representable in both formats, so the widening is lossless
       and the comparison turns on no rounding decision. */
    fold_xprec_agree = ((double)fold_xprec_flt == fold_xprec_dbl);
    rt_xprec_agree = ((double)rt_xprec_flt == rt_xprec_dbl);

    /* The folded and runtime spellings of every computation must agree.  Each pair
       is also printed individually below, so this is a summary on top of the
       per-property lines rather than a substitute for them: it catches a pair that
       agrees with neither expectation, which two separately wrong lines could
       otherwise be misread as. */
    all_pairs_agree =
        (fold_flt_add == rt_flt_add) && (fold_flt_sub == rt_flt_sub)
        && (fold_flt_mul == rt_flt_mul) && (fold_flt_div == rt_flt_div)
        && (fold_flt_neg == rt_flt_neg) && (fold_flt_chain == rt_flt_chain)
        && (fold_flt_compound == rt_flt_compound)
        && (fold_flt_call == rt_flt_call)
        && (fold_dbl_add == rt_dbl_add) && (fold_dbl_sub == rt_dbl_sub)
        && (fold_dbl_mul == rt_dbl_mul) && (fold_dbl_div == rt_dbl_div)
        && (fold_dbl_neg == rt_dbl_neg) && (fold_dbl_chain == rt_dbl_chain)
        && (fold_dbl_compound == rt_dbl_compound)
        && (fold_dbl_call == rt_dbl_call)
        && (fold_widen == rt_widen) && (fold_narrow == rt_narrow)
        && (fold_xprec_flt == rt_xprec_flt)
        && (fold_xprec_dbl == rt_xprec_dbl)
        && (fold_xprec_agree == rt_xprec_agree);

    /* Additive, subtractive and multiplicative arithmetic in binary32.  Every
       float argument to the variadic printf carries an explicit (double) cast, so
       the default argument promotion is stated rather than implied; the cast is
       lossless because the value is exact. */
    printf("fold_flt_add=%.6f\n", (double)fold_flt_add);
    printf("rt_flt_add=%.6f\n", (double)rt_flt_add);
    printf("fold_flt_sub=%.6f\n", (double)fold_flt_sub);
    printf("rt_flt_sub=%.6f\n", (double)rt_flt_sub);
    printf("fold_flt_mul=%.6f\n", (double)fold_flt_mul);
    printf("rt_flt_mul=%.6f\n", (double)rt_flt_mul);

    /* Division by an exact power of two, which is itself exact - the only kind of
       division this corpus may print, since any other divisor would round. */
    printf("fold_flt_div=%.6f\n", (double)fold_flt_div);
    printf("rt_flt_div=%.6f\n", (double)rt_flt_div);

    /* Unary negation, then a chain whose parentheses fix the grouping: add,
       multiply, divide and subtract in one expression, with a result still exact. */
    printf("fold_flt_neg=%.6f\n", (double)fold_flt_neg);
    printf("rt_flt_neg=%.6f\n", (double)rt_flt_neg);
    printf("fold_flt_chain=%.6f\n", (double)fold_flt_chain);
    printf("rt_flt_chain=%.6f\n", (double)rt_flt_chain);

    /* Compound assignment against a float lvalue, and one arithmetic step behind a
       call boundary. */
    printf("fold_flt_compound=%.6f\n", (double)fold_flt_compound);
    printf("rt_flt_compound=%.6f\n", (double)rt_flt_compound);
    printf("fold_flt_call=%.6f\n", (double)fold_flt_call);
    printf("rt_flt_call=%.6f\n", (double)rt_flt_call);

    /* The same operator set in binary64, on its own operands, so a defect present
       in only one of the two precisions cannot hide behind the other. */
    printf("fold_dbl_add=%.6f\n", fold_dbl_add);
    printf("rt_dbl_add=%.6f\n", rt_dbl_add);
    printf("fold_dbl_sub=%.6f\n", fold_dbl_sub);
    printf("rt_dbl_sub=%.6f\n", rt_dbl_sub);
    printf("fold_dbl_mul=%.6f\n", fold_dbl_mul);
    printf("rt_dbl_mul=%.6f\n", rt_dbl_mul);
    printf("fold_dbl_div=%.6f\n", fold_dbl_div);
    printf("rt_dbl_div=%.6f\n", rt_dbl_div);
    printf("fold_dbl_neg=%.6f\n", fold_dbl_neg);
    printf("rt_dbl_neg=%.6f\n", rt_dbl_neg);
    printf("fold_dbl_chain=%.6f\n", fold_dbl_chain);
    printf("rt_dbl_chain=%.6f\n", rt_dbl_chain);
    printf("fold_dbl_compound=%.6f\n", fold_dbl_compound);
    printf("rt_dbl_compound=%.6f\n", rt_dbl_compound);
    printf("fold_dbl_call=%.6f\n", fold_dbl_call);
    printf("rt_dbl_call=%.6f\n", rt_dbl_call);

    /* Mixed precision: a float operand widened into a double expression, and a
       double expression narrowed to float.  Both conversions are written out, and
       both are lossless on these values. */
    printf("fold_widen=%.6f\n", fold_widen);
    printf("rt_widen=%.6f\n", rt_widen);
    printf("fold_narrow=%.6f\n", (double)fold_narrow);
    printf("rt_narrow=%.6f\n", (double)rt_narrow);

    /* One computation, two precisions, and the assertion that they agree. */
    printf("fold_xprec_flt=%.6f\n", (double)fold_xprec_flt);
    printf("rt_xprec_flt=%.6f\n", (double)rt_xprec_flt);
    printf("fold_xprec_dbl=%.6f\n", fold_xprec_dbl);
    printf("rt_xprec_dbl=%.6f\n", rt_xprec_dbl);
    printf("fold_xprec_agree=%d\n", fold_xprec_agree);
    printf("rt_xprec_agree=%d\n", rt_xprec_agree);

    printf("all_pairs_agree=%d\n", all_pairs_agree);
    return 0;
}
