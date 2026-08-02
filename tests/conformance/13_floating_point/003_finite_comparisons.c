/* Floating-point comparison and ordering on finite values only.
 *
 * Area 13 program 3 of 4.  It drives the six relational and equality operators
 * over finite floating operands in both floating types and, through the runtime
 * twin of every comparison, the four backends' floating comparison instruction
 * selection and condition-code handling.  Comparison is where those four
 * backends differ most mechanically while being required to agree exactly:
 * x86-64 and i686 compare into flags and then materialize a boolean, AArch64
 * issues one fcmp and derives several booleans from that single result with
 * cset, and RISC-V 64 has dedicated feq / flt / fle instructions that produce an
 * integer directly (docs/technical-specifications.md lines 457-462, and the
 * per-backend register files at lines 546 and 565).  Every one of those
 * lowerings must produce the same answer here, so a divergence is a genuine
 * finding rather than an implementation-defined difference.
 *
 * THE AREA RULE, which organises the whole file: FINITE VALUES ONLY, and no NaN
 * or infinity identity assumption.  Concretely, and by construction:
 *
 *   - No NaN and no infinity appears anywhere, in any spelling.  There is no
 *     NAN, INFINITY, nan(), HUGE_VAL or __builtin_ form in this file, and no
 *     expression can produce either: there is no division at all, so no division
 *     by zero, and no magnitude exceeds 3.75, so nothing can overflow.
 *   - NO IDENTITY ASSUMPTION IS MADE ABOUT NaN.  In particular this file does
 *     NOT test x != x, the NaN self-inequality identity.  Every != below has two
 *     operands that are separately named values.
 *   - NO IDENTITY ASSUMPTION IS MADE ABOUT SIGNED ZERO.  This file does NOT test
 *     -0.0 == 0.0, and it never divides by a zero of either sign.  Negative zero
 *     is not merely untested but unreachable: no zero appears as an operand, no
 *     zero is negated, no zero is multiplied by a negative value, and no value is
 *     subtracted from itself - so no printed value can render as -0.000000 on one
 *     target and 0.000000 on another.
 *   - No subnormal arises.  The smallest magnitude used is 0.75, which is many
 *     orders of magnitude above the normal minimum of either format.
 *
 * That restriction is what gives this program its evidentiary value rather than
 * being caution for caution's sake.  NaN and infinity behaviour, signed-zero
 * identity and the quiet-versus-signalling distinction are precisely where the
 * four backends' comparison instructions, flag semantics and library behaviour
 * could legitimately differ, and where the requirements' implementation-defined
 * carve-out would leave a divergence uninterpretable.  Restricted to finite
 * values, every comparison below is a hard, unambiguous property of C semantics
 * that any correct compiler must reproduce byte for byte.
 *
 * WHY == AND != ARE LEGITIMATE HERE, and not the sloppiness they usually signal.
 * The customary advice against floating equality assumes rounded values.  Every
 * value in this file is a DYADIC RATIONAL - a finite sum of powers of two: 0.75,
 * 1.25, 1.5, 2.5, 3.75 and their negations - so each is exactly representable in
 * binary32 and in binary64 alike, and no operation is performed on any of them
 * beyond comparison and unary minus.  No rounding occurs at any step, so
 * equality is exact and is a hard property rather than an approximation.  That
 * is also why -Wfloat-equal is deliberately not a member of the audit gate.
 *
 * The same exactness is what makes the twelve cells of this program's matrix
 * comparable byte for byte.  i686 evaluates floating expressions in x87 extended
 * precision (FLT_EVAL_METHOD 2) while the other three targets do not, but excess
 * intermediate precision cannot perturb a value that is already exact in every
 * format involved, and so cannot change any comparison outcome.  No target
 * restriction is therefore needed, all three oracles are enabled, and this
 * program carries no expected-divergence marker.  The widest floating type is
 * deliberately absent: its representation differs across the four targets
 * (measured 16, 12, 16 and 16 bytes, x87 80-bit versus IEEE binary128), which is
 * 004_long_double_target_restricted.c's subject, and naming it here would
 * forfeit exactly the cross-backend comparability this program depends on.
 *
 * THE TWO-VARIANT RULE.  Every comparison is written ONCE as a macro and
 * instantiated TWICE: over literal constants, which the folder evaluates at
 * translation time, and over volatile-qualified operands, which must be re-read
 * at run time and so cannot be folded away.  Without the runtime variant the
 * comparison instructions would never be exercised at -O1 or -O2 - the folder's
 * answer would silently stand in for the backend's - and a code-generation
 * defect would escape detection entirely.  Verified at instruction level: the
 * runtime pairs emit real compares on all four backends while the folded pairs
 * become immediates.
 *
 * Each volatile operand is read ONCE into a plain local before any comparison
 * uses it.  That is what keeps the printed results independent of unspecified
 * evaluation order: a macro body may name a parameter more than once - the
 * ordering chains below do - but every argument is either a literal constant or
 * a plain non-volatile local, so no volatile object is ever read twice within
 * one expression and no ordering question can arise.
 *
 * Freedom from undefined behaviour.  There is no arithmetic here at all beyond
 * unary minus applied to exact non-zero constants, so there is no overflow of
 * any kind, no division, no shift, and no conversion that could be out of range;
 * the one conversion present, float to double, is a widening and is exact.
 * Nothing is type-punned through a pointer or a union, so nothing depends on
 * representation; no pointer is formed at all; no object is modified twice
 * between sequence points; every volatile object is initialized at its
 * declaration, so no uninitialized storage is read; and every result is stored
 * in its own named variable before it is printed, so no call receives a
 * side-effecting argument.  The && and || chains are deliberately
 * side-effect-free, side-effect suppression being area 06's subject rather than
 * this program's.
 *
 * Output discipline.  One line per group of related properties, so a single
 * divergent line localizes the defect to one comparison.  Every comparison
 * result is a plain int 0 or 1 printed with %d; the only floating values printed
 * are the two selected results, at the fixed precision the area rule requires,
 * %.6f - never default precision.  Nothing representation-dependent is printed:
 * no pointer, no plain long, no size_t and no plain-char value.  Every
 * float-to-double widening in a comparison carries an explicit (double) cast
 * rather than relying on the implicit usual arithmetic conversions, which is
 * what keeps the program clean under the audit gate's -Wconversion and
 * -Wsign-conversion; area 13 sanctions no deviation, so the default gate runs at
 * full strength and this program's record omits ub_audit_flags.
 *
 * No header is named: bcc ships no stdio.h, its bundled set being the nine
 * required freestanding headers plus a bonus stdatomic.h, ten files in all
 * (docs/technical-specifications.md line 19 and lines 202-214;
 * docs/project-guide.md line 212).  An #include <stdio.h> would fail against bcc
 * while succeeding against the reference compiler - a spurious divergence caused
 * by the test rather than the compiler.  float.h is bundled but is deliberately
 * not named either: no FLT_ or DBL_ characteristic is needed here, and the
 * bundled-header probe is 12_preprocessor/003_bundled_header_inclusion.c's
 * subject.
 */

int printf(const char *, ...);

/* Each comparison is written once here and instantiated twice below, so the
   folded and the runtime spelling are provably the same expression rather than
   two expressions that merely look alike.  Arguments are always literal
   constants or plain non-volatile locals, never volatile lvalues, so a parameter
   named more than once in a body costs no extra volatile read. */
#define FC_LT(A, B) ((A) < (B))
#define FC_LE(A, B) ((A) <= (B))
#define FC_GT(A, B) ((A) > (B))
#define FC_GE(A, B) ((A) >= (B))
#define FC_EQ(A, B) ((A) == (B))
#define FC_NE(A, B) ((A) != (B))

/* An explicit float-to-double widening, stated rather than implied. */
#define FC_WIDEN(A) ((double)(A))

/* Ordering over three values: A < B and B < C must hold, and so must A < C.
   Transitivity is asserted as an observable result rather than assumed. */
#define FC_CHAIN3(A, B, C) (((A) < (B)) && ((B) < (C)) && ((A) < (C)))

/* Short-circuit forms over floating comparisons, both side-effect-free.  In
   FC_OR2 the left operand is false and the right one true, so the || genuinely
   falls through to its second comparison instead of settling on the first. */
#define FC_AND2(A, B, C) (((A) < (B)) && ((B) < (C)))
#define FC_OR2(A, B, C, D) (((A) < (B)) || ((C) < (D)))

/* Boolean conversion of a comparison result. */
#define FC_NOT_LT(A, B) (!((A) < (B)))

/* A comparison result driving the conditional operator, which is the backend's
   select lowering; the if statements below exercise its branch lowering. */
#define FC_SELECT(A, B, T, F) (((A) < (B)) ? (T) : (F))

int main(void)
{
    /* Runtime operands.  Volatile, and each initialized at its declaration, so
       every read is a genuine load that the optimizer may neither elide nor fold
       nor reorder across another volatile access.  The values are exactly those
       the folded instantiation names as literals, which is what makes the two
       variants comparable rather than merely adjacent. */
    volatile float vf_small = 1.25f;
    volatile float vf_large = 2.5f;
    volatile float vf_equal = 1.25f;    /* numerically equal to vf_small */
    volatile float vf_neg = -1.25f;
    volatile float vf_neg_big = -2.5f;  /* more negative than vf_neg */
    volatile float vf_mix = 1.5f;
    volatile float vf_mix_lt = 1.25f;
    volatile double vd_small = 0.75;
    volatile double vd_mid = 1.5;
    volatile double vd_large = 3.75;
    volatile double vd_equal = 0.75;    /* numerically equal to vd_small */
    volatile double vd_neg = -0.75;
    volatile double vd_neg_big = -3.75; /* more negative than vd_neg */
    volatile double vd_mix = 1.5;
    volatile double vd_mix_gt = 3.75;

    /* Each volatile object is read EXACTLY ONCE, here, into a plain local.  Every
       comparison below reads only these locals, so no volatile object is read
       twice inside one expression and the printed results cannot depend on the
       order in which two reads happened to be scheduled.  A value that reached a
       local through a volatile load is still unknown to the optimizer, so the
       comparisons remain genuine run-time compares. */
    float rf_small = vf_small;
    float rf_large = vf_large;
    float rf_equal = vf_equal;
    float rf_neg = vf_neg;
    float rf_neg_big = vf_neg_big;
    float rf_mix = vf_mix;
    float rf_mix_lt = vf_mix_lt;
    double rd_small = vd_small;
    double rd_mid = vd_mid;
    double rd_large = vd_large;
    double rd_equal = vd_equal;
    double rd_neg = vd_neg;
    double rd_neg_big = vd_neg_big;
    double rd_mix = vd_mix;
    double rd_mix_gt = vd_mix_gt;

    /* Selected values, assigned by an if statement further down so that the
       backend's branch lowering is exercised alongside the select lowering.  Both
       arms of each if assign, so neither is ever read uninitialized. */
    double if_select;
    double rt_if_select;

    /* ---------------------------------------------------------------- folded --
       Every operand is a floating constant, so each initializer is a constant
       expression the folder evaluates outright at translation time. */

    /* The four relational operators, each in a direction that is true and a
       direction that is false, together with the equal-operand boundary where the
       non-strict predicate is true while the strict one is false.  That boundary
       is the highest-yield assertion in the file: it is exactly where an inverted
       predicate or a wrong condition code shows up while every other comparison
       still looks correct. */
    int f_lt = FC_LT(1.25f, 2.5f);            /* 1 */
    int f_lt_false = FC_LT(2.5f, 1.25f);      /* 0 */
    int f_le_equal = FC_LE(1.25f, 1.25f);     /* 1 - non-strict, operands equal */
    int f_lt_equal = FC_LT(1.25f, 1.25f);     /* 0 - strict, operands equal */
    int f_gt = FC_GT(2.5f, 1.25f);            /* 1 */
    int f_ge_equal = FC_GE(1.25f, 1.25f);     /* 1 - non-strict, operands equal */
    int f_le_false = FC_LE(2.5f, 1.25f);      /* 0 */
    int f_gt_false = FC_GT(1.25f, 2.5f);      /* 0 */
    int f_ge_false = FC_GE(1.25f, 2.5f);      /* 0 */
    int f_gt_equal = FC_GT(1.25f, 1.25f);     /* 0 - strict, operands equal */

    /* Equality and inequality on float.  Both operands of every != are separately
       named values, so no NaN self-inequality identity is assumed anywhere. */
    int f_eq = FC_EQ(1.25f, 1.25f);           /* 1 */
    int f_eq_false = FC_EQ(1.25f, 2.5f);      /* 0 */
    int f_ne = FC_NE(1.25f, 2.5f);            /* 1 */
    int f_ne_false = FC_NE(1.25f, 1.25f);     /* 0 */

    /* Ordering across sign, and ordering among negatives: a more negative value
       compares less, which is the property a sign-handling defect would break. */
    int f_neg_lt_pos = FC_LT(-1.25f, 1.25f);  /* 1 */
    int f_pos_gt_neg = FC_GT(1.25f, -1.25f);  /* 1 */
    int f_neg_lt_neg = FC_LT(-2.5f, -1.25f);  /* 1 */
    int f_neg_ge_neg = FC_GE(-1.25f, -2.5f);  /* 1 */

    /* The same surface in double, so both widths' compare instructions are
       exercised rather than only the narrower one, plus a transitivity chain. */
    int d_lt = FC_LT(0.75, 3.75);                      /* 1 */
    int d_le_equal = FC_LE(0.75, 0.75);                /* 1 */
    int d_gt_neg = FC_GT(0.75, -0.75);                 /* 1 */
    int d_ordering_chain = FC_CHAIN3(0.75, 1.5, 3.75); /* 1 */
    int d_eq = FC_EQ(0.75, 0.75);                      /* 1 */
    int d_ne = FC_NE(0.75, 3.75);                      /* 1 */
    int d_lt_false = FC_LT(3.75, 0.75);                /* 0 */
    int d_ge_false = FC_GE(0.75, 3.75);                /* 0 */
    int d_lt_equal = FC_LT(0.75, 0.75);                /* 0 */
    int d_neg_lt_neg = FC_LT(-3.75, -0.75);            /* 1 */

    /* Mixed float and double, with the float widened by an EXPLICIT cast rather
       than by the implicit usual arithmetic conversion, so -Wconversion has
       nothing to report.  The widening is exact, both values being dyadic. */
    int mixed_fd_eq = FC_EQ(FC_WIDEN(1.5f), 1.5);    /* 1 */
    int mixed_fd_lt = FC_LT(FC_WIDEN(1.25f), 3.75);  /* 1 */

    /* Comparison results driving control flow and boolean combination. */
    double cond_select = FC_SELECT(1.25f, 2.5f, 2.5, 1.25);  /* 2.5  */
    int and_chain = FC_AND2(0.75, 1.5, 3.75);                /* 1    */
    int or_chain = FC_OR2(3.75, 0.75, 0.75, 3.75);           /* 1    */
    int not_result = FC_NOT_LT(0.75, 3.75);                  /* 0    */

    /* --------------------------------------------------------------- runtime --
       The same macros over the plain locals loaded above.  Each value must be
       computed from a load performed at run time, so these lines exercise the
       backends' floating compare instructions and condition codes rather than
       the constant folder. */
    int rt_f_lt = FC_LT(rf_small, rf_large);
    int rt_f_lt_equal = FC_LT(rf_small, rf_equal);
    int rt_f_le_equal = FC_LE(rf_small, rf_equal);
    int rt_f_le_false = FC_LE(rf_large, rf_small);
    int rt_f_gt = FC_GT(rf_large, rf_small);
    int rt_f_ge_equal = FC_GE(rf_small, rf_equal);
    int rt_f_eq = FC_EQ(rf_small, rf_equal);
    int rt_f_ne_false = FC_NE(rf_small, rf_equal);
    int rt_f_neg_lt_neg = FC_LT(rf_neg_big, rf_neg);
    int rt_d_lt = FC_LT(rd_small, rd_large);
    int rt_d_eq = FC_EQ(rd_small, rd_equal);
    int rt_d_ge_false = FC_GE(rd_small, rd_large);
    int rt_d_neg_lt_neg = FC_LT(rd_neg_big, rd_neg);
    int rt_d_ordering_chain = FC_CHAIN3(rd_small, rd_mid, rd_large);
    int rt_mixed_fd_eq = FC_EQ(FC_WIDEN(rf_mix), rd_mix);
    int rt_mixed_fd_lt = FC_LT(FC_WIDEN(rf_mix_lt), rd_mix_gt);
    double rt_cond_select = FC_SELECT(rf_small, rf_large, 2.5, 1.25);
    int rt_and_chain = FC_AND2(rd_small, rd_mid, rd_large);
    int rt_or_chain = FC_OR2(rd_large, rd_small, rd_small, rd_large);
    int rt_not_result = FC_NOT_LT(rd_small, rd_large);

    /* An if statement rather than a conditional operator, so the branch lowering
       is exercised too.  The two arms carry different values, so the printed
       result proves which arm actually ran instead of merely being plausible. */
    if (FC_GT(2.5f, 1.25f)) {
        if_select = 1.25;
    } else {
        if_select = 2.5;
    }
    if (FC_GT(rf_large, rf_small)) {
        rt_if_select = 1.25;
    } else {
        rt_if_select = 2.5;
    }

    /* Every folded result and its runtime twin must agree.  Each pair is printed
       individually above and below as well, so this conjunction is a summary on
       top of the per-property lines rather than a substitute for them.  The two
       selected values are compared with == because both are exact dyadic
       rationals, so the equality is a hard property here. */
    int all_folded_matches_runtime =
        (f_lt == rt_f_lt) && (f_lt_equal == rt_f_lt_equal)
        && (f_le_equal == rt_f_le_equal) && (f_le_false == rt_f_le_false)
        && (f_gt == rt_f_gt) && (f_ge_equal == rt_f_ge_equal)
        && (f_eq == rt_f_eq) && (f_ne_false == rt_f_ne_false)
        && (f_neg_lt_neg == rt_f_neg_lt_neg) && (d_lt == rt_d_lt)
        && (d_eq == rt_d_eq) && (d_ge_false == rt_d_ge_false)
        && (d_neg_lt_neg == rt_d_neg_lt_neg)
        && (d_ordering_chain == rt_d_ordering_chain)
        && (mixed_fd_eq == rt_mixed_fd_eq) && (mixed_fd_lt == rt_mixed_fd_lt)
        && (cond_select == rt_cond_select) && (if_select == rt_if_select)
        && (and_chain == rt_and_chain) && (or_chain == rt_or_chain)
        && (not_result == rt_not_result);

    /* ----------------------------------------------------------------- output */

    /* The four relational operators on float, true and false directions, with the
       equal-operand boundary between them. */
    printf("f_lt=%d f_lt_false=%d f_le_equal=%d f_lt_equal=%d f_gt=%d"
           " f_ge_equal=%d\n",
           f_lt, f_lt_false, f_le_equal, f_lt_equal, f_gt, f_ge_equal);

    /* Equality and inequality on float, each in both directions. */
    printf("f_eq=%d f_eq_false=%d f_ne=%d f_ne_false=%d\n",
           f_eq, f_eq_false, f_ne, f_ne_false);

    /* Ordering and transitivity on double. */
    printf("d_lt=%d d_le_equal=%d d_gt_neg=%d d_ordering_chain=%d\n",
           d_lt, d_le_equal, d_gt_neg, d_ordering_chain);

    /* Mixed-width comparison through an explicit widening cast. */
    printf("mixed_fd_eq=%d mixed_fd_lt=%d\n", mixed_fd_eq, mixed_fd_lt);

    /* Select lowering, branch lowering, both short-circuit forms, and the
       boolean conversion of a comparison result. */
    printf("cond_select=%.6f if_select=%.6f and_chain=%d or_chain=%d"
           " not_result=%d\n",
           cond_select, if_select, and_chain, or_chain, not_result);

    /* The runtime twins of the properties above. */
    printf("rt_f_lt=%d rt_f_le_equal=%d rt_d_lt=%d rt_d_ordering_chain=%d"
           " rt_mixed_fd_eq=%d\n",
           rt_f_lt, rt_f_le_equal, rt_d_lt, rt_d_ordering_chain,
           rt_mixed_fd_eq);

    /* Folded against runtime, one field per pair, so a variant-specific defect
       shows up here as a 0 in a known column. */
    printf("folded_matches_runtime=%d %d %d %d %d\n",
           f_lt == rt_f_lt, f_le_equal == rt_f_le_equal, d_lt == rt_d_lt,
           d_ordering_chain == rt_d_ordering_chain,
           mixed_fd_eq == rt_mixed_fd_eq);

    /* The false directions of <=, > and >=, and the strict > at the equal-operand
       boundary: the operators the first line asserts only in their true
       direction are asserted here in their false one, so all six are covered
       twice over. */
    printf("f_le_false=%d f_gt_false=%d f_ge_false=%d f_gt_equal=%d\n",
           f_le_false, f_gt_false, f_ge_false, f_gt_equal);

    /* Ordering across sign and among negatives. */
    printf("f_neg_lt_pos=%d f_pos_gt_neg=%d f_neg_lt_neg=%d f_neg_ge_neg=%d\n",
           f_neg_lt_pos, f_pos_gt_neg, f_neg_lt_neg, f_neg_ge_neg);

    /* The remaining double properties, including its own equal-operand
       boundary. */
    printf("d_eq=%d d_ne=%d d_lt_false=%d d_ge_false=%d d_lt_equal=%d"
           " d_neg_lt_neg=%d\n",
           d_eq, d_ne, d_lt_false, d_ge_false, d_lt_equal, d_neg_lt_neg);

    /* The runtime float surface in full, which is what actually reaches the
       backends' compare instructions at -O1 and -O2. */
    printf("rt_f_lt_equal=%d rt_f_gt=%d rt_f_ge_equal=%d rt_f_eq=%d"
           " rt_f_ne_false=%d rt_f_le_false=%d\n",
           rt_f_lt_equal, rt_f_gt, rt_f_ge_equal, rt_f_eq, rt_f_ne_false,
           rt_f_le_false);

    /* The runtime sign, double and mixed-width properties. */
    printf("rt_f_neg_lt_neg=%d rt_d_eq=%d rt_d_ge_false=%d rt_d_neg_lt_neg=%d"
           " rt_mixed_fd_lt=%d\n",
           rt_f_neg_lt_neg, rt_d_eq, rt_d_ge_false, rt_d_neg_lt_neg,
           rt_mixed_fd_lt);

    /* The runtime select, branch, short-circuit and negation results. */
    printf("rt_cond_select=%.6f rt_if_select=%.6f rt_and_chain=%d"
           " rt_or_chain=%d rt_not_result=%d\n",
           rt_cond_select, rt_if_select, rt_and_chain, rt_or_chain,
           rt_not_result);

    /* The aggregate, on top of the per-property lines rather than instead of
       them. */
    printf("all_folded_matches_runtime=%d\n", all_folded_matches_runtime);

    return 0;
}

