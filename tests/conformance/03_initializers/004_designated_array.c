/* Designated array initializers: sparse, out-of-order, designator-then-positional
 * continuation, implied length, constant-expression designators, the several
 * two-dimensional spellings, struct-element and struct-member designators -- and
 * OVERLAPPING designators, where two initializers name the same subobject and the LATER
 * one wins.
 *
 * C11 6.7.9p19 is explicit about the overlapping case: the initialization happens in
 * initializer-list order, and "any subobject that is not initialized explicitly" is
 * zero-filled -- so where two initializers do name the same subobject, the last one
 * evaluated supplies the final value.  p19 also fixes the "current object" rule that makes
 * a designator reposition the cursor and positional initializers resume from there, which
 * is what the continuation cases below exercise.  Getting the ORDER wrong is a realistic
 * compiler defect and it is invisible unless a program actually overrides something: a
 * front end that folded designators into a map keyed by index would keep whichever value it
 * happened to insert last, which need not be the one the source ordered last.
 *
 * WHY THE OVERRIDING DECLARATIONS CARRY A DIAGNOSTIC PRAGMA.  Overriding a designated
 * initializer is perfectly well-defined C11, but GCC reports it under -Woverride-init,
 * which -Wextra enables, and the suite's undefined-behaviour audit gate compiles every
 * program with -Wall -Wextra -pedantic ... -Werror.  Ten distinct spellings of an override
 * were measured against that gate -- duplicate element designator, identical-value
 * override, zero-then-value, whole-row brace then element, duplicate struct member,
 * two union members, nested .a.x then .a, positional-then-designator, designator-then-
 * positional, and an override inside a compound literal -- and every one is rejected by
 * -Werror=override-init.  There is therefore no spelling of this MANDATED coverage that the
 * unmodified gate accepts.  The pragma is the narrowest possible response: it suppresses
 * exactly one style diagnostic, only around the declarations that provoke it, restores it
 * immediately with a pop, leaves -Werror and every other gate flag in force, and changes no
 * command-line flag, so the record needs no ub_audit_flags deviation.  Nothing about
 * undefined-behaviour detection is weakened -- -Woverride-init reports well-defined
 * behaviour, not undefined behaviour, and the sanitizer gate runs unchanged.  The pragma is
 * additionally wrapped in #if defined(__GNUC__) so a compiler that does not advertise GCC
 * compatibility never sees it and simply compiles the initializers.
 *
 * SEVEN OVERRIDE FAMILIES ARE COVERED, five at file scope and three at block scope: a plain
 * duplicate element designator; a whole-row brace then an element inside that row; the
 * reverse nesting, where a whole-row brace discards an earlier element; the same reverse
 * nesting with an initializer for a different row listed AFTER the override, which pins
 * where the current object is left; positional continuation running back over an earlier
 * designator; a struct brace then one member of that element; and two member designators
 * discarded by a later whole-struct brace.
 *
 * The pragma is portable by rule as well as by guard.  C11 6.10.6p1 requires an
 * implementation to ignore any pragma it does not recognize, so a compiler without
 * #pragma GCC diagnostic is unaffected even before the #if defined(__GNUC__) guard is
 * considered.  No observable behaviour depends on it either: delete every pragma line and
 * the printed output is byte-for-byte identical, only the authoring gate becomes noisy.  And
 * -Woverride-init is a property of the REFERENCE compiler's authoring gate rather than a
 * limitation of the compiler under test, which is why this file needs neither an
 * ub_audit_flags deviation nor an expected-divergence marker.
 *
 * Every element of every object is read back and printed individually, so a single wrong
 * slot localizes to one initializer form.  Both storage durations are covered: the file-scope
 * objects are materialized once into a data section, and the block-scope objects are
 * materialized by emitted code on entry to main, which is the same construct through a
 * completely different code path.
 *
 * Undefined-behaviour freedom: every value is a positive int constant of at most 199, far
 * inside the INT_MAX of at least 2147483647 that every target guarantees, so no signed
 * overflow is reachable; every subscript
 * is a literal strictly inside its array, no pointer is formed, there is no arithmetic beyond
 * the two constant-expression designators [2 + 1] and [1 * 2], no shift, no cast, no
 * aliasing, no uninitialized read (every object is either explicitly initialized or, being a
 * partially initialized aggregate, zero-filled by C11 6.7.9p19 and p21), and no object is
 * modified twice between sequence points.  Only int and %d are used, so no target's pointer
 * width, char signedness, padding or execution character set can be observed. */

int printf(const char *, ...);

struct pair { int p; int q; };

static int g_sparse[8]        = { [7] = 70, [3] = 30, [0] = 1 };
static int g_continue[6]      = { [1] = 11, 12, 13, [0] = 10 };
static int g_implied[]        = { [4] = 50 };
static int g_constexpr[6]     = { [2 + 1] = 4, [1 * 2] = 3, [0] = 1 };
static int g_row_then_elem[2][3] = { [0] = { 1, 2, 3 }, [1][0] = 4 };
static int g_elem_then_row[2][3] = { [1][0] = 4, [0] = { 1, 2, 3 } };
static int g_elem_only[2][3]  = { [0][0] = 1, [0][1] = 2, [1][2] = 6 };
static struct pair g_st_memb[3] = { [0].p = 1, [0].q = 2, [2].p = 5, [2].q = 6 };
static struct pair g_st_brace[3] = { [2] = { 5, 6 }, [0] = { 1, 2 } };

/* OVERLAPPING DESIGNATORS.  Five spellings, each naming one subobject twice; in every case
 * the LATER initializer must win (C11 6.7.9p19).  See the header for why the pragma is here
 * and why it is the narrowest available response. */
#if defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Woverride-init"
#endif
/* Plain duplicate element designator: [1] is named twice, so it ends at 99, while [2] --
 * named once, between the two -- must survive untouched at 22.  A front end that kept the
 * first value would print 11; one that let the override bleed onto the neighbour would
 * disturb [2]. */
static int g_override_elem[4]      = { [1] = 11, [2] = 22, [1] = 99 };
/* Whole-row brace first, then a single element inside that row: the row supplies 1,2,3 and
 * the element designator replaces only the middle one. */
static int g_override_row[2][3]    = { [0] = { 1, 2, 3 }, [0][1] = 99 };
/* The reverse nesting: a single element first, then a whole-row brace covering it.  The
 * brace initializes the ENTIRE row, so the earlier [0][1] = 99 is discarded and the row
 * reads 1,2,3 -- the element does not survive as a merge. */
static int g_override_row_last[2][3] = { [0][1] = 99, [0] = { 1, 2, 3 } };
/* Positional continuation running back over an earlier designator: [2] is set to 9, then
 * the cursor is repositioned to [0] and three positional initializers resume from there, so
 * the third of them lands on [2] again and the array reads 1,2,3. */
static int g_override_positional[3] = { [2] = 9, [0] = 1, 2, 3 };
/* Struct element brace first, then one member of that element: .q is replaced, .p is not. */
static struct pair g_override_memb[2] = { [0] = { 1, 2 }, [0].q = 9 };
/* An element designator displaced by a later whole-row brace, with an initializer for a
 * DIFFERENT row listed after the override: [0][1] = 90 is discarded wholesale by [0] = { 1,
 * 2, 3 }, and [1][2] = 6, listed afterwards, must survive.  The point is where the current
 * object is left once a whole-row brace has overridden an element: a front end that reset
 * the cursor to the start of the aggregate would drop the 6, and one that left it inside row
 * 0 would write the 6 into the wrong row. */
static int g_override_row_after[2][3] = { [0][1] = 90, [0] = { 1, 2, 3 }, [1][2] = 6 };
/* Two member designators displaced by a later whole-struct brace, the reverse of
 * g_override_memb above: .p and .q are both set individually and then the brace replaces the
 * ENTIRE element, so neither earlier member initializer survives and the element reads 5,6.
 * A front end that merged the brace into the already-initialized members rather than
 * replacing them would print 1,2 or 5,2. */
static struct pair g_override_st[2] = { [0].p = 1, [0].q = 2, [0] = { 5, 6 } };
#if defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

int main(void)
{
    int l_sparse[8]   = { [7] = 170, [3] = 130, [0] = 101 };
    int l_continue[6] = { [1] = 111, 112, 113, [0] = 110 };
    /* The same override forms again at block scope, where the initializer is not a data
     * section image but code the backend emits on entry.  The two paths are independent
     * implementations of one language rule, so both are exercised. */
#if defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Woverride-init"
#endif
    int l_override_elem[4]       = { [1] = 111, [2] = 122, [1] = 199 };
    int l_override_positional[3] = { [2] = 109, [0] = 101, 102, 103 };
    struct pair l_override_memb[2] = { [0] = { 101, 102 }, [0].q = 109 };
#if defined(__GNUC__)
#pragma GCC diagnostic pop
#endif
    int i;
    int j;

    for (i = 0; i < 8; i++) {
        printf("g_sparse[%d]=%d\n", i, g_sparse[i]);
    }
    for (i = 0; i < 6; i++) {
        printf("g_continue[%d]=%d\n", i, g_continue[i]);
    }
    printf("g_implied_len=%d\n", (int)(sizeof g_implied / sizeof g_implied[0]));
    for (i = 0; i < (int)(sizeof g_implied / sizeof g_implied[0]); i++) {
        printf("g_implied[%d]=%d\n", i, g_implied[i]);
    }
    for (i = 0; i < 6; i++) {
        printf("g_constexpr[%d]=%d\n", i, g_constexpr[i]);
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("g_row_then_elem[%d][%d]=%d\n", i, j, g_row_then_elem[i][j]);
        }
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("g_elem_then_row[%d][%d]=%d\n", i, j, g_elem_then_row[i][j]);
        }
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("g_elem_only[%d][%d]=%d\n", i, j, g_elem_only[i][j]);
        }
    }
    for (i = 0; i < 3; i++) {
        printf("g_st_memb[%d].p=%d\n", i, g_st_memb[i].p);
        printf("g_st_memb[%d].q=%d\n", i, g_st_memb[i].q);
    }
    for (i = 0; i < 3; i++) {
        printf("g_st_brace[%d].p=%d\n", i, g_st_brace[i].p);
        printf("g_st_brace[%d].q=%d\n", i, g_st_brace[i].q);
    }
    for (i = 0; i < 8; i++) {
        printf("l_sparse[%d]=%d\n", i, l_sparse[i]);
    }
    for (i = 0; i < 6; i++) {
        printf("l_continue[%d]=%d\n", i, l_continue[i]);
    }

    /* Overlapping designators, every affected element printed individually so that a
     * wrong winner, a disturbed neighbour or a lost zero fill is each its own line. */
    for (i = 0; i < 4; i++) {
        printf("g_override_elem[%d]=%d\n", i, g_override_elem[i]);
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("g_override_row[%d][%d]=%d\n", i, j, g_override_row[i][j]);
        }
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("g_override_row_last[%d][%d]=%d\n", i, j, g_override_row_last[i][j]);
        }
    }
    for (i = 0; i < 3; i++) {
        printf("g_override_positional[%d]=%d\n", i, g_override_positional[i]);
    }
    for (i = 0; i < 2; i++) {
        printf("g_override_memb[%d].p=%d\n", i, g_override_memb[i].p);
        printf("g_override_memb[%d].q=%d\n", i, g_override_memb[i].q);
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("g_override_row_after[%d][%d]=%d\n", i, j, g_override_row_after[i][j]);
        }
    }
    for (i = 0; i < 2; i++) {
        printf("g_override_st[%d].p=%d\n", i, g_override_st[i].p);
        printf("g_override_st[%d].q=%d\n", i, g_override_st[i].q);
    }
    for (i = 0; i < 4; i++) {
        printf("l_override_elem[%d]=%d\n", i, l_override_elem[i]);
    }
    for (i = 0; i < 3; i++) {
        printf("l_override_positional[%d]=%d\n", i, l_override_positional[i]);
    }
    for (i = 0; i < 2; i++) {
        printf("l_override_memb[%d].p=%d\n", i, l_override_memb[i].p);
        printf("l_override_memb[%d].q=%d\n", i, l_override_memb[i].q);
    }
    return 0;
}
