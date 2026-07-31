/* Area 05 - Pointer arithmetic and function pointers
 * 005_function_pointers_basic: function pointer declaration, assignment
 * and indirect call, in both the fp(...) and (*fp)(...) call forms.
 *
 * No header is included; printf is hand-declared.  No function address is
 * ever printed; function-pointer facts appear only as equality comparisons
 * and as the results of the calls themselves.
 */

int printf(const char *, ...);

static int add(int x, int y)
{
    return x + y;
}

static int sub(int x, int y)
{
    return x - y;
}

static int mul(int x, int y)
{
    return x * y;
}

static int negate(int x)
{
    return -x;
}

static int zero(void)
{
    return 0;
}

static int seven(void)
{
    return 7;
}

/* takes a function pointer as a parameter */
static int apply(int (*op)(int, int), int x, int y)
{
    return op(x, y);
}

/* takes a function designator, which decays to a pointer */
static int apply_twice(int (*op)(int, int), int x)
{
    return op(op(x, x), x);
}

/* returns a function pointer */
static int (*select_op(int which))(int, int)
{
    if (which == 0) {
        return add;
    }
    if (which == 1) {
        return sub;
    }
    return mul;
}

/* a typedef spelling of the same type */
typedef int (*binop)(int, int);
typedef int (*unop)(int);
typedef int (*nilop)(void);

static int fold(binop op, const int *v, int n, int seed)
{
    int acc = seed;
    int i;
    for (i = 0; i < n; ++i) {
        acc = op(acc, v[i]);
    }
    return acc;
}

static const int data[5] = { 1, 2, 3, 4, 5 };

int main(void)
{
    int (*fp)(int, int);
    binop bp;
    unop up;
    nilop np;
    volatile int vsel;
    int k;

    /* ---- declaration and assignment from a function designator ---- */
    fp = add;
    printf("call_plain=%d\n", fp(10, 3));
    printf("call_deref=%d\n", (*fp)(10, 3));
    printf("call_forms_agree=%d\n", fp(10, 3) == (*fp)(10, 3));
    printf("eq_add=%d\n", fp == add);
    printf("ne_sub=%d\n", fp != sub);

    /* ---- assignment from an explicitly address-of'd function ---- */
    fp = &sub;
    printf("addrof_call=%d\n", fp(10, 3));
    printf("addrof_eq_designator=%d\n", fp == sub);

    fp = mul;
    printf("mul_call=%d\n", fp(10, 3));
    printf("mul_deref_call=%d\n", (*fp)(6, 7));

    /* ---- reassignment observed through repeated calls ---- */
    fp = add;
    printf("reassign_a=%d\n", fp(20, 5));
    fp = sub;
    printf("reassign_b=%d\n", fp(20, 5));
    fp = mul;
    printf("reassign_c=%d\n", fp(20, 5));

    /* ---- typedef-declared pointers ---- */
    bp = add;
    printf("typedef_binop=%d\n", bp(8, 9));
    printf("typedef_eq_plain=%d\n", bp == add);
    up = negate;
    printf("typedef_unop=%d\n", up(12));
    printf("typedef_unop_neg=%d\n", up(-12));
    np = seven;
    printf("typedef_nilop=%d\n", np());
    np = zero;
    printf("typedef_nilop_zero=%d\n", np());

    /* ---- function pointers as parameters ---- */
    printf("apply_add=%d\n", apply(add, 14, 6));
    printf("apply_sub=%d\n", apply(sub, 14, 6));
    printf("apply_mul=%d\n", apply(mul, 14, 6));
    printf("apply_via_var=%d\n", apply(bp, 14, 6));
    printf("apply_twice_add=%d\n", apply_twice(add, 3));
    printf("apply_twice_mul=%d\n", apply_twice(mul, 3));

    /* ---- function pointers as return values ---- */
    printf("select0=%d\n", select_op(0)(30, 4));
    printf("select1=%d\n", select_op(1)(30, 4));
    printf("select2=%d\n", select_op(2)(30, 4));
    printf("select0_is_add=%d\n", select_op(0) == add);
    printf("select1_is_sub=%d\n", select_op(1) == sub);

    /* ---- a fold driven by an indirect call ---- */
    printf("fold_add=%d\n", fold(add, data, 5, 0));
    printf("fold_mul=%d\n", fold(mul, data, 5, 1));
    printf("fold_sub=%d\n", fold(sub, data, 5, 100));

    /* ---- null function pointer, compared but never called ---- */
    fp = 0;
    printf("null_fp_is_null=%d\n", fp == 0);
    printf("null_fp_is_false=%d\n", !fp);
    fp = add;
    printf("nonnull_fp_is_true=%d\n", fp ? 1 : 0);

    /* ---- runtime variant: a volatile selector defeats devirtualization ---- */
    vsel = 0;
    k = vsel;
    fp = select_op(k);
    printf("runtime_select_call=%d\n", fp(40, 8));
    printf("runtime_select_is_add=%d\n", fp == add);

    vsel = 1;
    k = vsel;
    fp = select_op(k);
    printf("runtime_select_call2=%d\n", fp(40, 8));
    printf("runtime_select_is_sub=%d\n", fp == sub);

    vsel = 2;
    k = vsel;
    printf("runtime_apply=%d\n", apply(select_op(k), 9, 9));
    printf("runtime_fold=%d\n", fold(select_op(k - 2), data, 5, 0));

    vsel = 1;
    k = vsel;
    bp = k ? sub : add;
    printf("runtime_conditional=%d\n", bp(50, 7));
    printf("runtime_conditional_is_sub=%d\n", bp == sub);

    return 0;
}
