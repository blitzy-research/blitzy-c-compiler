/* Mixed positional and designated initializers inside nested aggregates -- the
 * hardest combination in this area, because it puts the nested, the partial and
 * the designated forms into one initializer and so forces the compiler to track
 * the C11 "current object" (6.7.9) correctly as it descends into a brace,
 * applies a designator there, ascends again, and resumes positionally.
 *
 * Six mixed forms are covered, each in its own object so a wrong slot names the
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
 *
 * Forms 1, 3 and 5 are repeated at automatic storage duration inside main, so
 * the static-image path and the run-time path are both exercised; form 3's twin
 * uses the combined [2].q spelling rather than [2] = { ... }.
 *
 * ONE SPELLING IS CARRIED BY A SIBLING PROGRAM RATHER THAN HERE, and it is stated
 * here rather than left to be noticed.
 * An OVERLAPPING designator at depth -- an inner aggregate brace-initialized and
 * then one of its members named again, or the reverse of that order -- is
 * well-defined C11: 6.7.9p19 has initialization proceed in initializer-list
 * order, so the last initializer to name a subobject supplies its value and a
 * brace covering a whole subobject discards whatever earlier initializers had
 * placed inside it. It is worth testing separately from the flat override,
 * because the override has to interact correctly with the "current object"
 * descent and an implementation that resolved designators against the wrong
 * nesting level would override a sibling, or the outer object, rather than the
 * subobject named. It IS tested, in both nesting directions, by this area's
 * designated-array program: 004_designated_array.c carries g_over_row_elem, which
 * braces a whole row and then names one element inside it, and g_over_elem_row,
 * which names the same element and then braces the same whole row -- identical
 * initializers in opposite order, so element [0][1] must read 20 in the first and
 * 2 in the second, which separates "the last initializer wins" from "any
 * explicitly named subobject survives". The flat, struct-member and
 * automatic-duration overrides live there too.
 * It is not duplicated here because the spelling cannot be written under the
 * authoring gate untouched: -Wextra enables -Woverride-init and the gate's -Werror
 * makes it fatal, every override spelling measured against that gate with the
 * reference compiler -- the flat and the nested ones alike, enumerated in
 * 004_designated_array.c -- is rejected, and no admissible gate deviation exists,
 * because the gate's removable members are exactly -pedantic, for the
 * GCC-extensions area, and -Wconversion with -Wsign-conversion, for the
 * deliberate narrowing program, so -Wextra is not removable and area 03 is
 * granted no deviation at all. 004_designated_array.c therefore brackets its
 * overriding declarations with matched push/pop diagnostic-control directives that
 * suppress -Woverride-init for those declarations alone, declares them in its own
 * record, and keeps -Werror and -Woverride-init fatal everywhere else in its
 * translation unit. Concentrating that mechanism in ONE program of the area is the
 * point: THIS translation unit stays entirely free of diagnostic-control
 * directives, so mixed positional and designated initializers are demonstrated on
 * source that needs no suppression at all and a divergence here can never be
 * confused with an artefact of a suppressed diagnostic.
 * An override carrying a side effect is excluded from the area outright, and that
 * exclusion is not about the gate: -Woverride-init-side-effects is on by default
 * and C11 6.7.9p23 leaves the sequencing of an initializer list's side effects
 * unspecified, which requirement 1 forbids. Every override the area carries is a
 * constant expression, so it costs nothing. Constraint C3 is satisfied the way it
 * asks to be -- what is narrowed is narrow, it is scoped to one spelling, and its
 * reason is recorded here and in this program's expectation
 * record. What remains under test HERE is the harder half of the same question: forms
 * 1 through 6 make the compiler descend into a brace, apply a designator there,
 * ascend and resume positionally, and form 6's `.o = { .in.p = 42 }` with
 * `.pr = { { 43, 44 }, [1].q = 46 }` reaches two levels down and combines an
 * index with a member designator without ever revisiting a slot.
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
 * would trip the gate as well. Third, -Wextra enables -Woverride-init, so no
 * initializer here may name a subobject an earlier one already named -- which is
 * the exclusion recorded above. All three facts are properties of the REFERENCE
 * compiler's authoring gate rather than limitations of the compiler under test,
 * and all three are honoured by writing gate-clean source rather than by
 * suppressing a diagnostic: this translation unit contains no diagnostic-
 * suppression directive of any kind, so the record's claim to the unchanged
 * seven-flag gate is literally true and the audit result means what the harness
 * reports it to mean.
 *
 * Two neighbouring programs make the same choice for the same reason, so the
 * three exclusions across this area are a coherent set rather than three
 * accidents: 003_partial_zero_fill.c excludes the positional partial STRUCT
 * spelling that -Wmissing-field-initializers rejects, 002_nested_aggregate.c
 * excludes the brace-elided spelling that -Wmissing-braces rejects, and
 * 004_designated_array.c excludes the flat overlapping-designator spelling that
 * -Woverride-init rejects. Each records its own reason in its own header and its
 * own expectation record, and each keeps everything the gate does accept. */

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
    return 0;
}
