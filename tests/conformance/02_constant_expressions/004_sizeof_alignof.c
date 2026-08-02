/* Area 02, program 4 of 8 -- sizeof and _Alignof as constant expressions, over
 * scalars and over aggregates, width-normalized so that all four backends stay
 * comparable.
 *
 * THE HAZARD THIS PROGRAM IS BUILT AROUND.  sizeof and _Alignof are the two
 * places in the language where a target's ABI becomes an observable integer, so a
 * program that prints them naively prints different bytes on different targets and
 * destroys the four-way comparison it exists to perform.  The quantities that
 * differ, as measured with the four reference drivers oracle (a) uses:
 *
 *     quantity                x86_64   i686   aarch64   riscv64
 *     sizeof(long)                 8      4         8         8
 *     sizeof(void *)               8      4         8         8
 *     size_t width                 8      4         8         8
 *     ptrdiff_t width              8      4         8         8
 *     sizeof(long double)         16     12        16        16
 *     _Alignof(long)               8      4         8         8
 *     _Alignof(long long)          8      4         8         8
 *     _Alignof(double)             8      4         8         8
 *     _Alignof(long double)       16      4        16        16
 *
 * The first four rows are not incidental: the repository's target table fixes
 * pointer and long at 8 bytes with ELF64 on x86-64, AArch64 and RISC-V 64, and at
 * 4 bytes with ELF32 on i686 (docs/technical-specifications.md lines 457-462).
 * i686 is the only ILP32 backend, which is exactly why it has to stay in the
 * comparison.  Note also that _Alignof(double) is 4 on i686, because _Alignof
 * reports the alignment the i386 System V ABI requires rather than a preferred
 * one; the two must not be conflated.
 *
 * Restricting this program to the three 64-bit targets and printing raw numbers
 * would be the easy answer, and it is forbidden: constraint C3 excludes no feature
 * for difficulty, and dropping i686 would remove three of this program's twelve
 * cells and with them the only ILP32 backend -- the only target on which any of the
 * widths tabulated above actually differs, so exactly where a width defect could
 * hide with nothing left to notice it.  All four targets, all three optimization
 * levels and all three oracles are therefore kept, with no expected-divergence
 * marker, and the output is normalized instead.
 *
 * HOW THE OUTPUT IS NORMALIZED.  A quantity identical on all four targets is
 * printed RAW rather than as a predicate, so a target that disagreed diverges on
 * that line instead of being accepted by a predicate that held for the wrong
 * reason; every raw value below was measured on all four targets first.  A quantity
 * that differs by target is printed only as a RELATION, which yields the same 0 or
 * 1 everywhere.  Relations alone would not be sufficient, and this is the trap
 * worth stating plainly: a relation is satisfied by two values wrong in the same
 * direction, so sizeof(long) == sizeof(void *) prints 1 even if both were 2.  The
 * pointer-width family is therefore anchored to ONE exact, target-keyed equality --
 * ptr_width_exact -- with every other member tied to that anchor by a relation, so
 * one number per target pins the family exactly.  A permissive bound is never used
 * where an exact answer exists, because a range such as "4 <= x <= 8" accepts 5, 6
 * and 7 and accepts the wrong choice between 4 and 8.
 *
 * long double is the one type given bounds rather than an exact pin, because no
 * exact answer is available to assert: its representation is implementation-defined
 * -- 16 bytes on x86-64 and 12 on i686 for the same x87 80-bit format differently
 * padded, 16 on AArch64 and RISC-V 64 for IEEE binary128 -- and C11 permits it to
 * have exactly double's representation.  Its raw size is never printed; what is
 * printed are the three facts that hold everywhere: it is at least as wide as
 * double, at least as strictly aligned as double, and its size is a whole multiple
 * of its alignment.  Cross-backend VALUE equality for long double belongs to
 * 13_floating_point/004_long_double_target_restricted.c, where it is a recorded
 * exclusion; nothing is excluded here, because nothing representation-dependent is
 * printed.
 *
 * sizeof and _Alignof are always compile-time constants, so every derived
 * arithmetic result is computed twice: once from constant operands, and once from
 * the same quantities parked in volatile objects, which must be re-read from memory
 * so the backend has to emit the division, the remainder and the pointer scaling
 * itself.
 *
 * No header is named and printf is hand-declared.  bcc ships no stdio.h: its
 * bundled set is the nine required freestanding headers plus a bonus stdatomic.h,
 * ten files in all (docs/project-guide.md line 212).  _Alignof is a C11 keyword and
 * needs none; stdalign.h would only supply the lowercase alignof macro.  stddef.h
 * and stdint.h are absent too, so size_t, ptrdiff_t and intptr_t cannot be named
 * here at all -- their widths are probed without naming them, since sizeof yields a
 * size_t (making sizeof(sizeof(int)) size_t's own width) and a pointer difference
 * yields a ptrdiff_t.
 *
 * Nothing implementation-defined reaches stdout: no address, no pointer value, no
 * plain long and no plain-char signedness-dependent value is printed, every
 * character read back is converted through unsigned char first, and pointer facts
 * appear only as differences and comparisons.  The four lines that do print a
 * character's numeric CODE are a separate matter, because the unsigned char cast
 * settles signedness and says nothing about the execution character set: every such
 * code is pinned by a _Static_assert further down this file, so an implementation
 * whose character set disagreed would fail to translate rather than print a
 * different number for oracle (a) to charge to the compiler under test.
 *
 * Freedom from undefined behaviour: no signed overflow, no shift, no aliasing
 * violation, no read of uninitialized storage, and no dependence on padding bytes,
 * which are never read and whose offsets are never assumed.  The one-past-end
 * pointer bounding the array walk is formed but never dereferenced.  Each
 * expression reads each volatile object at most once, so nothing is modified twice
 * between sequence points and no call takes more than one side-effecting argument.
 */

int printf(const char *, ...);

/* The single target-keyed number this program needs, keyed to the ARCHITECTURE
 * rather than to a width macro such as __SIZEOF_POINTER__ on purpose: the
 * architecture is a categorical fact the harness fixes when it selects the cell's
 * target, whereas a width macro is a numeric claim by the compiler under test, and
 * checking that claim against itself would be circular.
 *
 * The #else is a hard stop rather than a fallback: a compiler predefining none of
 * these macros cannot be given an exact expectation, and quietly falling back to a
 * permissive one would turn rule 4 above into a comment. */
#if defined(__x86_64__) || defined(__amd64__)
#define EXPECTED_PTR_WIDTH 8u
#elif defined(__i386__) || defined(__i386)
#define EXPECTED_PTR_WIDTH 4u
#elif defined(__aarch64__) || defined(__arm64__)
#define EXPECTED_PTR_WIDTH 8u
#elif defined(__riscv) || defined(__riscv__)
#define EXPECTED_PTR_WIDTH 8u
#else
#error "004_sizeof_alignof: no pointer-width expectation selected; none of __x86_64__ / __i386__ / __aarch64__ / __riscv was predefined"
#endif

/* Element counts, spelled once so the same number drives the declaration, the
 * folded constant expression and the runtime twin.  Unsigned suffixes keep every
 * comparison against a sizeof result unsigned on both sides, because -Wextra
 * enables -Wsign-compare and the audit gate makes it fatal. */
#define ARR_ELEMS 7u
#define MAT_ROWS  3u
#define MAT_COLS  5u
#define TAB_ELEMS 4u

/* An enumeration, whose size and alignment were measured as 4 and 4 on all four
 * targets, so both may be printed raw. */
enum color { COLOR_RED = 1, COLOR_GREEN = 2, COLOR_BLUE = 4 };

/* An aggregate built only from members whose width is identical on all four
 * targets.  Measured at 12 bytes with 4-byte alignment everywhere, so its size
 * and alignment are printed raw -- an exact expectation rather than a bound. */
struct invariant {
    char  c;
    short s;
    int   i;
    float f;
};

/* A union of invariant-width members.  Measured at 8 bytes with 4-byte
 * alignment everywhere, so these too are printed raw. */
union invariant_union {
    int   i;
    float f;
    char  bytes[6];
};

/* An aggregate that deliberately carries the hazard: a pointer and a long.  Its
 * size is 24 bytes on the three LP64 targets and 12 on i686, and its alignment
 * is 8 and 4, so NEITHER may be printed raw.  Every fact about it below is a
 * relation, and because the pointer width is pinned exactly by the anchor, the
 * relation sizeof == 3 * sizeof(void *) pins this layout exactly too. */
struct widthful {
    char  c;
    void *p;
    long  l;
};

/* A nested aggregate, so the universal layout laws are exercised through one
 * more level of composition.  Measured at 48/28/48/48 bytes, hence relations
 * only. */
struct nested {
    struct invariant head;
    struct widthful  tail;
    char             trailer;
};

/* The corpus objects.  Every one of them is genuinely read at run time further
 * down, which matters for more than tidiness: a static object used only inside
 * an unevaluated sizeof operand is diagnosed by at least one reference compiler
 * (clang's -Wunneeded-internal-declaration), and the audit gate makes any such
 * diagnostic fatal.  Giving each object a real read keeps the program clean
 * under every reference compiler the harness may be pointed at, and it also
 * proves the declarations describe real storage rather than types alone. */
static int   arr[ARR_ELEMS] = { 2, 3, 5, 7, 11, 13, 17 };
static short mat[MAT_ROWS][MAT_COLS] = {
    {  1,  2,  3,  4,  5 },
    {  6,  7,  8,  9, 10 },
    { 11, 12, 13, 14, 15 }
};
static const char *const tab[TAB_ELEMS] = { "a", "bb", "ccc", "dddd" };

/* The carrier for the header-free ptrdiff_t width probe.  Two uses are made of
 * it and both are safe: inside sizeof, where the operand is never evaluated at
 * all and the difference is purely a type question, and once at the end of the
 * report, where the difference really is computed.  Either way only in-range
 * element addresses are formed, no object is read through them, and no address
 * is printed -- the difference itself is, which is one of the two shapes in
 * which a pointer fact may appear at all. */
static int probe_pair[2] = { 0, 0 };

/* sizeof and _Alignof used where an integer CONSTANT expression is required,
 * which is the property this area exists to test.  An array bound at file scope
 * is the strictest such context of all: a non-constant bound there is not merely
 * a variable-length array but a diagnosable error, so these two declarations
 * compile only because sizeof and _Alignof are constant expressions. */
static char bound_from_sizeof[sizeof(int)];
static char bound_from_alignof[_Alignof(int)];

/* An enumerator initializer is a second constant-expression context, and the
 * cast keeps the enumerator's value in int as C requires. */
enum sizeof_enumerators { SIZEOF_INT_AS_ENUMERATOR = (int)sizeof(int) };

static const struct invariant      inv_obj  = { 'x', 300, 70000, 1.5f };
static const union invariant_union un_obj   = { 42 };
static const struct widthful       wide_obj = { 'y', 0, 0 };
static const struct nested         nest_obj = { { 'z', 7, 9, 0.5f }, { 'w', 0, 0 }, 'T' };

/* The observable that proves sizeof does not evaluate its operand.  The counter
 * is volatile so the observation cannot be optimized away: the compiler must
 * store to it inside probe() and must re-read it at each check below, which is
 * what makes "the call did not happen" a fact about the generated program rather
 * than about the optimizer's mood. */
static volatile int probe_calls = 0;

static int probe(void)
{
    probe_calls = probe_calls + 1;
    return 1;
}

/* Compile-time corroboration of the claims the report prints at run time.
 *
 * Only claims that hold on all four targets are asserted.  A _Static_assert on a
 * target-varying property would break the build on the target that disagreed and
 * destroy the four-way comparison, and the same reasoning keeps the exact
 * pointer-width anchor out of this list: as a printed predicate a wrong width
 * costs one divergent line and leaves the remaining sixty-two readable, whereas
 * as a static assertion it would cost the whole translation unit.  Every assertion
 * carries a message string, because the message-less one-argument form is C23
 * and -pedantic rejects it. */
_Static_assert(sizeof(char) == 1u, "sizeof(char) is 1 by definition");
_Static_assert(sizeof(short) == 2u, "short is two bytes on every supported target");
_Static_assert(sizeof(int) == 4u, "int is four bytes on every supported target");
_Static_assert(sizeof(long long) == 8u,
               "long long is eight bytes on every supported target");
_Static_assert(sizeof(float) == 4u, "float is IEEE binary32 on every supported target");
_Static_assert(sizeof(double) == 8u, "double is IEEE binary64 on every supported target");
_Static_assert(_Alignof(char) == 1u, "char alignment is 1 by definition");
_Static_assert(_Alignof(int) == 4u,
               "int alignment matches its size on every supported target");
_Static_assert(sizeof(long) == sizeof(void *),
               "long is pointer width on every supported target");
_Static_assert(sizeof(void *) == sizeof(char *),
               "all object pointers share one width");
_Static_assert(sizeof(void *) == sizeof(void (*)(void)),
               "function pointers share the object pointer width on these ABIs");
_Static_assert(_Alignof(void *) == sizeof(void *),
               "a pointer is aligned to its own width");
_Static_assert(sizeof(sizeof(int)) == sizeof(void *),
               "size_t is pointer width, probed without naming size_t");
_Static_assert(sizeof(&probe_pair[1] - &probe_pair[0]) == sizeof(void *),
               "ptrdiff_t is pointer width, probed without naming ptrdiff_t");
_Static_assert(sizeof "abc" == 4u, "a string literal carries its terminator");
_Static_assert(sizeof('a') == sizeof(int), "a character constant has type int in C");
_Static_assert(sizeof(1 + 1) == sizeof(int), "an int expression has int size");
_Static_assert(sizeof(long double) >= sizeof(double),
               "long double is at least as wide as double");
_Static_assert(_Alignof(long double) >= _Alignof(double),
               "long double is at least as strictly aligned as double");
_Static_assert(sizeof(struct invariant) % _Alignof(struct invariant) == 0u,
               "an aggregate size is a whole multiple of its alignment");
_Static_assert(sizeof(struct widthful) % _Alignof(struct widthful) == 0u,
               "the layout law holds for the width-bearing aggregate too");

/* THE EXECUTION CHARACTER SET, PINNED RATHER THAN ASSUMED.
 *
 * Four of the printed lines carry a character's numeric code: the member
 * read-backs print inv_obj.c, wide_obj.c and nest_obj.trailer, and
 * ptrtab_first_chars prints the first byte of each string-table entry.  Those
 * codes exist because a read-back has to print SOMETHING, and a character
 * member is the narrowest way to prove that the byte the initializer placed at
 * a computed offset is the byte that comes back out.  Casting through unsigned
 * char removes plain char's implementation-defined SIGNEDNESS, which was
 * measured to differ between the targets, but it does nothing whatever about
 * the execution character set: C11 5.2.1 leaves the members and codes of that
 * set implementation-defined, so 'x' is 120 on an ASCII-family implementation
 * and something else on one that is not.
 *
 * The assertions below convert that from an unstated assumption into a
 * translation-time requirement.  Every character whose code this program prints
 * is pinned here, together with the two that appear only in initializers, so an
 * implementation whose execution character set differs FAILS TO TRANSLATE with a
 * named diagnostic instead of quietly printing different numbers that oracle (a)
 * or oracle (b) would then charge to the compiler under test.  A build failure
 * accuses the environment, which is where the difference actually lives; a
 * silent numeric change accuses the wrong party.
 *
 * These are the correct instrument for this job precisely BECAUSE they are
 * target-parametric in the same way the rest of the file's assertions are: all
 * four supported targets were measured to use the same ASCII-family basic
 * execution character set, so the condition holds on every enabled cell today,
 * and a future target that disagreed would announce itself at the build rather
 * than through a divergence report.  The record's impl_defined_notes carries the
 * same accounting in prose. */
_Static_assert('x' == 120, "the execution character set places x at 120 (ASCII)");
_Static_assert('y' == 121, "the execution character set places y at 121 (ASCII)");
_Static_assert('z' == 122, "the execution character set places z at 122 (ASCII)");
_Static_assert('w' == 119, "the execution character set places w at 119 (ASCII)");
_Static_assert('T' == 84, "the execution character set places T at 84 (ASCII)");
_Static_assert('a' == 97 && 'b' == 98 && 'c' == 99 && 'd' == 100,
               "the four string-table initials sit at their ASCII codes");

int main(void)
{
    /* --- the folded half of the two-variant rule -------------------------- *
     * Every operand here is a constant expression, so these five values are
     * produced by the constant evaluator and the backend emits only the store. */
    unsigned int fold_elems = (unsigned int)(sizeof arr / sizeof arr[0]);
    unsigned int fold_cells = (unsigned int)(sizeof mat / sizeof mat[0][0]);
    unsigned int fold_rows  = (unsigned int)(sizeof mat / sizeof mat[0]);
    unsigned int fold_ratio = (unsigned int)(sizeof(double) / sizeof(float));
    int          fold_law   =
        (int)(sizeof(struct widthful) % _Alignof(struct widthful) == 0u);

    /* --- the runtime half ------------------------------------------------- *
     * The same quantities, each parked in a volatile object first.  A volatile
     * object must be re-read from memory at every use, so the divisions and the
     * remainder below cannot be folded and the backend has to compute them.  The
     * narrowing to unsigned int is explicit so the audit gate's -Wconversion has
     * nothing to object to, and it is safe because every value is a small byte
     * count.  Nothing writes to these objects after initialization, so each
     * divisor is a fixed non-zero value and no division by zero is possible. */
    volatile unsigned int v_arr_bytes  = (unsigned int)sizeof arr;
    volatile unsigned int v_elem_bytes = (unsigned int)sizeof arr[0];
    volatile unsigned int v_mat_bytes  = (unsigned int)sizeof mat;
    volatile unsigned int v_row_bytes  = (unsigned int)sizeof mat[0];
    volatile unsigned int v_cell_bytes = (unsigned int)sizeof mat[0][0];
    volatile unsigned int v_dbl_bytes  = (unsigned int)sizeof(double);
    volatile unsigned int v_flt_bytes  = (unsigned int)sizeof(float);
    volatile unsigned int v_wide_bytes = (unsigned int)sizeof(struct widthful);
    volatile unsigned int v_wide_align = (unsigned int)_Alignof(struct widthful);
    volatile int          v_mat_rows   = (int)MAT_ROWS;
    volatile int          v_mat_cols   = (int)MAT_COLS;
    volatile int          v_stride     = 1;

    unsigned int run_elems = v_arr_bytes / v_elem_bytes;
    unsigned int run_cells = v_mat_bytes / v_cell_bytes;
    unsigned int run_rows  = v_mat_bytes / v_row_bytes;
    unsigned int run_ratio = v_dbl_bytes / v_flt_bytes;
    int          run_law   = (int)(v_wide_bytes % v_wide_align == 0u);

    /* The pointer walk: a third, independent way to arrive at the element count.
     * `stop` is a one-past-the-end pointer, which C permits forming and this
     * program never dereferences -- the loop tests against it and exits before
     * any read through it.  The stride comes from a volatile object, so the
     * scaling by the element size happens at run time. */
    int *walk = arr;
    int *stop = arr + ARR_ELEMS;
    int  walked   = 0;
    int  walk_sum = 0;

    int row;
    int col;
    int mat_sum = 0;

    int sizeof_call_is_int;
    int operand_unevaluated;
    int probe_called_once;
    int probe_ret;

    int all_equal;

    /* Compute everything first, then report.  Keeping the two phases apart is
     * what makes the ordered observations below trustworthy: the probe's three
     * checks depend on happening in sequence, and burying them among printf
     * arguments would make that sequence a matter of reading comprehension. */
    for (row = 0; row < v_mat_rows; row = row + 1) {
        for (col = 0; col < v_mat_cols; col = col + 1) {
            mat_sum = mat_sum + (int)mat[row][col];
        }
    }

    while (walk != stop) {
        walk_sum = walk_sum + *walk;
        walk     = walk + v_stride;
        walked   = walked + 1;
    }

    /* sizeof does not evaluate its operand, which is the defining property of a
     * compile-time operator and is therefore squarely this area's business.  The
     * three observations must happen in this order: take the size of a call
     * expression, confirm the call did not happen, then make the call for real
     * and confirm the counter can move.  The third check is what stops the
     * second from passing for the wrong reason -- a counter that never moved at
     * all would satisfy "still zero" while proving nothing. */
    sizeof_call_is_int  = (int)(sizeof(probe()) == sizeof(int));
    operand_unevaluated = (int)(probe_calls == 0);
    probe_ret           = probe();
    probe_called_once   = (int)(probe_calls == 1);

    /* Write into the two constant-expression-bounded arrays and read them back,
     * so their declared bounds describe storage that is genuinely used.  The
     * index is the last element of each, computed from the array's own size. */
    bound_from_sizeof[sizeof bound_from_sizeof - 1u]   = (char)5;
    bound_from_alignof[sizeof bound_from_alignof - 1u] = (char)6;

    all_equal = (fold_elems == run_elems) && (fold_cells == run_cells)
             && (fold_rows == run_rows) && (fold_ratio == run_ratio)
             && (fold_law == run_law)
             && (fold_elems == (unsigned int)walked);

    /* --- report: invariant scalar sizes and alignments, printed raw -------- */
    printf("sizeof_char_short_int_llong=%d %d %d %d\n",
           (int)sizeof(char), (int)sizeof(short),
           (int)sizeof(int), (int)sizeof(long long));
    printf("sizeof_float_double=%d %d\n", (int)sizeof(float), (int)sizeof(double));
    printf("sizeof_bool_enum=%d %d\n", (int)sizeof(_Bool), (int)sizeof(enum color));
    printf("alignof_char_short_int_float=%d %d %d %d\n",
           (int)_Alignof(char), (int)_Alignof(short),
           (int)_Alignof(int), (int)_Alignof(float));
    printf("alignof_bool_enum=%d %d\n",
           (int)_Alignof(_Bool), (int)_Alignof(enum color));

    /* --- the exact anchor, then the family tied to it --------------------- *
     * One exact equality against the target's own pointer width, followed by the
     * relations that make every other pointer-width quantity exact as well.
     * Digit order on the family line: long, char *, int *, function pointer,
     * pointer to array, pointer to pointer. */
    printf("ptr_width_exact=%d\n", (int)(sizeof(void *) == EXPECTED_PTR_WIDTH));
    printf("ptr_family_is_ptr_width=%d %d %d %d %d %d\n",
           (int)(sizeof(long) == sizeof(void *)),
           (int)(sizeof(char *) == sizeof(void *)),
           (int)(sizeof(int *) == sizeof(void *)),
           (int)(sizeof(void (*)(void)) == sizeof(void *)),
           (int)(sizeof(int (*)[10]) == sizeof(void *)),
           (int)(sizeof(void **) == sizeof(void *)));
    /* size_t and ptrdiff_t are probed without being named, because the headers
     * that declare them are not available here.  Digit order: size_t against the
     * pointer width, ptrdiff_t against the pointer width, and the two against
     * each other. */
    printf("stddef_widths_are_ptr_width=%d %d %d\n",
           (int)(sizeof(sizeof(int)) == sizeof(void *)),
           (int)(sizeof(&probe_pair[1] - &probe_pair[0]) == sizeof(void *)),
           (int)(sizeof(sizeof(int)) == sizeof(&probe_pair[1] - &probe_pair[0])));
    printf("alignof_ptr_is_ptr_width=%d\n",
           (int)(_Alignof(void *) == sizeof(void *)));
    /* The three alignments that vary by target, each tied to the pointer
     * alignment, which the anchor has already pinned.  Digit order: long, long
     * long, double.  Note that _Alignof(long long) and _Alignof(double) are 4 on
     * i686 and 8 elsewhere, so tying them to the pointer alignment is exactly
     * right, whereas tying them to their own size would print 0 on i686. */
    printf("alignof_wide_is_ptr_align=%d %d %d\n",
           (int)(_Alignof(long) == _Alignof(void *)),
           (int)(_Alignof(long long) == _Alignof(void *)),
           (int)(_Alignof(double) == _Alignof(void *)));

    /* --- universal ordering laws ------------------------------------------ *
     * Digit order on the ladder: char <= short, short <= int, int <= long,
     * long <= long long, float <= double, double <= long double. */
    printf("size_ladder=%d %d %d %d %d %d\n",
           (int)(sizeof(char) <= sizeof(short)),
           (int)(sizeof(short) <= sizeof(int)),
           (int)(sizeof(int) <= sizeof(long)),
           (int)(sizeof(long) <= sizeof(long long)),
           (int)(sizeof(float) <= sizeof(double)),
           (int)(sizeof(double) <= sizeof(long double)));
    /* Alignment never exceeds size.  Digit order: char, short, int, long, long
     * long, float, double, long double, void *. */
    printf("align_le_size_scalars=%d %d %d %d %d %d %d %d %d\n",
           (int)(_Alignof(char) <= sizeof(char)),
           (int)(_Alignof(short) <= sizeof(short)),
           (int)(_Alignof(int) <= sizeof(int)),
           (int)(_Alignof(long) <= sizeof(long)),
           (int)(_Alignof(long long) <= sizeof(long long)),
           (int)(_Alignof(float) <= sizeof(float)),
           (int)(_Alignof(double) <= sizeof(double)),
           (int)(_Alignof(long double) <= sizeof(long double)),
           (int)(_Alignof(void *) <= sizeof(void *)));
    /* The same law for the aggregates.  Digit order: invariant struct, invariant
     * union, width-bearing struct, nested struct, enumeration. */
    printf("align_le_size_aggregates=%d %d %d %d %d\n",
           (int)(_Alignof(struct invariant) <= sizeof(struct invariant)),
           (int)(_Alignof(union invariant_union) <= sizeof(union invariant_union)),
           (int)(_Alignof(struct widthful) <= sizeof(struct widthful)),
           (int)(_Alignof(struct nested) <= sizeof(struct nested)),
           (int)(_Alignof(enum color) <= sizeof(enum color)));

    /* --- long double: bounds and the layout law, never a representation --- */
    printf("ldouble_ge_double=%d\n", (int)(sizeof(long double) >= sizeof(double)));
    printf("alignof_ldouble_ge_alignof_double=%d\n",
           (int)(_Alignof(long double) >= _Alignof(double)));
    printf("ldouble_size_multiple_of_align=%d\n",
           (int)(sizeof(long double) % _Alignof(long double) == 0u));

    /* --- aggregates ------------------------------------------------------- *
     * The two aggregates built from invariant-width members are printed raw,
     * because 12/4 and 8/4 were measured on all four targets; the two that carry
     * a pointer or a long are printed only as relations. */
    printf("sizeof_struct_invariant=%d\n", (int)sizeof(struct invariant));
    printf("alignof_struct_invariant=%d\n", (int)_Alignof(struct invariant));
    printf("sizeof_union_invariant=%d\n", (int)sizeof(union invariant_union));
    printf("alignof_union_invariant=%d\n", (int)_Alignof(union invariant_union));
    /* sizeof does not evaluate its operand, so naming the union's inactive
     * member below asks about its type and never reads it. */
    printf("union_ge_largest_member=%d\n",
           (int)(sizeof(union invariant_union) >= sizeof un_obj.bytes));
    printf("union_align_ge_alignof_int=%d\n",
           (int)(_Alignof(union invariant_union) >= _Alignof(int)));
    printf("union_readback=%d\n", (int)(un_obj.i == 42));
    /* The char member is read back through unsigned char so its value cannot
     * depend on plain char's signedness, which was measured to differ between
     * the targets.  The float member is compared rather than printed, and 1.5 is
     * exactly representable, so the comparison is exact. */
    printf("struct_invariant_readback=%d %d %d %d\n",
           (int)(unsigned char)inv_obj.c, (int)inv_obj.s, inv_obj.i,
           (int)(inv_obj.f == 1.5f));
    printf("structp_size_multiple_of_align=%d\n",
           (int)(sizeof(struct widthful) % _Alignof(struct widthful) == 0u));
    printf("structp_size_ge_ptr_size=%d\n",
           (int)(sizeof(struct widthful) >= sizeof(void *)));
    printf("structp_align_is_ptr_align=%d\n",
           (int)(_Alignof(struct widthful) == _Alignof(void *)));
    /* Three pointer widths exactly: 24 bytes on the LP64 targets and 12 on i686.
     * Because the anchor pins the pointer width, this relation pins the whole
     * layout -- including the padding the compiler must insert after the leading
     * char -- without printing a byte count that varies. */
    printf("structp_size_is_three_ptr_widths=%d\n",
           (int)(sizeof(struct widthful) == 3u * sizeof(void *)));
    printf("structp_readback=%d %d\n",
           (int)(unsigned char)wide_obj.c, (int)(wide_obj.p == 0));
    printf("nested_size_multiple_of_align=%d\n",
           (int)(sizeof(struct nested) % _Alignof(struct nested) == 0u));
    printf("nested_align_is_widest_member_align=%d\n",
           (int)(_Alignof(struct nested) == _Alignof(struct widthful)));
    printf("nested_size_ge_member_sum=%d\n",
           (int)(sizeof(struct nested)
                 >= sizeof(struct invariant) + sizeof(struct widthful)
                    + sizeof(char)));
    printf("nested_readback=%d %d\n",
           nest_obj.head.i, (int)(unsigned char)nest_obj.trailer);
    printf("struct_array_size_is_count_times_elem=%d\n",
           (int)(sizeof(struct invariant[3]) == 3u * sizeof(struct invariant)));

    /* --- sizeof and _Alignof in constant-expression contexts -------------- *
     * Digit order: an array bound taken from sizeof, an array bound taken from
     * _Alignof, and an enumerator initialized from sizeof.  All three are 4 on
     * every target because they derive from int. */
    printf("constexpr_contexts=%d %d %d\n",
           (int)sizeof bound_from_sizeof, (int)sizeof bound_from_alignof,
           (int)SIZEOF_INT_AS_ENUMERATOR);
    /* Digit order: the untouched first element of each array (static storage is
     * zero-initialized, so reading it is defined), then the last element of each,
     * which the assignments above set to 5 and 6. */
    printf("constexpr_bound_readback=%d %d %d %d\n",
           (int)bound_from_sizeof[0], (int)bound_from_alignof[0],
           (int)bound_from_sizeof[sizeof bound_from_sizeof - 1u],
           (int)bound_from_alignof[sizeof bound_from_alignof - 1u]);
    /* An array type's alignment is its element's, and its size is the product.
     * Digit order: the alignment law, then the size law. */
    printf("array_type_laws=%d %d\n",
           (int)(_Alignof(int[10]) == _Alignof(int)),
           (int)(sizeof(int[10]) == 10u * sizeof(int)));

    /* --- element counts and array laws ------------------------------------ *
     * A count and a ratio are safe to print raw even when the operands are not,
     * because dividing one size by another cancels the width entirely.  That is
     * why the pointer table's element count is printable while its byte size is
     * not. */
    printf("arr_elems=%d\n", (int)(sizeof arr / sizeof arr[0]));
    printf("arr_size=%d\n", (int)sizeof arr);
    printf("arr_size_is_count_times_elem=%d\n",
           (int)(sizeof arr == ARR_ELEMS * sizeof arr[0]));
    printf("mat_rows_cols_cells=%d %d %d\n",
           (int)(sizeof mat / sizeof mat[0]),
           (int)(sizeof mat[0] / sizeof mat[0][0]),
           (int)(sizeof mat / sizeof mat[0][0]));
    printf("mat_size=%d\n", (int)sizeof mat);
    /* Digit order: the whole matrix against its row size, then against its cell
     * size. */
    printf("mat_size_laws=%d %d\n",
           (int)(sizeof mat == MAT_ROWS * sizeof mat[0]),
           (int)(sizeof mat == MAT_ROWS * MAT_COLS * sizeof mat[0][0]));
    printf("mat_sum=%d\n", mat_sum);
    /* The first character of each table entry, each converted through unsigned
     * char, which also gives the pointer table a genuine run-time read. */
    printf("ptrtab_first_chars=%d %d %d %d\n",
           (int)(unsigned char)tab[0][0], (int)(unsigned char)tab[1][0],
           (int)(unsigned char)tab[2][0], (int)(unsigned char)tab[3][0]);
    printf("ptrtab_elems=%d\n", (int)(sizeof tab / sizeof tab[0]));
    printf("ptrtab_size_is_count_times_ptr=%d\n",
           (int)(sizeof tab == TAB_ELEMS * sizeof(void *)));

    /* --- sizeof applied to expressions and to literals -------------------- */
    printf("sizeof_string_literal=%d\n", (int)sizeof "abc");
    printf("sizeof_string_concat=%d\n", (int)sizeof("abc" "def"));
    /* Digit order: a character constant, an int expression, two shorts whose
     * promotion makes the result an int, and a call expression whose type is the
     * function's return type.  Each is compared against sizeof(int) rather than
     * printed, so the line states the typing rule rather than a byte count. */
    printf("sizeof_expression_is_int=%d %d %d %d\n",
           (int)(sizeof('a') == sizeof(int)),
           (int)(sizeof(1 + 1) == sizeof(int)),
           (int)(sizeof(mat[0][0] + mat[0][1]) == sizeof(int)),
           sizeof_call_is_int);
    printf("sizeof_operand_not_evaluated=%d\n", operand_unevaluated);
    printf("probe_calls_after_real_call=%d\n", probe_called_once);
    printf("probe_return_value=%d\n", probe_ret);

    /* --- the two-variant block -------------------------------------------- *
     * Each line states the folded value, the value the backend computed from
     * volatile operands, and whether they agree.  A divergence on the folded
     * column implicates the constant evaluator, on the runtime column the
     * backend, and on both the quantity itself. */
    printf("elems folded=%d runtime=%d equal=%d\n",
           (int)fold_elems, (int)run_elems, (int)(fold_elems == run_elems));
    printf("cells folded=%d runtime=%d equal=%d\n",
           (int)fold_cells, (int)run_cells, (int)(fold_cells == run_cells));
    printf("rows folded=%d runtime=%d equal=%d\n",
           (int)fold_rows, (int)run_rows, (int)(fold_rows == run_rows));
    printf("ratio folded=%d runtime=%d equal=%d\n",
           (int)fold_ratio, (int)run_ratio, (int)(fold_ratio == run_ratio));
    printf("alignlaw folded=%d runtime=%d equal=%d\n",
           fold_law, run_law, (int)(fold_law == run_law));
    printf("walk folded=%d runtime=%d equal=%d\n",
           (int)fold_elems, walked, (int)(fold_elems == (unsigned int)walked));
    printf("walk_sum=%d\n", walk_sum);
    /* Two pointer differences, which is the only shape in which a pointer fact
     * may be printed: the span of the array in elements, and the distance
     * between two adjacent elements of the probe pair. */
    printf("span_elems=%d\n", (int)(stop - arr));
    printf("pair_difference=%d\n", (int)(&probe_pair[1] - &probe_pair[0]));
    printf("two_variant_all_equal=%d\n", all_equal);
    return 0;
}
