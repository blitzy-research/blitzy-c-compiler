/* 001_scalar_and_array -- the baseline of the aggregate and array initializer area.
 *
 * Every form is initialized twice: once at file scope, where the initializer is a
 * translation-time constant the compiler places in the program's data image, and
 * once inside main, where the same initializer must be materialized by generated
 * code on entry to the block.  The two storage durations exercise different paths
 * through the compiler for identical source, which is why the baseline carries both
 * rather than choosing one.  A single `struct point` definition serves both, so the
 * pair isolates storage duration instead of also varying the type.  Every element
 * of every array and every member of every struct is then read back on its own
 * line, so one wrong slot produces exactly one divergent line.
 *
 * Every printed value has type int or unsigned int, both four bytes on all four
 * targets, so %d and %u are the only conversions needed; 4294967295u is exactly
 * UINT_MAX wherever unsigned int is 32 bits, which is all four.  No character data
 * appears, so plain-char signedness cannot reach the output.
 *
 * The program only initializes and reads, which is why it is the area's baseline,
 * and that is also its freedom-from-undefined-behaviour argument: every object is
 * fully initialized before any part of it is read, no pointer is formed, no
 * arithmetic can overflow, and no object is modified anywhere.  Loop bounds are
 * literal, so iteration order is fixed.  No header is named -- bcc ships no
 * stdio.h, its bundled set being the nine required freestanding headers plus a
 * bonus stdatomic.h, ten files in all (docs/project-guide.md line 212) -- and
 * printf is declared by hand instead. */
int printf(const char *, ...);

static int g_scalar = 42;
static int g_arr[5] = { 10, 20, 30, 40, 50 };
static unsigned int g_u_arr[3] = { 0u, 1u, 4294967295u };

struct point { int x; int y; };
static struct point g_pt = { 7, -9 };

int main(void)
{
    int l_scalar = 42;
    int l_arr[5] = { 10, 20, 30, 40, 50 };
    struct point l_pt = { 7, -9 };
    int i;

    printf("g_scalar=%d\n", g_scalar);
    for (i = 0; i < 5; i++) {
        printf("g_arr[%d]=%d\n", i, g_arr[i]);
    }
    for (i = 0; i < 3; i++) {
        printf("g_u_arr[%d]=%u\n", i, g_u_arr[i]);
    }
    printf("g_pt.x=%d\n", g_pt.x);
    printf("g_pt.y=%d\n", g_pt.y);

    printf("l_scalar=%d\n", l_scalar);
    for (i = 0; i < 5; i++) {
        printf("l_arr[%d]=%d\n", i, l_arr[i]);
    }
    printf("l_pt.x=%d\n", l_pt.x);
    printf("l_pt.y=%d\n", l_pt.y);
    return 0;
}
