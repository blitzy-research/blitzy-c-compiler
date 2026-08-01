/* 001_scalar_and_array -- the baseline of the aggregate and array initializer area.
 *
 * No header is brought in and no preprocessor directive appears anywhere in this
 * file.  bcc ships no stdio.h: its bundled set is the nine required freestanding
 * headers plus a bonus stdatomic.h (docs/technical-specifications.md line 19 and
 * lines 202-214, docs/project-guide.md line 212), and no standard I/O header is
 * among them, so naming one would fail against bcc while succeeding against the
 * reference compiler -- a divergence caused by the test rather than by a compiler.
 * printf is declared by hand instead.  This area has no sanctioned header
 * exception; the only two in the corpus are area 07's <stdarg.h> and the dedicated
 * bundled-header probe in area 12.
 *
 * Every form is initialized twice: once at file scope, where the initializer is a
 * translation-time constant the compiler places in the program's data image, and
 * once inside main, where the same initializer must be materialized by generated
 * code on entry to the block.  The two storage durations exercise different paths
 * through the compiler for identical source, which is why the baseline carries both
 * rather than choosing one.  A single `struct point` definition serves both, so the
 * pair isolates storage duration instead of also varying the type.
 *
 * Every element of every array and every member of every struct is read back and
 * printed on its own line.  Nineteen lines are printed and nothing is summarized:
 * one wrong slot must produce exactly one divergent line, because that is what
 * localizes a defect to a single initializer and keeps minimization tractable.
 *
 * Every printed value has type int or unsigned int.  Both are four bytes on all
 * four targets, so %d and %u are the only conversions needed and no type whose
 * width varies by target -- long, size_t, ptrdiff_t, intptr_t, a pointer, any
 * floating type -- and no long-width length modifier ever reaches a format string.
 * That matters because i686 is ELF32 with a four-byte long and pointer while the
 * other three targets are ELF64 with eight (docs/technical-specifications.md target
 * table).  4294967295u is exactly UINT_MAX wherever unsigned int is 32 bits, which
 * is all four targets, so the widest initializer here is representable without any
 * conversion.  No character data appears at all, so the measured plain-char
 * signedness difference -- signed on x86-64 and i686, unsigned on AArch64 and
 * RISC-V 64 -- cannot reach the output.
 *
 * The program only initializes and reads, which is why it is the area's baseline.
 * Every object is fully initialized before any part of it is read; no pointer is
 * formed and no address is printed; no aliasing, no signed overflow, no shift and
 * no object modified twice between sequence points; every printf argument is free
 * of side effects.  Loop bounds are literal so iteration order is fixed, nothing
 * outside this file is read, and main returns 0, inside the 0-125 range the suite
 * requires.  Every cell is therefore reproducible from this source and its
 * expectation record alone, and the program compiles clean under the suite's
 * default warning gate with no sanctioned deviation. */
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
