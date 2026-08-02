/* Implicit zero fill: every member and element left unmentioned by an
 * initializer, and every object of static duration declared without an
 * initializer at all, must read back as a definite zero (C11 6.7.9p19 and
 * 6.7.9p21).  This program therefore asserts an OMISSION rather than a store:
 * nothing below writes the zeros it prints, so a slot that comes back non-zero
 * means the compiler failed to supply a guarantee the source never spelled out.
 *
 * Nine distinct zero-fill situations are covered, each read back slot by slot:
 *   1. partial array                  g_arr[6]       = { 1, 2 }
 *   2. the { 0 } array idiom          g_all_zero[4]  = { 0 }
 *   3. the { 0 } struct idiom         g_zero_struct  = { 0 }        (nested too)
 *   4. designated partial struct      g_part         = { .tag = 1 }
 *   5. designated nested member       g_part_mid     = { .tag, .in.p }
 *   6. partial array of struct        g_arr_st[3]    = { { 11, 12 } }
 *   7. partial two-dimensional array  g_matrix[2][3] = { { 1 } }
 *   8. no initializer, static         g_noinit[3]
 *   9. partial pointer array          g_ptrs[3]      = { "first" }
 * plus a partial array of unsigned, and automatic-duration twins of situations
 * 1, 2, 3, 4, 6 and 7.  The twins carry the most weight: a static object's zeros
 * are settled before program startup, whereas an automatic object's zeros must be
 * produced on every entry to the block.  A compiler can implement one correctly
 * and the other incorrectly, and only holding both forms side by side
 * distinguishes the two.
 *
 * ONE SPELLING IS EXCLUDED, and it is stated here rather than left to be noticed.
 * `struct outer v = { 1 };` -- a struct whose leading members are supplied
 * POSITIONALLY and whose trailing members are simply left off -- is well-defined
 * C11 and would exercise a different route through the initializer walker than
 * `{ .tag = 1 }` does, because the designated form repositions the cursor
 * explicitly whereas the positional form relies on the cursor stopping where the
 * list runs out.  It is nevertheless not written here.  -Wextra enables
 * -Wmissing-field-initializers and the authoring gate's -Werror makes it fatal,
 * and sixteen spellings were measured against that gate with the reference
 * compiler: the positional partial STRUCT is the only shape it rejects, and it
 * rejects every variant of that shape.  No admissible deviation exists either --
 * the gate's removable members are exactly -pedantic, for the GCC-extensions
 * area, and -Wconversion with -Wsign-conversion, for the deliberate narrowing
 * program, so -Wextra is not removable and area 03 is granted no deviation at
 * all.  A source-level diagnostic-suppression directive is not an alternative: it
 * neutralizes a gate member from inside the translation unit while the record
 * still claims the unchanged gate, which makes the record's own claim false
 * rather than making the program clean.  Constraint C3 asks for exactly what is
 * done instead -- the exclusion is narrow, it is scoped to one spelling, and its
 * reason is recorded here and in this program's expectation record.
 *
 * THE ZERO GUARANTEE ITSELF LOSES NOTHING to that exclusion, which is why it is
 * narrow.  The same measurement established that -Wmissing-field-initializers
 * does NOT fire for a designated initializer, nor for the `= { 0 }` idiom, nor
 * for a partial ARRAY, nor for a partial array-of-struct, nor for partial 2-D
 * rows -- so situations 1 through 9 above, which are the whole of this program's
 * mandate, are all written plainly and all remain under test.  Positional
 * exhaustion is itself still exercised: g_arr and l_arr are `int[6] = { 1, 2 }`,
 * which the gate accepts unaided because -Wmissing-field-initializers is about
 * members rather than elements, so the cursor-runs-out path is covered for
 * elements even though it is not covered for members.  The nearest gate-clean
 * relative of the excluded spelling, a designator followed by positional
 * continuation, is covered by 006_designated_mixed_nested.c, which spells
 * `{ 1, { .q = 3 }, 4 }` and `{ .tag = 11, { .p = 12, .q = 13 }, 14 }`.
 *
 * Portability and determinism.  Only int, unsigned and const char * appear, and
 * the pointers are never printed as values, so nothing depends on a width that
 * differs across the targets: on i686 a pointer, and the integer type that
 * matches it, are four bytes where the other three targets make them eight.
 * The two null elements of g_ptrs are reported as boolean equality relations,
 * never as addresses.  The one character read from a string literal is
 * normalized through (int)(unsigned char) so that the implementation-defined
 * signedness of plain char -- measured signed on x86-64 and i686, unsigned on
 * AArch64 and RISC-V 64 -- cannot reach the output.  Only named members are
 * read: the { 0 } idiom guarantees each member is zero and says nothing about
 * padding bytes, and this program never inspects a representation, so the
 * distinction cannot bite.  Iteration order is fixed and the output is a fixed
 * sequence of 75 key=value lines, one per property claimed.
 *
 * Freedom from undefined behaviour.  There is no arithmetic on the data and so no
 * overflow, no shift, no aliasing violation and no object modified after its
 * initialization; every subscript is strictly inside its array and no one-past-end
 * pointer is formed.  The one point worth checking first is that nothing here reads
 * uninitialized storage: g_noinit and the null pointer elements have STATIC storage
 * duration, so the standard defines their contents as zero rather than
 * indeterminate, which is precisely the guarantee under test.
 */

int printf(const char *, ...);

struct inner { int p; int q; };
struct outer { int tag; struct inner in; int trailer; };

/* Static duration.  These zeros are settled before program startup rather than
 * stored by any statement below, whatever route the implementation takes to supply
 * them; the wholly zero objects are g_all_zero, g_zero_struct and g_noinit.  Every
 * optimization level is swept because the values read back may not change however
 * the initialization is realized. */
static int          g_arr[6]       = { 1, 2 };
static int          g_all_zero[4]  = { 0 };
static struct outer g_zero_struct  = { 0 };
static struct outer g_part         = { .tag = 1 };
static struct outer g_part_mid     = { .tag = 1, .in.p = 2 };
static struct inner g_arr_st[3]    = { { 11, 12 } };
static int          g_matrix[2][3] = { { 1 } };
static int          g_noinit[3];
static const char  *g_ptrs[3]      = { "first" };
static unsigned     g_uarr[4]      = { 7u };

/* The one character this program reads out of a string literal is the 'f' of "first",
 * and C11 5.2.1 leaves the numeric values of the execution character set to the
 * implementation.  It is asserted rather than assumed, so an implementation with a
 * different execution character set fails to translate instead of printing a different
 * number that an oracle would have to attribute to a compiler defect.  All four
 * supported targets were measured to use ASCII. */
_Static_assert('f' == 102, "execution character set places 'f' at 102 (ASCII)");

int main(void)
{
    /* Automatic-duration twins of the static cases above.  The same guarantee
     * now has to be met by code the backend emits at run time. */
    int          l_arr[6]       = { 1, 2 };
    int          l_all_zero[4]  = { 0 };
    struct outer l_zero_struct  = { 0 };
    struct outer l_part         = { .tag = 71 };
    struct inner l_arr_st[3]    = { { 81, 82 } };
    int          l_matrix[2][3] = { { 1 } };
    int i;
    int j;

    for (i = 0; i < 6; i++) {
        printf("g_arr[%d]=%d\n", i, g_arr[i]);
    }
    for (i = 0; i < 4; i++) {
        printf("g_all_zero[%d]=%d\n", i, g_all_zero[i]);
    }
    printf("g_zero_struct.tag=%d\n", g_zero_struct.tag);
    printf("g_zero_struct.in.p=%d\n", g_zero_struct.in.p);
    printf("g_zero_struct.in.q=%d\n", g_zero_struct.in.q);
    printf("g_zero_struct.trailer=%d\n", g_zero_struct.trailer);
    printf("g_part.tag=%d\n", g_part.tag);
    printf("g_part.in.p=%d\n", g_part.in.p);
    printf("g_part.in.q=%d\n", g_part.in.q);
    printf("g_part.trailer=%d\n", g_part.trailer);
    printf("g_part_mid.tag=%d\n", g_part_mid.tag);
    printf("g_part_mid.in.p=%d\n", g_part_mid.in.p);
    printf("g_part_mid.in.q=%d\n", g_part_mid.in.q);
    printf("g_part_mid.trailer=%d\n", g_part_mid.trailer);
    for (i = 0; i < 3; i++) {
        printf("g_arr_st[%d].p=%d\n", i, g_arr_st[i].p);
        printf("g_arr_st[%d].q=%d\n", i, g_arr_st[i].q);
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("g_matrix[%d][%d]=%d\n", i, j, g_matrix[i][j]);
        }
    }
    for (i = 0; i < 3; i++) {
        printf("g_noinit[%d]=%d\n", i, g_noinit[i]);
    }
    /* Pointer facts as relations only: the two unmentioned elements are null
     * pointers, reported as a boolean, and the one string literal is read a
     * single character at a time through an unsigned char cast. */
    printf("g_ptrs0_null=%d\n", g_ptrs[0] == 0 ? 1 : 0);
    printf("g_ptrs0_first=%d\n", (int)(unsigned char)g_ptrs[0][0]);
    printf("g_ptrs1_null=%d\n", g_ptrs[1] == 0 ? 1 : 0);
    printf("g_ptrs2_null=%d\n", g_ptrs[2] == 0 ? 1 : 0);
    for (i = 0; i < 4; i++) {
        printf("g_uarr[%d]=%u\n", i, g_uarr[i]);
    }
    for (i = 0; i < 6; i++) {
        printf("l_arr[%d]=%d\n", i, l_arr[i]);
    }
    for (i = 0; i < 4; i++) {
        printf("l_all_zero[%d]=%d\n", i, l_all_zero[i]);
    }
    printf("l_zero_struct.tag=%d\n", l_zero_struct.tag);
    printf("l_zero_struct.in.p=%d\n", l_zero_struct.in.p);
    printf("l_zero_struct.in.q=%d\n", l_zero_struct.in.q);
    printf("l_zero_struct.trailer=%d\n", l_zero_struct.trailer);
    printf("l_part.tag=%d\n", l_part.tag);
    printf("l_part.in.p=%d\n", l_part.in.p);
    printf("l_part.in.q=%d\n", l_part.in.q);
    printf("l_part.trailer=%d\n", l_part.trailer);
    for (i = 0; i < 3; i++) {
        printf("l_arr_st[%d].p=%d\n", i, l_arr_st[i].p);
        printf("l_arr_st[%d].q=%d\n", i, l_arr_st[i].q);
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("l_matrix[%d][%d]=%d\n", i, j, l_matrix[i][j]);
        }
    }
    return 0;
}
