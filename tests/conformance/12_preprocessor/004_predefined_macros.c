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
 * WHICH MACROS MAY BE ASSERTED AS INVARIANTS, AND WHICH MAY NOT.  Only macros the C standard
 * itself requires are asserted as invariants here: __STDC__ (C11 6.10.8.1), __STDC_VERSION__
 * (same clause), __LINE__ and __FILE__ (6.10.8.1).  Every one of those a conforming
 * implementation must define, so a divergence in one is a real conformance gap and a legitimate
 * finding.
 *
 * The architecture macros are a different case, and an earlier form of this program got it
 * wrong.  __x86_64__, __i386__, __aarch64__ and __riscv are spellings of a particular compiler
 * FAMILY, not of the standard; no clause obliges any implementation to define them and this
 * repository documents no per-target predefined-macro contract for the compiler under test.
 * Asserting that exactly one of them is defined therefore accuses a correct compiler of a
 * defect merely for spelling its architecture macro differently, or for leaving the choice to a
 * target header - a false report manufactured by the test.  They are still exercised, because
 * dropping them would be excluding a feature for being awkward, but only through assertions no
 * correct compiler can fail: that NO MORE THAN ONE of the four is defined, which is a
 * consistency property rather than a naming one, and that ANY macro that IS defined agrees with
 * the pointer width the code generator actually produced, written as an implication so that an
 * undefined macro satisfies it vacuously.  Those two together still catch the defect worth
 * catching - a preprocessor that disagrees with its own back end, or that claims two
 * architectures at once - while a differently spelled or absent macro set passes.
 *
 * The strict-conformance macro is handled the same way, and for the same reason.  __STRICT_ANSI__
 * is a compiler-family spelling with no standard mandate; whether an implementation defines it
 * is its own business, and both of this suite's oracles pass no standard-selection flag to
 * either side.  A correct compiler may define it, so its absence is asserted NOWHERE.  What is
 * asserted instead is the property that actually matters to the corpus: that the language level
 * on offer meets the floor the programs are written against, which the standard-version macro
 * states directly.
 *
 * Self-contained: no header is included at all, and printf is hand-declared, because the
 * compiler under test bundles only freestanding headers and ships no hosted input/output
 * header.  Including one would succeed under the reference compiler and fail under the
 * compiler under test - a divergence caused by the test rather than by the compiler.  This one
 * file is therefore everything the source half of an isolated reproduction needs: it compiles,
 * links and runs on its own.  The other half is the sibling expectation record
 * 004_predefined_macros.expected, which is committed beside it and supplies the rest: all four
 * targets, all three optimization levels, all three oracles enabled, and a golden stdout
 * measured on this branch and byte-identical across all twelve cells.  Rendering that record's
 * three command templates is all an isolated reproduction takes - no harness, no Cargo and no
 * Rust toolchain.
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

/* Architecture: at most one branch may be taken.  Each of the four macros below contributes 0
   or 1 and their sum is compared against 1 further down as an INEQUALITY, so the architecture
   machinery is exercised without any architecture macro name or value ever reaching the output
   and without requiring any particular spelling to exist.

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

/* Language mode, again as relations, and only over macros the standard requires.  The
   conformance macro is asserted to equal one; the standard-version macro is asserted only
   through inequalities, so neither compiler is pinned to a literal edition while both still
   have to clear the same floor the corpus is written against.  No assertion is made about the
   strict-conformance macro in either direction: it is a compiler-family spelling that no clause
   obliges an implementation to define or to omit, so requiring either would be a false oracle.
   Whether GNU extensions are on offer is exercised where it belongs - in feature area 08, by
   programs that USE an extension - rather than inferred here from a macro's presence. */
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
int main(void)
{
    /* One line per semantic property claimed, so a single divergent line localizes the defect
       to one construct rather than to the program as a whole. */
    printf("stdc_is_one=%d\n", STDC_IS_ONE);
    printf("stdc_at_least_c99=%d\n", STDC_AT_LEAST_C99);
    printf("stdc_at_least_c11=%d\n", STDC_AT_LEAST_C11);
    /* At most one architecture macro, never exactly one.  A compiler that defines none of the
       four spellings this program knows about is not thereby defective, but a compiler that
       claims two architectures at once is: the two claims cannot both describe the machine the
       code generator is emitting for. */
    printf("arch_count_at_most_one=%d\n", (int)(ARCH_COUNT <= 1));
    /* The strongest assertion in the program: it ties the preprocessor's view of the target to
       the code generator's.  Written as an implication - if the 32-bit macro is defined THEN
       pointers are four bytes wide, and if any of the other three is defined THEN they are not -
       so that a compiler defining no architecture macro satisfies it vacuously while one whose
       preprocessor and back end disagree fails it.  The 32-bit target is the only one of the
       four whose pointers are four bytes wide.  The width literal is written unsigned so that
       comparing it against the unsigned result of sizeof cannot trip a sign-comparison
       diagnostic. */
    printf("arch_id_agrees_with_pointer_width=%d\n",
           (int)((ARCH_I386 == 0 || sizeof(void *) == 4u)
                 && (ARCH_X86_64 + ARCH_AARCH64 + ARCH_RISCV == 0 || sizeof(void *) != 4u)));
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
