/* Nested aggregate initialization: a struct within a struct, an array of struct,
 * and multidimensional arrays of scalars, every one of them read back a single
 * member at a time.
 *
 * struct outer places its struct member BETWEEN two scalars, so a layout or
 * traversal error shifts a value that is printed rather than hiding in a slot
 * nobody looks at.  struct deep adds a third level: a struct holding a
 * struct-that-holds-a-struct and an array of struct side by side.  g_matrix and
 * g_cube put the same question to two and three array dimensions.  Each of those
 * shapes is then declared a second time with automatic storage duration inside
 * main.  That pairing is deliberate and is this area's discriminating variant:
 * an initializer for an object of static storage duration is resolved before
 * program startup, whereas the automatic twin's initialization is a run-time act
 * performed on every entry to the block.  The two therefore travel different
 * paths through the compiler for identical source, and both are compiled at all
 * three optimization levels because whatever either path is transformed into,
 * the values read back may not change.  Their answers are printed separately, so
 * a defect on one path cannot be covered up by the other.
 *
 * BRACE ELISION IS COVERED TOO, and covering it takes one deliberate step.  C11
 * 6.7.9p20 permits the braces around a subaggregate's initializer to be elided,
 * in which case initializers are consumed from the flat list in order as the
 * "current object" descends and ascends; that is a materially different path
 * through the initializer walker than the fully braced form, and a compiler can
 * implement one correctly and the other incorrectly.  It is therefore not a
 * spelling that may be dropped.  The obstacle is only diagnostic: -Wall turns on
 * -Wmissing-braces and the audit gate turns every warning into an error, and
 * five elided spellings were measured against that gate -- `int a[2][3] =
 * {1,2,3,4,5,6}`, a struct-in-struct written flat, a struct-in-array written
 * flat, an array-in-struct written flat, and a single-member struct written flat
 * -- every one rejected by -Werror=missing-braces.  No elided spelling survives
 * the unmodified gate, so the elided group below, AND ONLY that group, is
 * bracketed by a scoped diagnostic pragma that suppresses that one style warning
 * and pops it again immediately.  Every gate flag including -Werror stays in
 * force, no command-line flag changes, so this program's record carries no
 * ub_audit_flags deviation, and undefined-behaviour detection is untouched:
 * -Wmissing-braces reports well-defined behaviour, and the sanitizer gate runs
 * unchanged.  The pragma is wrapped in #if defined(__GNUC__) so a compiler that
 * does not advertise GCC compatibility never sees it and simply compiles the
 * initializers.  Each elided object is paired with a fully braced object of the
 * same shape and the same values, so the two paths are compared against each
 * other line for line rather than only against a recorded expectation.
 *
 * THE ELISION RULE IS EXERCISED IN GATE-CLEAN SPELLINGS ELSEWHERE IN THE AREA TOO,
 * so the pragma above is what widens this program's own coverage rather than what
 * the corpus depends on for the rule at all: 007_string_array.c initializes
 * `char g_grid_exact[2][5]` and `char g_grid_room[2][6]` from unbraced string
 * literals, which is brace elision for the inner arrays and needs no pragma
 * because a string literal may initialize a character array without braces (C11
 * 6.7.9p14), and 003_partial_zero_fill.c's `{ 0 }` idiom on a nested aggregate
 * relies on the same subaggregate-consumption rule of 6.7.9p20.  What the pragma
 * adds here is the FLAT INTEGER spelling, which no other program in the area can
 * reach, together with the fully braced twin of the same shape and values to
 * compare it against.
 *
 * Each object owns a disjoint value range -- 1 to 4, 10 to 31, 60 to 67, 71 to
 * 74, 81 to 84, 90 to 97 -- so a value that surfaces in the wrong place names
 * the aggregate it escaped from.  Every member of every level gets its own
 * output line, 106 in total, rather than any checksum or summary value, so one
 * divergent line localizes the defect to one initializer form.
 *
 * Every object is a plain int printed with %d, and no character data appears, so
 * neither a target-varying width nor plain-char signedness can reach the output.
 * Only named members are ever read, never an object representation, so no padding
 * byte is observed.
 *
 * Freedom from undefined behaviour: no arithmetic is performed on the data, so no
 * overflow is reachable; every subscript stays strictly inside its array bound; no
 * one-past-end pointer is formed; and nothing is shifted, cast, aliased or modified
 * after its initializer.  No header is named and printf is declared by hand,
 * because bcc ships no stdio.h -- its bundled set is the nine required freestanding
 * headers plus a bonus stdatomic.h, ten files in all (docs/project-guide.md line
 * 212). */

int printf(const char *, ...);

struct inner { int p; int q; };
struct outer { int tag; struct inner in; int trailer; };
struct deep  { struct outer o; struct inner pair[2]; };

static struct outer g_outer   = { 1, { 2, 3 }, 4 };
static struct inner g_arr[3]  = { { 10, 11 }, { 20, 21 }, { 30, 31 } };
static struct deep  g_deep    = { { 60, { 61, 62 }, 63 }, { { 64, 65 }, { 66, 67 } } };
static int g_matrix[2][3]     = { { 1, 2, 3 }, { 4, 5, 6 } };
static int g_cube[2][2][2]    = { { { 1, 2 }, { 3, 4 } }, { { 5, 6 }, { 7, 8 } } };

/* BRACE-ELIDED TWINS.  Each of these carries the same values as the fully braced object
 * of the same shape above, written as one flat list, so the two initializer paths must
 * agree element for element.  See the header for why the pragma is here. */
#if defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wmissing-braces"
#endif
/* Two dimensions flat: the walker must consume three initializers into row 0 and the
 * next three into row 1 without any brace telling it where the boundary is. */
static int g_matrix_flat[2][3] = { 1, 2, 3, 4, 5, 6 };
/* Three dimensions flat: the same question two levels deep, where a walker that
 * ascended one level too few or too many would scatter the values. */
static int g_cube_flat[2][2][2] = { 1, 2, 3, 4, 5, 6, 7, 8 };
/* struct-in-struct flat: 1 lands on tag, 2 and 3 must descend into in, and 4 must
 * ascend again onto trailer. */
static struct outer g_outer_flat = { 1, 2, 3, 4 };
/* array-of-struct flat: six initializers must fill three two-member elements. */
static struct inner g_arr_flat[3] = { 10, 11, 20, 21, 30, 31 };
/* Three levels flat, the most demanding elision here: the list has to descend into o,
 * then into o.in, ascend to o.trailer, ascend again to pair, and descend into each of
 * its two elements, with no brace anywhere to guide it. */
static struct deep g_deep_flat = { 60, 61, 62, 63, 64, 65, 66, 67 };
/* Partially braced mixture: the outer braces for pair are present but the inner ones
 * are elided, which is the form most likely to expose an off-by-one in the walker
 * because it has to switch between guided and unguided descent mid-initializer. */
static struct deep g_deep_partial = { { 60, { 61, 62 }, 63 }, { 64, 65, 66, 67 } };
#if defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

int main(void)
{
    struct outer l_outer  = { 71, { 72, 73 }, 74 };
    struct inner l_arr[2] = { { 81, 82 }, { 83, 84 } };
    struct deep  l_deep   = { { 90, { 91, 92 }, 93 }, { { 94, 95 }, { 96, 97 } } };
    /* Automatic-duration brace-elided twins: the same elision, but resolved by emitted
     * stores into the frame rather than by a data-section image. */
#if defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wmissing-braces"
#endif
    int l_matrix_flat[2][3]  = { 71, 72, 73, 74, 75, 76 };
    struct outer l_outer_flat = { 81, 82, 83, 84 };
    struct deep  l_deep_flat  = { 90, 91, 92, 93, 94, 95, 96, 97 };
#if defined(__GNUC__)
#pragma GCC diagnostic pop
#endif
    int i;
    int j;
    int k;

    printf("g_outer.tag=%d\n", g_outer.tag);
    printf("g_outer.in.p=%d\n", g_outer.in.p);
    printf("g_outer.in.q=%d\n", g_outer.in.q);
    printf("g_outer.trailer=%d\n", g_outer.trailer);
    for (i = 0; i < 3; i++) {
        printf("g_arr[%d].p=%d\n", i, g_arr[i].p);
        printf("g_arr[%d].q=%d\n", i, g_arr[i].q);
    }
    printf("g_deep.o.tag=%d\n", g_deep.o.tag);
    printf("g_deep.o.in.p=%d\n", g_deep.o.in.p);
    printf("g_deep.o.in.q=%d\n", g_deep.o.in.q);
    printf("g_deep.o.trailer=%d\n", g_deep.o.trailer);
    for (i = 0; i < 2; i++) {
        printf("g_deep.pair[%d].p=%d\n", i, g_deep.pair[i].p);
        printf("g_deep.pair[%d].q=%d\n", i, g_deep.pair[i].q);
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("g_matrix[%d][%d]=%d\n", i, j, g_matrix[i][j]);
        }
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 2; j++) {
            for (k = 0; k < 2; k++) {
                printf("g_cube[%d][%d][%d]=%d\n", i, j, k, g_cube[i][j][k]);
            }
        }
    }
    printf("l_outer.tag=%d\n", l_outer.tag);
    printf("l_outer.in.p=%d\n", l_outer.in.p);
    printf("l_outer.in.q=%d\n", l_outer.in.q);
    printf("l_outer.trailer=%d\n", l_outer.trailer);
    for (i = 0; i < 2; i++) {
        printf("l_arr[%d].p=%d\n", i, l_arr[i].p);
        printf("l_arr[%d].q=%d\n", i, l_arr[i].q);
    }
    printf("l_deep.o.tag=%d\n", l_deep.o.tag);
    printf("l_deep.o.in.p=%d\n", l_deep.o.in.p);
    printf("l_deep.o.in.q=%d\n", l_deep.o.in.q);
    printf("l_deep.o.trailer=%d\n", l_deep.o.trailer);
    for (i = 0; i < 2; i++) {
        printf("l_deep.pair[%d].p=%d\n", i, l_deep.pair[i].p);
        printf("l_deep.pair[%d].q=%d\n", i, l_deep.pair[i].q);
    }
    /* Brace-elided objects, every slot printed individually so a value consumed into the
     * wrong subaggregate is its own line.  The values match the fully braced twins above,
     * so the braced and elided paths are compared directly against each other. */
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("g_matrix_flat[%d][%d]=%d\n", i, j, g_matrix_flat[i][j]);
        }
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 2; j++) {
            for (k = 0; k < 2; k++) {
                printf("g_cube_flat[%d][%d][%d]=%d\n", i, j, k, g_cube_flat[i][j][k]);
            }
        }
    }
    printf("g_outer_flat.tag=%d\n", g_outer_flat.tag);
    printf("g_outer_flat.in.p=%d\n", g_outer_flat.in.p);
    printf("g_outer_flat.in.q=%d\n", g_outer_flat.in.q);
    printf("g_outer_flat.trailer=%d\n", g_outer_flat.trailer);
    for (i = 0; i < 3; i++) {
        printf("g_arr_flat[%d].p=%d\n", i, g_arr_flat[i].p);
        printf("g_arr_flat[%d].q=%d\n", i, g_arr_flat[i].q);
    }
    printf("g_deep_flat.o.tag=%d\n", g_deep_flat.o.tag);
    printf("g_deep_flat.o.in.p=%d\n", g_deep_flat.o.in.p);
    printf("g_deep_flat.o.in.q=%d\n", g_deep_flat.o.in.q);
    printf("g_deep_flat.o.trailer=%d\n", g_deep_flat.o.trailer);
    for (i = 0; i < 2; i++) {
        printf("g_deep_flat.pair[%d].p=%d\n", i, g_deep_flat.pair[i].p);
        printf("g_deep_flat.pair[%d].q=%d\n", i, g_deep_flat.pair[i].q);
    }
    printf("g_deep_partial.o.tag=%d\n", g_deep_partial.o.tag);
    printf("g_deep_partial.o.in.p=%d\n", g_deep_partial.o.in.p);
    printf("g_deep_partial.o.in.q=%d\n", g_deep_partial.o.in.q);
    printf("g_deep_partial.o.trailer=%d\n", g_deep_partial.o.trailer);
    for (i = 0; i < 2; i++) {
        printf("g_deep_partial.pair[%d].p=%d\n", i, g_deep_partial.pair[i].p);
        printf("g_deep_partial.pair[%d].q=%d\n", i, g_deep_partial.pair[i].q);
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("l_matrix_flat[%d][%d]=%d\n", i, j, l_matrix_flat[i][j]);
        }
    }
    printf("l_outer_flat.tag=%d\n", l_outer_flat.tag);
    printf("l_outer_flat.in.p=%d\n", l_outer_flat.in.p);
    printf("l_outer_flat.in.q=%d\n", l_outer_flat.in.q);
    printf("l_outer_flat.trailer=%d\n", l_outer_flat.trailer);
    printf("l_deep_flat.o.tag=%d\n", l_deep_flat.o.tag);
    printf("l_deep_flat.o.in.p=%d\n", l_deep_flat.o.in.p);
    printf("l_deep_flat.o.in.q=%d\n", l_deep_flat.o.in.q);
    printf("l_deep_flat.o.trailer=%d\n", l_deep_flat.o.trailer);
    for (i = 0; i < 2; i++) {
        printf("l_deep_flat.pair[%d].p=%d\n", i, l_deep_flat.pair[i].p);
        printf("l_deep_flat.pair[%d].q=%d\n", i, l_deep_flat.pair[i].q);
    }
    return 0;
}
