/* _Static_assert across the type system, on SATISFIED conditions only.
 *
 * Every assertion here is satisfied, and that is a decision rather than a
 * compromise.  Both mandated oracles compare the behaviour of a binary that was
 * built successfully, so a deliberately failing _Static_assert would refuse the
 * translation unit under BOTH compilers and leave neither oracle anything to
 * compare.  Correct REJECTION of an invalid program is a noted non-goal of this
 * suite for exactly that reason.  Each assertion is instead paired with a printed
 * line reporting the same property at run time, so a compiler that accepted the
 * assertion while computing the property differently still changes stdout.
 *
 * EVERY ASSERTION IS TARGET-INVARIANT, which is the critical hazard here: an
 * assertion whose truth depended on a target-varying property would make this
 * program fail to compile on some targets, destroying the four-way comparison
 * rather than testing it.  Only two kinds of condition therefore appear -- exact
 * widths and alignments identical on all four supported targets (sizeof
 * char/short/int/long long/float/double = 1/2/4/8/4/8, _Alignof
 * char/short/int/float = 1/2/4/4), and RELATIONS between quantities, which hold
 * whatever the individual values are.
 *
 * Deliberately NOT asserted, with the reason: sizeof(long), sizeof(void *),
 * sizeof(long double) and the alignments of double, long long, long and pointers
 * all vary by target (pointer and long width is 4 bytes on i686 and 8 elsewhere;
 * long double measured 16, 12, 16 and 16).  Each is still covered, but only
 * through relations true everywhere -- long is exactly pointer width, size_t and
 * ptrdiff_t are exactly pointer width, double is no wider than long double.  Plain
 * char signedness is never asserted and never observed: plain char appears only as
 * a one-byte size probe.  Pinning the width-varying types per target belongs to
 * 10_declarations_and_types/007_alignof_alignas.c.
 *
 * A _Static_assert emits no code, so a folded line alone would let the constant
 * evaluator answer for the backend.  Each fold_ line is therefore paired with a
 * run_ line that recomputes the property from values no compiler may fold.  For
 * sizes the runtime form is a MEASUREMENT -- the byte distance between adjacent
 * elements of a static array, taken through volatile character pointers in
 * stride_of below, so the number comes from address arithmetic the code generator
 * emitted.  For arithmetic and alignments the operands are held in volatile
 * objects instead.
 *
 * No header is named and printf is hand-declared.  bcc ships no stdio.h: its
 * bundled set is the nine required freestanding headers plus a bonus stdatomic.h,
 * ten files in all (docs/project-guide.md, "9 bundled freestanding headers").
 * _Static_assert and _Alignof are C11 KEYWORDS needing no header; stdalign.h
 * would only add the lowercase alignas/alignof spellings.  Every assertion uses
 * the two-argument form with a message, because the message-less form is C23 and
 * the mandatory -pedantic -Werror gate rejects it.
 *
 * WARNING-GATE NOTE FOR MAINTAINERS.  This program is clean under the full default
 * gate with no deviation, and one detail must not be simplified away: the pair
 * array in local_type_probe is volatile and stride_of takes const volatile void *.
 * Making the array plain lets the optimizer delete its dead initializing stores
 * and then report -Wmaybe-uninitialized when its address is passed on, while
 * dropping the volatile from stride_of's parameters would discard a qualifier at
 * the call and trip -Wdiscarded-qualifiers.  As written the pair is both
 * gate-clean and unfoldable, which is what the runtime variant needs.
 *
 * The undefined-behaviour argument and the recorded command lines live in the
 * sibling .expected record.
 */

int printf(const char *, ...);

/* Constants supplied by the preprocessor, so the assertions below depend on a
 * macro-provided constant expression as well as on the type system.  Each has a
 * folded print and a volatile-operand runtime twin further down. */
#define TABLE_LEN 7
#define FOLD_SUM (2 + 3 * 4)
#define FOLD_SHIFT ((1 << 10) >> 4)
#define FOLD_WIDE (4294967296LL / 65536)
#define LOCAL_PAIR_SUM (11 + 22 + 33 + 44)

/* Self-describing output: how many _Static_assert declarations this file carries
 * and how many distinct contexts they appear in.  The five contexts are file
 * scope before any other declaration, a struct member list, file scope after the
 * declarations, block scope in a helper, and block scope in main. */
#define STATIC_ASSERT_COUNT 34
#define STATIC_ASSERT_CONTEXTS 5

/* ---------------------------------------------------------------------------
 * Context 1 of 5 -- file scope, BEFORE any other declaration.
 * ------------------------------------------------------------------------- */

/* The six widths that are identical on all four supported targets.  The u
 * suffix keeps each comparison unsigned-to-unsigned, so -Wsign-compare has
 * nothing to report even though the gate enables it through -Wextra. */
_Static_assert(sizeof(char) == 1u, "a char is exactly one byte by definition");
_Static_assert(sizeof(short) == 2u, "short is 2 bytes on all four supported targets");
_Static_assert(sizeof(int) == 4u, "int is 4 bytes on all four supported targets");
_Static_assert(sizeof(long long) == 8u, "long long is 8 bytes on all four supported targets");
_Static_assert(sizeof(float) == 4u, "float is IEEE binary32 on all four supported targets");
_Static_assert(sizeof(double) == 8u, "double is IEEE binary64 on all four supported targets");

/* Relations rather than values, which is how the width-varying types stay under
 * test without pinning a number that differs per target. */
_Static_assert(sizeof(char) <= sizeof(short) && sizeof(short) <= sizeof(int)
                   && sizeof(int) <= sizeof(long) && sizeof(long) <= sizeof(long long),
               "the integer size ordering the standard guarantees");
_Static_assert(sizeof(float) <= sizeof(double) && sizeof(double) <= sizeof(long double),
               "the floating size ordering the standard guarantees");
_Static_assert(sizeof(long) == sizeof(void *),
               "long is exactly pointer width on all four supported targets");

/* sizeof applied to a sizeof expression yields the width of size_t itself,
 * which is how this program probes that type without naming it: size_t needs
 * stddef.h, and this program includes no header. */
_Static_assert(sizeof(sizeof(int)) == sizeof(void *),
               "size_t is exactly pointer width, probed without naming the type");

/* The four alignments that are identical on all four supported targets, then
 * two families of relation that hold whatever the remaining alignments are. */
_Static_assert(_Alignof(char) == 1u, "char alignment is one byte");
_Static_assert(_Alignof(short) == 2u, "short alignment is two bytes on all four targets");
_Static_assert(_Alignof(int) == 4u, "int alignment is four bytes on all four targets");
_Static_assert(_Alignof(float) == 4u, "float alignment is four bytes on all four targets");
_Static_assert(_Alignof(char) <= _Alignof(int) && _Alignof(int) <= _Alignof(double),
               "alignment requirement does not decrease with rank");
_Static_assert(_Alignof(char) <= sizeof(char) && _Alignof(short) <= sizeof(short)
                   && _Alignof(int) <= sizeof(int) && _Alignof(float) <= sizeof(float)
                   && _Alignof(double) <= sizeof(double)
                   && _Alignof(long long) <= sizeof(long long),
               "no type aligns more strictly than its own size");

/* Constant expressions the folder must evaluate, one per printed arithmetic
 * family.  Each is recomputed at run time from volatile operands below. */
_Static_assert(FOLD_SUM == 14, "a macro-supplied constant expression folds to 14");
_Static_assert(FOLD_SHIFT == 64, "a shift identity within range folds to 64");
_Static_assert(FOLD_WIDE == 65536, "a wide constant division folds to 65536");
/* THE ONE EXECUTION-CHARACTER-SET CODE THIS PROGRAM COMPUTES, PINNED HERE.
 *
 * Exactly two of the printed lines carry a character's numeric code --
 * fold_char_a, which prints (int)'A' from a constant expression, and run_char_a,
 * which prints the same value back out of a volatile signed char.  Nothing else
 * in this file COMPUTES a character's value: msg is measured only by size and by
 * terminator offset, probe_char only by element size, and no line renders a
 * literal of the program's own.  One assertion therefore closes the accounting for
 * every character code this program computes.
 *
 * It does not, and cannot, account for the label bytes.  Every output line begins
 * with a literal label -- fold_sum=, run_char_a= and the rest -- and those bytes
 * are execution-character-set data too.  They are not asserted here because they
 * are not computed: printf copies them through unchanged, and both sides of every
 * comparison translate the same source with the same basic character set, so a
 * character-set difference would move whole labels rather than change one number,
 * on both sides identically.  That is a property of the environment, not of the
 * compiler under test, and it is why the assertion below is scoped to the code and
 * this paragraph is scoped to the labels.
 *
 * It is needed rather than decorative.  C11 5.2.1 leaves the members and codes of
 * the execution character set implementation-defined, and the explicit signed
 * char that carries the run-time half normalizes plain char's SIGNEDNESS, which
 * is a different property and says nothing about which code the letter A carries.
 * With this assertion in place an implementation whose character set differed
 * fails to TRANSLATE, naming the reason, instead of printing a different number
 * that oracle (a) or oracle (b) would charge to the compiler under test -- a
 * build failure accuses the environment, which is where the difference lives.
 * All four supported targets were measured to use the same ASCII-family basic
 * execution character set, so the condition holds on every enabled cell.  The
 * record's impl_defined_notes carries this accounting in prose. */
_Static_assert('A' == 65,
               "the execution character set places A at 65 (ASCII) on this target");

/* Declarations.  The aggregates are shaped so their sizes are invariant across
 * all four targets: struct triple is int, short, unsigned char at int alignment,
 * giving 8 everywhere, and union scalar_bytes is 8 everywhere because its widest
 * member is 8 bytes and its alignment divides 8 either way.  Anything whose
 * padding depended on the alignment of double or long long would differ on i686
 * and is avoided. */

struct triple {
    int first;
    short second;

    /* Context 2 of 5 -- inside a struct member list, which C11 permits as a
     * struct declaration.  The condition is a property of the two members
     * declared above it, and it is a relation, so it holds on every target. */
    _Static_assert(sizeof(short) < sizeof(int),
                   "the earlier members are ordered narrow before wide");

    unsigned char third;
};

union scalar_bytes {
    long long wide;
    int narrow;
    unsigned char raw[8];
};

/* Enumerators fixed by constant expressions: zero, an explicit value, a gap and
 * a negative value.  Only the enumerator VALUES are asserted and printed; the
 * size of the enumerated type is implementation-defined and is deliberately
 * neither asserted nor printed. */
enum limits { LIMIT_ZERO = 0, LIMIT_TWO = 2, LIMIT_GAP = 10, LIMIT_NEG = -1 };

/* Carrier for the local-type probe's six results, passed by pointer rather than
 * returned by value so this program does not depend on aggregate-return ABI
 * correctness and a defect there cannot be mistaken for one here. */
struct local_type_report {
    int fold_size_ok;
    int fold_align_ok;
    int fold_sum;
    int run_size_ok;
    int run_align_ok;
    int run_sum;
};

/* Two-element probe arrays, one per type whose size is measured at run time.
 * Their contents are never read: only the byte distance between element 0 and
 * element 1 is taken, which is exactly the element size.  probe_char is plain
 * char because sizeof(char) is the property measured, never its signedness. */
static char probe_char[2];
static short probe_short[2];
static long long probe_llong[2];
static float probe_float[2];
static double probe_double[2];
static long probe_long[2];
static void *probe_pointer[2];
static long double probe_ldouble[2];
static struct triple probe_triple[2];
static union scalar_bytes probe_union[2];

/* The table doubles as the int-stride probe and as the subject of the
 * macro-derived array-bound assertion.  msg is initialized from a string
 * literal so that the terminator can be located at run time. */
static const int table[TABLE_LEN] = { 1, 2, 3, 4, 5, 6, 7 };
static const char msg[] = "abc";

/* ---------------------------------------------------------------------------
 * Context 3 of 5 -- file scope, AFTER the declarations, so that the conditions
 * can depend on the types and objects declared above.
 * ------------------------------------------------------------------------- */

_Static_assert(sizeof(struct triple) >= sizeof(int) + sizeof(short) + sizeof(unsigned char),
               "a struct is at least the sum of its members");
_Static_assert(sizeof(union scalar_bytes) >= sizeof(long long),
               "a union is at least its largest member");
_Static_assert(_Alignof(struct triple) == _Alignof(int),
               "a struct aligns as strictly as its strictest member");
_Static_assert(LIMIT_ZERO == 0 && LIMIT_TWO == 2 && LIMIT_GAP == 10 && LIMIT_NEG == -1,
               "enumerator values are fixed by their constant expressions");
_Static_assert(LIMIT_GAP - LIMIT_TWO == 8, "arithmetic on enumerators folds");
_Static_assert(sizeof table / sizeof table[0] == TABLE_LEN,
               "the array bound came from the macro constant expression");
_Static_assert(sizeof msg == 4u && sizeof "abc" == 4u,
               "a string literal includes its terminator, as array and as literal");

/* Subtracting two pointers yields a ptrdiff_t, so sizeof of that difference is
 * the width of ptrdiff_t -- probed the same header-free way size_t was above.
 * The operand of sizeof is not evaluated, so no subtraction happens here, and
 * both element addresses are in range in any case. */
_Static_assert(sizeof(&table[1] - &table[0]) == sizeof(void *),
               "ptrdiff_t is exactly pointer width, probed without naming the type");

/* ---------------------------------------------------------------------------
 * Helpers.  Context 4 of 5 is block scope inside a function.
 * ------------------------------------------------------------------------- */

/* Measure, at run time, the byte distance between two addresses in the same
 * object.  Both pointers are read out of volatile objects, so neither the
 * difference nor its operands can be folded -- which is what makes every run_
 * size line an independent measurement.  The parameters are const volatile void *
 * so a pointer into either a plain or a volatile object converts without
 * discarding a qualifier. */
static int stride_of(const volatile void *first, const volatile void *second)
{
    const volatile unsigned char *volatile low = (const volatile unsigned char *)first;
    const volatile unsigned char *volatile high = (const volatile unsigned char *)second;

    /* Context 4 of 5 -- block scope inside a helper function.  The measurement
     * is only a size measurement because the carrier is one byte wide, so the
     * assumption the arithmetic rests on is stated where it is used. */
    _Static_assert(sizeof(unsigned char) == 1u, "the stride carrier is exactly one byte wide");

    return (int)(high - low);
}

/* Locate the terminator of a character array, bounded by the array's own size
 * so the scan cannot read past the end whatever the array contains.  If the
 * terminator were missing the scan stops at the bound and returns a larger
 * offset, so the printed length changes rather than the program misbehaving. */
static int nul_offset(const char *text, int limit)
{
    int index = 0;

    while (index < limit && text[index] != '\0') {
        index = index + 1;
    }
    return index;
}

/* Assert two properties of a type declared inside this block, then report each
 * of them twice: once as the folded constant expression the assertion itself
 * used, and once from volatile storage and a measured stride. */
static void local_type_probe(struct local_type_report *out)
{
    struct local_pair { int low; int high; };

    /* Volatile, so the initializing stores survive every optimization level and
     * the member reads below are genuine loads.  See the warning-gate note in
     * the file header before changing this. */
    volatile struct local_pair pair[2] = { { 11, 22 }, { 33, 44 } };
    volatile int pair_size = (int)sizeof(struct local_pair);
    volatile int pair_align = (int)_Alignof(struct local_pair);
    volatile int int_size = (int)sizeof(int);
    volatile int int_align = (int)_Alignof(int);

    _Static_assert(sizeof(struct local_pair) == 2u * sizeof(int),
                   "a locally declared type sizes as two ints");
    _Static_assert(_Alignof(struct local_pair) == _Alignof(int),
                   "a locally declared type aligns as its members do");

    out->fold_size_ok = (int)(sizeof(struct local_pair) == 2u * sizeof(int));
    out->fold_align_ok = (int)(_Alignof(struct local_pair) == _Alignof(int));
    out->fold_sum = LOCAL_PAIR_SUM;
    out->run_size_ok = (stride_of(&pair[0], &pair[1]) == pair_size)
        && (pair_size == 2 * int_size);
    out->run_align_ok = (pair_align == int_align);
    out->run_sum = pair[0].low + pair[0].high + pair[1].low + pair[1].high;
}


/* Context 5 of 5 -- block scope inside main, followed by the printed report.
 * Every fold_ line is immediately followed by its run_ twin, so a single
 * divergent line says whether the constant evaluator or the code generator
 * disagreed.  Raw values appear only where identical on all four targets; a
 * target-varying property is printed as a 0/1 relation instead. */

int main(void)
{
    /* Operands for the runtime arithmetic twins.  Volatile, so the optimizer
     * may not substitute the folded results printed on the fold_ lines. */
    volatile int two = 2;
    volatile int three = 3;
    volatile int four = 4;
    volatile int one = 1;
    volatile int ten = 10;
    volatile int shift_four = 4;
    volatile long long wide = 4294967296LL;
    volatile long long divisor = 65536;

    /* An explicitly signed char, so no plain-char signedness difference can
     * reach the printed value; 65 is representable in every char signedness. */
    volatile signed char letter = (signed char)'A';

    /* The enumerators, forced through volatile storage rather than folded. */
    volatile int e_zero = LIMIT_ZERO;
    volatile int e_two = LIMIT_TWO;
    volatile int e_gap = LIMIT_GAP;
    volatile int e_neg = LIMIT_NEG;

    /* The two header-free width probes, held in volatile storage so that the
     * comparison against the MEASURED pointer stride happens at run time. */
    volatile int size_t_width = (int)sizeof(sizeof(int));
    volatile int ptrdiff_width = (int)sizeof(&table[1] - &table[0]);

    /* Alignments in volatile storage, so the relations below are computed by
     * the backend from loaded values rather than folded to constants. */
    volatile int align_char = (int)_Alignof(char);
    volatile int align_short = (int)_Alignof(short);
    volatile int align_int = (int)_Alignof(int);
    volatile int align_float = (int)_Alignof(float);
    volatile int align_double = (int)_Alignof(double);
    volatile int align_llong = (int)_Alignof(long long);
    volatile int align_triple = (int)_Alignof(struct triple);

    /* Every size measured at run time as a byte stride between two adjacent
     * elements.  The long, pointer and long double strides are used only in
     * relations, never printed, because their values differ by target. */
    int stride_char = stride_of(&probe_char[0], &probe_char[1]);
    int stride_short = stride_of(&probe_short[0], &probe_short[1]);
    int stride_int = stride_of(&table[0], &table[1]);
    int stride_llong = stride_of(&probe_llong[0], &probe_llong[1]);
    int stride_float = stride_of(&probe_float[0], &probe_float[1]);
    int stride_double = stride_of(&probe_double[0], &probe_double[1]);
    int stride_long = stride_of(&probe_long[0], &probe_long[1]);
    int stride_pointer = stride_of(&probe_pointer[0], &probe_pointer[1]);
    int stride_ldouble = stride_of(&probe_ldouble[0], &probe_ldouble[1]);
    int stride_triple = stride_of(&probe_triple[0], &probe_triple[1]);
    int stride_union = stride_of(&probe_union[0], &probe_union[1]);

    /* The whole table measured in bytes.  table + TABLE_LEN is a one-past-end
     * pointer: it is formed and subtracted, and never dereferenced. */
    int table_span = stride_of(&table[0], table + TABLE_LEN);

    struct local_type_report local = { 0, 0, 0, 0, 0, 0 };

    /* Context 5 of 5 -- block scope inside main.  Both conditions restate
     * properties of objects declared at file scope in a form the earlier
     * assertions did not use: a product rather than a quotient, and the empty
     * string literal, whose single byte is its terminator. */
    _Static_assert(TABLE_LEN * sizeof(int) == sizeof table,
                   "the table spans exactly seven ints");
    _Static_assert(sizeof "" == 1u, "the empty string literal is its terminator alone");

    local_type_probe(&local);

    /* Two literal facts about this source file, so the output describes itself
     * rather than requiring the reader to count declarations. */
    printf("static_assert_count=%d\n", STATIC_ASSERT_COUNT);
    printf("static_assert_contexts=%d\n", STATIC_ASSERT_CONTEXTS);

    /* The six invariant widths, folded then measured. */
    printf("fold_sizes=%d %d %d %d %d %d\n", (int)sizeof(char), (int)sizeof(short),
           (int)sizeof(int), (int)sizeof(long long), (int)sizeof(float), (int)sizeof(double));
    printf("run_strides=%d %d %d %d %d %d\n", stride_char, stride_short, stride_int,
           stride_llong, stride_float, stride_double);

    /* The two size orderings, which cover long and long double without pinning
     * a width that differs per target. */
    printf("fold_size_order=%d %d\n",
           (int)(sizeof(char) <= sizeof(short) && sizeof(short) <= sizeof(int)
                 && sizeof(int) <= sizeof(long) && sizeof(long) <= sizeof(long long)),
           (int)(sizeof(float) <= sizeof(double) && sizeof(double) <= sizeof(long double)));
    printf("run_size_order=%d %d\n",
           (stride_char <= stride_short) && (stride_short <= stride_int)
               && (stride_int <= stride_long) && (stride_long <= stride_llong),
           (stride_float <= stride_double) && (stride_double <= stride_ldouble));

    /* long is pointer width.  Printed as a relation: the width itself is 4 on
     * i686 and 8 on the other three targets. */
    printf("fold_long_is_ptr_width=%d\n", (int)(sizeof(long) == sizeof(void *)));
    printf("run_long_is_ptr_width=%d\n", stride_long == stride_pointer);

    /* size_t and ptrdiff_t are pointer width.  The runtime twin compares the
     * compile-time widths against the pointer size MEASURED above, so a backend
     * whose pointer size disagreed with the analyser's would print 0. */
    printf("fold_width_probes=%d %d\n", (int)(sizeof(sizeof(int)) == sizeof(void *)),
           (int)(sizeof(&table[1] - &table[0]) == sizeof(void *)));
    printf("run_width_probes=%d %d\n", size_t_width == stride_pointer,
           ptrdiff_width == stride_pointer);

    /* The four invariant alignments, folded then loaded from volatile storage. */
    printf("fold_alignments=%d %d %d %d\n", (int)_Alignof(char), (int)_Alignof(short),
           (int)_Alignof(int), (int)_Alignof(float));
    printf("run_alignments=%d %d %d %d\n", align_char, align_short, align_int, align_float);

    /* Alignment relations: monotonic with rank, and never stricter than the
     * type's own size.  Both cover double and long long, whose alignments
     * differ per target, without naming a number. */
    printf("fold_align_bounds=%d %d\n",
           (int)(_Alignof(char) <= _Alignof(int) && _Alignof(int) <= _Alignof(double)),
           (int)(_Alignof(char) <= sizeof(char) && _Alignof(short) <= sizeof(short)
                 && _Alignof(int) <= sizeof(int) && _Alignof(float) <= sizeof(float)
                 && _Alignof(double) <= sizeof(double)
                 && _Alignof(long long) <= sizeof(long long)));
    printf("run_align_bounds=%d %d\n",
           (align_char <= align_int) && (align_int <= align_double),
           (align_char <= stride_char) && (align_short <= stride_short)
               && (align_int <= stride_int) && (align_float <= stride_float)
               && (align_double <= stride_double) && (align_llong <= stride_llong));

    /* The folded constant expressions, then the same values computed from
     * volatile operands: a multiply-and-add and a shift pair. */
    printf("fold_arith=%d %d\n", FOLD_SUM, FOLD_SHIFT);
    printf("run_arith=%d %d\n", two + three * four, (one << ten) >> shift_four);
    printf("fold_wide_arith=%lld\n", FOLD_WIDE);
    printf("run_wide_arith=%lld\n", wide / divisor);

    /* The character constant, then the same value read back from an explicitly
     * signed char object. */
    printf("fold_char_a=%d\n", (int)'A');
    printf("run_char_a=%d\n", (int)letter);

    /* Enumerator values, folded then loaded. */
    printf("fold_enumerators=%d %d %d %d\n", LIMIT_ZERO, LIMIT_TWO, LIMIT_GAP, LIMIT_NEG);
    printf("run_enumerators=%d %d %d %d\n", e_zero, e_two, e_gap, e_neg);

    /* The array bound, folded from sizeof and then measured as the whole span
     * divided by the measured element stride.  The guard makes the division
     * unreachable with a non-positive divisor, so no divide-by-zero is possible
     * even if a measurement were to fail; a failure prints -1 rather than
     * trapping, which keeps the divergence observable as output. */
    printf("fold_table_len=%d\n", (int)(sizeof table / sizeof table[0]));
    printf("run_table_len=%d\n",
           (table_span > 0 && stride_int > 0) ? table_span / stride_int : -1);

    /* String literal sizes, then the terminator located at run time.  The
     * printed length is the scan offset plus one for the terminator itself, so
     * a literal emitted without its terminator would print 5 rather than 4. */
    printf("fold_string_sizes=%d %d %d\n", (int)sizeof msg, (int)sizeof "abc", (int)sizeof "");
    printf("run_string_size=%d\n", nul_offset(msg, (int)sizeof msg) + 1);

    /* Aggregate sizes, folded then measured, and then the three aggregate
     * bounds the assertions above stated. */
    printf("fold_aggregate_sizes=%d %d\n", (int)sizeof(struct triple),
           (int)sizeof(union scalar_bytes));
    printf("run_aggregate_strides=%d %d\n", stride_triple, stride_union);
    printf("fold_aggregate_bounds=%d %d %d\n",
           (int)(sizeof(struct triple) >= sizeof(int) + sizeof(short) + sizeof(unsigned char)),
           (int)(sizeof(union scalar_bytes) >= sizeof(long long)),
           (int)(_Alignof(struct triple) == _Alignof(int)));
    printf("run_aggregate_bounds=%d %d %d\n",
           stride_triple >= stride_int + stride_short + 1, stride_union >= stride_llong,
           align_triple == align_int);

    /* The locally declared type: its two asserted properties and the sum of its
     * initialized members, each reported folded and then at run time. */
    printf("fold_local_type=%d %d %d\n", local.fold_size_ok, local.fold_align_ok,
           local.fold_sum);
    printf("run_local_type=%d %d %d\n", local.run_size_ok, local.run_align_ok, local.run_sum);
    return 0;
}
