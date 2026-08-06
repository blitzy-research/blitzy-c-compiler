/* Folding through the conditional operator and the comma operator, with a runtime
 * twin for every folded value.
 *
 * Each constant variant hands the folder a controlling expression and both arms as
 * constant expressions, so it must select an arm and then fold the arm it selected;
 * two of them sit in contexts - an array bound and an enumerator initializer -
 * where the language requires the fold rather than merely permitting it.  Each is
 * paired with a variant whose operands live in volatile storage, which no
 * optimization may fold away, so the same value is produced a second time by
 * instructions the backend actually emitted.  Every fold_ line therefore has a
 * runtime_ twin and their equality is the assertion: without the twin, folding would
 * silently substitute its own answer for the backend's, and a code-generation defect
 * would escape detection entirely.
 *
 * Every comma expression here has side-effecting left operands, without exception.
 * A comma operand that computes a value nobody uses is diagnosed - -Wunused-value is
 * part of -Wall, the audit gate adds -Werror, and this area sanctions no gate
 * deviation.  The restriction improves the program rather than constraining it: a
 * side-effecting left operand is exactly what makes the sequence point observable,
 * and observable sequencing is the interesting property of the comma operator.
 *
 * Freedom from undefined behaviour rests on sequencing here, which is worth stating
 * because sequencing is also the subject: the comma operator supplies a sequence
 * point between its operands and the conditional operator supplies one after its
 * controlling expression, so no object is modified twice between sequence points and
 * nothing depends on unspecified evaluation order.  No call receives more than one
 * side-effecting argument -- each volatile counter is copied into a plain local in
 * its own statement before being printed.  All signed arithmetic stays far inside
 * range, and the one unsigned subtraction is modular by definition rather than
 * overflow.  The undefined-behaviour argument in full, and the recorded command
 * lines, live in the sibling .expected record.
 */

int printf(const char *, ...);

/* Side-effect observation lives in volatile storage deliberately.  A volatile access
 * is part of the program's observable behaviour, so "the unselected arm did not run"
 * becomes an assertion the implementation is obliged to honour rather than one it is
 * free to optimize away - and an absence is precisely what the conditional
 * operator's only-one-arm-is-evaluated rule asks a program to observe. */
static volatile int then_hits;
static volatile int else_hits;
static volatile int ticks;

/* Reached only from the second operand of a conditional. */
static int hit_then(int value)
{
    then_hits = then_hits + 1;
    return value;
}

/* Reached only from the third operand of a conditional. */
static int hit_else(int value)
{
    else_hits = else_hits + 1;
    return value;
}

static void reset_hits(void)
{
    then_hits = 0;
    else_hits = 0;
}

/* Returns its argument unchanged, so a deliberately parenthesised comma expression
 * can be handed over as a single argument and observed through the result. */
static int identity(int value)
{
    return value;
}

/* A folded conditional in an enumerator initializer.  An enumerator's value must be
 * an integer constant expression, so the conditional is required to fold here. */
enum { FOLDED_ENUMERATOR = (1 ? 6 : 3) * 7 };

int main(void)
{
    /* A folded conditional in an array bound - likewise an integer constant
       expression, so the conditional is required to fold here too.  The array is
       only ever measured, never read, so its contents never matter; it has static
       storage duration, so those contents are nevertheless fully determined. */
    static const int folded_bound_probe[(1 ? 6 : 3) * 7];
    volatile int sel_true = 1;
    volatile int sel_false = 0;
    volatile int nest_outer = 1;
    volatile int nest_inner = 0;
    volatile int nest_leaf = 1;
    volatile unsigned int usel = 1u;
    volatile int vsix = 6;
    volatile int vseven = 7;
    /* The comma chain below deliberately overwrites both of these, which is how its
       left operands earn their side effects; their initial values are never read. */
    volatile int v7 = 7;
    volatile int v6 = 6;
    volatile int vchain = 5;
    volatile int vbound = 7;
    int plain_chain = 5;
    int chain_after_add = 0;
    int chain_after_mul = 0;
    int chain_value = 0;
    int chain_stored = 0;
    int value;
    int left_a = 0;
    int left_b = 0;
    int hits_then_seen;
    int hits_else_seen;
    int ticks_seen;
    int fold_sum;
    int runtime_sum;
    int i;
    int j;
    long long llfold;
    long long llruntime;
    unsigned int uwrap;

    /* A constant controlling expression with constant arms: the folder must select
       an arm and fold it.  The twin forces a genuine run-time selection. */
    printf("fold_cond=%d\n", (1 ? 40 + 2 : 0));
    printf("runtime_cond=%d\n", (sel_true ? 40 + 2 : 0));
    printf("fold_cond_false=%d\n", (0 ? 1000 : 6 * 7));
    printf("runtime_cond_false=%d\n", (sel_false ? 1000 : 6 * 7));

    /* Three levels of nesting, with a conditional as the controlling expression and
       a conditional inside each arm, so the folder has to recurse. */
    printf("fold_nested=%d\n",
           ((1 ? 1 : 0) ? (0 ? 10 : (1 ? 42 : 11)) : (1 ? 20 : 21)));
    printf("runtime_nested=%d\n",
           ((nest_outer ? 1 : 0) ? (nest_inner ? 10 : (nest_leaf ? 42 : 11))
                                 : (nest_leaf ? 20 : 21)));

    /* The result type is the common type of the second and third operands and does
       not depend on which arm is selected, nor on where the controlling expression
       came from.  It is observed through sizeof, which does not evaluate its operand
       - so the volatile object in the twin is not accessed there - on widths that
       are identical on all four targets. */
    printf("fold_result_size_int=%d\n", (int)sizeof(1 ? 1 : 2));
    printf("runtime_result_size_int=%d\n", (int)sizeof(sel_true ? 1 : 2));
    printf("fold_result_size_llong=%d\n", (int)sizeof(1 ? 1 : 2LL));
    printf("runtime_result_size_llong=%d\n", (int)sizeof(sel_true ? 1 : 2LL));
    printf("fold_result_size_uint=%d\n", (int)sizeof(1 ? 1u : 2u));
    printf("runtime_result_size_uint=%d\n", (int)sizeof(sel_true ? 1u : 2u));

    /* An int arm against a long long arm yields long long, so the selected int value
       survives the widening unchanged. */
    llfold = (1 ? 42 : -1LL);
    printf("fold_cond_llong=%lld\n", llfold);
    llruntime = (sel_true ? 42 : -1LL);
    printf("runtime_cond_llong=%lld\n", llruntime);

    /* Two unsigned arms yield unsigned int, and that unsignedness carries into the
       subtraction that follows, which is modular rather than overflowing.  unsigned
       int is 32 bits wide on all four targets, so the wrapped value is invariant. */
    uwrap = (1 ? 0u : 5u) - 1u;
    printf("fold_cond_uint_wrap=%lld\n", (long long)uwrap);
    uwrap = (usel ? 0u : 5u) - 1u;
    printf("runtime_cond_uint_wrap=%lld\n", (long long)uwrap);

    /* Exactly one arm is evaluated.  The counters are snapshotted into plain locals
       first, so that no call ever receives two side-effecting arguments. */
    reset_hits();
    value = (1 ? hit_then(11) : hit_else(22));
    hits_then_seen = then_hits;
    hits_else_seen = else_hits;
    printf("fold_then_value=%d\n", value);
    printf("fold_then_hits then_hits=%d else_hits=%d\n",
           hits_then_seen, hits_else_seen);

    reset_hits();
    value = (0 ? hit_then(11) : hit_else(22));
    hits_then_seen = then_hits;
    hits_else_seen = else_hits;
    printf("fold_else_value=%d\n", value);
    printf("fold_else_hits then_hits=%d else_hits=%d\n",
           hits_then_seen, hits_else_seen);

    reset_hits();
    value = (sel_true ? hit_then(11) : hit_else(22));
    hits_then_seen = then_hits;
    hits_else_seen = else_hits;
    printf("runtime_then_value=%d\n", value);
    printf("runtime_then_hits then_hits=%d else_hits=%d\n",
           hits_then_seen, hits_else_seen);

    reset_hits();
    value = (sel_false ? hit_then(11) : hit_else(22));
    hits_then_seen = then_hits;
    hits_else_seen = else_hits;
    printf("runtime_else_value=%d\n", value);
    printf("runtime_else_hits then_hits=%d else_hits=%d\n",
           hits_then_seen, hits_else_seen);

    /* The same folded conditional where the language demands a constant expression,
       then the same arithmetic performed entirely at run time. */
    printf("fold_array_bound=%d\n",
           (int)(sizeof folded_bound_probe / sizeof folded_bound_probe[0]));
    printf("fold_enumerator=%d\n", FOLDED_ENUMERATOR);
    printf("runtime_same_arithmetic=%d\n", (sel_true ? vsix : 3) * vseven);

    /* The comma operator's sequence point.  Every left operand assigns to storage,
       so the whole chain is observable and its order is fully defined; a wrong order
       shows up as one specific divergent line.  The folded variant runs on plain
       storage, where constants may be propagated the whole way through; the twin
       runs on volatile storage, where they may not. */
    chain_value = (plain_chain = plain_chain + 1, chain_after_add = plain_chain,
                   plain_chain = plain_chain * 2, chain_after_mul = plain_chain,
                   plain_chain);
    chain_stored = plain_chain;
    printf("fold_comma_seq_after_add=%d\n", chain_after_add);
    printf("fold_comma_seq_after_mul=%d\n", chain_after_mul);
    printf("fold_comma_seq_value=%d\n", chain_value);
    printf("fold_comma_seq_stored=%d\n", chain_stored);

    chain_value = (vchain = vchain + 1, chain_after_add = vchain,
                   vchain = vchain * 2, chain_after_mul = vchain, vchain);
    chain_stored = vchain;
    printf("runtime_comma_seq_after_add=%d\n", chain_after_add);
    printf("runtime_comma_seq_after_mul=%d\n", chain_after_mul);
    printf("runtime_comma_seq_value=%d\n", chain_value);
    printf("runtime_comma_seq_stored=%d\n", chain_stored);

    /* The value of a comma expression is that of its right operand: once with a
       right operand the folder can evaluate, once with one only the backend can.
       The two variants are otherwise identical in shape - two assigning left
       operands, then a product - so their left-effect lines must agree exactly. */
    printf("fold_comma=%d\n", (left_a = 3, left_b = 14, 6 * 7));
    printf("fold_comma_left_effects a=%d b=%d\n", left_a, left_b);

    printf("runtime_comma=%d\n", (v7 = 3, v6 = 14, v7 * v6));
    left_a = v7;
    left_b = v6;
    printf("runtime_comma_left_effects a=%d b=%d\n", left_a, left_b);

    /* The type of a comma expression is also that of its right operand, shown here
       by a value no int could hold: were the expression typed int, the result could
       not be printed unchanged.  This is stronger evidence than sizeof would be,
       because the expression is actually evaluated. */
    ticks = 0;
    llfold = (ticks = ticks + 1, 6LL * 1000000000LL);
    printf("fold_comma_llong=%lld\n", llfold);
    llruntime = (ticks = ticks + 1, (long long)vsix * 1000000000LL);
    printf("runtime_comma_llong=%lld\n", llruntime);

    /* Comma in the initialisation clause and in the increment clause of a for
       statement.  The folded bound is a constant expression; the twin's is not. */
    ticks = 0;
    fold_sum = 0;
    for (i = 0, j = (1 ? 7 : 3); i < j; i++, fold_sum = fold_sum + i)
        ticks = ticks + 1;
    ticks_seen = ticks;
    printf("for_comma_fold_i=%d\n", i);
    printf("for_comma_fold_j=%d\n", j);
    printf("for_comma_fold_sum=%d\n", fold_sum);
    printf("for_comma_fold_iterations=%d\n", ticks_seen);

    ticks = 0;
    runtime_sum = 0;
    for (i = 0, j = vbound; i < j; i++, runtime_sum = runtime_sum + i)
        ticks = ticks + 1;
    ticks_seen = ticks;
    printf("for_comma_runtime_i=%d\n", i);
    printf("for_comma_runtime_j=%d\n", j);
    printf("for_comma_runtime_sum=%d\n", runtime_sum);
    printf("for_comma_runtime_iterations=%d\n", ticks_seen);

    /* The two operators together - a comma expression as the controlling expression
       of a conditional - which is where an implementation is most likely to fold the
       selection while losing the left operand's side effect. */
    reset_hits();
    ticks = 0;
    value = ((ticks = ticks + 1, 1) ? hit_then(11) : hit_else(22));
    hits_then_seen = then_hits;
    hits_else_seen = else_hits;
    ticks_seen = ticks;
    printf("fold_comma_control_value=%d\n", value);
    printf("fold_comma_control_hits then_hits=%d else_hits=%d\n",
           hits_then_seen, hits_else_seen);
    printf("fold_comma_control_ticks=%d\n", ticks_seen);

    reset_hits();
    ticks = 0;
    value = ((ticks = ticks + 1, sel_true) ? hit_then(11) : hit_else(22));
    hits_then_seen = then_hits;
    hits_else_seen = else_hits;
    ticks_seen = ticks;
    printf("runtime_comma_control_value=%d\n", value);
    printf("runtime_comma_control_hits then_hits=%d else_hits=%d\n",
           hits_then_seen, hits_else_seen);
    printf("runtime_comma_control_ticks=%d\n", ticks_seen);

    /* A comma expression inside each arm.  The unselected arm's counter step is 100
       rather than 1, so an arm evaluated in error is unmistakable in the output. */
    ticks = 0;
    value = (1 ? (ticks = ticks + 1, 40 + 2) : (ticks = ticks + 100, 0));
    ticks_seen = ticks;
    printf("fold_comma_arms_then_value=%d\n", value);
    printf("fold_comma_arms_then_ticks=%d\n", ticks_seen);

    ticks = 0;
    value = (0 ? (ticks = ticks + 1, 0) : (ticks = ticks + 100, 6 * 7));
    ticks_seen = ticks;
    printf("fold_comma_arms_else_value=%d\n", value);
    printf("fold_comma_arms_else_ticks=%d\n", ticks_seen);

    ticks = 0;
    value = (sel_true ? (ticks = ticks + 1, vsix * vseven)
                      : (ticks = ticks + 100, 0));
    ticks_seen = ticks;
    printf("runtime_comma_arms_then_value=%d\n", value);
    printf("runtime_comma_arms_then_ticks=%d\n", ticks_seen);

    ticks = 0;
    value = (sel_false ? (ticks = ticks + 1, 0)
                       : (ticks = ticks + 100, vsix * vseven));
    ticks_seen = ticks;
    printf("runtime_comma_arms_else_value=%d\n", value);
    printf("runtime_comma_arms_else_ticks=%d\n", ticks_seen);

    /* A comma expression handed over as a single function argument has to be
       parenthesised deliberately; this call has exactly one side-effecting
       argument, so nothing depends on the order arguments are evaluated in. */
    ticks = 0;
    value = identity((ticks = ticks + 1, 6 * 7));
    ticks_seen = ticks;
    printf("fold_comma_in_argument=%d\n", value);
    printf("fold_comma_in_argument_ticks=%d\n", ticks_seen);

    ticks = 0;
    value = identity((ticks = ticks + 1, vsix * vseven));
    ticks_seen = ticks;
    printf("runtime_comma_in_argument=%d\n", value);
    printf("runtime_comma_in_argument_ticks=%d\n", ticks_seen);
    return 0;
}
