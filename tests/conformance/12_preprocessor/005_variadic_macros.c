/*
 * Area 12 / program 005 - variadic macros, INCLUDING the empty variadic list.
 *
 * WHAT IS UNDER TEST
 *
 * The preprocessor's handling of a function-like macro declared with a trailing
 * ellipsis: forwarding the variadic list at three different arities, forwarding
 * it onward through a second variadic macro, invoking a bare variadic macro with
 * no arguments at all, and applying the stringize operator `#` to the variadic
 * list when that list is empty, a single token, and several tokens.
 *
 * Every construct here is ISO C. Nothing in this program depends on a GNU
 * extension, and the one extension spelling that would have been the obvious
 * shortcut is excluded deliberately and for a stated reason - see TWO SPELLINGS
 * DELIBERATELY AVOIDED below. That matters because a divergence must be
 * attributable: a construct the standard defines is one both compilers have
 * agreed to, so a difference in it is a difference in the compilers.
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
 * Every construct below is the ISO C form of exactly that documented capability,
 * which is what makes the citation load-bearing: the documentation covers what is
 * tested, rather than something adjacent to it. A divergence this program provokes
 * - the empty variadic list emphatically included - is therefore a genuine FINDING
 * rather than an expected divergence. No marker belongs in the sibling expectation
 * record, and the register admits none for this area.
 *
 * THE EMPTY VARIADIC LIST, AND WHY IT IS EXERCISED THREE TIMES OVER TWO SHAPES
 *
 * An empty variadic list is the case preprocessor implementations most often get
 * wrong, so it is not dropped - it is reached the way ISO C actually allows,
 * through a macro whose parameter list is nothing BUT an ellipsis. `#define
 * M(...)` invoked as `M()` supplies one empty argument to `__VA_ARGS__`, and the
 * standard imposes no minimum there because there is no named parameter for the
 * variadic part to follow. Two shapes of empty list appear in main, the first of
 * them twice, so a divergence in one still leaves the others reporting, which
 * localises a defect instead of merely announcing it:
 *
 *   1. NOARG(), whose body calls a function declared to take `void`, so an
 *      expansion that failed to produce an empty argument list would be a
 *      diagnosable error rather than a silently different call;
 *   2. ADD_NONE(), a second, independent instance of that same shape, returning
 *      a different value so the two lines cannot be confused;
 *   3. VA_STR(), which stringizes the empty list and whose result must have
 *      length zero.
 *
 * A third shape sits beside them and is a different construct rather than a
 * fourth empty-list case: LOG0("...\n") and, one level of expansion deeper,
 * WRAP0("...\n"), where the WHOLE argument list is variadic and nothing follows
 * it. Those lists are NOT empty - each carries the format string - but because
 * the ellipsis has no named parameter in front of it, no comma is left dangling
 * when nothing follows the format. That is the standard answer to the
 * dangling-comma problem, and it is what lets this program keep the capability -
 * a log-style macro invoked with no arguments beyond its format - while writing
 * only ISO C. Constraint C3 forbids dropping a construct because it is difficult;
 * the construct is kept and only one non-standard SPELLING of it is set aside,
 * with the reason recorded.
 *
 * NESTED FORWARDING
 *
 * WRAP_LOG and WRAP0 are variadic macros whose bodies invoke another variadic
 * macro. They are the second-highest-yield construct here: an implementation that
 * forwards `__VA_ARGS__` correctly at one level of expansion but not through a
 * second breaks exactly there. Both the named-parameter form and the wholly
 * variadic form are forwarded, and the wholly variadic one at two arities, which
 * is what distinguishes a broken forward from a broken argument count.
 *
 * THREE SPELLINGS DELIBERATELY AVOIDED
 *
 * The C2x optional-comma variadic operator is not used anywhere in this program.
 * No repository document enumerates support for it, so a compile failure arising
 * from it would be attributable to this test rather than to the compiler under
 * test - the one kind of divergence a differential suite must never manufacture.
 *
 * THE GNU COMMA-SWALLOWING FORM `, ##__VA_ARGS__` IS NOT USED EITHER, and this is
 * the one exclusion in this program that has to be stated rather than assumed.
 * It is excluded for two independent reasons, both of which would make a
 * divergence in it uninterpretable rather than informative:
 *
 *   1. It is a GNU extension, not ISO C, and no repository inventory enumerates
 *      it. The documented extension set is __attribute__, __builtin_*
 *      intrinsics, inline assembly with operand constraints, statement
 *      expressions, typeof and __typeof__, computed goto and __extension__
 *      (docs/technical-specifications.md lines 13, 107 and 761); the documented
 *      preprocessor capability is "stringification (#), token pasting (##),
 *      variadic macros (__VA_ARGS__)" (line 491), which names the standard
 *      feature and not the comma-deletion behaviour. A divergence would
 *      therefore have neither a documented basis to be an expected divergence
 *      nor a documented capability to be a finding against.
 *   2. Reaching its comma-deleting behaviour requires OMITTING the variadic
 *      argument altogether from a macro that has a named parameter, and ISO C
 *      before C23 forbids that: the reference compiler under `-std=c17 -pedantic`
 *      reports "ISO C99 requires at least one argument for the \"...\" in a
 *      variadic macro", and the alternate reference compiler reports the token
 *      paste itself as a GNU extension. -pedantic is a member of this area's
 *      warning gate, from which area 12 sanctions no deviation, so the construct
 *      could not clear the suite's own undefined-behaviour gate.
 *
 * The exclusion is one spelling wide. The capability it spells is exercised above
 * through the wholly variadic form, the empty variadic list is exercised three
 * times, and no oracle, target or optimization level is switched off anywhere in
 * this program.
 *
 * An EXPLICITLY EMPTY trailing macro argument is the third avoided spelling, and
 * the reason is not that the preprocessor refuses it. It does not: `LOG("x\n", )`
 * supplies one empty argument and preprocesses without a diagnostic under the full
 * gate. What it produces is `printf("x\n", )`, whose trailing comma is a syntax
 * error in the CALL - the very dangling comma the GNU swallow exists to delete.
 * The construct is therefore unusable for a comparison for a reason that has
 * nothing to do with either compiler's variadic machinery, and the behaviour it
 * would have reached is already covered by the wholly variadic form above.
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
 * Output is a fixed sequence of sixteen `key=value` lines, one per semantic
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
 * LOG      - a named parameter followed by an ellipsis, forwarding the variadic
 *            list after a comma.  ISO C requires the list to be non-empty here,
 *            so every invocation of LOG below supplies at least one argument for
 *            it; the empty case is reached through the wholly variadic forms
 *            instead, for the reason recorded in the header.
 * WRAP_LOG - the same shape expanding into another variadic macro, so forwarding
 *            is exercised through a second level of expansion.
 * LOG0     - a WHOLLY variadic macro: the format itself travels in
 *            __VA_ARGS__, so nothing follows the list and no comma can be left
 *            dangling.  This is the standard spelling of a log-style macro
 *            invoked with no arguments beyond its format.
 * WRAP0    - LOG0 forwarded through a second wholly variadic macro.
 * NOARG    - a wholly variadic macro invoked with no argument at all.
 * ADD_NONE - a second, independent instance of the same shape.
 * VA_STR   - the stringizing macro, itself wholly variadic so that it may
 *            legally receive an empty list, a single token, or several.
 */
#define LOG(fmt, ...) printf(fmt, __VA_ARGS__)
#define WRAP_LOG(fmt, ...) LOG(fmt, __VA_ARGS__)
#define LOG0(...) printf(__VA_ARGS__)
#define WRAP0(...) LOG0(__VA_ARGS__)
#define NOARG(...) no_arg_probe(__VA_ARGS__)
#define ADD_NONE(...) add0(__VA_ARGS__)
#define VA_STR(...) #__VA_ARGS__

int main(void)
{
    /* The wholly variadic form at one, two and three arguments.  The first is
       the empty-trailing-list case written the way ISO C allows: everything the
       call needs is inside __VA_ARGS__, so there is no comma to delete. */
    LOG0("all_variadic_one=ok\n");
    LOG0("all_variadic_two=%d\n", 7);
    LOG0("all_variadic_three=%d %d\n", 3, 4);

    /* A named parameter plus a non-empty variadic list, at one and at two. */
    LOG("named_plus_one=%d\n", 11);
    LOG("named_plus_two=%d %d\n", 5, 6);

    /* Both forms forwarded through a second variadic macro: the named form at
       one variadic argument, and the wholly variadic form at one argument - the
       empty-trailing-list case again, now two levels deep - and at three. */
    WRAP_LOG("nested_named_plus_one=%d\n", 13);
    WRAP0("nested_all_variadic_one=ok\n");
    WRAP0("nested_all_variadic_three=%d %d\n", 8, 9);

    /* The empty variadic list through a wholly variadic macro, twice and
       independently, each expanding to a call on a function declared void. */
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
