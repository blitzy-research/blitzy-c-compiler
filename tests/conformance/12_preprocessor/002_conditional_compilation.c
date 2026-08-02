/* Area 12, program 002 - nested conditional compilation, defined(), and integer
 * arithmetic in a #if controlling expression, checked end to end through the whole
 * pipeline rather than structurally.
 *
 * Documented basis for every construct below:
 *   docs/technical-specifications.md line 93  - conditional.rs: "#ifdef, #ifndef, #if,
 *       #elif, #else, #endif conditional compilation evaluation"
 *   docs/technical-specifications.md line 94  - expression.rs: "Preprocessor constant
 *       expression evaluator for #if directives (integer arithmetic, defined())"
 *   docs/technical-specifications.md line 493 - expression.rs is specified to evaluate
 *       integer arithmetic, the defined() operator, logical and comparison operators
 *   docs/project-guide.md line 79 - the preprocessor component covers
 *       #define/#undef and #if/#ifdef/#elif/#else/#endif
 * Every feature exercised here is therefore documented as supported, so a divergence is
 * a genuine FINDING and no expected-divergence marker belongs to this program.
 *
 * Self-containment: no header is included at all.  bcc bundles only the nine required
 * freestanding headers and ships no <stdio.h> - a grep for "stdio" across docs/ returns
 * zero matches - so including <stdio.h> would succeed under the reference compiler and
 * fail under bcc, a spurious divergence caused by the test rather than by the compiler.
 * printf is hand-declared instead.  The one sanctioned header exception in this area is
 * program 003, the dedicated bundled-header probe, and this is not it.
 *
 * Structural discipline: no preprocessor directive ever appears inside a function-call
 * argument list.  Every conditional is resolved at file scope into a plain integer macro
 * and printed with an ordinary single-expression printf argument, which keeps each call
 * a single well-formed expression.
 *
 * Determinism: output is seven fixed key=value lines, one line per semantic property
 * claimed, so a single divergent line localizes the defect to one construct.  Only int
 * values are printed and always with %d, so neither the measured plain-char signedness
 * difference (signed on x86_64 and i686, unsigned on aarch64 and riscv64) nor the
 * measured i686 width difference can be observed.  No address or pointer value is
 * printed.  No date, time or source-file-name predefined macro is referenced, and there
 * is no randomness and no locale-dependent formatting; statement order is fixed
 * throughout, so repeated runs are byte-identical.  The program is hermetic: every
 * input is a literal in this source, nothing is opened, read from the environment or
 * allocated, and the exit status is 0, inside the permitted 0-125 range.
 *
 * Undefined-behaviour freedom, which is the precondition that makes a divergence
 * readable as evidence about a compiler at all:
 *   - No signed overflow.  Every value computed by the preprocessor is a small integer
 *     constant (largest magnitude 42) and every value computed at run time is 0 or 1.
 *   - The single shift, 1 << 4, has a shift count of 4 on a positive int, strictly
 *     within range for a 32-bit int on all four targets.
 *   - No pointers, arrays or aggregates exist, so there is no aliasing violation, no
 *     out-of-bounds access and no one-past-end pointer.
 *   - No uninitialized read: dead_marker is initialized at its declaration and read
 *     afterwards, and it is the only object in the program.
 *   - No object is modified twice between sequence points, and no call takes more than
 *     one argument with a side effect - none of the arguments here has any.
 *   - Nothing depends on padding bytes or on the addresses of unrelated objects.
 *   - -7 / 2 and 17 % 5 rely on truncation toward zero and a dividend-signed remainder,
 *     which C99 onward mandates and which were measured identical on all four targets,
 *     so no target restriction is needed and all three oracles stay enabled.
 */

int printf(const char *, ...);

/* LEVEL selects the #elif arm below; FEATURE_A is defined with an empty replacement
 * list, which defined() must still report as defined; FEATURE_B is deliberately left
 * undefined and then #undef'd anyway. */
#define LEVEL 3
#define FEATURE_A
/* #undef of a macro that was never defined is explicitly ignored rather than diagnosed
 * (C11 6.10.3.5p2), so this line must compile silently.  It also guarantees FEATURE_B is
 * undefined at every use below, independently of anything above. */
#undef FEATURE_B
#define WIDTH_BITS 32

/* Three levels of nesting inside the arm that is taken, so the branch actually selected
 * is an #elif rather than an #if or an #else.  Level 1 selects the #elif, level 2
 * combines defined() and !defined() with && in one controlling expression, and level 3
 * evaluates arithmetic on a macro-expanded operand.  Only one NESTED_RESULT definition
 * can survive; a preprocessor that took the wrong arm, or that evaluated a skipped one,
 * would either print a different value or fail to compile with a redefinition. */
#if LEVEL > 5
#  define NESTED_RESULT 1
#elif LEVEL == 3
#  if defined(FEATURE_A) && !defined(FEATURE_B)
#    if (LEVEL * 2 + 1) == 7
#      define NESTED_RESULT 7
#    else
#      define NESTED_RESULT 2
#    endif
#  else
#    define NESTED_RESULT 3
#  endif
#else
#  define NESTED_RESULT 4
#endif

/* The #ifdef spelling, distinct from defined() above and from #ifndef below. */
#ifdef FEATURE_A
#  define FEATURE_A_ON 1
#else
#  define FEATURE_A_ON 0
#endif

/* The #ifndef spelling.  FEATURE_B is undefined, so the first arm is the one taken and
 * FEATURE_B_ON is 0 - the inverted arm order is deliberate, so that a preprocessor that
 * treated #ifndef as #ifdef would print 1 here instead of 0. */
#ifndef FEATURE_B
#  define FEATURE_B_ON 0
#else
#  define FEATURE_B_ON 1
#endif

/* Integer arithmetic in one controlling expression: shift, modulo, signed division
 * truncating toward zero, and multiplication, joined by && and compared with ==. */
#if (1 << 4) == 16 && (17 % 5) == 2 && (-7 / 2) == -3 && (6 * 7) == 42
#  define PP_ARITH_OK 1
#else
#  define PP_ARITH_OK 0
#endif

/* C11 6.10.1p4: after macro replacement, every identifier remaining in a #if controlling
 * expression is replaced by 0.  Pairing the arithmetic half with the defined() half
 * proves both at once - the identifier evaluates to 0 and is genuinely not defined.
 * -Wundef is not a member of the warning gate, so this is gate-clean. */
#if NOT_DEFINED_ANYWHERE == 0 && !defined(NOT_DEFINED_ANYWHERE)
#  define UNDEF_IS_ZERO 1
#else
#  define UNDEF_IS_ZERO 0
#endif

/* A range test on a defined macro, combining >= and <= in one expression. */
#if WIDTH_BITS >= 32 && WIDTH_BITS <= 64
#  define WIDTH_IN_RANGE 1
#else
#  define WIDTH_IN_RANGE 0
#endif

/* A diagnostic directive inside a skipped group.  The preprocessor must not evaluate it,
 * so this program compiling at all is the assertion; there is no output line to check.
 * It is the only such directive in the file, and the group guarding it is never taken, so
 * the directive is unreachable by construction. */
#if 0
#  error This skipped group must never be evaluated by the preprocessor.
#endif

int main(void)
{
    /* A skipped group containing a nested group whose own condition is true.  Neither
     * assignment may be emitted: a preprocessor that failed to skip the outer group
     * would leave dead_marker at 1 or 2 and dead_block_skipped would print 0.  This is
     * the strongest observable in the program because it tests an absence, which is
     * exactly where a structural test is weakest and an oracle is strongest. */
    int dead_marker = 0;
#if 0
    dead_marker = 1;
#  if 1
    dead_marker = 2;
#  endif
#endif

    printf("nested_result=%d\n", NESTED_RESULT);
    printf("feature_a_on=%d\n", FEATURE_A_ON);
    printf("feature_b_on=%d\n", FEATURE_B_ON);
    printf("pp_arith_ok=%d\n", PP_ARITH_OK);
    printf("undef_is_zero=%d\n", UNDEF_IS_ZERO);
    printf("width_in_range=%d\n", WIDTH_IN_RANGE);
    printf("dead_block_skipped=%d\n", dead_marker == 0 ? 1 : 0);
    return 0;
}
