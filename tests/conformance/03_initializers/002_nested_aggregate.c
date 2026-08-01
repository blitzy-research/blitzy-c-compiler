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
 * program startup and emitted into a data section, whereas the automatic twin's
 * initialization is a run-time act performed on every entry to the block.  The
 * two therefore travel different code paths, most visibly with optimization off,
 * where the statics appear as data-section bytes while the automatic twins
 * appear as a run of stores into the frame; at a higher level either path may be
 * folded away, which is precisely why both spellings are compiled at all three
 * levels rather than at one.  Their answers are printed separately, so a defect
 * on one path cannot be covered up by the other.
 *
 * Every initializer below is FULLY BRACED, which is forced rather than
 * stylistic.  -Wall turns on -Wmissing-braces and the suite's audit gate turns
 * every warning into an error, so a brace-elided spelling -- `int a[2][3] =
 * {1,2,3,4,5,6};`, or a nested struct written flat, or a partially braced
 * mixture -- cannot be compiled under the gate at all.  This area is granted no
 * gate deviation, so the elided SPELLING is excluded: narrowly, and with its
 * reason recorded in this program's expectation record.  The FEATURE this
 * program owns -- nested aggregate initialization -- remains completely covered
 * by the braced forms below, so nothing is dropped because it is difficult.
 *
 * Each object owns a disjoint value range -- 1 to 4, 10 to 31, 60 to 67, 71 to
 * 74, 81 to 84, 90 to 97 -- so a value that surfaces in the wrong place names
 * the aggregate it escaped from.  Every member of every level gets its own
 * output line, 48 in total, rather than any checksum or summary value, so one
 * divergent line localizes the defect to one initializer form.
 *
 * Portability and determinism hold by construction.  No header is named
 * anywhere and printf is declared by hand, because bcc bundles only freestanding
 * headers and ships no standard I/O header, so naming one would fail on that
 * side while succeeding on the reference side -- a divergence caused by the test
 * rather than by a compiler.  Every object is a plain int printed with %d, so no
 * type whose width differs between the 32-bit target and the other three ever
 * reaches a format string.  No character data appears at all, so the measured
 * plain-char signedness split between the targets cannot arise.  Only named
 * members are ever read, never an object representation, so no padding byte is
 * observed; these all-int aggregates carry no internal padding on any of the four
 * targets in any case, but the rule is honoured by construction regardless.
 * Nothing address-valued, clock-derived, random or locale-dependent is printed,
 * and iteration order is fixed.
 *
 * Freedom from undefined behaviour: no arithmetic is performed on the data, so
 * no overflow is reachable; every subscript stays strictly inside its array
 * bound; no one-past-end pointer is formed; nothing is shifted, cast, aliased or
 * modified after its initializer; and every call carries at most one
 * side-effecting argument.  The exit status is 0, inside the 0 to 125 range the
 * suite requires.  Nothing outside this file is read and no input arrives from
 * anywhere but these literals, so every cell is reproducible from this source
 * and its expectation record alone. */

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
