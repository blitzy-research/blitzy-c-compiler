/* Area 05 - Pointer arithmetic and function pointers
 * 010_qualified_pointers: const and volatile qualified pointers and
 * qualified pointed-to types.
 *
 * The four distinct shapes are exercised separately: pointer to const,
 * const pointer, const pointer to const, and pointer to volatile.  const
 * is never cast away and then written through, which would be undefined for
 * an object actually defined const; every write goes to an object that is
 * not itself const.  volatile reads are used to force genuine loads so the
 * qualifier's effect is observable rather than optimized away.
 *
 * No header is included; printf is hand-declared.  No address or pointer
 * value is printed; pointer identity appears only as equality comparisons
 * and as differences cast to long long.
 */

int printf(const char *, ...);

struct pair {
    int a;
    int b;
};

static const int cval = 11;
static volatile int vval = 22;
static const volatile int cvval = 33;
static int plain = 44;

static const int carr[5] = { 1, 2, 3, 4, 5 };
static volatile int varr[5] = { 10, 20, 30, 40, 50 };
static const struct pair cpair = { 6, 7 };
static struct pair mpair = { 8, 9 };

static volatile int vcounter;
static volatile int vsink;

/* a parameter that promises not to write through its pointer */
static int sum_const(const int *p, int n)
{
    int total = 0;
    int i;
    for (i = 0; i < n; ++i) {
        total += p[i];
    }
    return total;
}

/* a parameter that must re-read the object on every access */
static int sum_volatile(const volatile int *p, int n)
{
    int total = 0;
    int i;
    for (i = 0; i < n; ++i) {
        total += p[i];
    }
    return total;
}

/* the pointer itself is const; the pointed-to object is not */
static void bump(int *const p, int by)
{
    *p += by;
}

int main(void)
{
    const int *pc;                  /* pointer to const int */
    int *const cp = &plain;         /* const pointer to int */
    const int *const cpc = &cval;   /* const pointer to const int */
    volatile int *pv;               /* pointer to volatile int */
    const volatile int *pcv;        /* pointer to const volatile int */
    int *pi;
    volatile int vidx;
    int k;

    /* ---- pointer to const: reads only ---- */
    pc = &cval;
    printf("read_pointer_to_const=%d\n", *pc);
    printf("pointer_to_const_identity=%d\n", pc == &cval);
    pc = carr;
    printf("const_array_first=%d\n", *pc);
    printf("const_array_index=%d\n", pc[3]);
    ++pc;
    printf("const_array_stepped=%d\n", *pc);
    printf("const_array_offset=%lld\n", (long long)(pc - carr));
    printf("const_array_span=%lld\n", (long long)((carr + 5) - carr));

    /* ---- const pointer to non-const: the object may be written ---- */
    printf("const_pointer_read=%d\n", *cp);
    *cp = 45;
    printf("const_pointer_write=%d\n", plain);
    printf("const_pointer_reread=%d\n", *cp);
    printf("const_pointer_identity=%d\n", cp == &plain);
    bump(cp, 5);
    printf("const_pointer_param_bump=%d\n", plain);
    bump(&plain, 10);
    printf("const_pointer_param_bump2=%d\n", plain);

    /* ---- const pointer to const: neither may change ---- */
    printf("const_pointer_to_const=%d\n", *cpc);
    printf("const_pointer_to_const_identity=%d\n", cpc == &cval);

    /* ---- pointer to volatile: every access is a genuine load or store ---- */
    pv = &vval;
    printf("volatile_read=%d\n", *pv);
    *pv = 23;
    printf("volatile_after_write=%d\n", *pv);
    printf("volatile_object=%d\n", vval);
    printf("volatile_identity=%d\n", pv == &vval);

    pv = varr;
    printf("volatile_array_first=%d\n", *pv);
    printf("volatile_array_index=%d\n", pv[4]);
    pv += 2;
    printf("volatile_array_stepped=%d\n", *pv);
    printf("volatile_array_offset=%lld\n", (long long)(pv - varr));
    *pv = 31;
    printf("volatile_array_written=%d\n", varr[2]);

    /* ---- pointer to const volatile: reads only, but genuine reads ---- */
    pcv = &cvval;
    printf("const_volatile_read=%d\n", *pcv);
    printf("const_volatile_reread=%d\n", *pcv);
    printf("const_volatile_identity=%d\n", pcv == &cvval);

    /* ---- adding a qualifier is an implicit, legal conversion ---- */
    pi = &plain;
    pc = pi;                        /* int * converts to const int * */
    printf("qual_added_read=%d\n", *pc);
    printf("qual_added_same_object=%d\n", pc == pi);
    *pi = 71;                       /* write through the unqualified pointer */
    printf("qual_added_sees_write=%d\n", *pc);

    pv = pi;                        /* int * converts to volatile int * */
    printf("volatile_added_read=%d\n", *pv);
    *pv = 72;
    printf("volatile_added_write_seen=%d\n", *pi);

    pcv = pi;                       /* int * converts to const volatile int * */
    printf("both_added_read=%d\n", *pcv);

    /* ---- qualified struct access through pointers ---- */
    {
        const struct pair *pcs = &cpair;
        struct pair *const cps = &mpair;
        printf("const_struct=%d %d\n", pcs->a, pcs->b);
        printf("const_struct_identity=%d\n", pcs == &cpair);
        printf("mutable_struct=%d %d\n", cps->a, cps->b);
        cps->a = 80;
        cps->b = 90;
        printf("mutable_struct_written=%d %d\n", mpair.a, mpair.b);
        pcs = &mpair;               /* a non-const object read through const * */
        printf("const_view_of_mutable=%d %d\n", pcs->a, pcs->b);
    }

    /* ---- qualified parameters ---- */
    printf("sum_const_array=%d\n", sum_const(carr, 5));
    printf("sum_const_partial=%d\n", sum_const(carr, 3));
    printf("sum_const_interior=%d\n", sum_const(&carr[2], 3));
    printf("sum_volatile_array=%d\n", sum_volatile(varr, 5));
    printf("sum_volatile_partial=%d\n", sum_volatile(varr, 2));
    printf("sum_const_of_mutable=%d\n", sum_const(&plain, 1));

    /* ---- a volatile counter: every increment must survive ---- */
    vcounter = 0;
    {
        int i;
        for (i = 0; i < 7; ++i) {
            vcounter = vcounter + 1;
        }
    }
    printf("volatile_counter=%d\n", vcounter);
    vcounter = vcounter + vcounter;
    printf("volatile_counter_doubled=%d\n", vcounter);

    /* a volatile sink read repeatedly: each read is a genuine load */
    vsink = 3;
    {
        int total = 0;
        int i;
        for (i = 0; i < 4; ++i) {
            total += vsink;
        }
        printf("volatile_sink_total=%d\n", total);
    }

    /* ---- runtime variant: volatile indices into qualified arrays ---- */
    vidx = 3;
    k = vidx;
    printf("runtime_const_index=%d\n", carr[k]);
    printf("runtime_volatile_index=%d\n", varr[k]);
    pc = &carr[k];
    printf("runtime_const_ptr_read=%d\n", *pc);
    printf("runtime_const_ptr_offset=%lld\n", (long long)(pc - carr));
    pv = &varr[k];
    printf("runtime_volatile_ptr_read=%d\n", *pv);
    *pv = 41;
    printf("runtime_volatile_ptr_written=%d\n", varr[3]);
    printf("runtime_volatile_ptr_offset=%lld\n", (long long)(pv - varr));

    vidx = 5;
    k = vidx;
    printf("runtime_sum_const=%d\n", sum_const(carr, k));
    printf("runtime_sum_volatile=%d\n", sum_volatile(varr, k));

    vidx = 2;
    k = vidx;
    pcv = &varr[k];
    printf("runtime_const_volatile_read=%d\n", *pcv);
    printf("runtime_const_volatile_offset=%lld\n", (long long)(pcv - varr));
    pi = &plain;
    bump(pi, k);
    printf("runtime_bump=%d\n", plain);

    return 0;
}
