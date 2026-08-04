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
 * EVERY INITIALIZER IN THIS FILE IS FULLY BRACED, and one spelling is therefore
 * excluded from this program -- narrowly, deliberately, and stated here rather
 * than left to be noticed.  C11 6.7.9p20 permits the braces around a
 * subaggregate's initializer to be elided, in which case initializers are
 * consumed from the flat list in order as the "current object" descends and
 * ascends.  -Wall enables -Wmissing-braces and the suite's authoring gate
 * promotes every diagnostic to an error, and five elided spellings were measured
 * against that gate with the reference compiler -- `int a[2][3] = {1,2,3,4,5,6}`,
 * a struct-in-struct written flat, a struct-in-array written flat, an
 * array-in-struct written flat, and a partially braced mixture -- every one
 * rejected by -Werror=missing-braces.  No elided spelling of a flat integer list
 * survives the gate, and no admissible deviation exists either: the gate's
 * removable members are exactly -pedantic, for the GCC-extensions area, and
 * -Wconversion with -Wsign-conversion, for the deliberate narrowing program, so
 * -Wall is not removable and area 03 is granted no deviation at all.  The
 * FLAT INTEGER spelling is accordingly not written here.  A source-level
 * diagnostic-suppression directive is not an alternative here: it would neutralize a
 * gate member for a spelling this program does not need, and the area reserves that
 * mechanism for the one construct it is required to carry and cannot otherwise
 * obtain -- the overlapping designators in 004_designated_array.c, whose record
 * declares its directives and their verified scope explicitly.  Constraint C3 asks for exactly what is done instead -- the
 * exclusion is narrow, it is scoped to one spelling, and its reason is recorded
 * here and in this program's expectation record.
 *
 * THE ELISION RULE ITSELF IS STILL EXERCISED, GATE-CLEAN, ELSEWHERE IN THE AREA,
 * so what is excluded is one spelling of the rule and not the rule:
 * 007_string_array.c initializes `char g_grid_exact[2][5]` and
 * `char g_grid_room[2][6]` from unbraced string literals, which is brace elision
 * for the inner arrays and which the gate accepts unaided because a string
 * literal may initialize a character array without braces (C11 6.7.9p14), and
 * 003_partial_zero_fill.c's `{ 0 }` idiom on a nested aggregate relies on the
 * same subaggregate-consumption rule of 6.7.9p20.
 *
 * Each object owns a disjoint value range -- 1 to 4, 10 to 31, 60 to 67, 71 to
 * 74, 81 to 84, 90 to 97 -- so a value that surfaces in the wrong place names
 * the aggregate it escaped from.  Every member of every level gets its own
 * output line, 48 in total, rather than any checksum or summary value, so one
 * divergent line localizes the defect to one initializer form.
 *
 * Every value this program computes and prints is a plain int printed with %d; the
 * objects that hold them are ints, arrays of int and structs of those, and the only
 * character data is the format-string literals, whose bytes printf copies through
 * unchanged.  So neither a target-varying width nor plain-char signedness can reach the
 * output.  Only named members are ever read, never an object representation, so no
 * padding byte is observed.
 *
 * Freedom from undefined behaviour: no arithmetic is performed on the data, so no
 * overflow is reachable; every subscript is a literal or a loop counter strictly inside
 * its array bound, so the pointer arithmetic the language performs for it never leaves
 * the object; no one-past-end pointer is dereferenced and no pointer value is printed,
 * compared or converted; and nothing is shifted, cast, aliased or modified after its
 * initializer.  No header is named and printf is declared by hand,
 * because bcc ships no stdio.h -- its bundled set is the nine required freestanding
 * headers plus a bonus stdatomic.h, ten files in all (docs/project-guide.md, "9 bundled
 * freestanding headers"). */

int printf(const char *, ...);

struct inner { int p; int q; };
struct outer { int tag; struct inner in; int trailer; };
struct deep  { struct outer o; struct inner pair[2]; };

static struct outer g_outer   = { 1, { 2, 3 }, 4 };
static struct inner g_arr[3]  = { { 10, 11 }, { 20, 21 }, { 30, 31 } };
static struct deep  g_deep    = { { 60, { 61, 62 }, 63 }, { { 64, 65 }, { 66, 67 } } };
static int g_matrix[2][3]     = { { 1, 2, 3 }, { 4, 5, 6 } };
static int g_cube[2][2][2]    = { { { 1, 2 }, { 3, 4 } }, { { 5, 6 }, { 7, 8 } } };

int main(void)
{
    struct outer l_outer  = { 71, { 72, 73 }, 74 };
    struct inner l_arr[2] = { { 81, 82 }, { 83, 84 } };
    struct deep  l_deep   = { { 90, { 91, 92 }, 93 }, { { 94, 95 }, { 96, 97 } } };
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
    return 0;
}
