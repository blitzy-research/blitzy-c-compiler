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
 * come from the data or BSS image the linker emits, whereas an automatic
 * object's zeros must be produced by instructions the backend emits on entry to
 * the block.  A compiler can implement one correctly and the other incorrectly,
 * and only holding both forms side by side distinguishes the two.
 *
 * One spelling is deliberately absent.  The positional partial struct
 * initializer -- `struct outer v = { 1 };` -- is excluded because -Wextra
 * enables -Wmissing-field-initializers, which the mandated authoring gate's
 * -Werror promotes to an error; no gate deviation is sanctioned for this area,
 * so the exclusion is recorded in this program's expectation record instead.
 * It costs no coverage of the semantics under test: the { 0 } idiom and the
 * designated forms above exercise exactly the same implicit-zero guarantee.
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
 * Freedom from undefined behaviour.  There is no arithmetic on the data and so
 * no signed overflow, no shift, no aliasing violation and no object modified
 * after its initialization; every subscript is strictly inside its array and no
 * one-past-end pointer is formed; each call passes at most one side-effecting
 * argument.  The one point worth checking first is that nothing here reads
 * uninitialized storage: g_noinit and the null pointer elements have STATIC
 * storage duration, so the standard defines their contents as zero rather than
 * indeterminate, which is precisely the guarantee under test.
 */

int printf(const char *, ...);

struct inner { int p; int q; };
struct outer { int tag; struct inner in; int trailer; };

/* Static duration.  At -O0 these zeros come from the emitted image itself: the
 * wholly zero objects -- g_all_zero, g_zero_struct and g_noinit -- reach BSS,
 * while the partially initialized ones carry literal zero bytes in the data
 * section, and both placements were confirmed on all four targets.  At a higher
 * level the compiler may instead propagate the contents it already knows, so
 * sweeping every optimization level is what keeps both paths under test. */
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
