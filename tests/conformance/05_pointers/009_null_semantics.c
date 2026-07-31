/* Area 05 - Pointer arithmetic and function pointers
 * 009_null_semantics: null pointer constant forms, comparison and
 * conversion.
 *
 * The stddef.h null-pointer macro is deliberately not used: it comes from a
 * header, and no header may be included.  The two null pointer constant
 * spellings exercised here are the integer constant 0 and the cast form
 * (void *)0, which are exactly the forms the standard defines as null
 * pointer constants.
 *
 * A null pointer is never dereferenced anywhere in this program.  Null
 * facts appear only as equality comparisons, as truth-value tests and as
 * conversions, and nothing but those results is printed.
 */

int printf(const char *, ...);

struct pair {
    int a;
    int b;
};

struct node {
    int value;
    struct node *next;
};

static int obj = 5;
static double dobj = 1.5;
static struct pair sobj = { 1, 2 };

/* static objects with no initializer, and a partly filled table: the
 * unmentioned pointer members are null by static initialization */
static int *implicit_null;
static int *table[4] = { &obj };
static struct node lone = { 9, 0 };

static int returns_zero(void)
{
    return 0;
}

static int takes_pointer(const int *p)
{
    return p == 0 ? -1 : *p;
}

int main(void)
{
    int *p;
    int *q;
    char *cp;
    double *dp;
    struct pair *sp;
    void *vp;
    int (*fp)(void);
    volatile int vzero;
    int k;

    /* ---- the two null pointer constant spellings ---- */
    p = 0;
    q = (void *)0;
    printf("zero_form_is_null=%d\n", p == 0);
    printf("cast_form_is_null=%d\n", q == (void *)0);
    printf("both_forms_equal=%d\n", p == q);
    printf("zero_form_vs_cast=%d\n", p == (void *)0);
    printf("cast_form_vs_zero=%d\n", q == 0);

    /* ---- truth value of a null pointer ---- */
    printf("null_is_false=%d\n", !p);
    printf("null_in_condition=%d\n", p ? 1 : 0);
    printf("null_and=%d\n", p && 1);
    printf("null_or=%d\n", p || 1);
    if (!p) {
        printf("null_if_branch=%d\n", 1);
    } else {
        printf("null_if_branch=%d\n", 0);
    }

    /* ---- a non-null pointer contrasted against it ---- */
    p = &obj;
    printf("nonnull_ne_zero=%d\n", p != 0);
    printf("nonnull_ne_cast=%d\n", p != (void *)0);
    printf("nonnull_is_true=%d\n", p ? 1 : 0);
    printf("nonnull_not_is_false=%d\n", !p);
    printf("nonnull_value=%d\n", *p);
    printf("nonnull_ne_null_var=%d\n", p != q);

    /* ---- null for every pointed-to type ---- */
    cp = 0;
    dp = (void *)0;
    sp = 0;
    vp = 0;
    fp = 0;
    printf("char_null=%d\n", cp == 0);
    printf("double_null=%d\n", dp == 0);
    printf("struct_null=%d\n", sp == 0);
    printf("void_null=%d\n", vp == 0);
    printf("func_null=%d\n", fp == 0);
    printf("func_null_not=%d\n", !fp);

    /* ---- assignment away from null and back ---- */
    dp = &dobj;
    printf("double_assigned=%.3f\n", *dp);
    printf("double_now_nonnull=%d\n", dp != 0);
    dp = 0;
    printf("double_back_to_null=%d\n", dp == 0);
    sp = &sobj;
    printf("struct_assigned=%d %d\n", sp->a, sp->b);
    sp = (void *)0;
    printf("struct_back_to_null=%d\n", sp == 0);

    /* ---- conversion to and from void * ---- */
    p = 0;
    vp = p;
    printf("null_to_void=%d\n", vp == 0);
    printf("null_void_roundtrip=%d\n", (int *)vp == p);
    p = &obj;
    vp = p;
    printf("nonnull_to_void=%d\n", vp != 0);
    printf("nonnull_void_roundtrip=%d\n", (int *)vp == p);

    /* ---- static initialization produces null without an initializer ---- */
    printf("implicit_null=%d\n", implicit_null == 0);
    printf("table0_nonnull=%d\n", table[0] != 0);
    printf("table0_value=%d\n", *table[0]);
    printf("table1_null=%d\n", table[1] == 0);
    printf("table2_null=%d\n", table[2] == 0);
    printf("table3_null=%d\n", table[3] == 0);
    printf("struct_member_null=%d\n", lone.next == 0);
    printf("struct_member_value=%d\n", lone.value);

    /* ---- null as an argument and as a guard ---- */
    printf("arg_null=%d\n", takes_pointer(0));
    printf("arg_cast_null=%d\n", takes_pointer((void *)0));
    printf("arg_nonnull=%d\n", takes_pointer(&obj));
    printf("arg_from_table=%d\n", takes_pointer(table[0]));
    printf("arg_from_null_table=%d\n", takes_pointer(table[2]));

    /* ---- null in a conditional operator ---- */
    p = 1 ? 0 : &obj;
    printf("conditional_null=%d\n", p == 0);
    p = 0 ? 0 : &obj;
    printf("conditional_nonnull=%d\n", p != 0);
    printf("conditional_nonnull_value=%d\n", *p);

    /* ---- counting null entries in a table ---- */
    {
        int nulls = 0;
        int nonnulls = 0;
        int i;
        for (i = 0; i < 4; ++i) {
            if (table[i] == 0) {
                ++nulls;
            } else {
                ++nonnulls;
            }
        }
        printf("table_nulls=%d\n", nulls);
        printf("table_nonnulls=%d\n", nonnulls);
    }

    /* ---- runtime variant: the null constant arrives through a volatile ---- */
    vzero = 0;
    k = vzero;
    p = k ? &obj : 0;
    printf("runtime_null=%d\n", p == 0);
    printf("runtime_null_is_false=%d\n", !p);
    vzero = 1;
    k = vzero;
    p = k ? &obj : 0;
    printf("runtime_nonnull=%d\n", p != 0);
    printf("runtime_nonnull_value=%d\n", *p);
    printf("runtime_guarded=%d\n", takes_pointer(p));

    vzero = 2;
    k = vzero;
    printf("runtime_table_null=%d\n", table[k] == 0);
    printf("runtime_guarded_null=%d\n", takes_pointer(table[k]));

    vzero = 0;
    k = vzero;
    printf("runtime_table_nonnull=%d\n", table[k] != 0);
    printf("runtime_table_value=%d\n", *table[k]);

    /* a function that returns 0 is not a null pointer constant source, but
     * comparing its result against a pointer's null-ness still holds */
    fp = returns_zero;
    printf("runtime_fp_nonnull=%d\n", fp != 0);
    printf("runtime_fp_result=%d\n", fp());

    return 0;
}
