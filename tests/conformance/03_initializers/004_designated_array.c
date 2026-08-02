/* Designated array initializers: sparse, out-of-order, designator-then-positional
 * continuation, implied length, constant-expression designators, the several
 * two-dimensional spellings, and struct-element and struct-member designators.
 *
 * C11 6.7.9p19 supplies both rules these forms rest on.  It fixes the "current object"
 * rule that makes a designator reposition the cursor so that positional initializers
 * resume from there, which is what the continuation and the two-dimensional cases below
 * exercise; and it zero-fills "any subobject that is not initialized explicitly", which is
 * what the sparse cases rest on.  The initialization happens in initializer-list order, so
 * a form whose answer depends on the order of the list is a form that tests the ordering.
 *
 * ONE SPELLING IS EXCLUDED, and it is stated here rather than left to be noticed.
 * OVERLAPPING designators -- two initializers naming the SAME subobject, where the later one
 * must win -- are well-defined C11 by that same paragraph, and getting the order wrong is a
 * realistic compiler defect that stays invisible unless a program actually overrides
 * something: a front end that folded designators into a map keyed by index would keep
 * whichever value it happened to insert last, which need not be the one the source ordered
 * last.  The spelling is nevertheless not written here.  -Wextra enables -Woverride-init and
 * the suite's authoring gate promotes every diagnostic to an error, and fourteen override
 * spellings were measured against that gate with the reference compiler -- duplicate element
 * designator, identical-value override, zero-then-value, whole-row brace then element,
 * element then whole-row brace, that reverse nesting with a later initializer for a different
 * row, positional continuation running back over an earlier designator, struct-element brace
 * then one member of it, two member designators then a whole-struct brace, duplicate struct
 * member, two union members, a nested whole-member brace then one of its members, the reverse
 * of that, and an override inside a compound literal -- and every one is rejected by
 * -Werror=override-init.  An override carrying a side effect is not an escape either:
 * -Woverride-init-side-effects is on by default, and C11 6.7.9 leaves the evaluation of an
 * overridden initializer's side effects unspecified, which requirement 1 forbids outright.
 * No admissible deviation exists: the gate's removable members are exactly -pedantic, for
 * the GCC-extensions area, and -Wconversion with -Wsign-conversion, for the deliberate
 * narrowing program, so -Wextra is not removable and area 03 is granted no deviation at all.
 * A source-level diagnostic-suppression directive is not an alternative: it neutralizes a
 * gate member from inside the translation unit while the record still claims the unchanged
 * gate, which makes the record's own claim false rather than making the program clean.
 * Constraint C3 asks for exactly what is done instead -- the exclusion is narrow, it is
 * scoped to one spelling, and its reason is recorded here and in this program's expectation
 * record.
 *
 * THE ORDERING RULE ITSELF IS STILL EXERCISED, GATE-CLEAN, IN THIS VERY FILE, so what is
 * excluded is the overriding spelling and not the rule that governs it.  Order decides the
 * answer in g_continue, where [1] = 11 is followed by two positional initializers and then by
 * [0] = 10, so the cursor has to move backwards and resume correctly; in g_row_then_elem and
 * g_elem_then_row, which spell one two-dimensional aggregate in both orders and must agree;
 * in g_elem_only, where three element designators arrive out of row order; and in
 * g_st_brace, whose two element braces are listed [2] before [0].  Each of those is
 * gate-clean because no initializer in it revisits a slot an earlier one already named.  The
 * bracketed-range designator `[0 ... 3] = 7`, which would express a repeated fill without an
 * override, is a GCC extension rejected by -Werror=pedantic and belongs to area 08 rather
 * than here.
 *
 * Every element of every object is read back and printed individually, 70 lines in total, so a
 * single wrong slot localizes to one initializer form.  Both storage durations are covered:
 * the file-scope objects are materialized once into a data section, and the block-scope
 * objects are materialized by emitted code on entry to main, which is the same construct
 * through a completely different code path.
 *
 * Undefined-behaviour freedom: every value is a positive int constant of at most 170, far
 * inside the INT_MAX of at least 2147483647 that every target guarantees, so no signed
 * overflow is reachable; every subscript
 * is a literal strictly inside its array, no pointer is formed, there is no arithmetic beyond
 * the two constant-expression designators [2 + 1] and [1 * 2] and the loop counters'
 * increments, no shift, no
 * aliasing, no uninitialized read (every object is either explicitly initialized or, being a
 * partially initialized aggregate, zero-filled by C11 6.7.9p19 and p21), and no object is
 * modified twice between sequence points.  Only int and %d are used, so no target's pointer
 * width, char signedness, padding or execution character set can be observed.  The one cast
 * in the program is the (int) applied to `sizeof g_implied / sizeof g_implied[0]`: a ratio of
 * two sizeofs of the same array and its own element type, so the target-varying size_t
 * cancels, the quotient is the element count on every target, and 5 is representable in int
 * everywhere. */

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

int main(void)
{
    /* The same designated forms again at block scope, where the initializer is not a data
     * section image but code the backend emits on entry.  The two paths are independent
     * implementations of one language rule, so both are exercised. */
    int l_sparse[8]   = { [7] = 170, [3] = 130, [0] = 101 };
    int l_continue[6] = { [1] = 111, 112, 113, [0] = 110 };
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
    return 0;
}
