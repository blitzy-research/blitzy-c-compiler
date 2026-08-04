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
 * OVERLAPPING DESIGNATORS ARE EXERCISED HERE, not described and skipped.  Two initializers
 * naming the SAME subobject, where the later one must win, are well-defined C11 by that same
 * paragraph -- p19 says each initializer provided for a particular subobject overrides any
 * previously listed initializer for the same subobject -- and getting the order wrong is a
 * realistic compiler defect that stays invisible unless a program actually overrides
 * something: a front end that folded designators into a map keyed by index would keep
 * whichever value it happened to insert last, which need not be the one the source ordered
 * last.  Five objects carry that spelling.  g_over_flat repeats [1] with a different value, so
 * the second value must survive.  g_over_row_elem names a whole row and then one element of
 * it; g_over_elem_row names the same element and then the same whole row.  Those two are the
 * DISCRIMINATING PAIR: they contain the identical two initializers in opposite order, so
 * "the last initializer to name a subobject wins" prints 20 for the first and 2 for the
 * second, whereas the weaker rule "any explicitly named subobject survives" would print 20
 * for both.  g_over_memb overrides one member of an array-of-struct element whose whole value
 * an earlier brace supplied, and l_over_flat repeats the flat form at automatic duration,
 * where the override has to be resolved by emitted code rather than by a data-section image.
 *
 * WHY A SCOPED DIAGNOSTIC DIRECTIVE, AND WHY NOT A GATE DEVIATION.  -Wextra enables
 * -Woverride-init and the suite's authoring gate promotes every diagnostic to an error, and a
 * gate-clean spelling of a genuine override does not exist: every override the reference
 * compiler was measured against -- duplicate element designator, identical-value override,
 * zero-then-value, whole-row brace then element, element then whole-row brace, positional
 * continuation running back over an earlier designator, struct-element brace then one member
 * of it, duplicate struct member, two union members, a bitfield member, a nested whole-member
 * brace then one of its members, the reverse of that, and an override inside a compound
 * literal -- is rejected by -Werror=override-init.  Nor is a recorded gate deviation
 * admissible: the gate's removable members are exactly -pedantic, for the GCC-extensions
 * area, and -Wconversion with -Wsign-conversion, for the deliberate narrowing program, so
 * -Wextra is not removable and area 03 is granted no deviation at all.  The one initializer
 * diagnostic that stands between this file and the construct the area requires is therefore
 * suppressed FROM INSIDE, around exactly the declarations that need it and no further, and the
 * previous diagnostic state is restored immediately afterwards.  This program is the ONLY one
 * in the corpus that carries a diagnostic-control directive, and that is deliberate: the
 * mechanism is admissible here because it is the only route to a construct the area is
 * REQUIRED to cover, and it is confined to this one file so that nowhere else can a divergence
 * be confused with an artefact of a suppressed diagnostic.  What makes it admissible is the
 * scope, not the mechanism -- the suppression covers a handful of declarations rather than a
 * translation unit, it is visible where it applies, it is declared in this program's own
 * expectation record rather than hidden, and the audit gate stays at full strength for this
 * program as for every other: no ub_audit_flags deviation is claimed, -Werror remains in
 * force, and an override written outside the push/pop region is still rejected, which was
 * measured.  A directive used to make a program pass a gate it would otherwise fail on its own
 * subject would be the opposite case and is not admissible anywhere in this corpus.  An
 * override carrying a side effect remains excluded on its own merits, and that exclusion is
 * not about the gate at all: -Woverride-init-side-effects is on by default and is not a gate
 * member, and C11 6.7.9p23 leaves the sequencing of an initializer list's side effects
 * unspecified, which requirement 1 forbids outright.  Every override here is therefore a
 * constant, so nothing depends on when it is evaluated -- only on the order it is listed in.
 *
 * THE ORDERING RULE IS ALSO EXERCISED WITHOUT ANY OVERRIDE, so a divergence can be localized
 * to the overriding spelling or to the current-object rule rather than to "one of the two".
 * Order decides the answer in g_continue, where [1] = 11 is followed by two positional
 * initializers and then by [0] = 10, so the cursor has to move backwards and resume
 * correctly; in g_row_then_elem and g_elem_then_row, which spell one two-dimensional
 * aggregate in both orders and must agree; in g_elem_only, where three element designators
 * arrive out of row order; and in g_st_brace, whose two element braces are listed [2] before
 * [0].  Each of those needs no suppression because no initializer in it revisits a slot an
 * earlier one already named.  The bracketed-range designator `[0 ... 3] = 7` is a different
 * feature rather than a spelling of this one: it is a GCC extension rejected by
 * -Werror=pedantic and belongs to area 08 rather than here.
 *
 * Every element of every object is read back and printed individually, 94 lines in total, so a
 * single wrong slot localizes to one initializer form.  Both storage durations are covered:
 * the file-scope objects are materialized once into a data section, and the block-scope
 * objects are materialized by emitted code on entry to main, which is the same construct
 * through a completely different code path.
 *
 * Undefined-behaviour freedom: every value is a positive int constant of at most 170, far
 * inside the INT_MAX of at least 2147483647 that every target guarantees, so no signed
 * overflow is reachable; every subscript is a literal or a loop counter strictly inside its
 * array, so the pointer arithmetic the language performs for it never leaves the object, no
 * address of an object is taken, and no pointer value is printed, compared or converted to an
 * integer -- the only pointers in the program are the format-string literals decaying at each
 * printf call; there is no arithmetic beyond the two constant-expression designators [2 + 1] and
 * [1 * 2] and the loop counters' increments, no shift, no
 * aliasing, no uninitialized read (every object is either explicitly initialized or, being a
 * partially initialized aggregate, zero-filled by C11 6.7.9p19 and p21), and no object is
 * modified twice between sequence points.  Only int and %d are used, so no target's pointer
 * width, char signedness, padding or execution character set can be observed.  The one cast
 * in the program is the (int) applied to `sizeof g_implied / sizeof g_implied[0]`: a ratio of
 * two sizeofs of the same array and its own element type, so the target-varying size_t
 * cancels, the quotient is the element count on every target, and 5 is representable in int
 * everywhere.
 *
 * The overriding objects add no undefined or unspecified behaviour of their own.  Overriding is
 * DEFINED rather than merely tolerated: C11 6.7.9p19 requires initialization to occur in
 * initializer-list order and makes each initializer override any previously listed initializer
 * for the same subobject, so every subobject named twice among the five objects below has
 * exactly one standard-mandated value -- the one its source listed last.  Overriding is not a
 * double modification: p19 describes which initializer supplies a
 * subobject's value, and an initializer list is not a sequence of assignments to a
 * live object.  Every overridden and overriding initializer is an integer constant expression,
 * so none has a side effect, none is a function call and none reads an object -- which is why
 * p23's unspecified sequencing of an initializer list's evaluations cannot be observed here
 * and why -Woverride-init-side-effects, a diagnostic outside the gate, has nothing to report.
 * Each override names a subobject strictly inside its declared bounds: indices [1] and [2] in
 * g_over_flat and l_over_flat, both declared [4]; row [0] and element [0][1] in
 * g_over_row_elem and g_over_elem_row, both declared [2][3]; and element [0] with its member
 * .q in g_over_memb, declared [2].  Elements no initializer names -- g_over_flat[3], row [1] of both
 * two-dimensional objects, g_over_memb[1].q and l_over_flat[3] -- are zero by p19 and p21, so
 * no read of an indeterminate value is reachable.  The two diagnostic-control directives are
 * the only pragmas in the file; C11 6.10.6 governs them, they change which messages the
 * translator emits and nothing about the program's semantics, and each is matched by a pop, so
 * neither can affect the values printed or leak past the declarations it brackets. */

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

/* THE OVERLAPPING SPELLING.  Each object names one subobject twice, and C11 6.7.9p19 makes the
 * later initializer win.  -Woverride-init, which -Wextra enables and the gate's -Werror makes
 * fatal, reports exactly that; it is suppressed for these four declarations alone and the
 * previous state is restored on the next line, so every other diagnostic -- and -Woverride-init
 * itself everywhere else in this translation unit -- remains in force.
 *
 * g_over_row_elem and g_over_elem_row are the discriminating pair: identical initializers,
 * opposite order, so element [0][1] must read 20 in the first and 2 in the second.  Any
 * implementation that answered 20 for both would be preserving explicitly named subobjects
 * instead of honouring initializer order. */
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Woverride-init"
static int g_over_flat[4]         = { [0] = 1, [1] = 2, [1] = 20, [2] = 3 };
static int g_over_row_elem[2][3]  = { [0] = { 1, 2, 3 }, [0][1] = 20 };
static int g_over_elem_row[2][3]  = { [0][1] = 20, [0] = { 1, 2, 3 } };
static struct pair g_over_memb[2] = { [0] = { 1, 2 }, [0].q = 20, [1].p = 3 };
#pragma GCC diagnostic pop

int main(void)
{
    /* The same designated forms again at block scope, where the initializer is not a data
     * section image but code the backend emits on entry.  The two paths are independent
     * implementations of one language rule, so both are exercised. */
    int l_sparse[8]   = { [7] = 170, [3] = 130, [0] = 101 };
    int l_continue[6] = { [1] = 111, 112, 113, [0] = 110 };
    /* The overriding spelling at automatic duration, where the winning value has to be
     * resolved by emitted code rather than settled into a data-section image.  The suppression
     * brackets this one declaration only. */
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Woverride-init"
    int l_over_flat[4] = { [0] = 101, [1] = 102, [1] = 120, [2] = 103 };
#pragma GCC diagnostic pop
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
    /* The overriding objects, every element printed individually so that a wrong winner
     * localizes to one slot of one spelling.  g_over_row_elem[0][1] and g_over_elem_row[0][1]
     * are the two lines that separate initializer order from mere explicit naming. */
    for (i = 0; i < 4; i++) {
        printf("g_over_flat[%d]=%d\n", i, g_over_flat[i]);
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("g_over_row_elem[%d][%d]=%d\n", i, j, g_over_row_elem[i][j]);
        }
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("g_over_elem_row[%d][%d]=%d\n", i, j, g_over_elem_row[i][j]);
        }
    }
    for (i = 0; i < 2; i++) {
        printf("g_over_memb[%d].p=%d\n", i, g_over_memb[i].p);
        printf("g_over_memb[%d].q=%d\n", i, g_over_memb[i].q);
    }
    for (i = 0; i < 8; i++) {
        printf("l_sparse[%d]=%d\n", i, l_sparse[i]);
    }
    for (i = 0; i < 6; i++) {
        printf("l_continue[%d]=%d\n", i, l_continue[i]);
    }
    for (i = 0; i < 4; i++) {
        printf("l_over_flat[%d]=%d\n", i, l_over_flat[i]);
    }
    return 0;
}
