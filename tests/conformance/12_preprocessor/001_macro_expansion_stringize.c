/* Area 12 (Preprocessor), program 001 of 6 - function-like macro expansion,
   stringize (#) and token paste (##), asserted behaviourally.

   Documented basis for every feature exercised here:
     - docs/technical-specifications.md line 92 - macros.rs provides "object-like and
       function-like macro expansion with stringification (#) and token pasting (##)".
     - docs/technical-specifications.md line 491 - the same module provides the
       "recursive expansion guard", which the final line observes directly.
     - docs/project-guide.md line 79 - the preprocessor component covers object-like,
       function-like, stringification and token-pasting expansion.
   Every construct below is therefore documented as SUPPORTED, so this program carries
   no expected-divergence marker: a divergence here is a genuine finding.

   Why this program exists at all. The repository already has 68 integration tests for
   the preprocessor (docs/project-guide.md line 141), but they assert expansion
   STRUCTURALLY - they inspect the token stream. This program instead asserts that a
   preprocessed-then-compiled program's OBSERVABLE BEHAVIOUR agrees with an independent
   oracle end to end. A structural test cannot detect a wrong answer it was not written
   to anticipate; an independent oracle can.

   Self-containment. No header is included. printf is hand-declared because <stdio.h> is
   not among the freestanding headers bcc ships, so an include of it would succeed under
   the reference compiler and fail under bcc, manufacturing a divergence caused by the
   test rather than by the compiler.
   Keeping the program to a single file is what a reproducible-in-isolation program needs
   from its source half. The other half is the sibling record
   001_macro_expansion_stringize.expected, committed beside it, which supplies all four
   targets, all three optimization levels, all three oracles and a 13-line golden stdout
   byte-identical across all twelve cells. Reproducing this program by hand needs only a C
   compiler and this one file; reproducing a harness cell of it needs the record's three
   command templates and nothing else.

   Determinism. Output is a fixed 13-line sequence of key=value pairs, exactly one line
   per preprocessing property claimed, so a single divergent line localises the defect to
   one construct. No address or pointer value is printed; no plain-char value is printed,
   so the measured plain-char signedness difference (signed on x86_64 and i686, unsigned
   on aarch64 and riscv64) cannot be observed. Only int and string values are printed, so
   the measured i686 width difference (sizeof(long) and sizeof(void *) are 4 on i686 and
   8 on the other three targets, docs/technical-specifications.md lines 457-462) has no
   effect on the output. There is no timestamp, no randomness, no locale-dependent
   formatting and no uninitialised read; the translation-date, translation-time and
   source-file predefined macros are never referenced, so no printed value can vary with
   when or where the program was translated. main returns 0, inside the 0-125 range the
   suite requires.

   The two-variant rule. Lines funclike_folded and funclike_runtime apply the SAME
   function-like macro twice: once to a literal, where the constant folder answers, and
   once to an operand sourced from a volatile object, where the backend must emit a real
   multiply. Without the second variant, optimisation substitutes the folder's answer for
   the backend's at -O1 and -O2 and a code-generation defect escapes detection entirely.

   Freedom from undefined and unspecified behaviour. Every printed value is a small
   non-negative int far from any overflow boundary; there is no shift, no aliasing
   violation, no uninitialised read, no pointer arithmetic beyond walking a string
   literal to its terminator, no object modified twice between sequence points, and no
   argument to any call has a side effect. The program depends on no padding byte and on
   no relative address of unrelated objects. Nothing is allocated and nothing outside the
   program is read: every input is a literal in this source, so execution is hermetic. */
int printf(const char *, ...);

/* Length of a NUL-terminated string, computed without <string.h>.
   Its only purpose is to prove that a stringize result is a real string OBJECT with a
   terminator, not merely a token the compiler happened to accept. Walking stops at the
   terminator, so no one-past-end pointer is ever dereferenced. */
static int slen(const char *s)
{
    int n = 0;
    while (s[n] != '\0') {
        n = n + 1;
    }
    return n;
}

/* The two-level stringize and paste idioms.

   These are required by the standard, not stylistic. C11 6.10.3.2p2 makes the operand of
   # a sequence of tokens that is NOT macro-expanded, and 6.10.3.3p3 does the same for
   the operands of ##. So a single-level macro can only ever see the spelling it was
   handed. Interposing an outer macro whose replacement list mentions the parameter
   without # or ## forces the argument to be fully expanded first, and the inner macro
   then applies the operator to that expanded form. Both levels are exercised below so
   the suppressing behaviour and the expanding behaviour are asserted separately. */
#define STR1(x) #x
#define STR(x) STR1(x)
#define CAT1(a, b) a##b
#define CAT(a, b) CAT1(a, b)

/* Object-like macros, an empty one, and two function-like macros. */
#define WIDTH 32
#define NAME_PART_1 seg
#define NAME_PART_2 ment
#define EMPTY_OBJ
#define SQUARE(x) ((x) * (x))
#define SUM3(a, b, c) ((a) + (b) + (c))

/* Objects the pasted and blue-painted identifiers must resolve to.
   segment is what NAME_PART_1 pasted to ment names, so a successful paste is observable
   as a value rather than as a compilation that merely succeeded. */
static const int segment = 55;
static const int selfref = 9;

/* The recursive expansion guard, in its observable form. C11 6.10.3.4p2 forbids
   re-expanding a macro name that appears during its own replacement, so expanding
   selfref yields the identifier selfref, which is then "blue painted" and left alone. It
   therefore resolves to the object declared immediately above. A preprocessor lacking the
   guard would instead recurse forever and never produce a program to run. The definition
   deliberately follows the declaration so the identifier it yields is already in scope. */
#define selfref selfref

int main(void)
{
    /* Read the volatile source exactly once into a plain int. Passing the plain copy to
       the macro keeps the volatile access sequenced on its own, which is why no question
       of an unsequenced volatile access can arise even though the macro mentions its
       parameter twice, and it honours the rule of at most one side-effecting argument per
       call. The volatile qualifier is what denies the optimiser the value at compile
       time, so the multiply below must be materialised as an instruction. */
    volatile int vsrc = 4;
    int runtime_operand = (int)vsrc;

    /* # suppresses expansion of its operand, so the macro NAME survives verbatim. */
    printf("stringize_direct=%s\n", STR1(WIDTH));

    /* The outer level expands the argument first, so the macro VALUE is stringized. */
    printf("stringize_expanded=%s\n", STR(WIDTH));

    /* C11 6.10.3.2p2 deletes leading and trailing white space in the operand and
       collapses each internal run to a single space, so this spelling is mandated
       rather than incidental and is identical across conforming compilers. */
    printf("stringize_expression=%s\n", STR1(a + b * 2));

    /* A stringize result is a real string object, terminator included. */
    printf("stringize_length=%d\n", slen(STR1(WIDTH)));

    /* ## pastes two tokens into one identifier that must name a real object. */
    printf("paste_identifier=%d\n", CAT(NAME_PART_1, ment));

    /* Both operands are macros, and both must be expanded by the outer level before the
       inner ## joins them - the case a single-level paste cannot express. */
    printf("paste_both_expanded=%d\n", CAT(NAME_PART_1, NAME_PART_2));

    /* Pasting two digits must form the single pp-number 12, not the two tokens 1 and 2. */
    printf("paste_number=%d\n", CAT(1, 2));

    /* Composition of both operators: paste to WIDTH, rescan it to 32, then stringize. */
    printf("stringize_of_paste=%s\n", STR(CAT(WID, TH)));

    /* Folded variant - a constant expression the optimiser may answer entirely. */
    printf("funclike_folded=%d\n", SQUARE(5));

    /* Runtime variant of the SAME macro - the backend must emit a genuine multiply. */
    printf("funclike_runtime=%d\n", SQUARE(runtime_operand));

    /* Nested invocation: each argument is itself a function-like macro call, so the
       arguments must be expanded before the outer replacement list is rescanned. */
    printf("funclike_nested=%d\n", SUM3(SQUARE(2), SQUARE(3), 1));

    /* An empty object-like macro must expand to nothing at all, leaving 5 + 3. */
    printf("empty_object_macro=%d\n", 5 EMPTY_OBJ + 3);

    /* The blue-painted self-reference resolves to the object, proving the guard holds. */
    printf("self_reference_guard=%d\n", selfref);

    return 0;
}
