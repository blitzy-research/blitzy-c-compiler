/* 02_constant_expressions/002_constant_expression_contexts.c
 *
 * The four syntactic contexts in which C *requires* an integer constant
 * expression, exercised end to end so that the value the compiler computed at
 * translation time becomes observable at run time:
 *
 *   1. array bounds            -- static int vec[(3 * 4) - 2];
 *   2. case labels             -- case (2 * 3) - 1:
 *   3. bitfield widths         -- unsigned int u : (2 + 3);
 *   4. enumerator initializers -- enum { CTX_DERIVED = CTX_BASE * 4 };
 *
 * Two further required-constant-expression contexts come along at no cost and
 * are covered as well: the controlling expression of a _Static_assert, and the
 * initializer of an object with static storage duration.
 *
 * Every constant expression here is a genuine expression rather than a bare
 * literal, so the constant evaluator is actually asked to compute something,
 * and every value it fixes is printed.  A required-constant-expression context
 * is exactly where a constant evaluator's edge cases surface, which is the
 * repository's own open risk -- "C11 corner case non-compliance ... edge cases
 * in complex declarators and type conversions may remain -- Open, requires
 * targeted testing" (docs/project-guide.md line 248) -- and its remaining-work
 * item "C11 Standard Corner Case Compliance Testing" (line 111).  No existing
 * suite compares the run-time observable results of these contexts against an
 * independent compiler.
 *
 * THE TWO-VARIANT RULE.  Every value a constant context fixes is also produced
 * by a run-time computation over volatile-qualified operands and printed as a
 * twin line, so a divergence localizes to the constant evaluator or to the code
 * generator rather than to "one of the two".  The mechanism is not theoretical:
 * `volatile int x = 7; return x * 6;` emits a genuine runtime multiply at -O2,
 * whereas the non-volatile form folds to a single immediate move.  Because -O0
 * runs no passes, -O1 adds constant folding and -O2 adds algebraic
 * simplification iterated to a fixed point (docs/technical-specifications.md
 * line 535), the folded variant exercises the semantic evaluator at -O0 and the
 * folder above it, while the volatile twin asks the backend to compute at every
 * level.  Without the twin, optimization would substitute the folder's answer
 * for the backend's and a code-generation defect would escape detection.
 *
 * The runtime twins follow one deliberate pattern: each volatile object is read
 * exactly once into a plain local, and the arithmetic is then performed on the
 * locals.  The volatile read is the optimization barrier -- its result is opaque
 * to the compiler, so nothing computed from it can be folded -- and the pattern
 * keeps every full expression to a single side effect and leaves every printf
 * argument free of side effects altogether, so no printed value can depend on
 * an unspecified order of evaluation.  The two switch dispatch lines are the
 * one place worth naming explicitly: their two arguments are a
 * constant-argument call and a volatile-argument call to the same pure
 * function, so exactly one argument has a side effect (the volatile read) and
 * the other cannot be affected by it in either evaluation order.
 *
 * A bound is also twinned by a volatile *counter*: the loop that fills an array
 * increments a volatile counter once per iteration, so the increments may not be
 * collapsed and the accumulated total is a genuine run-time count of how many
 * elements the bound actually admitted -- the same number the folded
 * element-count ratio claims, arrived at by a route the optimizer cannot fold.
 *
 * NO HEADER IS INCLUDED and printf is hand-declared.  bcc bundles only nine
 * freestanding headers -- stddef.h, stdint.h, stdarg.h, stdbool.h, limits.h,
 * float.h, stdalign.h, stdnoreturn.h and iso646.h -- and ships no stdio.h
 * (docs/technical-specifications.md lines 202-214), so naming one would fail
 * against bcc while succeeding against the reference compiler: a spurious
 * divergence caused by the test rather than by the compiler.  _Static_assert is
 * a C11 keyword and needs no header either (line 497), and stdalign.h supplies
 * only the lowercase alignas/alignof macros (line 212).  Area 02 is not one of
 * the two areas sanctioned to include a header, and it needs no exception.
 *
 * STRICTLY CONFORMING C11.  The suite's default warning gate is
 * -Wall -Wextra -pedantic -Wconversion -Wsign-conversion -Wshadow -Werror and
 * area 02 sanctions no deviation from it, so nothing here is a GCC extension:
 * no statement expression, no typeof, no computed goto, no __attribute__, no
 * __builtin_*, no inline assembly, no __extension__, no __int128, no
 * zero-length array and no binary 0b literal.  Every _Static_assert carries its
 * message string, because the message-less form is C23 and -pedantic rejects
 * it.  No constant comma expression appears anywhere either: a comma expression
 * whose left operand has no side effect is rejected by -Werror=unused-value,
 * and folding through the comma operator is the subject of
 * 006_conditional_and_comma_folding.c, which supplies side-effecting operands.
 *
 * DETERMINISM.  One line per semantic property claimed, in fixed order.  Only
 * %d and %lld appear, each matched exactly to its argument type, because GCC
 * treats the hand-declared printf as a builtin and applies -Wformat to it.  No
 * address or pointer value is printed; no plain-char value is printed, since
 * plain char is signed on x86-64 and i686 but unsigned on AArch64 and RISC-V
 * 64; no plain long is printed, since sizeof(long) is 4 on i686 and 8 on the
 * other three targets (docs/technical-specifications.md lines 457-462).  Array
 * bounds are printed only as element-count ratios, which are target-invariant,
 * never as a sizeof in its own right.  Wide arithmetic uses long long with LL
 * literals and %lld, which is 64 bits on every target.  There is no timestamp,
 * no randomness, no uninitialized read, no locale-dependent formatting and no
 * data-dependent iteration order.
 *
 * One target assumption is relied upon and is worth stating: the sparse switch
 * carries the label (int)sizeof(int) * 1000, which is 4000 because int is four
 * bytes on all four supported targets.  It is deliberately NOT wrapped in a
 * _Static_assert.  A failed static assertion would refuse the translation unit
 * and cost every other assertion in the cell; leaving it to the dispatch means
 * a target where int were some other width would show up as exactly one
 * differing line -- sparse_sel4000 -- with the other 53 still compared.
 *
 * WHAT sizeof(struct bits) WOULD ADD, AND WHY IT IS NOT PRINTED.  The width of
 * a bitfield is what this program puts under test, and a width is observable
 * through the values the field holds: each field here is written with both the
 * largest and the smallest value it can represent, and read back.  A struct's
 * total size is a different claim -- about allocation units and padding -- and
 * 04_bitfields/001_layout_and_size.c already owns it, printing sizeof and
 * _Alignof for five bitfield structs.  Printing it here would duplicate that
 * coverage while adding an implementation-defined layout property to a program
 * whose subject is constant evaluation.  The omission is therefore a decision
 * rather than an oversight, and it is narrow: no declared width goes untested.
 *
 * HERMETIC.  Every input is a literal in this file.  Nothing is opened, no
 * environment variable and no command line is consulted, no clock and no
 * randomness is read, and printf is the only external symbol, so every cell is
 * reproducible from this source and its expectation record alone.
 *
 * UNDEFINED-BEHAVIOUR FREEDOM, which is what makes a divergence mean anything
 * at all: every array index is provably inside its bound, including the last
 * element of every array; every bitfield store is inside the field's
 * representable range, asserted statically below, so no implementation-defined
 * truncation occurs; every switch selector is either matched by a label or
 * deliberately routed to default; every shift count is a small non-negative
 * constant far inside the width of its promoted operand; no signed computation
 * approaches INT_MAX or the long long maximum; no pointer is formed at all, so
 * no aliasing violation and no one-past-end question arises; no object is
 * modified twice between sequence points; no argument beyond the one noted
 * above has a side effect; and nothing depends on padding bytes or on the
 * addresses of unrelated objects.
 */
int printf(const char *, ...);

/* ------------------------------------------------------------------------- *
 * The constant expressions under test, named once and reused, so that the
 * same expression supplies an array bound, a bitfield width, a case label, an
 * enumerator initializer and a static initializer.  Reuse is the point: it is
 * what makes a divergence between two contexts attributable to the context
 * rather than to two differently written expressions.
 * ------------------------------------------------------------------------- */
#define WIDTH_BASE     (2 + 1)                             /*  3 */
#define WIDTH_SIGNED   (WIDTH_BASE + 1)                    /*  4 */
#define WIDTH_UNSIGNED (2 + 3)                             /*  5 */
#define WIDTH_TAIL     ((WIDTH_UNSIGNED + 1) / 2)          /*  3 */
#define VEC_LEN        ((3 * 4) - 2)                       /* 10 */
#define MAT_ROWS       (2 + 1)                             /*  3 */
#define MAT_COLS       (4 / 2)                             /*  2 */

/* A 64-bit intermediate narrowed by an explicit cast, still an integer
 * constant expression and therefore still legal as an array bound: (1 << 33)
 * divided by (1 << 30) is 8.  This is the one place the constant evaluator is
 * asked for wide arithmetic in a context that requires a constant. */
#define WIDE_LEN ((int)((1LL << 33) / (1LL << 30)))         /*  8 */

/* A sizeof-derived case label.  The explicit (int) cast makes the label's type
 * match the controlling expression, which is what keeps -Wsign-compare and
 * -Wconversion quiet without weakening either. */
#define SEL_SIZEOF ((int)sizeof(int) * 1000)                /* 4000 */

/* Context 4: enumerator initializers.  Plain arithmetic, a value derived from
 * the preceding enumerator, a bitwise fold, a negative value, a value derived
 * from that negative one, a subtraction that feeds an array bound below, and a
 * shift whose count comes from a macro.  Every value is inside int range, so
 * the enumeration's underlying type cannot vary across targets in a way any
 * printed value could observe. */
enum ctx {
    CTX_BASE     = 2 + 3,                   /*   5 */
    CTX_DERIVED  = CTX_BASE * 4,            /*  20 */
    CTX_MASKED   = 0x10 | 0x01,             /*  17 */
    CTX_NEGATIVE = -3,                      /*  -3 */
    CTX_STEPPED  = CTX_NEGATIVE + 1,        /*  -2 */
    CTX_SPAN     = CTX_MASKED - 10,         /*   7 */
    CTX_SHIFTED  = 1 << (WIDTH_BASE + 1)    /*  16 */
};

/* Context 3: bitfield widths, every one of them a constant expression rather
 * than a literal.  The base types are explicitly signed and unsigned, never
 * plain int, because a bitfield declared on unqualified int has
 * implementation-defined signedness and this program prints its fields' values.
 * The zero-width unnamed member is the standard separator: it ends the current
 * allocation unit, so t begins a new one.  No total size is printed -- see the
 * header block -- so no padding bit is ever observed. */
struct bits {
    signed int   s : WIDTH_SIGNED;    /* 4 bits: -8 .. 7 */
    unsigned int u : WIDTH_UNSIGNED;  /* 5 bits:  0 .. 31 */
    unsigned int   : 0;               /* separator: t starts a fresh unit */
    unsigned int t : WIDTH_TAIL;      /* 3 bits:  0 .. 7 */
};

/* Context 1: array bounds.  Four different shapes of constant expression --
 * arithmetic, a two-dimensional pair, a narrowed 64-bit quotient, and an
 * enumerator -- so a bound that folds wrongly shows up as a wrong element
 * count and as a wrong end value rather than as silence. */
static int vec[VEC_LEN];
static int mat[MAT_ROWS][MAT_COLS];
static int wide[WIDE_LEN];
static int spanned[CTX_SPAN];

/* Bonus context: an object with static storage duration whose initializer is a
 * list of constant expressions, one of them drawn from an enumerator and one
 * from two array-bound macros. */
static const int folded_seeds[MAT_ROWS] = {
    WIDTH_SIGNED * 2,   /*  8 */
    CTX_MASKED + 1,     /* 18 */
    VEC_LEN - MAT_ROWS  /*  7 */
};

/* Constant variant of the bitfield contexts.  Three instances: one ordinary,
 * one holding the minimum each field can represent, one holding the maximum.
 * An unnamed member takes no initializer, so the three values below land on s,
 * u and t in order. */
static struct bits fixed_bits     = { -6, 21u, 5u };
static struct bits edge_low_bits  = { -8,  0u, 0u };
static struct bits edge_high_bits = {  7, 31u, 7u };

/* Runtime variant of the bitfield contexts.  The destination is volatile, so
 * every store must emit a real insert and every read a real extract; it has
 * static storage duration and therefore starts zeroed, so nothing
 * uninitialized is ever read even before the first store. */
static volatile struct bits live_bits;

/* Volatile operand sources.  Each is read exactly once into a plain local and
 * the arithmetic is done on the copy, which is opaque to the optimizer.  The
 * signed sources feed expressions whose value range is provably inside the
 * destination field, and the unsigned sources are masked to their destination
 * field's exact width, which is what makes every store value-preserving for the
 * strict conversion warnings without weakening the test. */
static volatile int          v_bits_signed = 6;    /* -(x & 7)     -> -6 */
static volatile int          v_bits_zero   = 0;    /* selects the -8 arm  */
static volatile int          v_bits_seven  = 7;    /* (x & 7)      ->  7 */
static volatile unsigned int v_bits_u      = 21u;  /* x & 31u      -> 21 */
static volatile unsigned int v_bits_u_zero = 0u;   /* x & 31u      ->  0 */
static volatile unsigned int v_bits_u_max  = 31u;  /* x & 31u      -> 31 */

/* Volatile operand sources for the enumerator twins.  Each enumerator's
 * DEFINING expression is recomputed from these at run time -- an add, a
 * multiply, a bitwise or, a negation, a subtraction and a shift -- so the
 * backend performs the operation the constant evaluator folded. */
static volatile int v_two     = 2;
static volatile int v_three   = 3;
static volatile int v_four    = 4;
static volatile int v_one     = 1;
static volatile int v_ten     = 10;
static volatile int v_mask_hi = 0x10;
static volatile int v_mask_lo = 0x01;
static volatile int v_shift   = WIDTH_BASE + 1;    /* 4 */

/* Volatile fill factors, so that every element stored into an array is computed
 * at run time rather than materialized from a folded constant. */
static volatile int v_step   = 3;
static volatile int v_factor = 7;

/* Volatile 64-bit sources for the wide-arithmetic twin. */
static volatile long long v_wide_num = 1LL << 33;
static volatile long long v_wide_den = 1LL << 30;

/* Volatile iteration counters: one per bound, each incremented once per loop
 * iteration.  Because they are volatile, the increments may not be collapsed
 * into one, so the accumulated total is a genuine run-time count of the
 * iterations the bound admitted. */
static volatile int v_walk_vec     = 0;
static volatile int v_walk_rows    = 0;
static volatile int v_walk_cols    = 0;
static volatile int v_walk_wide    = 0;
static volatile int v_walk_spanned = 0;

/* Bonus context: _Static_assert controlling expressions.  Each is satisfied and
 * each carries its message string.  Together they tie the four contexts to one
 * another -- an array bound to an enumerator, the bitfield widths to the values
 * stored in them -- so a constant expression that folded wrongly is refused at
 * translation time rather than merely printed wrongly. */
_Static_assert(sizeof vec / sizeof vec[0] == 10,
               "array bound: (3 * 4) - 2 folds to 10 elements");
_Static_assert(sizeof mat / sizeof mat[0] == 3,
               "array bound: 2 + 1 folds to 3 rows");
_Static_assert(sizeof mat[0] / sizeof mat[0][0] == 2,
               "array bound: 4 / 2 folds to 2 columns");
_Static_assert(sizeof wide / sizeof wide[0] == 8,
               "array bound: (1LL << 33) / (1LL << 30) narrows to 8 elements");
_Static_assert(sizeof spanned / sizeof spanned[0] == 7,
               "array bound: the enumerator CTX_SPAN folds to 7 elements");
_Static_assert(sizeof folded_seeds / sizeof folded_seeds[0] == 3,
               "static initializer: three constant expressions in three slots");
_Static_assert(CTX_BASE == 5 && CTX_DERIVED == 20,
               "enumerator initializers: 2 + 3, and 4 times the predecessor");
_Static_assert(CTX_MASKED == 17 && CTX_SHIFTED == 16,
               "enumerator initializers: a bitwise or, and a macro-driven shift");
_Static_assert(CTX_NEGATIVE == -3 && CTX_STEPPED == -2,
               "enumerator initializers: a negative value and its successor");
_Static_assert(CTX_SPAN == 7 && CTX_SPAN == CTX_MASKED - 10,
               "enumerator initializers: the value that also bounds an array");
_Static_assert(WIDTH_SIGNED + WIDTH_UNSIGNED + WIDTH_TAIL == 12,
               "bitfield widths: 4 + 5 + 3 folds to 12 bits of payload");
_Static_assert(-8 >= -(1 << (WIDTH_SIGNED - 1)) && 7 <= (1 << (WIDTH_SIGNED - 1)) - 1,
               "every value stored in the signed field is representable in it");
_Static_assert(31 <= (1 << WIDTH_UNSIGNED) - 1 && 7 <= (1 << WIDTH_TAIL) - 1,
               "every value stored in an unsigned field is representable in it");

/* ------------------------------------------------------------------------- *
 * Context 2: case labels.  Two switches, deliberately with different label
 * densities, because density is what selects the lowering strategy in the
 * backend: a dense set is normally lowered to an indexed jump, a sparse set to
 * a comparison tree.  Every label is a constant expression and every label
 * value is distinct, so no duplicate-label question arises.  Each function
 * returns a value unique to the label it matched, so a mis-dispatch is visible
 * as a wrong number rather than as an absence.
 * ------------------------------------------------------------------------- */

/* Dense: six consecutive selector values, 0 through 5, none of them written as
 * a bare literal.
 *
 * The returned values are deliberately NOT monotonic in the selector.  This was
 * measured rather than guessed: with the returns in ascending order the whole
 * switch is recognised as `k + 10` for k in range and lowered to a range check
 * and an add at -O2, so no indexed dispatch survives and the dense lowering path
 * this function exists to reach is never taken.  A shuffled result set has no
 * arithmetic relation to the selector, so the dense label set must be lowered as
 * a table at every optimization level -- which is the whole point of pairing it
 * with the sparse set below. */
static int dense_pick(int k)
{
    switch (k) {
    case 1 - 1:       return 11;
    case 2 - 1:       return 15;
    case 4 / 2:       return 12;
    case 1 + 2:       return 18;
    case 8 / 2:       return 13;
    case (2 * 3) - 1: return 17;
    default:          return -1;
    }
}

/* Sparse: six selector values spread from -5 to 4000, drawn from a negation, a
 * pair of bitfield-width macros, arithmetic, an enumerator, a shift, and
 * sizeof.  The gaps are what make this a different lowering problem from
 * dense_pick above. */
static int sparse_pick(int k)
{
    switch (k) {
    case -(3 + 2):                      return 20;  /*   -5 */
    case WIDTH_SIGNED + WIDTH_UNSIGNED: return 21;  /*    9 */
    case (7 * 7) - 7:                   return 22;  /*   42 */
    case CTX_DERIVED * 5:               return 23;  /*  100 */
    case 1 << 10:                       return 24;  /* 1024 */
    case SEL_SIZEOF:                    return 25;  /* 4000 */
    default:                            return -1;
    }
}

int main(void)
{
    /* Loop indices and the volatile selector that drives every run-time
     * dispatch.  Nothing here shadows a file-scope name. */
    int row;
    int col;
    int idx;
    volatile int sel;

    /* Plain copies of the volatile fill factors and operands.  One volatile
     * read each, after which the compiler knows nothing about the values, so
     * every computation below them is emitted rather than folded. */
    int step   = v_step;
    int factor = v_factor;
    int ten    = v_ten;
    int one    = v_one;

    /* ---- Context 1: array bounds, the folded facts ---------------------- */
    /* Element counts are ratios of sizeof and therefore target-invariant; no
     * sizeof is printed in its own right. */
    printf("bound_vec=%d\n", (int)(sizeof vec / sizeof vec[0]));
    printf("bound_mat_rows=%d\n", (int)(sizeof mat / sizeof mat[0]));
    printf("bound_mat_cols=%d\n", (int)(sizeof mat[0] / sizeof mat[0][0]));
    printf("bound_wide=%d\n", (int)(sizeof wide / sizeof wide[0]));
    printf("bound_spanned=%d\n", (int)(sizeof spanned / sizeof spanned[0]));

    /* The wide constant expression that bounds `wide`, printed twice: the
     * quotient the bound narrows to, and the 64-bit dividend itself, so the
     * evaluator's wide arithmetic is observable and not merely inferred. */
    printf("fold_wide_quotient=%d\n", WIDE_LEN);
    printf("fold_wide_shifted=%lld\n", 1LL << 33);

    /* ---- Context 1: the same bounds, computed at run time --------------- */
    /* Each loop fills its array to the bound and increments a volatile counter
     * once per iteration.  The counter cannot be folded, so its total is a
     * genuine run-time count of the elements the bound admitted, and the fill
     * values are products and sums of an opaque factor rather than constants. */
    for (idx = 0; idx < (int)(sizeof vec / sizeof vec[0]); ++idx) {
        vec[idx] = (idx + 1) * step;
        v_walk_vec = v_walk_vec + 1;
    }
    for (row = 0; row < (int)(sizeof mat / sizeof mat[0]); ++row) {
        v_walk_rows = v_walk_rows + 1;
        for (col = 0; col < (int)(sizeof mat[0] / sizeof mat[0][0]); ++col) {
            mat[row][col] = (row + 1) * ten + col;
            /* Counted on the first row only, so the total is the column bound
             * rather than the element bound. */
            if (row == 0) {
                v_walk_cols = v_walk_cols + 1;
            }
        }
    }
    for (idx = 0; idx < (int)(sizeof wide / sizeof wide[0]); ++idx) {
        wide[idx] = idx + one;
        v_walk_wide = v_walk_wide + 1;
    }
    for (idx = 0; idx < (int)(sizeof spanned / sizeof spanned[0]); ++idx) {
        spanned[idx] = (idx + 1) * factor;
        v_walk_spanned = v_walk_spanned + 1;
    }
    printf("run_bound_vec=%d\n", (int)v_walk_vec);
    printf("run_bound_mat_rows=%d\n", (int)v_walk_rows);
    printf("run_bound_mat_cols=%d\n", (int)v_walk_cols);
    printf("run_bound_wide=%d\n", (int)v_walk_wide);
    printf("run_bound_spanned=%d\n", (int)v_walk_spanned);

    /* The wide twin.  Division is deliberately performed by repeated addition
     * and comparison in 64-bit arithmetic: every backend implements those
     * directly, so this twin measures the code generator rather than the
     * presence of a compiler support routine for 64-bit division, which is not
     * this program's subject.  Eight additions of 2^30 reach exactly 2^33, so
     * the quotient is 8 and the accumulated dividend is 8589934592, and neither
     * value comes anywhere near the long long maximum. */
    {
        long long num = v_wide_num;
        long long den = v_wide_den;
        long long acc = 0;
        int quotient = 0;

        while (acc + den <= num) {
            acc = acc + den;
            quotient = quotient + 1;
        }
        printf("run_wide_quotient=%d\n", quotient);
        printf("run_wide_shifted=%lld\n", acc);
    }

    /* Both ends of every array, read back after the fills.  The last index of
     * each is written as its bound minus one, which is provably inside the
     * array, so the bound is exercised rather than merely computed. */
    printf("vec_ends first=%d last=%d\n", vec[0], vec[VEC_LEN - 1]);
    printf("mat_ends first=%d last=%d\n",
           mat[0][0], mat[MAT_ROWS - 1][MAT_COLS - 1]);
    printf("wide_ends first=%d last=%d\n", wide[0], wide[WIDE_LEN - 1]);
    printf("spanned_ends first=%d last=%d\n", spanned[0], spanned[CTX_SPAN - 1]);

    /* ---- Context 2: case labels ---------------------------------------- */
    /* One line per selector value, each carrying both variants: the folded
     * dispatch, whose argument is the same constant expression that spells the
     * label, and the run-time dispatch through the volatile selector, which no
     * optimization level may fold away.  The last line of each group routes a
     * value that matches no label to default. */
    sel = 1 - 1;
    printf("dense_sel0 fold=%d run=%d\n", dense_pick(1 - 1), dense_pick(sel));
    sel = 2 - 1;
    printf("dense_sel1 fold=%d run=%d\n", dense_pick(2 - 1), dense_pick(sel));
    sel = 4 / 2;
    printf("dense_sel2 fold=%d run=%d\n", dense_pick(4 / 2), dense_pick(sel));
    sel = 1 + 2;
    printf("dense_sel3 fold=%d run=%d\n", dense_pick(1 + 2), dense_pick(sel));
    sel = 8 / 2;
    printf("dense_sel4 fold=%d run=%d\n", dense_pick(8 / 2), dense_pick(sel));
    sel = (2 * 3) - 1;
    printf("dense_sel5 fold=%d run=%d\n",
           dense_pick((2 * 3) - 1), dense_pick(sel));
    sel = 2 * 3;
    printf("dense_default fold=%d run=%d\n",
           dense_pick(2 * 3), dense_pick(sel));

    sel = -(3 + 2);
    printf("sparse_sel_minus5 fold=%d run=%d\n",
           sparse_pick(-(3 + 2)), sparse_pick(sel));
    sel = WIDTH_SIGNED + WIDTH_UNSIGNED;
    printf("sparse_sel9 fold=%d run=%d\n",
           sparse_pick(WIDTH_SIGNED + WIDTH_UNSIGNED), sparse_pick(sel));
    sel = (7 * 7) - 7;
    printf("sparse_sel42 fold=%d run=%d\n",
           sparse_pick((7 * 7) - 7), sparse_pick(sel));
    sel = CTX_DERIVED * 5;
    printf("sparse_sel100 fold=%d run=%d\n",
           sparse_pick(CTX_DERIVED * 5), sparse_pick(sel));
    sel = 1 << 10;
    printf("sparse_sel1024 fold=%d run=%d\n",
           sparse_pick(1 << 10), sparse_pick(sel));
    sel = SEL_SIZEOF;
    printf("sparse_sel4000 fold=%d run=%d\n",
           sparse_pick(SEL_SIZEOF), sparse_pick(sel));
    sel = (1 << 10) + 1;
    printf("sparse_default fold=%d run=%d\n",
           sparse_pick((1 << 10) + 1), sparse_pick(sel));

    /* ---- Context 3: bitfield widths, the folded facts ------------------- */
    /* The read-back values are the assertion: a field narrower than declared
     * could not return them, and every one of them is inside the field's
     * representable range, which the static assertions above establish. */
    printf("bits_fixed s=%d u=%d t=%d\n",
           (int)fixed_bits.s, (int)fixed_bits.u, (int)fixed_bits.t);
    printf("bits_edge_low s=%d u=%d t=%d\n",
           (int)edge_low_bits.s, (int)edge_low_bits.u, (int)edge_low_bits.t);
    printf("bits_edge_high s=%d u=%d t=%d\n",
           (int)edge_high_bits.s, (int)edge_high_bits.u, (int)edge_high_bits.t);

    /* ---- Context 3: the same fields, stored and read at run time -------- */
    /* Each group stores all three fields from opaque operands and then reads
     * each field into a plain local before printing, so every full expression
     * performs exactly one volatile access. */
    {
        int      src_s = v_bits_signed;   /*  6 */
        unsigned src_u = v_bits_u;        /* 21 */
        int read_s;
        int read_u;
        int read_t;

        live_bits.s = -(src_s & 7);       /* range -7 .. 0, value -6 */
        live_bits.u = src_u & 31u;        /* range  0 .. 31, value 21 */
        live_bits.t = src_u & 7u;         /* range  0 .. 7, value 5 */
        read_s = (int)live_bits.s;
        read_u = (int)live_bits.u;
        read_t = (int)live_bits.t;
        printf("run_bits_fixed s=%d u=%d t=%d\n", read_s, read_u, read_t);
    }
    {
        int      src_s = v_bits_zero;     /* 0 */
        unsigned src_u = v_bits_u_zero;   /* 0 */
        int read_s;
        int read_u;
        int read_t;

        /* The field's minimum is selected at run time between two literals that
         * both sit inside the declared width, rather than computed as
         * "(src_s & 7) - 8".  Both forms store -8 and both are opaque to the
         * optimizer, but the subtraction form was measured to provoke
         * "conversion from int to signed char:4 may change value" once
         * instrumentation is added, because the instrumented form loses the
         * value range the plain form carries through the mask and the subtract.
         * The two gates are separate invocations and the harness refuses to mix
         * them, so the subtraction form would have passed; the selection form is
         * preferred anyway because it is clean under every combination and it
         * matches the idiom the runtime signed-bitfield stores elsewhere in the
         * corpus already use.  The condition derives from the volatile read
         * taken above, so the store is still a genuine run-time insert into a
         * volatile bitfield and the minimum is still the value written. */
        live_bits.s = ((src_s & 7) != 0) ? -1 : -8;   /* both in range, -8 */
        live_bits.u = src_u & 31u;        /* value 0 */
        live_bits.t = src_u & 7u;         /* value 0 */
        read_s = (int)live_bits.s;
        read_u = (int)live_bits.u;
        read_t = (int)live_bits.t;
        printf("run_bits_edge_low s=%d u=%d t=%d\n", read_s, read_u, read_t);
    }
    {
        int      src_s = v_bits_seven;    /*  7 */
        unsigned src_u = v_bits_u_max;    /* 31 */
        int read_s;
        int read_u;
        int read_t;

        live_bits.s = src_s & 7;          /* range 0 .. 7, value 7 */
        live_bits.u = src_u & 31u;        /* value 31 */
        live_bits.t = src_u & 7u;         /* value 7 */
        read_s = (int)live_bits.s;
        read_u = (int)live_bits.u;
        read_t = (int)live_bits.t;
        printf("run_bits_edge_high s=%d u=%d t=%d\n", read_s, read_u, read_t);
    }

    /* ---- Context 4: enumerator initializers, the folded values ---------- */
    printf("enum_base=%d\n", (int)CTX_BASE);
    printf("enum_derived=%d\n", (int)CTX_DERIVED);
    printf("enum_masked=%d\n", (int)CTX_MASKED);
    printf("enum_negative=%d\n", (int)CTX_NEGATIVE);
    printf("enum_stepped=%d\n", (int)CTX_STEPPED);
    printf("enum_span=%d\n", (int)CTX_SPAN);
    printf("enum_shifted=%d\n", (int)CTX_SHIFTED);

    /* ---- Context 4: each defining expression recomputed at run time ----- */
    /* Not a copy of the enumerator's value: the operation that produced it is
     * performed again on opaque operands, so the add, the multiply, the bitwise
     * or, the negation, the subtraction and the shift are all emitted. */
    {
        int two    = v_two;
        int three  = v_three;
        int four   = v_four;
        int hi     = v_mask_hi;
        int lo     = v_mask_lo;
        int shift  = v_shift;
        int base   = two + three;      /*   5 */
        int masked = hi | lo;          /*  17 */
        int neg    = -three;           /*  -3 */

        printf("run_enum_base=%d\n", base);
        printf("run_enum_derived=%d\n", base * four);
        printf("run_enum_masked=%d\n", masked);
        printf("run_enum_negative=%d\n", neg);
        printf("run_enum_stepped=%d\n", neg + one);
        printf("run_enum_span=%d\n", masked - ten);
        printf("run_enum_shifted=%d\n", one << shift);

        /* ---- Bonus context: the static initializer, both variants ------- */
        /* The folded line reads the object the compiler initialized; the twin
         * recomputes the same three constant expressions from opaque operands,
         * reusing the values already read above. */
        printf("seeds=%d %d %d\n",
               folded_seeds[0], folded_seeds[1], folded_seeds[2]);
        printf("run_seeds=%d %d %d\n",
               four * 2, masked + 1, ten - three);
    }
    return 0;
}

