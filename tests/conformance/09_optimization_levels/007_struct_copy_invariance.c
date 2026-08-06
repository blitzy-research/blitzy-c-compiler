/* Aggregate construction, return by value, assignment copy and pass by value must
 * all produce the same values at every optimization level.  The four aggregates
 * differ in size and in the kinds of members they hold, and ALL FOUR of them go
 * through ALL FOUR operations: none of the shapes is exempted from pass by value,
 * because the shape most likely to break there -- mixed, whose int and double span
 * two register classes -- is exactly the one it would be most tempting to leave
 * out.  Which of those shapes travel in registers and which in memory is the ABI's
 * decision, not this program's, and only the retrieved member values are compared.
 *
 * Every member is read back individually, and for pass by value it is read back
 * from INSIDE the consumer, which is the only place the members that actually
 * crossed the parameter-passing boundary can be seen.  That is eighteen direct
 * parameter-member observations: small a and b; boundary a, b, c and d; mixed tag
 * and value; large v[0] through v[7], checksum and flag.  The scalar reductions
 * each consumer returns -- small_byvalue, boundary_byvalue, mixed_byvalue and
 * large_byvalue -- are kept as additional cross-checks rather than as substitutes,
 * because a sum agrees with the per-member lines only if every member arrived
 * intact, whereas a sum on its own can be reproduced by two compensating errors,
 * and a pair of swapped members is precisely the ABI defect this program hunts. */

int printf(const char *, ...);

struct small {
    int a;
    int b;
};

struct boundary {
    int a;
    int b;
    int c;
    int d;
};

struct mixed {
    int tag;
    double value;
};

struct large {
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

/* The four by-value consumers.  Each prints every member of the parameter it
   received before reducing it, so the members are observed on the callee side of
   the calling convention rather than in the caller, and returns the reduction as
   an additional cross-check.  Each call is a statement of its own in main whose
   result is stored in a local before it is printed, so no printf argument
   expression has a side effect even though the consumers now write to stdout. */
static int sum_small(struct small s)
{
    printf("small_param a=%d b=%d\n", s.a, s.b);
    return s.a + s.b;
}

static int sum_boundary(struct boundary s)
{
    printf("boundary_param a=%d b=%d c=%d d=%d\n", s.a, s.b, s.c, s.d);
    return s.a + s.b + s.c + s.d;
}

/* mixed is passed by value too, and its reduction is a double so that the value
   member is carried through arithmetic rather than truncated: an int result would
   have discarded the fractional part that makes this the SSE / v-register /
   fa-register half of the shape under test.  The tag is converted explicitly
   because 5 and 6.75 are both exactly representable and the sum, 11.75, is exact
   in binary floating point, so %.3f prints it identically on every target. */
static double sum_mixed(struct mixed s)
{
    printf("mixed_param tag=%d value=%.3f\n", s.tag, s.value);
    return (double)s.tag + s.value;
}

static int sum_large(struct large s)
{
    int i;
    int total = s.flag;
    for (i = 0; i < 8; i++) {
        printf("large_param v[%d]=%d\n", i, s.v[i]);
        total += s.v[i];
    }
    printf("large_param checksum=%d flag=%d\n", s.checksum, s.flag);
    return total + s.checksum;
}

int main(void)
{
    volatile int v_base = 5;
    volatile double v_val = 6.75;
    int base = v_base;
    double val = v_val;
    int i;
    int r_small;
    int r_boundary;
    double r_mixed;
    int r_large;

    struct small s1 = make_small(base, base * 2);
    struct small s2 = s1;
    struct boundary b1 = make_boundary(base);
    struct boundary b2 = b1;
    struct mixed m1 = make_mixed(base, val);
    struct mixed m2 = m1;
    struct large l1 = make_large(base);
    struct large l2 = l1;

    printf("small a=%d b=%d\n", s1.a, s1.b);
    printf("small_copy a=%d b=%d\n", s2.a, s2.b);
    r_small = sum_small(s2);
    printf("small_byvalue=%d\n", r_small);

    printf("boundary a=%d b=%d c=%d d=%d\n", b1.a, b1.b, b1.c, b1.d);
    printf("boundary_copy a=%d b=%d c=%d d=%d\n", b2.a, b2.b, b2.c, b2.d);
    r_boundary = sum_boundary(b2);
    printf("boundary_byvalue=%d\n", r_boundary);

    printf("mixed tag=%d value=%.3f\n", m1.tag, m1.value);
    printf("mixed_copy tag=%d value=%.3f\n", m2.tag, m2.value);
    r_mixed = sum_mixed(m2);
    printf("mixed_byvalue=%.3f\n", r_mixed);

    for (i = 0; i < 8; i++) {
        printf("large v[%d]=%d copy_v[%d]=%d\n", i, l1.v[i], i, l2.v[i]);
    }
    printf("large checksum=%d flag=%d\n", l1.checksum, l1.flag);
    printf("large_copy checksum=%d flag=%d\n", l2.checksum, l2.flag);
    r_large = sum_large(l2);
    printf("large_byvalue=%d\n", r_large);
    return 0;
}
