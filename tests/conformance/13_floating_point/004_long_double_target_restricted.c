/* Area 13, program 004 -- `long double`, exercised on all four targets at all three
   optimization levels with cross-backend VALUE equality alone excluded.  That is the design,
   and the sibling record `004_long_double_target_restricted.expected` is committed beside this
   source and implements it exactly: `targets` names all four, `opt_levels` names all three,
   `oracle_a` and `oracle_c` are enabled, `oracle_b` alone is disabled with its measured reason
   recorded, and a single golden stdout - byte-identical across all twelve cells - is what
   oracle (c) holds every cell to.  The record also carries the active expected-divergence
   marker XD-TYPE-LONGDOUBLE-001, class stdout_mismatch, scoped to oracle_b across all targets
   and all optimization levels, which is what keeps the one disabled arm auditable in
   EXPECTED_DIVERGENCES.md rather than silent.

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

   HOW THE EXCLUSION IS RECORDED: A DISABLED ORACLE AND A MARKER, WHICH ARE TWO HALVES OF ONE
   STATEMENT.  The sibling record `004_long_double_target_restricted.expected` sets
   `oracle_b = disabled`, carries the measurement above as its `impl_defined_notes` reason, and
   carries the expected-divergence marker `XD-TYPE-LONGDOUBLE-001` whose scope names oracle (b) on
   all four targets at all three optimization levels.  All nine not-attempted comparisons -- each of
   the three non-baseline targets against the x86-64 baseline at each level -- are still counted and
   still listed, reported XFAIL citing that marker and its documented basis alongside the reason
   printed in full.

   Why both halves are required, and why the record format now refuses either alone.  The toggle
   says WHICH comparison is not made.  The marker says on whose authority, under which identifier
   and over which cells -- and it is the identifier that puts the exclusion into
   `EXPECTED_DIVERGENCES.md`, where the register's bidirectional audit resolves every marker against
   an entry and every entry against a marker.  A toggle and a reason WITHOUT a marker would not be
   equivalent, and the difference is precise rather than cosmetic: the run would still report XFAIL,
   so the verdict would claim the authority of a documented expected divergence while the audit could
   not see it, no identifier would exist for a report row to cite, and no basis would have been
   resolved against any committed document.  That is a silent exclusion in the clothes of a
   documented one, and it is why a record that disables an oracle without a marker scoping it is
   rejected at parse time.

   What the marker does NOT claim.  Not that the four backends disagree today -- they do not; every
   printed value was chosen to be exact in all three representations, and all twelve cells were
   measured to produce identical bytes.  The claim is that a cross-backend VALUE comparison over this
   type could not be read as evidence about the compiler under test even if they did disagree.  That
   is why the marker can never reach XPASS -- nothing is compared on the arm it scopes, so no
   divergence can disappear from it -- and why it stays correct if a later maintainer adds a value
   that does diverge.

   A maintainer must therefore keep three things in step in a single edit: this record's `oracle_b`
   toggle, its `impl_defined_notes` reason, and the marker block with its register entry.  Re-enabling
   oracle (b) means deleting all three together.

   Nothing about the exclusion touches oracle (a), oracle (c) or the warning gate.  Only the
   cross-backend VALUE comparison is switched off, so a same-target disagreement against the
   reference compiler or a golden-record disagreement is still a FINDING, and the strict
   undefined-behaviour gate still runs at full strength: this program's record declares no
   `ub_audit_flags` deviation.

   WHAT THE RECORD LEAVES FULLY IN FORCE.  Oracle (a) compares every target's output
   byte-for-byte against that SAME target's reference compiler, which is the comparison that can
   actually detect a defect here, because both sides then use one representation.  Oracle (c)
   asserts every cell against the golden record, so a regression that moved both compilers
   together is still caught.  A divergence under either is a FINDING -- captured with its
   reproducer and its exact reproduction commands, never patched, since nothing about
   representation excuses a same-target disagreement.  Both arms are enabled in the record as
   committed; what defers their first verdict is the absent compiler under test, not the
   record.

   THE DESIGN THAT MAKES ONE GOLDEN RECORD VALID ON FOUR TARGETS.  Oracle (c) holds a single
   `expected_stdout` shared by all twelve cells, so this program prints only values that
   are identical on every target.  That is achieved by restricting the printable surface rather
   than by restricting the target list: every literal, every intermediate and every result below
   is a dyadic rational exactly representable in `double`, hence exact in a 53-bit, a 64-bit and
   a 113-bit significand alike.  No rounding occurs in any of the three formats, so the printed
   digits cannot differ.  Every value also terminates well inside the six fractional digits that
   `%.6Lf` prints.  That property is a fact about the program and holds whether or not a record
   exists to compare against.

   The discipline is load-bearing, not decorative, and the cost of ignoring it is reproducible
   rather than asserted.  Take the non-dyadic quotient 1.0L divided by 3.0L and print it at 25
   fractional digits.  Four commands, one per target, reproduce the measurement from a q.c of the
   same shape as this program -- a hand-declared printf, then a main holding
   `volatile long double a = 1.0L, b = 3.0L;` and `printf("%.25Lf\n", a / b);`:

       gcc                   -O1 -static q.c -o q && ./q
       i686-linux-gnu-gcc    -O1 -static q.c -o q && qemu-i386 ./q
       aarch64-linux-gnu-gcc -O1 -static q.c -o q && qemu-aarch64 ./q
       riscv64-linux-gnu-gcc -O1 -static q.c -o q && qemu-riscv64 ./q

   The first two print 0.3333333333333333333423684 and the last two print
   0.3333333333333333333333333: the x87 significand and the binary128 significand diverge at the
   twentieth fractional digit.  One value chosen carelessly would therefore produce a real
   per-target difference and break the shared record.  So the precondition is not left to a
   comment at all: the `exact_in_double` line below prints, per target and per value, whether the
   value survives a round trip through `double` unchanged.  Every field is 1 for the values this
   program uses.  A mis-chosen value would print 0 and announce itself in the very first run
   instead of silently invalidating the record.

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
   an object of another type, or reinterpreted through overlapping storage or a pointer cast.
   Reading an object's representation THROUGH A CHARACTER TYPE is permitted -- that is exactly what
   C11 6.5p7 allows and it is not the problem here.  The problem is what such a read would return.
   On the x87 targets the significant bytes occupy 10 of the object's measured 12 or 16, and the
   remaining bytes are not part of the value: C11 6.2.6.1p6 makes padding bytes take UNSPECIFIED
   values when a value is stored, and before any store an object with automatic storage duration
   holds an INDETERMINATE value (3.19.2).  Neither is something an implementation must reproduce,
   so no golden record could pin such a byte down and no divergence in one would say anything
   about a compiler.  That is the reason the program stays away from the representation entirely
   and observes only arithmetic results, comparisons and conversions -- and it is a different
   reason from undefined behaviour, which this is not.  No address, no pointer
   value and no plain `char` value is printed either, and no integer of a target-varying width:
   the only integer types this program names at all are `int` and `long long`, as a conversion
   destination and as a widening source, and both are the same width on all four targets, where
   `long` and `size_t` are not.  That is why the integer widening source is a `long long` and not
   a `long`: a `long` operand would be 4 bytes on i686 and 8 on the other three, so the same
   source literal would exercise a different conversion per target for no gain.

   WHY NO HEADER IS INCLUDED.  The compiler under test bundles nine freestanding headers
   (`docs/technical-specifications.md` line 19 and lines 202-214) and ships no `stdio.h`, so an
   inclusion would fail on that side while succeeding on the reference side -- a spurious
   divergence caused by the test rather than by a compiler.  `printf` is therefore declared by
   hand, which published output-comparison experience also identifies as the fix for the most
   common portability problem in this class of suite.

   THE TWO-VARIANT RULE, APPLIED TO THE WHOLE SURFACE.  Every property below is computed twice:
   once from compile-time constants, which is what the constant folder sees, and once from
   `volatile`-qualified operands, which the optimizer may not fold and which therefore force the
   backend to emit genuine `long double` instructions -- the x87 stack on the two x86 targets, and
   the binary128 path, largely `libgcc` soft-float helpers, on the other two.  Without the second
   variant the folder's answer would stand in for the backend's at `-O1` and `-O2` and a
   code-generation defect would escape entirely.

   "Every property" means all twenty-three, not the arithmetic alone: seven arithmetic results,
   eight conversions and eight comparisons each have a runtime twin computed from the snapshots,
   printed on its own mirrored line, and asserted pairwise on the three equivalence lines
   `folded_matches_runtime`, `folded_matches_runtime_conv` and `folded_matches_runtime_cmp` --
   one column per pair, with none omitted.  Conversion and comparison are exactly where this type
   is most likely to go wrong in a backend, because they are the operations that leave the
   representation: `long double` to `int` and to `long long` is x87 `fistp` against a binary128
   helper call, and the six relational and equality operators are `fcomi` against a
   soft-compare helper.  A conversion or comparison with no runtime twin would be asserted by the
   folder alone, and a defect in exactly those paths would print nothing different anywhere.

   UNDEFINED-BEHAVIOUR FREEDOM.  All arithmetic is on finite, exactly representable values; no
   division has a zero divisor; the conversions to an integer type truncate 6.25, -6.25 and 12.5,
   so each result sits far inside its destination range on every target, which is what keeps a
   floating-to-integer conversion defined; nothing is uninitialized; no object is
   modified twice between sequence points; and each `volatile` object is read exactly once, in a
   statement of its own, because the relative order of side effects within one argument list is
   unspecified.  The significand probe follows the same rule: each store into the `volatile
   double` is its own full expression and each reload is staged into a plain `int` before the
   printing call, so no call anywhere in the program receives a side-effecting argument.  Every
   input is a literal in this file: nothing is opened, nothing is read from the environment or the
   command line.  The program prints EIGHTEEN lines -- six folded, six runtime, three equivalence
   lines covering all twenty-three folded/runtime pairs one column each, the exactness
   precondition, its significand probe, and the one width relation -- and exits 0.  Each equivalence
   column appears exactly once: restating a column under a second label would add no coverage while
   making the stated line count wrong. */
int printf(const char *, ...);

/* The two structural preconditions the printable surface rests on, pinned at compile time
   rather than assumed -- and each stating only what it actually establishes.  A `long double`
   narrower than a `double` would make the round-trip guards below vacuous.  A `double` of some
   other width would not be the 8-byte format every one of the four target ABIs specifies.  Both
   are integer constant expressions, which is what `_Static_assert` requires.

   WHAT A WIDTH DOES AND DOES NOT PROVE.  `sizeof(double) == 8` establishes that the type
   occupies eight bytes and nothing more: it does not by itself establish the radix, the
   significand width, or that the format is IEEE binary64 at all, since eight bytes could in
   principle hold some other encoding.  The properties the round-trip guards below actually
   depend on -- a radix-2 significand of exactly 53 bits -- are therefore established WHERE THEY
   CAN BE, at run time, by the `double_significand` line in `main`: a floating comparison is not
   an integer constant expression and C11 6.6p6 keeps it out of a static assertion, so the
   translator cannot be asked the question and the generated program is asked instead.  The
   documented basis for the format itself is each target's own ABI -- the System V i386 and
   x86-64 psABIs, AAPCS64, and the RISC-V calling convention all specify IEEE binary64 for
   `double` -- and the run-time check is what confirms the ABI was honoured on the machine the
   cell ran on. */
_Static_assert(sizeof(long double) >= sizeof(double),
               "long double must be at least as wide as double for this program to mean anything");
_Static_assert(sizeof(double) == 8,
               "double must occupy the 8 bytes every one of the four target ABIs gives it; the "
               "radix-2 53-bit significand this program relies on is checked at run time, "
               "because a width alone does not establish a format");

/* Staging object for the significand probe.  `volatile` and of type `double` for one reason
   each, and both are essential.  `double`, because C11 6.3.1.8p2 removes extra range and
   precision on assignment, so storing a sum here yields exactly what binary64 holds; `volatile`,
   so the store and the reload genuinely happen instead of the value staying in a wider register.
   Without both, the x87 targets would evaluate the sums in 64-bit-significand registers, the
   half-way term would NOT be absorbed there, and the probe would report a different answer on
   two targets than on the other two -- a difference in the test, not in any compiler. */
static volatile double dbl_significand_probe;

/* The two increments that bracket a 53-bit binary significand, written as exact dyadic
   quotients so no decimal literal has to be parsed to the nearest representable value: 1/2^52
   is the smallest increment that changes 1.0 in that format, and 1/2^53 is exactly half of it
   and lies precisely half-way to the next value, so round-to-nearest-even returns 1.0 for it.
   A format with a wider significand would resolve the second increment, and one with a narrower
   significand or a non-binary radix would not resolve the first, so the pair together witnesses
   the format rather than merely sampling it. */
#define POW2_M52 (1.0 / 4503599627370496.0)
#define POW2_M53 (1.0 / 9007199254740992.0)

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
   mirror the macros above exactly, so the two variants must agree.  The last three are the
   sources of the three conversions INTO `long double`: `rt_i42` for the `int` source, `rt_ll` for
   the `long long` source and `rt_d1p5` for the `double` source, each holding exactly the literal
   the folded conversion converts, so those three widenings reach the backend as well as the
   folder.  `int`, `long long` and `double` are 4, 8 and 8 bytes on all four targets, so none of
   the three introduces a width the shared golden record could not hold. */
static volatile long double rt_a     = 2.5L;
static volatile long double rt_b     = 1.25L;
static volatile long double rt_c     = 2.5L;
static volatile long double rt_two   = 2.0L;
static volatile long double rt_four  = 4.0L;
static volatile long double rt_p75   = 0.75L;
static volatile long double rt_p25   = 0.25L;
static volatile long double rt_e     = 6.25L;
static volatile int         rt_i42   = 42;
static volatile long long   rt_ll    = 1234567LL;
static volatile double      rt_d1p5  = 1.5;
static volatile long double rt_terms[ACCUM_N] = {
    0.125L, 0.25L, 0.5L, 1.0L, 2.0L, 4.0L, 8.0L
};

int main(void)
{
    int idx;
    int resolves_2_52;
    int absorbs_2_53;

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
       `long double` would fail the gate.  `1.5` is deliberately unsuffixed here and in the
       matching `volatile double` operand above -- it is a `double`, and widening a `double` to a
       `long double` is the property those two lines test.  Every other floating literal in the
       file carries the `L` suffix. */
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
    long long   sn_ll;
    double      sn_d1p5;

    /* Runtime results, recomputed from those snapshots.  Every folded result above
       has exactly one twin here -- seven arithmetic, eight conversion and eight
       comparison -- because a folded result with no twin is asserted by the
       constant folder alone and the backend is never asked to compute it. */
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
    int         rt_ld2i_neg;
    long long   rt_ld2ll;
    long double rt_i2ld;
    long double rt_ll2ld;
    long double rt_d2ld;
    int         rt_lt;
    int         rt_gt;
    int         rt_le_equal;
    int         rt_ge_equal;
    int         rt_lt_equal;
    int         rt_eq;
    int         rt_ne_equal;
    int         rt_ne_diff;

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
    sn_ll   = rt_ll;
    sn_d1p5 = rt_d1p5;

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

    /* Runtime conversions, one per folded conversion above, in the same order and
       each with the same explicit cast.  These are the conversions the backend has
       to emit: x87 fistp and fld on the two x86 targets, and the binary128
       conversion helpers on the other two. */
    rt_ld2d     = (double)rt_mul;
    rt_ld2f     = (float)sn_a;
    rt_ld2i     = (int)sn_e;
    rt_ld2i_neg = (int)(-sn_e);
    rt_ld2ll    = (long long)(sn_e * sn_two);
    rt_i2ld     = (long double)sn_i42;
    rt_ll2ld    = (long double)sn_ll;
    rt_d2ld     = (long double)sn_d1p5;

    /* Runtime comparisons, one per folded comparison above, including both
       directions of the strict and non-strict predicates and both directions of
       the equality operators at the equal-operands boundary.  All six relational
       and equality operators reach the backend here rather than the folder. */
    rt_lt       = (sn_b < sn_a);
    rt_gt       = (sn_a > sn_b);
    rt_le_equal = (sn_a <= sn_c);
    rt_ge_equal = (sn_a >= sn_c);
    rt_lt_equal = (sn_a < sn_c);
    rt_eq       = (sn_a == sn_c);
    rt_ne_equal = (sn_a != sn_c);
    rt_ne_diff  = (sn_a != sn_b);

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
    printf("rt_ld2d=%.6f rt_ld2f=%.6f rt_d2ld=%.6Lf rt_i2ld=%.6Lf\n",
           rt_ld2d, (double)rt_ld2f, rt_d2ld, rt_i2ld);
    printf("rt_ld2i=%d rt_ld2i_neg=%d rt_ld2ll=%lld rt_ll2ld=%.6Lf\n",
           rt_ld2i, rt_ld2i_neg, rt_ld2ll, rt_ll2ld);
    printf("rt_lt=%d rt_gt=%d rt_le_equal=%d rt_ge_equal=%d\n",
           rt_lt, rt_gt, rt_le_equal, rt_ge_equal);
    printf("rt_lt_equal=%d rt_eq=%d rt_ne_equal=%d rt_ne_diff=%d\n",
           rt_lt_equal, rt_eq, rt_ne_equal, rt_ne_diff);

    /* The two-variant rule made observable across the WHOLE surface, one column per pair with
       none left out: seven arithmetic pairs in the order add, sub, mul, div, neg, chain, accum;
       then eight conversion pairs in the order ld2d, ld2f, ld2i, ld2i_neg, ld2ll, i2ld, ll2ld,
       d2ld; then eight comparison pairs in the order lt, gt, le_equal, ge_equal, lt_equal, eq,
       ne_equal, ne_diff.  A column that reads 0 names the one computation whose folded and
       runtime spellings disagree, which localizes a defect to the folder or to the backend
       rather than merely reporting that the two disagree somewhere. */
    printf("folded_matches_runtime=%d %d %d %d %d %d %d\n",
           (int)(rt_add == ld_add), (int)(rt_sub == ld_sub),
           (int)(rt_mul == ld_mul), (int)(rt_div == ld_div),
           (int)(rt_neg == ld_neg), (int)(rt_chain == ld_chain),
           (int)(rt_accum == ld_accum));
    printf("folded_matches_runtime_conv=%d %d %d %d %d %d %d %d\n",
           (int)(rt_ld2d == ld2d), (int)(rt_ld2f == ld2f),
           (int)(rt_ld2i == ld2i), (int)(rt_ld2i_neg == ld2i_neg),
           (int)(rt_ld2ll == ld2ll), (int)(rt_i2ld == i2ld),
           (int)(rt_ll2ld == ll2ld), (int)(rt_d2ld == d2ld));
    printf("folded_matches_runtime_cmp=%d %d %d %d %d %d %d %d\n",
           (int)(rt_lt == ld_lt), (int)(rt_gt == ld_gt),
           (int)(rt_le_equal == ld_le_equal), (int)(rt_ge_equal == ld_ge_equal),
           (int)(rt_lt_equal == ld_lt_equal), (int)(rt_eq == ld_eq),
           (int)(rt_ne_equal == ld_ne_equal), (int)(rt_ne_diff == ld_ne_diff));

    /* The precondition that makes one golden record valid on four targets, checked on the target
       rather than assumed: each result survives a round trip through `double` unchanged, so it is
       exact in a 53-bit significand and therefore exact in the 64-bit and 113-bit significands
       too.  Same seven-field order as the `folded_matches_runtime` line above -- add, sub, mul,
       div, neg, chain, accum -- because those are the computed values; the conversion results are
       42, 1234567 and 3/2, exact in `double` by inspection.  Every field is 1; a 0 would mean a
       value in this program is not double-representable and the shared record cannot hold. */
    printf("exact_in_double=%d %d %d %d %d %d %d\n",
           (int)((long double)(double)ld_add == ld_add),
           (int)((long double)(double)ld_sub == ld_sub),
           (int)((long double)(double)ld_mul == ld_mul),
           (int)((long double)(double)ld_div == ld_div),
           (int)((long double)(double)ld_neg == ld_neg),
           (int)((long double)(double)ld_chain == ld_chain),
           (int)((long double)(double)ld_accum == ld_accum));

    /* The format that `exact_in_double` above rests on, established rather than assumed.  Each
       volatile store is its own full expression and each volatile read is staged into a plain int
       before the call, so the line carries no side-effecting argument and nothing depends on the
       order in which one argument list is evaluated.  Both fields are 1 on every target: the
       first says the smallest increment a 53-bit binary significand can resolve at 1.0 IS
       resolved, the second says the increment exactly half-way below it is absorbed, and together
       they pin the significand at 53 radix-2 bits -- which is what makes "survives a round trip
       through double" mean "exact in 53 bits" in the line above. */
    dbl_significand_probe = 1.0 + POW2_M52;
    resolves_2_52 = (int)(dbl_significand_probe != 1.0);
    dbl_significand_probe = 1.0 + POW2_M53;
    absorbs_2_53 = (int)(dbl_significand_probe == 1.0);
    printf("double_significand=%d %d\n", resolves_2_52, absorbs_2_53);

    /* The only thing this program says about the type's width, and the only form in which it can
       be said: a relation that is true on all four targets.  The absolute size is 16, 12, 16 and
       16, so printing it would break the shared record on i686. */
    printf("ld_at_least_double=%d\n", (int)(sizeof(long double) >= sizeof(double)));
    return 0;
}
