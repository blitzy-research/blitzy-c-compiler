/* Area 12 (Preprocessor) / program 004 - predefined macros asserted as RELATIONS only.
 *
 * Architecture and standard-version macros legitimately differ per target, so this program
 * prints only normalized derived results that are identical on every target.  No literal macro
 * value is ever printed: every printed field is either a 0/1 macro resolved by one of the
 * conditional blocks below, or a comparison cast to int.  A naive program printing the
 * architecture macro or the standard-version macro verbatim would diverge across all four
 * backends on every single run and drown any real signal; the normalized form diverges only
 * when something is genuinely wrong, which is what makes this program discriminating rather
 * than noisy.
 *
 * That discipline is load-bearing rather than stylistic.  The repository documents no inventory
 * of the macros the compiler under test predefines, so no documented basis exists on which a
 * per-target difference could be recorded as an expected divergence.  Relations sidestep the
 * gap while still exercising the feature - the feature is normalized, never dropped - and any
 * divergence they do produce is a precise, actionable report rather than a wall of expected
 * per-target differences.
 *
 * Self-contained: no header is included at all, and printf is hand-declared, because the
 * compiler under test bundles only freestanding headers and ships no hosted input/output
 * header.  Including one would succeed under the reference compiler and fail under the
 * compiler under test - a divergence caused by the test rather than by the compiler.  A single
 * source file plus its sibling expectation record therefore reproduces every cell with no
 * harness at all.
 *
 * Determinism: nothing time-dependent is examined anywhere; no address or pointer value is
 * printed; no plain-char signedness-dependent value is printed; no raw type width is printed;
 * and the source-file-name macro is reduced to a path-independent relation, so the output is
 * byte-identical whichever directory the program is built and run in.  No conversion
 * specification other than the int form appears, so no string is ever interpolated into the
 * output.
 *
 * Free of undefined and unspecified behaviour by construction: no signed overflow (the only
 * arithmetic is a bounded increment over the length of a string literal, and a sum of four
 * operands each of which is 0 or 1); no shift operation of any kind, so no shift count can be
 * out of range; no aliasing violation; no uninitialized read, since n is initialized at its
 * declaration; the helper walks a string literal and stops at its terminator, so no pointer is
 * formed past the end of an object and none is dereferenced outside one; no string literal is
 * modified; no object is modified twice between sequence points; every call below passes a
 * single argument that has no side effect at all, so nothing depends on unspecified evaluation
 * order; and nothing depends on padding bytes or on the addresses of unrelated objects.  The
 * exit status is 0, well inside the 0-125 range the operating system delivers intact.
 */
int printf(const char *, ...);

/* Length of a string literal, computed here rather than taken from a hosted header so that the
   program remains a single self-contained file.  Used solely to reduce the source-file-name
   macro to a non-emptiness relation; the length itself is never printed, so no workspace path
   can leak into the output. */
static int slen(const char *s)
{
    int n = 0;
    while (s[n] != '\0') {
        n = n + 1;
    }
    return n;
}

/* Architecture: exactly one branch must be taken.  Each of the four macros below contributes 0
   or 1 and their sum is compared against 1 further down, so the architecture is verified
   without any architecture macro name or value ever reaching the output.

   Each block carries an alternative arm that defines a zero fallback rather than raising a
   preprocessor diagnostic.  That choice is deliberate: were a diagnostic raised instead, a
   compiler that happened not to define architecture macros would fail to compile this program,
   turning an informative output difference into an uninformative compile failure.  With the
   zero fallback the program always compiles under both compilers, and any macro gap surfaces
   as a comparable difference with a precise diff - report the difference, never work around
   it. */
#if defined(__x86_64__)
#  define ARCH_X86_64 1
#else
#  define ARCH_X86_64 0
#endif
#if defined(__i386__)
#  define ARCH_I386 1
#else
#  define ARCH_I386 0
#endif
#if defined(__aarch64__)
#  define ARCH_AARCH64 1
#else
#  define ARCH_AARCH64 0
#endif
#if defined(__riscv)
#  define ARCH_RISCV 1
#else
#  define ARCH_RISCV 0
#endif
#define ARCH_COUNT (ARCH_X86_64 + ARCH_I386 + ARCH_AARCH64 + ARCH_RISCV)

/* Language mode, again as relations.  The conformance macro is asserted to equal one; the
   standard-version macro is asserted only through inequalities, so neither compiler is pinned
   to a literal edition while both still have to clear the same floor.  The strict-conformance
   macro is asserted absent: the reference compiler runs in its default mode because no
   standard-selection flag is ever passed to either side, and the compiler under test has no
   such flag at all, so neither can be placed in a strict mode. */
#if defined(__STDC__) && (__STDC__ == 1)
#  define STDC_IS_ONE 1
#else
#  define STDC_IS_ONE 0
#endif
#if defined(__STDC_VERSION__) && (__STDC_VERSION__ >= 199901L)
#  define STDC_AT_LEAST_C99 1
#else
#  define STDC_AT_LEAST_C99 0
#endif
#if defined(__STDC_VERSION__) && (__STDC_VERSION__ >= 201112L)
#  define STDC_AT_LEAST_C11 1
#else
#  define STDC_AT_LEAST_C11 0
#endif
#if defined(__STRICT_ANSI__)
#  define STRICT_ANSI_ABSENT 0
#else
#  define STRICT_ANSI_ABSENT 1
#endif

int main(void)
{
    /* One line per semantic property claimed, so a single divergent line localizes the defect
       to one construct rather than to the program as a whole. */
    printf("stdc_is_one=%d\n", STDC_IS_ONE);
    printf("stdc_at_least_c99=%d\n", STDC_AT_LEAST_C99);
    printf("stdc_at_least_c11=%d\n", STDC_AT_LEAST_C11);
    printf("strict_ansi_absent=%d\n", STRICT_ANSI_ABSENT);
    printf("arch_count_is_one=%d\n", (int)(ARCH_COUNT == 1));
    /* The strongest assertion in the program: it ties the preprocessor's view of the target to
       the code generator's.  The 32-bit target is the only one of the four whose pointers are
       four bytes wide, so the two sides of this equality are both true there and both false
       everywhere else, and the equality itself holds on every target.  The width literal is
       written unsigned so that comparing it against the unsigned result of sizeof cannot trip a
       sign-comparison diagnostic. */
    printf("arch_id_matches_pointer_width=%d\n",
           (int)((ARCH_I386 == 1) == (sizeof(void *) == 4u)));
    /* A genuine four-target invariant: pointer width equals long width on all four targets,
       even though the shared value differs between the 32-bit target and the other three. */
    printf("pointer_width_eq_long_width=%d\n", (int)(sizeof(void *) == sizeof(long)));
    /* Both source-position macros are examined as relations only.  The line number's literal
       value is never printed, so this assertion stays correct under any later edit to the
       comments or the layout above it, and the file name is reduced to non-emptiness, so no
       absolute path can reach the output. */
    printf("line_macro_positive=%d\n", (int)(__LINE__ > 0));
    printf("file_macro_nonempty=%d\n", (int)(slen(__FILE__) > 0));
    return 0;
}
