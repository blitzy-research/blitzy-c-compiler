/* 006_short_circuit_evaluation.c -- Area 06 control flow.
 * Short-circuit evaluation of && and ||.  The property under test is an
 * ABSENCE: when the left operand already settles the result, the right operand
 * must NOT be evaluated.  Asserting an absence requires an observation the
 * optimizer is not permitted to elide, so every side effect is recorded in a
 * volatile counter and the counter is printed after each expression.  A plain
 * (non-volatile) counter could legally be removed by dead-code elimination at
 * -O1 and above, which would make the assertion vacuous.
 *
 * Each expression appears twice: once with a compile-time-constant left
 * operand (so the folder decides) and once with the left operand read from
 * volatile storage (so the backend must genuinely emit the branch).
 */

int printf(const char *, ...);

static volatile int rhs_calls;

static int bump(int r)
{
    rhs_calls = rhs_calls + 1;
    return r;
}

int main(void)
{
    volatile int vzero;
    volatile int vone;
    int a;
    int b;
    int r;

    vzero = 0;
    vone = 1;

    /* Constant left operand: the result is settled at translation time. */
    rhs_calls = 0;
    r = (0 && bump(1));
    printf("const_and_false r=%d rhs_calls=%d\n", r, rhs_calls);

    rhs_calls = 0;
    r = (1 || bump(1));
    printf("const_or_true r=%d rhs_calls=%d\n", r, rhs_calls);

    rhs_calls = 0;
    r = (1 && bump(7));
    printf("const_and_true r=%d rhs_calls=%d\n", r, rhs_calls);

    rhs_calls = 0;
    r = (0 || bump(7));
    printf("const_or_false r=%d rhs_calls=%d\n", r, rhs_calls);

    /* Runtime left operand read from volatile storage. */
    rhs_calls = 0;
    a = vzero;
    r = (a && bump(1));
    printf("runtime_and_false r=%d rhs_calls=%d\n", r, rhs_calls);

    rhs_calls = 0;
    a = vone;
    r = (a || bump(1));
    printf("runtime_or_true r=%d rhs_calls=%d\n", r, rhs_calls);

    rhs_calls = 0;
    a = vone;
    r = (a && bump(7));
    printf("runtime_and_true r=%d rhs_calls=%d\n", r, rhs_calls);

    rhs_calls = 0;
    a = vzero;
    r = (a || bump(7));
    printf("runtime_or_false r=%d rhs_calls=%d\n", r, rhs_calls);

    /* Chained operators stop at the first operand that settles the result. */
    rhs_calls = 0;
    a = vzero;
    r = (a && bump(1) && bump(1) && bump(1));
    printf("chain_and_stop_at_first r=%d rhs_calls=%d\n", r, rhs_calls);

    rhs_calls = 0;
    a = vone;
    r = (a || bump(1) || bump(1));
    printf("chain_or_stop_at_first r=%d rhs_calls=%d\n", r, rhs_calls);

    rhs_calls = 0;
    a = vone;
    b = vone;
    r = (a && b && bump(1) && bump(0) && bump(1));
    printf("chain_and_stop_midway r=%d rhs_calls=%d\n", r, rhs_calls);

    rhs_calls = 0;
    a = vzero;
    b = vzero;
    r = (a || b || bump(0) || bump(5) || bump(1));
    printf("chain_or_stop_midway r=%d rhs_calls=%d\n", r, rhs_calls);

    /* && binds tighter than ||; the grouping is written out explicitly. */
    rhs_calls = 0;
    a = vzero;
    b = vone;
    r = ((a && bump(1)) || (b && bump(1)));
    printf("mixed_grouping r=%d rhs_calls=%d\n", r, rhs_calls);

    /* The guard idiom: the right operand is never evaluated when the divisor
       is zero, so no division by zero is ever performed. */
    rhs_calls = 0;
    a = vzero;
    r = (a != 0 && (100 / a) > 3);
    printf("guarded_divide_zero r=%d rhs_calls=%d\n", r, rhs_calls);

    a = vone;
    r = (a != 0 && (100 / a) > 3);
    printf("guarded_divide_one r=%d\n", r);

    /* Short circuit in a controlling expression. */
    rhs_calls = 0;
    a = vzero;
    if (a && bump(1))
        r = 1;
    else
        r = 2;
    printf("if_and_short_circuit r=%d rhs_calls=%d\n", r, rhs_calls);

    rhs_calls = 0;
    a = vone;
    if (a || bump(1))
        r = 1;
    else
        r = 2;
    printf("if_or_short_circuit r=%d rhs_calls=%d\n", r, rhs_calls);

    /* Short circuit in a while controlling expression: the right operand runs
       only while the left one holds. */
    rhs_calls = 0;
    a = 0;
    while (a < 3 && bump(1))
        a++;
    printf("while_short_circuit a=%d rhs_calls=%d\n", a, rhs_calls);

    /* Both operators normalise their result to exactly 0 or 1. */
    a = vone;
    printf("normalisation not=%d notnot=%d and=%d or=%d\n",
           !a, !!a, (a && 5), (a || 0));

    a = vzero;
    printf("normalisation_zero not=%d notnot=%d and=%d or=%d\n",
           !a, !!a, (a && 5), (a || 0));
    return 0;
}
