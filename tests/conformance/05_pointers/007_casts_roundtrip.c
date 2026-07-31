/* Area 05 - Pointer arithmetic and function pointers
 * 007_casts_roundtrip: object-pointer casts round-tripped through a
 * sufficiently wide integer type.
 *
 * WIDTH DISCIPLINE.  sizeof(void *) is 4 on i686 and 8 on the other three
 * supported targets, so the carrier type must be wide enough everywhere.
 * unsigned long long is 8 bytes on all four targets and is therefore always
 * wide enough; uintptr_t is not used because it would require stdint.h and
 * no header may be included.  The conversion back to a pointer goes through
 * unsigned long, which is exactly pointer width on all four targets (8 on
 * x86-64, AArch64 and RISC-V 64; 4 on i686), so no cast ever crosses a
 * width boundary and no value is ever truncated.
 *
 * Neither the pointer nor the integer carrier is ever printed.  The only
 * things printed are equality results, dereferenced object values and
 * pointer differences cast to long long.
 */

int printf(const char *, ...);

struct pair {
    int a;
    int b;
};

static int obj = 7;
static double dobj = 2.5;
static char carr[16] = { 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h',
                         'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p' };
static int iarr[8] = { 10, 20, 30, 40, 50, 60, 70, 80 };
static struct pair sobj = { 3, 4 };
static const int cobj = 99;

int main(void)
{
    unsigned long long carrier;
    volatile int vidx;
    int k;

    /* the width facts this program relies on, asserted rather than assumed */
    printf("carrier_wide_enough=%d\n",
           sizeof(unsigned long long) >= sizeof(void *));
    printf("bridge_is_pointer_width=%d\n",
           sizeof(unsigned long) == sizeof(void *));

    /* ---- int * round-trip ---- */
    {
        int *p = &obj;
        int *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (int *)(void *)(unsigned long)carrier;
        printf("int_roundtrip_eq=%d\n", q == p);
        printf("int_roundtrip_value=%d\n", *q);
        printf("int_roundtrip_same_object=%d\n", q == &obj);
    }

    /* ---- double * round-trip ---- */
    {
        double *p = &dobj;
        double *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (double *)(void *)(unsigned long)carrier;
        printf("double_roundtrip_eq=%d\n", q == p);
        printf("double_roundtrip_value=%.3f\n", *q);
    }

    /* ---- char * round-trip, including an interior element ---- */
    {
        char *p = carr;
        char *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (char *)(void *)(unsigned long)carrier;
        printf("char_roundtrip_eq=%d\n", q == p);
        printf("char_roundtrip_value=%d\n", (int)(unsigned char)*q);

        p = &carr[5];
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (char *)(void *)(unsigned long)carrier;
        printf("char_interior_eq=%d\n", q == p);
        printf("char_interior_value=%d\n", (int)(unsigned char)*q);
        printf("char_interior_offset=%lld\n", (long long)(q - carr));
    }

    /* ---- struct * round-trip ---- */
    {
        struct pair *p = &sobj;
        struct pair *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (struct pair *)(void *)(unsigned long)carrier;
        printf("struct_roundtrip_eq=%d\n", q == p);
        printf("struct_roundtrip_values=%d %d\n", q->a, q->b);
    }

    /* ---- const int * round-trip preserving the qualifier ---- */
    {
        const int *p = &cobj;
        const int *q;
        carrier = (unsigned long long)(unsigned long)(const void *)p;
        q = (const int *)(const void *)(unsigned long)carrier;
        printf("const_roundtrip_eq=%d\n", q == p);
        printf("const_roundtrip_value=%d\n", *q);
    }

    /* ---- a pointer into the middle of an array still supports arithmetic
     *      after the round-trip ---- */
    {
        int *p = &iarr[3];
        int *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (int *)(void *)(unsigned long)carrier;
        printf("interior_roundtrip_eq=%d\n", q == p);
        printf("interior_value=%d\n", *q);
        printf("interior_offset=%lld\n", (long long)(q - iarr));
        printf("interior_step_back=%d\n", q[-1]);
        printf("interior_step_forward=%d\n", q[2]);
        printf("interior_diff_matches=%lld\n", (long long)(q - &iarr[0]));
    }

    /* ---- a null pointer survives the round-trip as a null pointer ---- */
    {
        int *p = 0;
        int *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (int *)(void *)(unsigned long)carrier;
        printf("null_roundtrip_eq=%d\n", q == p);
        printf("null_roundtrip_is_null=%d\n", q == 0);
    }

    /* ---- void * as an intermediate type, no integer involved ---- */
    {
        int *p = &iarr[6];
        void *v = p;
        int *q = (int *)v;
        printf("void_roundtrip_eq=%d\n", q == p);
        printf("void_roundtrip_value=%d\n", *q);
        printf("void_roundtrip_offset=%lld\n", (long long)(q - iarr));
    }

    /* ---- char * as an intermediate byte view, then back ---- */
    {
        int *p = &iarr[2];
        char *bytes = (char *)(void *)p;
        int *q = (int *)(void *)bytes;
        printf("bytes_roundtrip_eq=%d\n", q == p);
        printf("bytes_roundtrip_value=%d\n", *q);
    }

    /* ---- runtime variant: a volatile index defeats constant folding ---- */
    vidx = 4;
    k = vidx;
    {
        int *p = &iarr[k];
        int *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (int *)(void *)(unsigned long)carrier;
        printf("runtime_roundtrip_eq=%d\n", q == p);
        printf("runtime_roundtrip_value=%d\n", *q);
        printf("runtime_roundtrip_offset=%lld\n", (long long)(q - iarr));
        printf("runtime_roundtrip_target=%d\n", q == &iarr[4]);
    }

    vidx = 7;
    k = vidx;
    {
        char *p = &carr[k];
        char *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (char *)(void *)(unsigned long)carrier;
        printf("runtime_char_eq=%d\n", q == p);
        printf("runtime_char_value=%d\n", (int)(unsigned char)*q);
        printf("runtime_char_offset=%lld\n", (long long)(q - carr));
    }

    /* ---- the round-trip is idempotent when repeated ---- */
    vidx = 1;
    k = vidx;
    {
        int *p = &iarr[k];
        int *q;
        int i;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        for (i = 0; i < 3; ++i) {
            int *tmp = (int *)(void *)(unsigned long)carrier;
            carrier = (unsigned long long)(unsigned long)(void *)tmp;
        }
        q = (int *)(void *)(unsigned long)carrier;
        printf("repeated_roundtrip_eq=%d\n", q == p);
        printf("repeated_roundtrip_value=%d\n", *q);
    }

    return 0;
}
