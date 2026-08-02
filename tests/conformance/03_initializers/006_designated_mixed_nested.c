/* Mixed positional and designated initializers inside nested aggregates -- the
 * hardest combination in this area, because it puts the nested, the partial and
 * the designated forms into one initializer and so forces the compiler to track
 * the C11 "current object" (6.7.9) correctly as it descends into a brace,
 * applies a designator there, ascends again, and resumes positionally.
 *
 * Eight mixed forms are covered, each in its own object so a wrong slot names the
 * form that produced it:
 *
 *   1. positional outer, designated inner        g_pos_desig_in
 *   2. designated, then unlabelled brace, then
 *      positional again                          g_desig_pos_in
 *   3. array of struct: positional element 0,
 *      designated element 2, element 1 skipped   g_arr_pos_then_desig
 *   4. two-dimensional: positional row 0, then
 *      a single [2][1] element designator        g_m_pos_then_desig
 *   5. three levels: form 1 nested in a struct
 *      whose sibling array is designated         g_deep_mixed
 *   6. three levels, all designated, including a
 *      nested submember designator and a
 *      combined index-and-member designator      g_deep_desig
 *   7. OVERLAPPING designators at depth: an inner
 *      aggregate brace-initialized, then one of
 *      its members named again                   g_deep_override
 *   8. OVERLAPPING designators on a whole nested
 *      element, plus the same submember named
 *      twice at two different depths              g_deep_override_elem
 *
 * Forms 1, 3, 5 and 7 are repeated at automatic storage duration inside main, so
 * the static-image path and the run-time path are both exercised; form 3's twin
 * uses the combined [2].q spelling rather than [2] = { ... }.
 *
 * Forms 7 and 8 are the nested half of the overlapping-designator coverage whose
 * flat forms live in 004_designated_array.c. They matter separately because the
 * override has to interact correctly with the "current object" descent: an
 * implementation that resolved designators against the wrong nesting level would
 * override a sibling, or the outer object, rather than the subobject named.
 * C11 6.7.9p19 fixes the answer -- initialization proceeds in initializer-list
 * order, so the last initializer to name a subobject supplies its value, and a
 * brace covering a whole subobject discards whatever earlier initializers had
 * placed inside it.
 *
 * Every member at every nesting level is read back and printed on its own line,
 * including every member left implicitly zero, so a single divergent line
 * localizes the defect to one initializer form. No aggregate is printed whole
 * and no representation is inspected, so no padding byte is ever observed.
 *
 * Three gate facts govern the shape of these initializers and must not be
 * disturbed. First, -Wextra enables -Wmissing-field-initializers, which -Werror
 * makes fatal; in C it does not fire for a designated initializer, only for a
 * positional partial one, so every struct here that omits a member omits it via
 * a designator and every fully positional struct initializer supplies all of
 * its members. Removing a designator would trip the gate. Second, -Wall enables
 * -Wmissing-braces, so every nested aggregate is fully braced; eliding a brace
 * would trip the gate as well. Third, -Wextra enables -Woverride-init, which
 * reports the well-defined override that forms 7 and 8 exist to test; no spelling
 * of an override survives the unmodified gate, so those two declarations -- and
 * only those -- are bracketed by a scoped diagnostic pragma that suppresses that
 * one style warning and pops it again immediately. Every gate flag, -Werror
 * included, stays in force, no command-line flag changes, so the record carries no
 * ub_audit_flags deviation, and nothing about undefined-behaviour detection is
 * weakened: -Woverride-init describes defined behaviour, and the sanitizer gate
 * runs unchanged. The pragma is wrapped in #if defined(__GNUC__) so a compiler
 * that does not advertise GCC compatibility never sees it. */

int printf(const char *, ...);

struct inner { int p; int q; };
struct outer { int tag; struct inner in; int trailer; };
struct deep  { struct outer o; struct inner pr[2]; };

/* Form 1: 1 lands on tag positionally, the brace descends into in where a
 * designator sets q and leaves p zero, then 4 resumes positionally on trailer. */
static struct outer g_pos_desig_in = { 1, { .q = 3 }, 4 };
/* Form 2: a designator on tag, then an unlabelled brace that must bind to the
 * next member in, then a bare 14 that must continue on to trailer. */
static struct outer g_desig_pos_in = { .tag = 11, { .p = 12, .q = 13 }, 14 };
/* Form 3: element 0 positional, element 2 designated; element 1 is never
 * mentioned and must be zero in both of its members. */
static struct inner g_arr_pos_then_desig[3] = { { 21, 22 }, [2] = { 25, 26 } };
/* Form 4: row 0 positional, then a jump straight to one element of row 2. Five
 * elements must remain zero -- all three of row 1, plus [2][0] and [2][2], the
 * two elements of row 2 the designator does not name -- which makes a designator
 * that lands in the wrong row or column immediately visible. */
static int          g_m_pos_then_desig[3][3] = { { 1, 2, 3 }, [2][1] = 8 };
/* Form 5: form 1 nested one level deeper, beside a sibling array whose element
 * 1 is designated while element 0 stays zero. */
static struct deep  g_deep_mixed = { { 31, { .q = 33 }, 34 }, { [1] = { 37, 38 } } };
/* Form 6, the most demanding initializer in this area: .o = { .in.p = 42 }
 * descends two levels and must leave o.tag, o.in.q and o.trailer zero, while
 * .pr takes a positional element 0 followed by the combined index-and-member
 * designator [1].q, which must set pr[1].q and leave pr[1].p zero. */
static struct deep  g_deep_desig = { .o = { .in.p = 42 }, .pr = { { 43, 44 }, [1].q = 46 } };

#if defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Woverride-init"
#endif
/* Form 7: inside .o, the whole .in is brace-initialized to { 51, 52 } and then .in.q is
 * named again.  Only q may change, so in reads { 51, 59 }; .o.tag and .o.trailer are set
 * positionally around it and must be unaffected by the override two levels down.  Inside
 * .pr, element 0 is brace-initialized and then its .p is overridden, so pr[0] reads
 * { 58, 54 } while pr[1] stays entirely zero. */
static struct deep g_deep_override = {
    .o  = { .tag = 50, .in = { 51, 52 }, .in.q = 59, .trailer = 53 },
    .pr = { [0] = { 55, 54 }, [0].p = 58 }
};
/* Form 8: the reverse direction at every level.  .in.q is named FIRST and then the whole
 * .in is brace-initialized, so the brace wins and q reads 62, not 69 -- the mirror image of
 * form 7 and the case that separates "last writer wins" from "any explicit member survives".
 * Likewise .pr element 1 gets a submember, then the whole element, so pr[1] reads { 65, 66 }
 * and the earlier 68 is gone. */
static struct deep g_deep_override_elem = {
    .o  = { .tag = 60, .in.q = 69, .in = { 61, 62 }, .trailer = 63 },
    .pr = { [1].p = 68, [1] = { 65, 66 } }
};
#if defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

int main(void)
{
    /* Automatic-duration twins of forms 1, 3 and 5. An automatic aggregate with
     * any initializer at all has its unmentioned members zeroed exactly as a
     * static one does, so these read back with the same shape from a different
     * code path. l_arr_mixed uses the combined [2].q spelling, so element 2's p
     * must stay zero while its q is set. */
    struct outer l_pos_desig_in = { 51, { .q = 53 }, 54 };
    struct inner l_arr_mixed[3] = { { 61, 62 }, [2].q = 66 };
    struct deep  l_deep_mixed   = { { 71, { .p = 72 }, 74 }, { [0].q = 76 } };
    /* Automatic twin of form 7: the same nested override, but materialized by emitted
     * code on entry rather than as a data-section image. */
#if defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Woverride-init"
#endif
    struct deep l_deep_override = {
        .o  = { .tag = 80, .in = { 81, 82 }, .in.q = 89, .trailer = 83 },
        .pr = { [0] = { 85, 84 }, [0].p = 88 }
    };
#if defined(__GNUC__)
#pragma GCC diagnostic pop
#endif
    int i;
    int j;

    printf("g_pos_desig_in.tag=%d\n", g_pos_desig_in.tag);
    printf("g_pos_desig_in.in.p=%d\n", g_pos_desig_in.in.p);
    printf("g_pos_desig_in.in.q=%d\n", g_pos_desig_in.in.q);
    printf("g_pos_desig_in.trailer=%d\n", g_pos_desig_in.trailer);
    printf("g_desig_pos_in.tag=%d\n", g_desig_pos_in.tag);
    printf("g_desig_pos_in.in.p=%d\n", g_desig_pos_in.in.p);
    printf("g_desig_pos_in.in.q=%d\n", g_desig_pos_in.in.q);
    printf("g_desig_pos_in.trailer=%d\n", g_desig_pos_in.trailer);
    for (i = 0; i < 3; i++) {
        printf("g_arr_pos_then_desig[%d].p=%d\n", i, g_arr_pos_then_desig[i].p);
        printf("g_arr_pos_then_desig[%d].q=%d\n", i, g_arr_pos_then_desig[i].q);
    }
    for (i = 0; i < 3; i++) {
        for (j = 0; j < 3; j++) {
            printf("g_m_pos_then_desig[%d][%d]=%d\n", i, j, g_m_pos_then_desig[i][j]);
        }
    }
    printf("g_deep_mixed.o.tag=%d\n", g_deep_mixed.o.tag);
    printf("g_deep_mixed.o.in.p=%d\n", g_deep_mixed.o.in.p);
    printf("g_deep_mixed.o.in.q=%d\n", g_deep_mixed.o.in.q);
    printf("g_deep_mixed.o.trailer=%d\n", g_deep_mixed.o.trailer);
    for (i = 0; i < 2; i++) {
        printf("g_deep_mixed.pr[%d].p=%d\n", i, g_deep_mixed.pr[i].p);
        printf("g_deep_mixed.pr[%d].q=%d\n", i, g_deep_mixed.pr[i].q);
    }
    printf("g_deep_desig.o.tag=%d\n", g_deep_desig.o.tag);
    printf("g_deep_desig.o.in.p=%d\n", g_deep_desig.o.in.p);
    printf("g_deep_desig.o.in.q=%d\n", g_deep_desig.o.in.q);
    printf("g_deep_desig.o.trailer=%d\n", g_deep_desig.o.trailer);
    for (i = 0; i < 2; i++) {
        printf("g_deep_desig.pr[%d].p=%d\n", i, g_deep_desig.pr[i].p);
        printf("g_deep_desig.pr[%d].q=%d\n", i, g_deep_desig.pr[i].q);
    }
    printf("l_pos_desig_in.tag=%d\n", l_pos_desig_in.tag);
    printf("l_pos_desig_in.in.p=%d\n", l_pos_desig_in.in.p);
    printf("l_pos_desig_in.in.q=%d\n", l_pos_desig_in.in.q);
    printf("l_pos_desig_in.trailer=%d\n", l_pos_desig_in.trailer);
    for (i = 0; i < 3; i++) {
        printf("l_arr_mixed[%d].p=%d\n", i, l_arr_mixed[i].p);
        printf("l_arr_mixed[%d].q=%d\n", i, l_arr_mixed[i].q);
    }
    printf("l_deep_mixed.o.tag=%d\n", l_deep_mixed.o.tag);
    printf("l_deep_mixed.o.in.p=%d\n", l_deep_mixed.o.in.p);
    printf("l_deep_mixed.o.in.q=%d\n", l_deep_mixed.o.in.q);
    printf("l_deep_mixed.o.trailer=%d\n", l_deep_mixed.o.trailer);
    for (i = 0; i < 2; i++) {
        printf("l_deep_mixed.pr[%d].p=%d\n", i, l_deep_mixed.pr[i].p);
        printf("l_deep_mixed.pr[%d].q=%d\n", i, l_deep_mixed.pr[i].q);
    }

    /* Forms 7 and 8 and form 7's automatic twin, every member at every level printed
     * individually including the ones that must stay zero, so an override that landed on
     * a sibling or on the wrong nesting level is its own line. */
    printf("g_deep_override.o.tag=%d\n", g_deep_override.o.tag);
    printf("g_deep_override.o.in.p=%d\n", g_deep_override.o.in.p);
    printf("g_deep_override.o.in.q=%d\n", g_deep_override.o.in.q);
    printf("g_deep_override.o.trailer=%d\n", g_deep_override.o.trailer);
    for (i = 0; i < 2; i++) {
        printf("g_deep_override.pr[%d].p=%d\n", i, g_deep_override.pr[i].p);
        printf("g_deep_override.pr[%d].q=%d\n", i, g_deep_override.pr[i].q);
    }
    printf("g_deep_override_elem.o.tag=%d\n", g_deep_override_elem.o.tag);
    printf("g_deep_override_elem.o.in.p=%d\n", g_deep_override_elem.o.in.p);
    printf("g_deep_override_elem.o.in.q=%d\n", g_deep_override_elem.o.in.q);
    printf("g_deep_override_elem.o.trailer=%d\n", g_deep_override_elem.o.trailer);
    for (i = 0; i < 2; i++) {
        printf("g_deep_override_elem.pr[%d].p=%d\n", i, g_deep_override_elem.pr[i].p);
        printf("g_deep_override_elem.pr[%d].q=%d\n", i, g_deep_override_elem.pr[i].q);
    }
    printf("l_deep_override.o.tag=%d\n", l_deep_override.o.tag);
    printf("l_deep_override.o.in.p=%d\n", l_deep_override.o.in.p);
    printf("l_deep_override.o.in.q=%d\n", l_deep_override.o.in.q);
    printf("l_deep_override.o.trailer=%d\n", l_deep_override.o.trailer);
    for (i = 0; i < 2; i++) {
        printf("l_deep_override.pr[%d].p=%d\n", i, l_deep_override.pr[i].p);
        printf("l_deep_override.pr[%d].q=%d\n", i, l_deep_override.pr[i].q);
    }
    return 0;
}
