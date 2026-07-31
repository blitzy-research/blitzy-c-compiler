/* The sequence point between the two operands of a comma operator fully defines
 * the order of their side effects, and that order is what is under test: each
 * side effect appends one decimal digit to a trace held in volatile storage, so
 * the sequence is part of the program's observable behaviour and is printed after
 * each expression.  Every append is guarded by an explicit range check, so no
 * write can leave the trace array. */

int printf(const char *, ...);

static int trace[16];
static volatile int trace_len;
static volatile int tick;

static int mark(int id)
{
    int n;

    n = trace_len;
    if (n >= 0 && n < 16) {
        trace[n] = id;
        trace_len = n + 1;
    }
    tick = tick + 1;
    return id;
}

/* Records its identifier in the trace but always yields zero, so a falsy
 * right operand can still be traced with an unambiguous non-zero digit.
 */
static int mark_false(int id)
{
    (void)mark(id);
    return 0;
}

static void reset_trace(void)
{
    int i;

    for (i = 0; i < 16; i++)
        trace[i] = 0;
    trace_len = 0;
    tick = 0;
}

static void print_trace(const char *label)
{
    int n;
    int i;

    n = trace_len;
    printf("%s order=", label);
    for (i = 0; i < n; i++)
        printf("%d", trace[i]);
    printf(" len=%d tick=%d\n", n, tick);
}

int main(void)
{
    volatile int vsel;
    int c;
    int r;
    int i;
    int j;
    int sum;

    /* A chain of comma operators evaluates strictly left to right and yields
       the value of the rightmost operand. */
    reset_trace();
    r = (mark(1), mark(2), mark(3));
    print_trace("comma_chain");
    printf("comma_chain_value=%d\n", r);

    /* Nested comma expressions still evaluate strictly left to right. */
    reset_trace();
    r = ((mark(1), mark(2)), (mark(3), mark(4)));
    print_trace("comma_nested");
    printf("comma_nested_value=%d\n", r);

    /* Assignment binds tighter than the comma operator, so r receives the
       value of the LEFT operand while both operands are still evaluated. */
    reset_trace();
    r = 0;
    r = mark(1), mark(2);
    print_trace("comma_precedence");
    printf("comma_precedence_value=%d\n", r);

    /* A comma expression used as a function argument must be parenthesised;
       the sequence point inside it fully defines the order. */
    reset_trace();
    r = mark((mark(4), 5));
    print_trace("comma_in_argument");
    printf("comma_in_argument_value=%d\n", r);

    /* Comma in a for-loop initialisation clause and increment clause. */
    reset_trace();
    sum = 0;
    for (i = 0, j = 10; i < j; i++, j--)
        sum += 1;
    printf("for_comma i=%d j=%d iterations=%d\n", i, j, sum);

    /* Comma in a while controlling expression: the left operand's side effect
       happens on every test, including the final failing one. */
    reset_trace();
    i = 0;
    while ((mark(6), i < 3))
        i++;
    print_trace("while_comma_condition");
    printf("while_comma_condition_i=%d\n", i);

    /* Comma inside the selected branch of a conditional operator: only the
       selected branch's side effects occur. */
    reset_trace();
    vsel = 1;
    c = vsel;
    r = c ? (mark(7), 70) : (mark(8), 80);
    print_trace("comma_in_ternary_true");
    printf("comma_in_ternary_true_value=%d\n", r);

    reset_trace();
    vsel = 0;
    c = vsel;
    r = c ? (mark(7), 70) : (mark(8), 80);
    print_trace("comma_in_ternary_false");
    printf("comma_in_ternary_false_value=%d\n", r);

    /* Comma sequencing volatile writes: the sequence points make the final
       stored value and the value of the whole expression fully defined. */
    reset_trace();
    vsel = 0;
    r = (vsel = 1, vsel = 2, vsel);
    printf("comma_volatile_writes r=%d vsel=%d\n", r, vsel);

    /* Comma as the controlling expression of a switch. */
    reset_trace();
    switch ((mark(9), 2)) {
    case 1:
        r = 10;
        break;
    case 2:
        r = 20;
        break;
    default:
        r = 30;
        break;
    }
    print_trace("comma_in_switch_control");
    printf("comma_in_switch_control_value=%d\n", r);

    /* Comma inside an if controlling expression. */
    reset_trace();
    if ((mark(1), mark_false(2)))
        r = 1;
    else
        r = 2;
    print_trace("comma_in_if_control");
    printf("comma_in_if_control_value=%d\n", r);

    /* Comma inside a do-while controlling expression. */
    reset_trace();
    i = 0;
    do {
        i++;
    } while ((mark(5), i < 3));
    print_trace("comma_in_do_while");
    printf("comma_in_do_while_i=%d\n", i);
    return 0;
}
