/* AREA 12 / PROGRAM 003 -- inclusion of bcc's own bundled freestanding headers.
 *
 * This is the ONLY program in the corpus that exercises include/ at all, which makes
 * it the one place a regression in the bundled header set becomes visible: a missing
 * macro, a wrong fixed-width typedef, a broken offsetof or a broken va_copy would
 * pass unnoticed everywhere else.  The behaviour under test is include RESOLUTION
 * itself.  No -I flag is ever passed in a differential invocation -- the shared set
 * is -o, one of -O0/-O1/-O2, and -static, and nothing more -- so every directive
 * below must resolve through the compiler's OWN bundled header path
 * (docs/technical-specifications.md line 95: "Include path resolution against -I
 * directories and bundled header paths"; line 494 names that path as include/).
 *
 * THE HEADER EXCEPTION, AND ITS LIMITS.  The corpus rule is that a program includes
 * no header and hand-declares the single libc prototype it needs.  Exactly two
 * exceptions are sanctioned, and this program is the second of them
 * (tests/conformance/README.md lines 824-835): it may include the nine REQUIRED
 * bundled freestanding headers and nothing else.  Those nine -- stddef.h, stdint.h,
 * stdarg.h, stdbool.h, limits.h, float.h, stdalign.h, stdnoreturn.h and iso646.h --
 * are exactly the set bcc ships (docs/technical-specifications.md line 19, with the
 * per-header contents tabulated at lines 202-214), and every one of them is ALSO a
 * freestanding header the reference compiler provides.  That intersection is the
 * entire reason the exception is safe rather than reckless: the program compiles
 * under both sides of oracle (a).  Restricting it to <stdarg.h> as area 07 is
 * restricted would leave eight of the nine shipped headers included by nothing,
 * which is a coverage hole rather than a discipline.  Each header is included in its
 * documented table order, and nothing here relies on one header transitively
 * supplying another's contents.
 *
 * WHAT MAY NOT BE INCLUDED, AND WHY THAT MATTERS MORE THAN IT LOOKS.  Nothing beyond
 * those nine appears below, and the standard input/output header is the instructive
 * case: bcc ships none, so including it would succeed on the reference side and fail
 * on the side under test -- a divergence manufactured by the test rather than found
 * in the compiler, which is strictly worse than having no test at all.  printf is
 * therefore still hand-declared below, exactly as in every other corpus program;
 * published output-comparison experience identifies a missing printf prototype as
 * the most common portability problem in this class of suite.  The wide-character,
 * Unicode-character and string headers are not in the bundled set either.  The bonus
 * atomics header is excluded deliberately even though bcc ships it: it is not among
 * the nine required headers, no requirement mandates it, and atomics can require
 * -latomic, which is not in the shared flag set -- so using it would risk a link
 * failure attributable to the test.  tests/conformance/README.md lines 843-845 is
 * the authority on the prohibited set; this banner names no member of it verbatim,
 * so a mechanical audit of the file for a forbidden header stays free of a comment
 * that would otherwise read as a hit.
 *
 * WIDTH DISCIPLINE -- the central design constraint of this program.  The four
 * targets do not agree on pointer or long width: i686 has a 4-byte pointer and a
 * 4-byte long while x86-64, AArch64 and RISC-V 64 have 8 of each
 * (docs/technical-specifications.md lines 457-462).  size_t, ptrdiff_t and intptr_t
 * follow the pointer, so each is 4 bytes on i686 and 8 on the other three.  Every
 * fact that depends on one of those widths is therefore printed ONLY as a 0/1
 * relation -- size_t_eq_ptr, ptrdiff_eq_ptr, intptr_eq_ptr -- or as an inequality
 * with margin -- int_max_ge_32767, dbl_dig_ge_10.  A relation is 1 on all four
 * targets precisely BECAUSE the widths move together, so the line still carries a
 * real claim about the headers while staying byte-identical everywhere.  No
 * pointer-width or long-width quantity is printed as a value anywhere -- neither a
 * size, nor a limit macro, nor the widest aligned type -- and the only place any such
 * quantity is named at all is inside the three relations above, where it is consumed
 * by a comparison and never reaches the output.  Everything printed as a value is
 * fixed by a fixed-width type or by a constant identical on all four targets.  The
 * extended-precision floating type is absent entirely: its representation was
 * measured at 16, 12, 16 and 16 bytes across the four targets (x87 80-bit versus IEEE
 * binary128), which is area 13's subject and its recorded per-oracle exclusion, never
 * this program's.
 *
 * OFFSETOF DISCIPLINE.  struct Fixed carries only int8_t and int32_t members.  That
 * is deliberate and measured: the i386 System V ABI aligns int64_t to 4 inside a
 * struct while the other three targets align it to 8, so a struct containing an
 * int64_t would yield DIFFERENT member offsets on i686 and turn a correct compiler
 * into an apparent divergence.  With int32_t members the offsets are 4 and 8 on
 * every target, so offsetof is asserted as an exact value rather than hedged.
 *
 * DETERMINISM AND UNDEFINED-BEHAVIOUR FREEDOM.  Output is a fixed 18-line sequence,
 * one line per semantic property claimed, so a single divergent line localizes the
 * defect to one header fact.  No address is printed: the alignas check appears only
 * as an alignment RESIDUE relation, and the NULL check only as a pointer COMPARISON.
 * No plain char is used where signedness could matter -- signedness is signed on
 * x86-64 and i686 and unsigned on AArch64 and RISC-V 64 -- so limits are read
 * through SCHAR_MIN/SCHAR_MAX and fixed-width values through int8_t.  No translation
 * date, translation time or source-name macro is consulted, and there is no
 * randomness, no uninitialised read and no locale-dependent formatting.  Nothing is
 * read from a file, a network endpoint, the environment or the program arguments, and
 * no storage is obtained from the heap: every input is a literal in this file, so a
 * cell is hermetic.
 * No signed overflow, no shift, no aliasing violation, no pointer arithmetic beyond
 * the alignment cast, no object modified twice between sequence points, and at most
 * one side-effecting argument per call.  main returns 0, inside the permitted 0-125
 * exit range.
 */

#include <stddef.h>
#include <stdint.h>
#include <stdarg.h>
#include <stdbool.h>
#include <limits.h>
#include <float.h>
#include <stdalign.h>
#include <stdnoreturn.h>
#include <iso646.h>

int printf(const char *, ...);

/* stdnoreturn.h is probed by presence rather than by use.  Testing the macro with
 * #if defined(noreturn) needs no non-returning function, so the program gains
 * neither an unreachable path nor any interaction with -Wmissing-noreturn, both of
 * which the strict warning gate would have to be argued with. */
#if defined(noreturn)
#  define NORETURN_MACRO_PRESENT 1
#else
#  define NORETURN_MACRO_PRESENT 0
#endif

/* iso646.h is probed twice over: for the presence of several alternative spellings
 * here, and for one of them actually driving a computation in main.  In a #if the
 * operand of defined is not macro-expanded, so naming `and`, `or` and `not` is
 * safe even though each expands to an operator. */
#if defined(and) && defined(or) && defined(not) && defined(bitand)
#  define ISO646_MACROS_PRESENT 1
#else
#  define ISO646_MACROS_PRESENT 0
#endif

/* Only int8_t and int32_t members: the i386 System V ABI aligns int64_t to 4 inside
 * a struct, so a struct containing int64_t would yield different offsets on i686. */
struct Fixed {
    int8_t a;
    int32_t b;
    int32_t c;
};

/* stdalign.h's alignas, applied to static storage and checked in main through an
 * alignment residue.  16 bytes is more than the requested alignment needs, so the
 * declaration cannot be satisfied accidentally by the object's size alone. */
alignas(8) static unsigned char aligned_buf[16];

/* stdarg.h exercised end to end: va_list, va_start, va_arg, va_end and va_copy all
 * appear, and the argument list is traversed TWICE.  The copy is taken before the
 * original has been read, so both lists start at the same position; each list is
 * created or copied exactly once and ended exactly once in the same function; and
 * the second traversal reads the copy, never the exhausted original.  Neither list
 * is read past the number of arguments actually supplied. */
static int sum_two_passes(int count, ...)
{
    va_list ap;
    va_list ap_copy;
    int total = 0;
    int i;

    va_start(ap, count);
    va_copy(ap_copy, ap);
    for (i = 0; i < count; i = i + 1) {
        total = total + va_arg(ap, int);
    }
    va_end(ap);
    for (i = 0; i < count; i = i + 1) {
        total = total + va_arg(ap_copy, int);
    }
    va_end(ap_copy);
    return total;
}

int main(void)
{
    /* stdbool.h: bool and true, one used as a value and one through iso646.h's
     * `and` below.  stddef.h: NULL, held in a pointer that is only ever compared. */
    bool flag = true;
    const void *null_ptr = NULL;

    /* limits.h.  CHAR_BIT is 8 and SCHAR_MIN/SCHAR_MAX are -128/127 on all four
     * targets, and all three expand to int-typed constant expressions, so %d is
     * correct without a cast. */
    printf("char_bit=%d\n", CHAR_BIT);
    printf("schar_min=%d schar_max=%d\n", SCHAR_MIN, SCHAR_MAX);

    /* stdint.h.  The fixed-width limits and sizes are identical on every target by
     * definition of the types; each is cast to int so the argument type matches %d
     * whatever integer type the header chose for the macro. */
    printf("int8_min=%d int8_max=%d\n", (int)INT8_MIN, (int)INT8_MAX);
    printf("uint16_max=%d\n", (int)UINT16_MAX);
    printf("sizeof_int32=%d sizeof_int64=%d\n", (int)sizeof(int32_t), (int)sizeof(int64_t));

    /* stdalign.h's alignof on fixed-width types: 4 and 1 on all four targets. */
    printf("alignof_int32=%d alignof_int8=%d\n", (int)alignof(int32_t), (int)alignof(int8_t));

    /* stddef.h's offsetof on the int64_t-free struct: exactly 4 and 8 everywhere. */
    printf("offsetof_b=%d offsetof_c=%d\n",
           (int)offsetof(struct Fixed, b), (int)offsetof(struct Fixed, c));

    /* The three width hazards, normalised into relations rather than dropped.  Each
     * prints 1 on all four targets even though the underlying widths differ -- 4
     * bytes on i686 against 8 on the other three -- because the header must define
     * size_t, ptrdiff_t and intptr_t to follow the target's pointer width. */
    printf("size_t_eq_ptr=%d\n", (int)(sizeof(size_t) == sizeof(void *)));
    printf("ptrdiff_eq_ptr=%d\n", (int)(sizeof(ptrdiff_t) == sizeof(void *)));
    printf("intptr_eq_ptr=%d\n", (int)(sizeof(intptr_t) == sizeof(void *)));

    /* limits.h again, as the inequality the C standard guarantees, so the line holds
     * on a 16-bit-int implementation as well as on these four. */
    printf("int_max_ge_32767=%d\n", (int)(INT_MAX >= 32767));

    /* stddef.h's NULL: a comparison against a null pointer constant, never a printed
     * address. */
    printf("null_compares_equal=%d\n", (int)(null_ptr == (const void *)0));

    /* stdbool.h and iso646.h together: `and` drives a real expression, so the header
     * is genuinely exercised rather than merely included. */
    printf("bool_true=%d bool_iso646_and=%d\n", (int)flag, (int)(flag and true));
    printf("iso646_macros_present=%d\n", ISO646_MACROS_PRESENT);

    /* float.h.  FLT_RADIX is an int-typed constant expression and is 2 on all four
     * targets; DBL_DIG appears only inside an inequality, because its exact value is
     * a property the corpus does not print raw. */
    printf("flt_radix=%d dbl_dig_ge_10=%d\n", FLT_RADIX, (int)(DBL_DIG >= 10));

    /* alignas verified as a residue relation.  The cast target uintptr_t comes from
     * stdint.h and is exactly pointer width on every target, so the conversion
     * crosses no width boundary; the modulus is unsigned throughout, which is what
     * keeps -Wsign-conversion satisfied. */
    printf("alignas_residue_ok=%d\n", (int)(((uintptr_t)aligned_buf % 8u) == 0u));

    /* stdnoreturn.h, by presence. */
    printf("noreturn_macro_present=%d\n", NORETURN_MACRO_PRESENT);

    /* stdarg.h: 10 + 20 + 30 traversed twice through va_copy, so 120. */
    printf("varargs_two_passes=%d\n", sum_two_passes(3, 10, 20, 30));

    return 0;
}
