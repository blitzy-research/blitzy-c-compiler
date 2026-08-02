/* Area 14 - ABI and calling convention.
 * 003_mixed_parameter_classes: interleaved integer, floating, pointer and
 * aggregate parameters.  Twenty-six parameters arranged as five repetitions of
 * (int, double, double, pointer, 8-byte aggregate) followed by an integer tag.
 * That is sixteen integer-class consumers and ten floating-class consumers, so
 * BOTH register files are exhausted on System V AMD64, AAPCS64 and LP64D, while
 * System V i386 cdecl marshals all twenty-six on the stack.  Interleaving is
 * the point: it forces each ABI to advance its integer and floating allocators
 * independently and in step, which is exactly where the four conventions
 * disagree.
 *
 * Pointer discipline: pointers are passed but no pointer value is ever printed.
 * Only the pointed-to int is printed.  Aggregate discipline: only named members
 * are read back; no padding byte is printed, compared or memcmp'd.
 */

int printf(const char *, ...);

struct pair2 {
    int lo;
    int hi;
};

static const char *variant_tag(int variant);
static void take_mixed26(int i1, double d1, double e1, const int *p1,
                         struct pair2 s1,
                         int i2, double d2, double e2, const int *p2,
                         struct pair2 s2,
                         int i3, double d3, double e3, const int *p3,
                         struct pair2 s3,
                         int i4, double d4, double e4, const int *p4,
                         struct pair2 s4,
                         int i5, double d5, double e5, const int *p5,
                         struct pair2 s5,
                         int variant);

static const int ptgt[5] = { 3001, -3002, 3003, -3004, 3005 };

static volatile int vint[5] = { 201, -202, 203, -204, 205 };
static volatile double vdbl[5] = { 1.5, -2.25, 3.125, -4.0625, 5.5 };
static volatile double vdbe[5] = { -6.75, 7.875, -8.1875, 9.25, -10.5 };
static volatile int vslo[5] = { 41, 43, 45, 47, 49 };
static volatile int vshi[5] = { -42, -44, -46, -48, -50 };
static volatile int vtag = 1;

static const char *variant_tag(int variant)
{
    return (variant == 0) ? "folded" : "runtime";
}

static void take_mixed26(int i1, double d1, double e1, const int *p1,
                         struct pair2 s1,
                         int i2, double d2, double e2, const int *p2,
                         struct pair2 s2,
                         int i3, double d3, double e3, const int *p3,
                         struct pair2 s3,
                         int i4, double d4, double e4, const int *p4,
                         struct pair2 s4,
                         int i5, double d5, double e5, const int *p5,
                         struct pair2 s5,
                         int variant)
{
    const char *t = variant_tag(variant);
    printf("mix_%s_i1=%d\n", t, i1);
    printf("mix_%s_d1=%.4f\n", t, d1);
    printf("mix_%s_e1=%.4f\n", t, e1);
    printf("mix_%s_p1_deref=%d\n", t, *p1);
    printf("mix_%s_s1_lo=%d s1_hi=%d\n", t, s1.lo, s1.hi);
    printf("mix_%s_i2=%d\n", t, i2);
    printf("mix_%s_d2=%.4f\n", t, d2);
    printf("mix_%s_e2=%.4f\n", t, e2);
    printf("mix_%s_p2_deref=%d\n", t, *p2);
    printf("mix_%s_s2_lo=%d s2_hi=%d\n", t, s2.lo, s2.hi);
    printf("mix_%s_i3=%d\n", t, i3);
    printf("mix_%s_d3=%.4f\n", t, d3);
    printf("mix_%s_e3=%.4f\n", t, e3);
    printf("mix_%s_p3_deref=%d\n", t, *p3);
    printf("mix_%s_s3_lo=%d s3_hi=%d\n", t, s3.lo, s3.hi);
    printf("mix_%s_i4=%d\n", t, i4);
    printf("mix_%s_d4=%.4f\n", t, d4);
    printf("mix_%s_e4=%.4f\n", t, e4);
    printf("mix_%s_p4_deref=%d\n", t, *p4);
    printf("mix_%s_s4_lo=%d s4_hi=%d\n", t, s4.lo, s4.hi);
    printf("mix_%s_i5=%d\n", t, i5);
    printf("mix_%s_d5=%.4f\n", t, d5);
    printf("mix_%s_e5=%.4f\n", t, e5);
    printf("mix_%s_p5_deref=%d\n", t, *p5);
    printf("mix_%s_s5_lo=%d s5_hi=%d\n", t, s5.lo, s5.hi);
}

int main(void)
{
    struct pair2 c1 = { 41, -42 };
    struct pair2 c2 = { 43, -44 };
    struct pair2 c3 = { 45, -46 };
    struct pair2 c4 = { 47, -48 };
    struct pair2 c5 = { 49, -50 };
    struct pair2 r1;
    struct pair2 r2;
    struct pair2 r3;
    struct pair2 r4;
    struct pair2 r5;

    take_mixed26(201, 1.5, -6.75, &ptgt[0], c1,
                 -202, -2.25, 7.875, &ptgt[1], c2,
                 203, 3.125, -8.1875, &ptgt[2], c3,
                 -204, -4.0625, 9.25, &ptgt[3], c4,
                 205, 5.5, -10.5, &ptgt[4], c5,
                 0);

    r1.lo = vslo[0];
    r1.hi = vshi[0];
    r2.lo = vslo[1];
    r2.hi = vshi[1];
    r3.lo = vslo[2];
    r3.hi = vshi[2];
    r4.lo = vslo[3];
    r4.hi = vshi[3];
    r5.lo = vslo[4];
    r5.hi = vshi[4];

    take_mixed26(vint[0], vdbl[0], vdbe[0], &ptgt[0], r1,
                 vint[1], vdbl[1], vdbe[1], &ptgt[1], r2,
                 vint[2], vdbl[2], vdbe[2], &ptgt[2], r3,
                 vint[3], vdbl[3], vdbe[3], &ptgt[3], r4,
                 vint[4], vdbl[4], vdbe[4], &ptgt[4], r5,
                 vtag);
    return 0;
}
