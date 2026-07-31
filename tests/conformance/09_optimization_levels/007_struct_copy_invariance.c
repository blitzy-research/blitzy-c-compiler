/* 007_struct_copy_invariance.c -- Area 09, differential conformance suite.
 *
 * Claim under test: aggregate construction, return by value, assignment copy and
 * pass by value produce IDENTICAL results at -O0, -O1 and -O2, and across all
 * four backends.  The aggregates straddle every target's by-register versus
 * by-memory threshold, and struct mixed deliberately spans two register classes
 * (integer plus floating point), which is the shape of a documented -O1/-O2
 * defect the project has already experienced once.  Every member is read back
 * individually so a single wrong slot is localized immediately.
 */

int printf(const char *, ...);

/* Aggregates on both sides of every target's by-register / by-memory
   parameter-passing threshold (16 bytes on x86-64, AArch64 and RISC-V 64;
   i686 passes every aggregate in memory). */
struct small {           /* 8 bytes: comfortably by register */
    int a;
    int b;
};

struct boundary {        /* 16 bytes: exactly at the threshold */
    int a;
    int b;
    int c;
    int d;
};

struct mixed {           /* 16 bytes, two register classes */
    int tag;
    double value;
};

struct large {           /* 40 bytes: by memory on every target */
    int v[8];
    int checksum;
    int flag;
};

static struct small make_small(int a, int b)
{
    struct small s;
    s.a = a;
    s.b = b;
    return s;
}

static struct boundary make_boundary(int base)
{
    struct boundary s;
    s.a = base;
    s.b = base + 1;
    s.c = base + 2;
    s.d = base + 3;
    return s;
}

static struct mixed make_mixed(int tag, double value)
{
    struct mixed s;
    s.tag = tag;
    s.value = value;
    return s;
}

static struct large make_large(int base)
{
    struct large s;
    int i;
    s.checksum = 0;
    for (i = 0; i < 8; i++) {
        s.v[i] = base + i * 3;
        s.checksum += s.v[i];
    }
    s.flag = 1;
    return s;
}

static int sum_small(struct small s)
{
    return s.a + s.b;
}

static int sum_boundary(struct boundary s)
{
    return s.a + s.b + s.c + s.d;
}

static int sum_large(struct large s)
{
    int i;
    int total = s.flag;
    for (i = 0; i < 8; i++) {
        total += s.v[i];
    }
    return total + s.checksum;
}

int main(void)
{
    volatile int v_base = 5;
    volatile double v_val = 6.75;
    int base = v_base;
    double val = v_val;
    int i;

    /* Construct, return by value, copy by assignment, pass by value. */
    struct small s1 = make_small(base, base * 2);
    struct small s2 = s1;                       /* assignment copy */
    struct boundary b1 = make_boundary(base);
    struct boundary b2 = b1;
    struct mixed m1 = make_mixed(base, val);
    struct mixed m2 = m1;
    struct large l1 = make_large(base);
    struct large l2 = l1;

    /* Every member is read back individually so a single wrong slot is
       localized immediately. */
    printf("small a=%d b=%d\n", s1.a, s1.b);
    printf("small_copy a=%d b=%d\n", s2.a, s2.b);
    printf("small_byvalue=%d\n", sum_small(s2));

    printf("boundary a=%d b=%d c=%d d=%d\n", b1.a, b1.b, b1.c, b1.d);
    printf("boundary_copy a=%d b=%d c=%d d=%d\n", b2.a, b2.b, b2.c, b2.d);
    printf("boundary_byvalue=%d\n", sum_boundary(b2));

    printf("mixed tag=%d value=%.3f\n", m1.tag, m1.value);
    printf("mixed_copy tag=%d value=%.3f\n", m2.tag, m2.value);

    for (i = 0; i < 8; i++) {
        printf("large v[%d]=%d copy_v[%d]=%d\n", i, l1.v[i], i, l2.v[i]);
    }
    printf("large checksum=%d flag=%d\n", l1.checksum, l1.flag);
    printf("large_copy checksum=%d flag=%d\n", l2.checksum, l2.flag);
    printf("large_byvalue=%d\n", sum_large(l2));
    return 0;
}
