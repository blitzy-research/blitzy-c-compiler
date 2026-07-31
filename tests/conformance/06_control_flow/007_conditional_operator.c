/* 007_conditional_operator.c -- Area 06 control flow.
 * The conditional operator: exactly one of the two branches is evaluated, and
 * the result type is the common type of the two branches after the usual
 * arithmetic conversions.  Single-branch evaluation is again an ABSENCE
 * assertion, so the same technique as 006 is used: side effects are recorded in
 * a volatile counter which the optimizer may not legally elide, and the counter
 * is printed after each expression.  Result typing is observed through sizeof,
 * which yields identical values on all four targets, and through two exactly
 * representable double values.  No address is ever printed; the pointer-typed
 * branches are observed only by comparison.
 */

int printf(const char *, ...);

static volatile int branch_calls;

static int taken(int r)
{
    branch_calls = branch_calls + 1;
    return r;
}

int main(void)
{
    static const int sel4[4] = { 1, 2, 3, 9 };
    volatile int vsel;
    int c;
    int r;
    int i;
    int out[4];
    int x = 11;
    int y = 22;
    double d;

    /* Exactly one branch is evaluated -- constant condition. */
    branch_calls = 0;
    r = (1 ? taken(10) : taken(20));
    printf("const_true_branch r=%d branch_calls=%d\n", r, branch_calls);

    branch_calls = 0;
    r = (0 ? taken(10) : taken(20));
    printf("const_false_branch r=%d branch_calls=%d\n", r, branch_calls);

    /* Exactly one branch is evaluated -- condition read from volatile. */
    vsel = 1;
    c = vsel;
    branch_calls = 0;
    r = (c ? taken(10) : taken(20));
    printf("runtime_true_branch r=%d branch_calls=%d\n", r, branch_calls);

    vsel = 0;
    c = vsel;
    branch_calls = 0;
    r = (c ? taken(10) : taken(20));
    printf("runtime_false_branch r=%d branch_calls=%d\n", r, branch_calls);

    /* Nested in both branch positions: still exactly one leaf is evaluated. */
    vsel = 1;
    c = vsel;
    branch_calls = 0;
    r = (c ? (c ? taken(1) : taken(2)) : (c ? taken(3) : taken(4)));
    printf("nested_one_leaf r=%d branch_calls=%d\n", r, branch_calls);

    /* Result typing observed through sizeof: char and short branches are
       promoted to int; an int branch against a double branch yields double;
       two float branches yield float. */
    printf("size_char=%d size_short=%d size_int_double=%d size_float=%d\n",
           (int)sizeof(1 ? (char)1 : (char)2),
           (int)sizeof(1 ? (short)1 : (short)2),
           (int)sizeof(1 ? 1 : 2.5),
           (int)sizeof(1 ? 1.0f : 2.0f));

    /* The int branch is converted to double even when it is the one selected.
       1.00 and 2.50 are exactly representable, so the printed text is exact. */
    d = (1 ? 1 : 2.5);
    printf("const_int_double_branches=%.2f\n", d);
    vsel = 1;
    c = vsel;
    d = (c ? 1 : 2.5);
    printf("runtime_int_double_true=%.2f\n", d);
    vsel = 0;
    c = vsel;
    d = (c ? 1 : 2.5);
    printf("runtime_int_double_false=%.2f\n", d);

    /* Right associativity: a ? b : c ? d : e parses as a ? b : (c ? d : e). */
    for (i = 0; i < 4; i++) {
        vsel = sel4[i];
        c = vsel;
        out[i] = (c == 1 ? 10 : c == 2 ? 20 : c == 3 ? 30 : 40);
    }
    printf("right_associative=%d %d %d %d\n", out[0], out[1], out[2], out[3]);

    /* Pointer-typed branches: the selection is observed by comparison only. */
    vsel = 1;
    c = vsel;
    printf("pointer_branch_true=%d\n", (c ? &x : &y) == &x);
    vsel = 0;
    c = vsel;
    printf("pointer_branch_false=%d\n", (c ? &x : &y) == &y);

    /* void-typed branches are legal and still evaluate exactly one side. */
    branch_calls = 0;
    vsel = 1;
    c = vsel;
    c ? (void)taken(1) : (void)taken(2);
    printf("void_branches_true branch_calls=%d\n", branch_calls);

    branch_calls = 0;
    vsel = 0;
    c = vsel;
    c ? (void)taken(1) : (void)taken(2);
    printf("void_branches_false branch_calls=%d\n", branch_calls);

    /* The operator in argument position, and applied to a comparison. */
    vsel = 5;
    c = vsel;
    printf("in_argument=%d of_comparison=%d\n",
           (c > 3 ? c * 2 : c * 3), (c > 3) ? 1 : 0);
    vsel = 2;
    c = vsel;
    printf("in_argument_other=%d of_comparison_other=%d\n",
           (c > 3 ? c * 2 : c * 3), (c > 3) ? 1 : 0);

    /* The result used as the right operand of a compound assignment. */
    r = 0;
    vsel = 1;
    c = vsel;
    r += c ? 5 : 7;
    vsel = 0;
    c = vsel;
    r += c ? 5 : 7;
    printf("compound_assignment=%d\n", r);

    /* A conditional operator controlling a switch's controlling expression. */
    vsel = 1;
    c = vsel;
    switch (c ? 10 : 20) {
    case 10:
        r = 1;
        break;
    case 20:
        r = 2;
        break;
    default:
        r = 3;
        break;
    }
    printf("as_switch_control=%d\n", r);
    return 0;
}
