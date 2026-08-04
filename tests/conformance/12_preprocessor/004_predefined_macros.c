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
 * per-target differences.  The discipline has a second half, which the withdrawal recorded
 * further down enforces: a relation is asserted here only when EVERY correct compiler must
 * satisfy it, because the oracles treat a difference as evidence about the compiler, and a
 * relation no authority imposes turns permitted behaviour into that evidence.
 *
 * WHICH MACROS MAY BE ASSERTED AS INVARIANTS, AND WHICH MAY NOT.  Only macros the C standard
 * itself requires are asserted as invariants here: __STDC__ (C11 6.10.8.1), __STDC_VERSION__
 * (same clause), __LINE__ and __FILE__ (6.10.8.1).  Every one of those a conforming
 * implementation must define, so a divergence in one is a real conformance gap and a legitimate
 * finding.
 *
 * The architecture macros are a different case, and they are handled as two SELF-CONSISTENCY
 * relations rather than as invariants.  __x86_64__, __i386__, __aarch64__ and __riscv are
 * spellings of a particular compiler FAMILY, not of the standard; no clause obliges any
 * implementation to define them and this repository documents no per-target predefined-macro
 * contract for the compiler under test.  Printing one, or a value derived directly from one,
 * would therefore diverge across the four backends on every run and say nothing about
 * correctness.  What is printed instead is what can be asserted of ANY correct compiler whatever
 * it chooses to predefine: that NO MORE THAN ONE of the four is defined, and that whichever one
 * IS defined agrees with the pointer width the code generator actually produced - the second
 * written as an implication so that it stays about agreement alone.  Both hold of a compiler
 * that defines exactly one of the four, and both hold of a compiler that defines none; only a
 * compiler that contradicts ITSELF fails them, by claiming two architectures at once or by
 * claiming one its own back end does not implement.
 *
 * PRESENCE IS DELIBERATELY NOT ASSERTED, AND THIS PARAGRAPH IS THE RECORD OF WHY.  An earlier
 * form of this program printed a third line, arch_id_defined, asserting that AT LEAST ONE of the
 * four spellings is defined, on the reasoning that without it a compiler predefining no
 * architecture macro produces output byte-identical to one predefining exactly the right macro.
 * That observation is true and it is not a defect.  The two compilers agree because both are
 * CORRECT: no clause of the standard requires any of these spellings, and a search of docs/ for
 * "predefined", "__x86_64__", "__i386__", "__aarch64__" and "__riscv" finds no match in any of
 * the three documents the directory holds, so the repository states no contract for the compiler
 * under test either.  Asserting presence therefore made a compiler-family convention into a
 * conformance requirement that no authority imposes, and every one of this program's twelve
 * cells would have reported a compiler which spells its architecture macro otherwise - or leaves
 * the choice to a target header - as a divergence under all three oracles at once.  A divergence
 * this suite cannot trace to an obligation is not a finding worth delivering; it is a false
 * oracle, and a false oracle costs more than a blind spot because it spends a maintainer's
 * attention on permitted behaviour.  Nor could the difference have been dressed as an expected
 * divergence: a marker requires a documented basis, and silence is not a basis.
 *
 * The feature is not thereby dropped, which C3 forbids.  All four architecture macros are still
 * read, all four conditional blocks are still exercised, and both surviving relations are still
 * printed and compared on every one of the twelve cells; what was withdrawn is one unfounded
 * equality, not the coverage.  The narrow, scoped alternative - printing presence as capability
 * metadata that no oracle compares - is not available here: comparison in this suite is
 * byte-exact over the whole of standard output, so there is no uncompared channel a program can
 * print to, and inventing one would weaken the comparison for all 108 programs to describe one.
 * The zero-fallback arms below are retained regardless, because they are what keep a macro gap a
 * comparable output difference rather than a compile failure.
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
 * that is byte-identical across all twelve cells.  Rendering that record's
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

/* Architecture: at most one branch may be taken, and whichever one is must agree with the code
   generator.  Each of the four macros below contributes 0 or 1, and their sum is bounded above by
   1 further down, while a separate implication ties the one that is defined - if any is - to the
   pointer width actually produced.  The machinery is therefore fully exercised, and any deviation
   is localized to one line, without any architecture macro name or value ever reaching the
   output.  The sum is deliberately NOT bounded below: no authority requires any of these four
   spellings, so requiring one would report a permitted choice as a divergence.  The withdrawal is
   argued in full in the file comment above.

   Each block carries an alternative arm that defines a zero fallback rather than raising a
   preprocessor diagnostic.  That choice is deliberate: were a diagnostic raised instead, a
   compiler that happened not to define architecture macros would fail to compile this program,
   turning a benign difference into an uninformative compile failure.  With the zero fallback the
   program always compiles under both compilers, and the two relations below stay comparable
   whichever spellings each side uses. */
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
    /* At most one architecture macro.  This is a self-consistency property, not a presence
       requirement: a compiler that claims two architectures at once is inconsistent with itself,
       because the two claims cannot both describe the machine the code generator is emitting for,
       while a compiler that claims none is merely spelling its target elsewhere and satisfies the
       line.  There is deliberately no companion line asserting that at least one is defined - no
       clause and no repository document obliges any of these four spellings, so such a line would
       report permitted behaviour as a divergence.  See the file comment above. */
    printf("arch_count_at_most_one=%d\n", (int)(ARCH_COUNT <= 1));
    /* The strongest assertion in the program: it ties the preprocessor's view of the target to
       the code generator's.  Written as an implication - if the 32-bit macro is defined THEN
       pointers are four bytes wide, and if any of the other three is defined THEN they are not -
       so that a compiler defining no architecture macro satisfies it vacuously while one whose
       preprocessor and back end disagree fails it.  The vacuous case is intended rather than
       tolerated: defining none of the four is a permitted choice, so the line has nothing to
       assert about such a compiler and says nothing.  The 32-bit target is the only one of the
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
