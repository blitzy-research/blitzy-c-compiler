/* Area 13, program 004 -- `long double`, exercised on all four targets at all three
   optimization levels, with cross-backend VALUE equality alone excluded.

   WHY THIS PROGRAM EXISTS.  Constraint C3 of the suite's brief forbids dropping a language
   feature because testing it is difficult, and requires that where a feature genuinely cannot
   be compared, the exclusion be stated explicitly and explained rather than made silently.
   `long double` is the only type in this corpus whose representation is not shared by the four
   supported backends, so a plan driven by difficulty would have dropped it.  It is not dropped.
   It is compiled and run on every target at every optimization level, and the single comparison
   that is genuinely meaningless for it -- cross-backend value equality -- is the only thing
   switched off.  This program and `08_gcc_extensions/004_case_ranges.c` are the two the plan
   names as existing because of C3.

   THE MEASURED DIFFERENCE.  `sizeof(long double)` was measured in this environment as:

       x86-64     16 bytes    x87 80-bit extended, padded
       i686       12 bytes    x87 80-bit extended, padded
       AArch64    16 bytes    IEEE binary128
       RISC-V 64  16 bytes    IEEE binary128

   Three sizes and two radically different formats: a 64-bit significand on the two x86 targets
   against a 113-bit significand on the other two.  Two backends printing different digits for
   the same `long double` computation are therefore BOTH correct for their own target, so a
   cross-backend difference here is not evidence of a defect in either.  That is exactly the
   implementation-defined carve-out the brief reserves.  The repository documents the principle
   itself: `docs/technical-specifications.md` line 511 specifies the type representation as
   "target-parametric sizes" covering "floating types (float, double, long double)", and the
   target table at lines 457-462 records the same parametricity for pointer width, `long` width
   and ELF class.  The byte sizes above are the corroborating measurement.

   HOW THE EXCLUSION IS RECORDED -- AND WHY THERE IS NO MARKER.  The sibling record
   `004_long_double_target_restricted.expected` sets `oracle_b = disabled` and carries the
   measurement above as its `impl_defined_notes` reason; the record format refuses a narrowing
   that has no recorded reason, so the exclusion cannot be silent.  The not-attempted cells are
   still counted and still listed, reported XFAIL with that reason printed in full -- the second
   of the two XFAIL forms in `EXPECTED_DIVERGENCES.md` section 3.3.

   No expected-divergence marker is attached, and that is deliberate rather than an oversight.
   `EXPECTED_DIVERGENCES.md` section 4.2 analyses this exact program and records that no marker
   is provisioned for it, for two independent reasons given in section 3.3: a marker whose scope
   named only an oracle the record has switched off would be dormant -- nothing could ever
   consult it, so it could reach neither XFAIL nor XPASS -- and the record format REJECTS that
   state at parse time, so such a record would not even load; and an identifier written into the
   register without a live marker behind it fails the reverse direction of the register audit and
   takes the whole suite down with it.  A maintainer reading this file should not "complete" it by
   minting a marker: the recorded exclusion above already reaches the same reported outcome
   through the mechanism the harness actually implements.

   WHAT REMAINS FULLY IN FORCE.  Oracle (a) compares every target's output byte-for-byte against
   that SAME target's reference compiler, which is the comparison that can actually detect a
   defect here, because both sides then use one representation.  Oracle (c) asserts every cell
   against the golden record, so a regression that moved both compilers together is still caught.
   A divergence under either is a FINDING -- captured with its reproducer and its exact
   reproduction commands, never patched, since nothing about representation excuses a same-target
   disagreement.

   THE DESIGN THAT MAKES ONE GOLDEN RECORD VALID ON FOUR TARGETS.  Oracle (c) holds a single
   `expected_stdout` shared by all twelve cells, so this program prints only values that are
   identical on every target.  That is achieved by restricting the printable surface rather than
   by restricting the target list: every literal, every intermediate and every result below is a
   dyadic rational exactly representable in `double`, hence exact in a 53-bit, a 64-bit and a
   113-bit significand alike.  No rounding occurs in any of the three formats, so the printed
   digits cannot differ.  Every value also terminates well inside the six fractional digits that
   `%.6Lf` prints.

   The discipline is load-bearing, not decorative.  Measured here: the non-dyadic quotient
   1.0L divided by 3.0L, printed at 25 fractional digits, gives 0.3333333333333333333423684 on
   the two x87 targets and 0.3333333333333333333333333 on the two binary128 targets.  One value
   chosen carelessly would produce a real per-target difference and break the shared record.  So
   the precondition is not merely asserted in this comment: the `exact_in_double` line below
   prints, per target and per value, whether the value survives a round trip through `double`
   unchanged.  Every field is 1 here.  A mis-chosen value would print 0 and announce itself in
   the very first run instead of silently invalidating the record.

   WHAT IS DELIBERATELY NOT PRINTED.  `sizeof(long double)` is never printed as an absolute
   value: it is 16, 12, 16 and 16, so printing it would break the shared record on i686
   immediately.  Exactly one target-invariant relation about the type's width is printed instead,
   and it is 1 everywhere.  No `long double` characteristic appears either -- no significand
   digit count, no epsilon, no maximum, no exponent range -- because every one of them differs by
   target.  For the same reason no `float.h` macro is referenced: that header is bundled, but
   every long-double characteristic macro it defines is target-dependent, and exercising the
   bundled set is the job of `12_preprocessor/003_bundled_header_inclusion.c`, not of this
   program.

   No non-finite value, no negative zero and no subnormal appears: the boundaries of the exponent
   range differ enormously between x87 and binary128, and identity properties of those values are
   not what this program is testing.  No `long double` object is inspected byte-wise, copied into
   an object of another type, or reinterpreted through overlapping storage or a pointer cast -- a
   12-byte or 16-byte object of this type carries padding beyond the significant x87 bytes, and
   depending on padding is both undefined behaviour and target-dependent.  No address, no pointer
   value and no plain `char` value is printed either, and no integer of a target-varying width:
   the only integer conversions printed are to `int` and to `long long`, both of which are the
   same width on all four targets, where `long` and `size_t` are not.

   WHY NO HEADER IS INCLUDED.  The compiler under test bundles nine freestanding headers
   (`docs/technical-specifications.md` line 19 and lines 202-214) and ships no `stdio.h`, so an
   inclusion would fail on that side while succeeding on the reference side -- a spurious
   divergence caused by the test rather than by a compiler.  `printf` is therefore declared by
   hand, which published output-comparison experience also identifies as the fix for the most
   common portability problem in this class of suite.

   THE TWO-VARIANT RULE.  Every arithmetic property below is computed twice: once from
   compile-time constants, which is what the constant folder sees, and once from
   `volatile`-qualified operands, which the optimizer may not fold and which therefore force the
   backend to emit genuine `long double` instructions -- the x87 stack on the two x86 targets, and
   the binary128 path, largely `libgcc` soft-float helpers, on the other two.  Without the second
   variant the folder's answer would stand in for the backend's at `-O1` and `-O2` and a
   code-generation defect would escape entirely.  The `folded_matches_runtime` line asserts the
   two agree.

   UNDEFINED-BEHAVIOUR FREEDOM.  All arithmetic is on finite, exactly representable values; no
   division has a zero divisor; the conversions to an integer type truncate 6.25, -6.25 and 12.5,
   so each result sits far inside its destination range on every target, which is what keeps a
   floating-to-integer conversion defined; nothing is uninitialized; no object is
   modified twice between sequence points; and each `volatile` object is read exactly once, in a
   statement of its own, because the relative order of side effects within one argument list is
   unspecified.  Every input is a literal in this file: nothing is opened, nothing is read from
   the environment or the command line.  The program prints thirteen lines and exits 0. */
int printf(const char *, ...);

/* The two structural preconditions the printable surface rests on, pinned at compile time
   rather than assumed.  A `long double` narrower than a `double` would make the round-trip
   guards below vacuous, and "exactly representable in `double`" is a statement about IEEE
   binary64, whose width is 8 bytes on all four targets as measured.  Both are integer constant
   expressions, which is what `_Static_assert` requires: a floating comparison here would be
   rejected by the strict warning gate under C11 6.6p6, so exactness itself is verified at run
   time by `exact_in_double` instead. */
_Static_assert(sizeof(long double) >= sizeof(double),
               "long double must be at least as wide as double for this program to mean anything");
_Static_assert(sizeof(double) == 8,
               "double must be IEEE binary64 for double-representability to imply exactness");

/* Folded operands.  Macros rather than objects, so that every expression built from them below
   really is a constant expression for the folder to evaluate.  Each is a dyadic rational: 5/2,
   5/4, 5/2, 2, 4, 3/4, 1/4 and 25/4. */
#define LD_A    2.5L
#define LD_B    1.25L
#define LD_C    2.5L
#define LD_TWO  2.0L
#define LD_FOUR 4.0L
#define LD_P75  0.75L
#define LD_P25  0.25L
#define LD_E    6.25L

/* Accumulation terms: consecutive powers of two from 1/8 to 8, summing to 15.875 exactly.  The
   fixed length and the fixed iteration order below are what keep the sum deterministic. */
#define ACCUM_N 7
static const long double accum_terms[ACCUM_N] = {
    0.125L, 0.25L, 0.5L, 1.0L, 2.0L, 4.0L, 8.0L
};

/* Runtime operands.  `volatile` is what stops the optimizer substituting the folder's answer for
   the backend's: each read is an observable side effect the compiler must perform.  The values
   mirror the macros above exactly, so the two variants must agree. */
static volatile long double rt_a     = 2.5L;
static volatile long double rt_b     = 1.25L;
static volatile long double rt_c     = 2.5L;
static volatile long double rt_two   = 2.0L;
static volatile long double rt_four  = 4.0L;
static volatile long double rt_p75   = 0.75L;
static volatile long double rt_p25   = 0.25L;
static volatile long double rt_e     = 6.25L;
static volatile int         rt_i42   = 42;
static volatile long double rt_terms[ACCUM_N] = {
    0.125L, 0.25L, 0.5L, 1.0L, 2.0L, 4.0L, 8.0L
};

int main(void)
{
    int idx;

    /* Folded arithmetic: every operand a constant, so the folder evaluates all of it. */
    long double ld_add   = LD_A + LD_B;                             /* 3.75   */
    long double ld_sub   = LD_A - LD_B;                             /* 1.25   */
    long double ld_mul   = LD_A * LD_B;                             /* 3.125  */
    long double ld_div   = LD_A / LD_TWO;                           /* 1.25   */
    long double ld_neg   = -LD_A;                                   /* -2.5   */
    long double ld_chain = LD_A + LD_B * LD_FOUR - LD_P75 / LD_P25; /* 4.5    */
    long double ld_accum = 0.0L;                                    /* 15.875 */

    /* Folded conversions.  Every one carries an explicit cast: the strict warning gate keeps
       -Wconversion and -Wsign-conversion active for this area, so an implicit narrowing from
       `long double` would fail the gate.  `1.5` is the one deliberately unsuffixed literal in
       the file -- it is a `double`, and widening it is the property under test. */
    double      ld2d     = (double)(LD_A * LD_B);                   /* 3.125     */
    float       ld2f     = (float)LD_A;                             /* 2.5       */
    int         ld2i     = (int)LD_E;                               /* 6         */
    int         ld2i_neg = (int)(-LD_E);                            /* -6        */
    long long   ld2ll    = (long long)(LD_E * LD_TWO);              /* 12        */
    long double i2ld     = (long double)42;                         /* 42.0      */
    long double ll2ld    = (long double)1234567LL;                  /* 1234567.0 */
    long double d2ld     = (long double)1.5;                        /* 1.5       */

    /* Folded comparisons on finite, exactly representable values, including the equal-operands
       boundary where `<=` holds and `<` does not.  All six relational and equality operators
       appear. */
    int ld_lt       = (LD_B < LD_A);                                /* 1 */
    int ld_gt       = (LD_A > LD_B);                                /* 1 */
    int ld_le_equal = (LD_A <= LD_C);                               /* 1 */
    int ld_ge_equal = (LD_A >= LD_C);                               /* 1 */
    int ld_lt_equal = (LD_A < LD_C);                                /* 0 */
    int ld_eq       = (LD_A == LD_C);                               /* 1 */
    int ld_ne_equal = (LD_A != LD_C);                               /* 0 */
    int ld_ne_diff  = (LD_A != LD_B);                               /* 1 */

    /* Snapshots of the volatile operands.  Each is assigned in a statement of its own below, so
       every volatile access is separated from the next by a sequence point and no printed value
       can depend on unspecified evaluation order. */
    long double sn_a;
    long double sn_b;
    long double sn_c;
    long double sn_two;
    long double sn_four;
    long double sn_p75;
    long double sn_p25;
    long double sn_e;
    long double sn_term;
    int         sn_i42;

    /* Runtime results, recomputed from those snapshots. */
    long double rt_add;
    long double rt_sub;
    long double rt_mul;
    long double rt_div;
    long double rt_neg;
    long double rt_chain;
    long double rt_accum   = 0.0L;
    double      rt_ld2d;
    float       rt_ld2f;
    int         rt_ld2i;
    long long   rt_ld2ll;
    long double rt_i2ld;
    int         rt_lt;
    int         rt_le_equal;
    int         rt_lt_equal;
    int         rt_eq;

    for (idx = 0; idx < ACCUM_N; idx++) {
        ld_accum = ld_accum + accum_terms[idx];
    }

    sn_a    = rt_a;
    sn_b    = rt_b;
    sn_c    = rt_c;
    sn_two  = rt_two;
    sn_four = rt_four;
    sn_p75  = rt_p75;
    sn_p25  = rt_p25;
    sn_e    = rt_e;
    sn_i42  = rt_i42;

    rt_add   = sn_a + sn_b;
    rt_sub   = sn_a - sn_b;
    rt_mul   = sn_a * sn_b;
    rt_div   = sn_a / sn_two;
    rt_neg   = -sn_a;
    rt_chain = sn_a + sn_b * sn_four - sn_p75 / sn_p25;

    /* One volatile element read per statement, in fixed index order. */
    for (idx = 0; idx < ACCUM_N; idx++) {
        sn_term  = rt_terms[idx];
        rt_accum = rt_accum + sn_term;
    }

    rt_ld2d  = (double)rt_mul;
    rt_ld2f  = (float)sn_a;
    rt_ld2i  = (int)sn_e;
    rt_ld2ll = (long long)(sn_e * sn_two);
    rt_i2ld  = (long double)sn_i42;

    rt_lt       = (sn_b < sn_a);
    rt_le_equal = (sn_a <= sn_c);
    rt_lt_equal = (sn_a < sn_c);
    rt_eq       = (sn_a == sn_c);

    printf("ld_add=%.6Lf ld_sub=%.6Lf ld_mul=%.6Lf ld_div=%.6Lf\n",
           ld_add, ld_sub, ld_mul, ld_div);
    printf("ld_neg=%.6Lf ld_chain=%.6Lf ld_accum=%.6Lf\n",
           ld_neg, ld_chain, ld_accum);
    printf("ld2d=%.6f ld2f=%.6f d2ld=%.6Lf i2ld=%.6Lf\n",
           ld2d, (double)ld2f, d2ld, i2ld);
    printf("ld2i=%d ld2i_neg=%d ld2ll=%lld ll2ld=%.6Lf\n",
           ld2i, ld2i_neg, ld2ll, ll2ld);
    printf("ld_lt=%d ld_gt=%d ld_le_equal=%d ld_ge_equal=%d\n",
           ld_lt, ld_gt, ld_le_equal, ld_ge_equal);
    printf("ld_lt_equal=%d ld_eq=%d ld_ne_equal=%d ld_ne_diff=%d\n",
           ld_lt_equal, ld_eq, ld_ne_equal, ld_ne_diff);
    printf("rt_add=%.6Lf rt_sub=%.6Lf rt_mul=%.6Lf rt_div=%.6Lf\n",
           rt_add, rt_sub, rt_mul, rt_div);
    printf("rt_neg=%.6Lf rt_chain=%.6Lf rt_accum=%.6Lf\n",
           rt_neg, rt_chain, rt_accum);
    printf("rt_ld2d=%.6f rt_ld2f=%.6f rt_ld2i=%d rt_ld2ll=%lld rt_i2ld=%.6Lf\n",
           rt_ld2d, (double)rt_ld2f, rt_ld2i, rt_ld2ll, rt_i2ld);
    printf("rt_lt=%d rt_le_equal=%d rt_lt_equal=%d rt_eq=%d\n",
           rt_lt, rt_le_equal, rt_lt_equal, rt_eq);

    /* The two-variant rule made observable: the folder's answer and the backend's must agree,
       field by field, in the order add, sub, mul, div, neg, chain, accum. */
    printf("folded_matches_runtime=%d %d %d %d %d %d %d\n",
           (int)(rt_add == ld_add), (int)(rt_sub == ld_sub),
           (int)(rt_mul == ld_mul), (int)(rt_div == ld_div),
           (int)(rt_neg == ld_neg), (int)(rt_chain == ld_chain),
           (int)(rt_accum == ld_accum));

    /* The precondition that makes one golden record valid on four targets, checked on the target
       rather than assumed: each result survives a round trip through `double` unchanged, so it is
       exact in a 53-bit significand and therefore exact in the 64-bit and 113-bit significands
       too.  Same field order as the line above.  Every field is 1; a 0 would mean a value in this
       program is not double-representable and the shared record cannot hold. */
    printf("exact_in_double=%d %d %d %d %d %d %d\n",
           (int)((long double)(double)ld_add == ld_add),
           (int)((long double)(double)ld_sub == ld_sub),
           (int)((long double)(double)ld_mul == ld_mul),
           (int)((long double)(double)ld_div == ld_div),
           (int)((long double)(double)ld_neg == ld_neg),
           (int)((long double)(double)ld_chain == ld_chain),
           (int)((long double)(double)ld_accum == ld_accum));

    /* The only thing this program says about the type's width, and the only form in which it can
       be said: a relation that is true on all four targets.  The absolute size is 16, 12, 16 and
       16, so printing it would break the shared record on i686. */
    printf("ld_at_least_double=%d\n", (int)(sizeof(long double) >= sizeof(double)));
    return 0;
}
