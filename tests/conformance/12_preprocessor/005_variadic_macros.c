/*
 * Area 12 / program 005 - variadic macros, INCLUDING the zero-argument case.
 *
 * WHAT IS UNDER TEST
 *
 * The preprocessor's handling of a function-like macro declared with a trailing
 * ellipsis: forwarding the variadic list at three different arities, forwarding
 * it onward through a second variadic macro, invoking a bare variadic macro with
 * no arguments at all, and applying the stringize operator `#` to the variadic
 * list when that list is empty, a single token, and several tokens.
 *
 * Nothing here touches the runtime argument-retrieval machinery. That belongs to
 * area 07 and, within this area, to program 003; conflating the two would blur a
 * preprocessor defect into a code-generation one. Every helper below is ordinary
 * fixed-arity C, so a divergence can only have come from the expansion.
 *
 * DOCUMENTED BASIS, AND WHY THIS PROGRAM CARRIES NO MARKER
 *
 * Variadic macros are documented as supported. docs/technical-specifications.md
 * line 491 describes the preprocessor's macro module as providing
 * "stringification (`#`), token pasting (`##`), variadic macros
 * (`__VA_ARGS__`)", and docs/project-guide.md line 79 lists "macro expansion
 * (object-like, function-like, variadic, stringification, token pasting)".
 * Because the feature is documented as supported, a divergence this program
 * provokes - the zero-argument case emphatically included - is a genuine FINDING
 * rather than an expected divergence. No marker belongs in the sibling
 * expectation record, and the register admits none for this area.
 *
 * THE ZERO-ARGUMENT CASE, AND WHY IT IS EXERCISED FOUR TIMES
 *
 * A macro of the form `LOG(fmt, ...)` whose body forwards the variadic list
 * after a comma leaves that comma dangling when the list is empty. That
 * trailing-comma problem is what makes the zero-argument case the one
 * preprocessor implementations most often get wrong, so it is exercised through
 * two mutually independent techniques - a divergence in one then still leaves
 * the other reporting, which localises a defect instead of merely announcing it:
 *
 *   1. the GNU comma-swallowing form `, ##__VA_ARGS__`, in which the paste
 *      operator deletes the preceding comma when the variadic list is empty
 *      (LOG and WRAP_LOG below);
 *   2. a bare `#define M(...)` invoked as `M()`, which expands to a call whose
 *      argument list is empty, applied to a function declared to take `void`
 *      (NOARG and ADD_NONE below).
 *
 * Four distinct zero-variadic-argument invocations therefore appear in main:
 * one through the comma swallow, one through the comma swallow at a second level
 * of expansion, and two through the bare form. Constraint C3 forbids dropping a
 * construct because it is difficult, and this is the construct that rule exists
 * to protect.
 *
 * NESTED FORWARDING
 *
 * WRAP_LOG is a variadic macro whose body invokes another variadic macro. It is
 * the second-highest-yield construct here: an implementation that special-cases
 * the comma swallow at one level of expansion but not through a second will
 * break exactly there, and testing it at both zero and one variadic argument is
 * what distinguishes a broken swallow from a broken forward.
 *
 * TWO SPELLINGS DELIBERATELY AVOIDED
 *
 * The C2x optional-comma variadic operator is not used anywhere in this program.
 * No repository document enumerates support for it, so a compile failure arising
 * from it would be attributable to this test rather than to the compiler under
 * test - the one kind of divergence a differential suite must never manufacture.
 *
 * An explicit empty trailing macro argument is not used either. The reference
 * compiler itself rejects that spelling with a syntax error, so it could not
 * form the basis of a comparison between two compilers in the first place.
 *
 * STRINGIZING A VARIADIC LIST
 *
 * VA_STR is itself declared variadic, because a one-parameter stringizing macro
 * handed a comma-separated list is an error - "passed 2 arguments, but takes
 * just 1" - rather than a comparison. C11 6.10.3.2p2 fixes the spelling of the
 * result: leading and trailing white space is deleted and each internal run of
 * white space collapses to a single space, so stringizing the two-token list
 * yields "a, b" under any conforming implementation and the comparison is
 * deterministic rather than incidental.
 *
 * HEADERS, DETERMINISM AND HERMETICITY
 *
 * No header is used. printf is hand-declared instead, because the compiler under
 * test ships no standard input/output header at all, so naming one would fail
 * against it while succeeding against the reference compiler - a divergence
 * caused by the test rather than by either compiler. The corpus sanctions
 * exactly two header exceptions, area 07 and program 003 of this area; this
 * program is neither of them, and needs neither.
 *
 * Output is a fixed sequence of thirteen `key=value` lines, one per semantic
 * property claimed, so a single divergent line localises the defect to one
 * construct. No address or pointer value is printed, no value whose plain-char
 * signedness would matter, and no width-dependent quantity, so all four targets
 * print identical bytes at every optimization level. No translation-time macro
 * reporting the date, the time or the source file name is referenced, nothing is
 * random, and nothing is read before it is written. Every input is a literal in
 * this file: the program opens no external resource, consults no environment,
 * allocates nothing dynamically, and returns 0 - inside the 0-125 range the
 * suite requires so that no status is truncated.
 *
 * UNDEFINED-BEHAVIOUR FREEDOM
 *
 * Every arithmetic operand is a small int constant, so no signed overflow is
 * possible and no shift appears at all. There is no cast, no union and no
 * pointer arithmetic beyond indexing a string literal up to and including its
 * terminator, so no aliasing rule and no bound is stressed. No object is
 * modified twice between sequence points, and no call takes more than one
 * side-effecting argument. Each printf conversion matches its argument's
 * promoted type: %d receives int, and %s receives a string literal produced by
 * the stringize operator.
 */
int printf(const char *, ...);

/*
 * Length of a terminated string, computed without recourse to any header.
 *
 * This exists to assert an omission - that `#` applied to an empty variadic list
 * yields an empty string rather than something spurious. Printing the string
 * itself could not distinguish an empty result from a run of spaces, whereas its
 * length can, and an assertion about an omission is precisely where an oracle
 * beats a structural check.
 *
 * The loop terminates because every argument passed here is a string literal
 * produced by the stringize operator, which always supplies a terminator, and it
 * stops at that terminator rather than reading past it. The counter advances at
 * most a handful of times, so its increment cannot overflow.
 */
static int slen(const char *s)
{
    int n = 0;
    while (s[n] != '\0') {
        n = n + 1;
    }
    return n;
}

/*
 * First of the two zero-parameter helpers reached through a bare `#define M(...)`
 * invoked with no arguments.
 *
 * Declared to take `void`, so an expansion that failed to produce an empty
 * argument list would be a diagnosable error rather than a silently different
 * call. Two distinct helpers are used rather than one so that a divergence in
 * either bare-form expansion cannot be masked by the other.
 */
static int add0(void)
{
    return 0;
}

/*
 * Fixed-arity controls of one, two and three parameters.
 *
 * No variadic machinery of any kind is involved in these three, which is the
 * point: they establish that ordinary argument passing agrees with the results
 * read back through the variadic macros, so a divergence in the variadic lines
 * cannot be blamed on ordinary calls. Each sums small int constants, well inside
 * the int range.
 */
static int add1(int a)
{
    return a;
}

static int add2(int a, int b)
{
    return a + b;
}

static int add3(int a, int b, int c)
{
    return a + b + c;
}

/*
 * Second of the two zero-parameter helpers reached through a bare
 * `#define M(...)` invoked with no arguments.
 *
 * It returns a value distinct from the first helper's, so the two bare-form
 * lines cannot be confused with one another in the output and a defect that
 * affected only one of them stays visible.
 */
static int no_arg_probe(void)
{
    return 3;
}

/*
 * LOG      - the GNU comma-swallowing form: the paste operator deletes the
 *            preceding comma when the variadic list is empty, so the zero,
 *            one and two argument invocations all travel one code path.
 * WRAP_LOG - a variadic macro expanding into another variadic macro, so the
 *            comma swallow is exercised through a second level of expansion.
 * NOARG    - a bare variadic macro invoked with no arguments at all.
 * ADD_NONE - a second, independent instance of the same bare form.
 * VA_STR   - the stringizing macro, itself variadic so that it may legally
 *            receive an empty list, a single token, or several.
 */
#define LOG(fmt, ...) printf(fmt, ##__VA_ARGS__)
#define WRAP_LOG(fmt, ...) LOG(fmt, ##__VA_ARGS__)
#define NOARG(...) no_arg_probe(__VA_ARGS__)
#define ADD_NONE(...) add0(__VA_ARGS__)
#define VA_STR(...) #__VA_ARGS__

int main(void)
{
    /* Comma swallow at zero, one and two variadic arguments. */
    LOG("zero_args_forwarded=ok\n");
    LOG("one_arg=%d\n", 7);
    LOG("two_args=%d %d\n", 3, 4);

    /* The same swallow, reached through a second variadic macro. */
    WRAP_LOG("nested_zero_args=ok\n");
    WRAP_LOG("nested_one_arg=%d\n", 11);

    /* The bare-form zero-argument path, twice and independently. */
    printf("bare_noarg_macro=%d\n", NOARG());
    printf("bare_noarg_add=%d\n", ADD_NONE());

    /* Stringize an empty list, a two-token list, and a single token. */
    printf("va_str_empty_len=%d\n", slen(VA_STR()));
    printf("va_str_list=%s\n", VA_STR(a, b));
    printf("va_str_single=%s\n", VA_STR(alpha));

    /* Fixed-arity controls, with no variadic macro involved. */
    printf("fixed_arity_one=%d\n", add1(5));
    printf("fixed_arity_two=%d\n", add2(5, 6));
    printf("fixed_arity_three=%d\n", add3(5, 6, 7));
    return 0;
}
