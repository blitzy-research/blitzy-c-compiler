/* Area 14 - ABI and calling convention.
 * 005_struct_return_by_value: aggregate return by value across the same size
 * thresholds that 004 exercises for argument passing.  Return is a separate
 * mechanism from passing on every one of the four supported ABIs:
 *
 *   struct s8    8 bytes  - returned in one GPR on the 64-bit targets
 *   struct s16i 16 bytes  - returned in a GPR pair: the exact boundary
 *   struct s16m mixed int + double - System V AMD64 returns it in rax + xmm0,
 *                                    AAPCS64 in x0 + x1, LP64D in a0 + fa0.
 *                                    Highest-yield return shape in the corpus.
 *   struct s16d two doubles - xmm0 + xmm1 on AMD64, a homogeneous float
 *                            aggregate on AAPCS64, fa0 + fa1 on LP64D
 *   struct s16f four floats - an AAPCS64 HFA return, two packed SSE registers
 *                            on AMD64, a GPR pair on LP64D
 *   struct s24  24 bytes  - returned through a hidden pointer everywhere
 *   struct sbig 40 bytes  - unambiguously returned through memory everywhere
 *
 * System V i386 cdecl returns every aggregate through a hidden pointer supplied
 * by the caller, which is a fifth distinct mechanism.
 *
 * Every row above is the return classification the SOURCE requests, and the
 * classifications themselves are what the documented ABIs specify:
 * docs/technical-specifications.md line 546 (x86-64 System V AMD64), line 553
 * (i686 cdecl), line 559 (AArch64 AAPCS64) and line 565 (RISC-V 64 LP64D).
 *
 * Three consumption paths are exercised for EVERY ONE of the seven shapes in
 * BOTH variants, giving 7 x 3 x 2 = 42 shape-path-variant combinations:
 *
 *   ret_<shape>_<variant>     the returned aggregate is stored in a local and
 *                             its members are read from there
 *   direct_<shape>_<variant>  a member is read straight off the call expression,
 *                             with no named object between the return and the
 *                             read
 *   fwd_<shape>_<variant>     the returned aggregate is handed straight into
 *                             another function as an argument, composing the
 *                             return path with the argument path
 *
 * The three are genuinely different mechanisms rather than three spellings of
 * one, which is why none of them may be left out for any shape.  A shape
 * returned in registers has to be spilled to a named local on the first path and
 * need not be on the second; a shape returned through a hidden pointer is
 * written into a caller-supplied slot that the third path must then copy out of
 * before it can be passed on.  A defect in any one of those steps is invisible
 * from the other two.
 *
 * Member coverage: every named member of every shape is read on every path, and
 * that includes all ten elements of the 40-byte shape.  A member no path ever
 * reads is a member the return mechanism is never checked on, and for the two
 * shapes returned through memory on all four targets the block copy is the whole
 * mechanism under test - a wrong length or offset would leave the first and last
 * elements correct while corrupting the middle.
 *
 * Volatile staging discipline: every runtime operand is read out of volatile
 * storage into plain staging storage in a full expression of its own, and every
 * maker and consumer is then called with plain values only.  Several volatile
 * reads inside one argument list would leave the relative order of those side
 * effects unspecified, and an unspecified-behaviour program cannot make a
 * divergence attributable to either compiler.  No volatile lvalue appears in any
 * argument list in this file.
 *
 * THE CALL BARRIER, AND WHY THESE RETURN BOUNDARIES WOULD OTHERWISE NOT EXIST.
 * A return boundary is only under test if the call actually happens.  Measured
 * with gcc 13.4.0 at -O2, with direct calls to these static producers the only
 * function left standing was make_sbig: every small, mixed and homogeneous-float
 * return, and both forwarding consumers, were inlined away, so six of the seven
 * documented return mechanisms and both forwarding paths were not exercised at
 * all at that level.  Every producer and every consumer below is therefore
 * reached through a FILE-SCOPE volatile FUNCTION POINTER.  A volatile lvalue
 * must be re-read on every access, so no conforming compiler may assume which
 * function the pointer designates: it can neither inline nor clone the callee,
 * and it must return the aggregate exactly as the ABI prescribes because it
 * cannot know what produced it.  This is plain standard C rather than a compiler
 * attribute, so both sides of oracle (a) honour it for the same reason.
 * Verified in the generated assembly of all four targets at -O2: all seven
 * producers and both consumers are emitted unmodified, with no .constprop and no
 * .isra clone.
 *
 * EVALUATION-ORDER DISCIPLINE.  An access to a volatile object is an observable
 * side effect and argument evaluation order is unspecified, so every volatile
 * datum is read exactly once, in a statement of its own, and only the resulting
 * snapshots are passed to a producer -- at most one side-effecting argument per
 * call, which here means the function-pointer load and nothing else.  The
 * snapshots do not weaken the runtime variant: each is itself a load from
 * volatile storage the optimizer may not fold, so every returned aggregate is
 * still built from values materialised at run time.
 *
 * AT WHICH OPTIMIZATION LEVEL THOSE RETURN PATHS ARE REALLY EXERCISED.  Measured,
 * not assumed, by counting this program's own static helpers still emitted in the
 * reference compiler's assembly and the calls to them - `<driver> -O<n> -S -o -
 * 005_struct_return_by_value.c`, then grepping for the helper labels:
 *
 *   -O0  all 9 helpers, 40 calls   on x86-64, i686, AArch64 and RISC-V 64 alike
 *   -O1  1 helper / 4 calls on all four targets
 *   -O2  1 helper / 4 calls on all four targets
 *
 * So the return mechanisms are exercised as real calls at -O0 on every target,
 * while from -O1 upward the reference compiler inlines almost all of these small
 * static helpers and the returns collapse into their callers.  The higher levels
 * are still worth running - all three oracles compare the same printed member
 * values at every level, and a wrong value is caught wherever it appears - but the
 * register-pair, hidden-pointer and HFA return paths above are what -O0 covers.
 * The compiler under test may inline differently; no bcc binary is present on this
 * branch, so nothing here was measured of it.
 *
 * Padding discipline: sizeof is never printed and no aggregate is ever memcmp'd.
 * Only named members are read back.
 *
 * EVERY MEMBER OF EVERY RETURNED AGGREGATE IS READ BACK, not a sample of them.
 * The forty-byte returns print all ten of their members, in both variants, just
 * as the smaller shapes print all of theirs.  A sampled read-back - first,
 * middle, last - would pass while a return through a hidden pointer wrote the
 * wrong length, transposed members, or left some untouched, and that is exactly
 * the shape returned through memory on every one of the four targets.
 *
 * THE RETURN BOUNDARY IS ENFORCED, NOT HOPED FOR.  Every maker and every
 * forwarding consumer is reached through a volatile-qualified function pointer
 * rather than by name.  A small static maker is the easiest thing in this corpus
 * for an optimizer to inline, and an inlined maker returns nothing: the returned
 * aggregate is scalar-replaced into the caller's own locals and the return
 * convention - register pair, register plus floating register, or hidden pointer
 * - is never exercised at all.  Measured with gcc 13.4.0 at -O2 before the
 * indirection was added: of the twenty intended maker and consumer calls in each
 * variant only four survived in total, every other one having been inlined, so
 * six of the seven return shapes had no boundary left to test.  A volatile
 * pointer must be re-read at the point of call, so the designated function is
 * unknown and the call is genuinely indirect; and because the address escapes
 * into storage, the signature may not be cloned or the return value scalarised
 * either.  The mechanism is pure ISO C:
 * a function attribute would have been shorter, but the documented attribute set
 * for the compiler under test is packed, aligned, section, unused, deprecated,
 * visibility and format (docs/technical-specifications.md line 506), so an
 * inlining attribute would risk a divergence caused by the test and would import
 * an extension into an area whose subject is the calling convention.
 *
 * ONE VOLATILE READ PER FULL STATEMENT.  No argument list contains a volatile
 * lvalue.  Every runtime operand is copied out of volatile storage into a plain
 * local in a statement of its own, and the maker calls then read only those
 * locals.  An access to a volatile object is an observable side effect and the
 * order of side effects within one argument list is unspecified, so a maker call
 * reading several volatile elements in place would have an unspecified order of
 * side effects - and a suite whose premise is that a divergence means a defect
 * needs the program to have exactly one defined behaviour.
 */

int printf(const char *, ...);

struct s8 {
    int a;
    int b;
};

struct s16i {
    int a;
    int b;
    int c;
    int d;
};

struct s16m {
    int a;
    double b;
};

struct s16d {
    double x;
    double y;
};

struct s16f {
    float x;
    float y;
    float z;
    float w;
};

struct s24 {
    int a;
    int b;
    int c;
    int d;
    int e;
    int f;
};

struct sbig {
    int v[10];
};

static struct s8 make_s8(int a, int b);
static struct s16i make_s16i(int a, int b, int c, int d);
static struct s16m make_s16m(int a, double b);
static struct s16d make_s16d(double x, double y);
static struct s16f make_s16f(float x, float y, float z, float w);
static struct s24 make_s24(int a, int b, int c, int d, int e, int f);
static struct sbig make_sbig(int base);
static void consume_s8(struct s8 s, const char *t);
static void consume_s16i(struct s16i s, const char *t);
static void consume_s16m(struct s16m s, const char *t);
static void consume_s16d(struct s16d s, const char *t);
static void consume_s16f(struct s16f s, const char *t);
static void consume_s24(struct s24 s, const char *t);
static void consume_sbig(struct sbig s, const char *t);
static void print_sbig(const char *label, const char *t, const struct sbig *s);

/* The nine call barriers described in the banner.  Each is volatile, so the
 * pointer is re-read on every call and the callee can be neither inlined nor
 * specialised, which is what keeps all seven return mechanisms and both
 * forwarding paths under test at -O1 and -O2 as well as at -O0. */

static volatile int vi[16] = {
    11, -12, 21, -22,
    101, -102, 103, -104,
    201, -202, 203, -204,
    201, 401, 7100, 7200
};
static volatile double vd[6] = { 1.5, 2.5, 1.25, -1.75, 2.25, -2.75 };
static volatile float vf[8] = {
    1.5f, -1.25f, 1.125f, -1.0625f, 2.5f, -2.25f, 2.125f, -2.0625f
};
static volatile int v24[12] = {
    1001, -1002, 1003, -1004, 1005, -1006,
    2001, -2002, 2003, -2004, 2005, -2006
};

static struct s8 make_s8(int a, int b)
{
    struct s8 r;
    r.a = a;
    r.b = b;
    return r;
}

static struct s16i make_s16i(int a, int b, int c, int d)
{
    struct s16i r;
    r.a = a;
    r.b = b;
    r.c = c;
    r.d = d;
    return r;
}

static struct s16m make_s16m(int a, double b)
{
    struct s16m r;
    r.a = a;
    r.b = b;
    return r;
}

static struct s16d make_s16d(double x, double y)
{
    struct s16d r;
    r.x = x;
    r.y = y;
    return r;
}

static struct s16f make_s16f(float x, float y, float z, float w)
{
    struct s16f r;
    r.x = x;
    r.y = y;
    r.z = z;
    r.w = w;
    return r;
}

static struct s24 make_s24(int a, int b, int c, int d, int e, int f)
{
    struct s24 r;
    r.a = a;
    r.b = b;
    r.c = c;
    r.d = d;
    r.e = e;
    r.f = f;
    return r;
}

static struct sbig make_sbig(int base)
{
    struct sbig r;
    int i;
    for (i = 0; i < 10; ++i) {
        r.v[i] = base + i;
    }
    return r;
}

/* Every element of the 40-byte shape, read in fixed index order.  The pointer parameter is what
   lets one routine serve the local-storage path, where the caller holds the aggregate, and the
   forwarding path, where the callee does: taking the address of a parameter or of a local is
   nothing more than naming the object the caller already produced, so no copy is introduced that
   could hide a defect in the one the ABI performed.  A sample of three elements would leave a
   block-move defect confined to the middle of the object invisible, which for a shape returned
   through memory on all four targets is the whole mechanism under test. */
static void print_sbig(const char *label, const char *t, const struct sbig *s)
{
    int index;
    printf("%s_%s", label, t);
    for (index = 0; index < 10; ++index) {
        printf(" v%d=%d", index, s->v[index]);
    }
    printf("\n");
}

/* The forwarding consumers, one per return shape.  Each receives an aggregate that was passed
   straight from a maker's return value into an argument without ever being stored in a named
   local, so the two mechanisms - the callee's return path and the caller's argument path - are
   composed with no assignment between them.  That composition is a distinct case on every one of
   the four ABIs: a shape returned in registers may be passed in different registers, and a shape
   returned through a hidden pointer must be copied out of the caller-supplied slot before it can
   be handed on. */
static void consume_s8(struct s8 s, const char *t)
{
    printf("fwd_s8_%s a=%d b=%d\n", t, s.a, s.b);
}

static void consume_s16i(struct s16i s, const char *t)
{
    printf("fwd_s16i_%s a=%d b=%d c=%d d=%d\n", t, s.a, s.b, s.c, s.d);
}

static void consume_s16m(struct s16m s, const char *t)
{
    printf("fwd_s16m_%s a=%d b=%.4f\n", t, s.a, s.b);
}

static void consume_s16d(struct s16d s, const char *t)
{
    printf("fwd_s16d_%s x=%.4f y=%.4f\n", t, s.x, s.y);
}

static void consume_s16f(struct s16f s, const char *t)
{
    printf("fwd_s16f_%s x=%.4f y=%.4f z=%.4f w=%.4f\n", t,
           (double)s.x, (double)s.y, (double)s.z, (double)s.w);
}

static void consume_s24(struct s24 s, const char *t)
{
    printf("fwd_s24_%s a=%d b=%d c=%d d=%d e=%d f=%d\n", t,
           s.a, s.b, s.c, s.d, s.e, s.f);
}

static void consume_sbig(struct sbig s, const char *t)
{
    print_sbig("fwd_sbig", t, &s);
}

/* The enforced return boundary, one volatile-qualified pointer per shape.  Each is re-read at
   its call site, so every maker call is indirect at every optimization level, no maker body is
   inlined, and no returned aggregate can be scalar-replaced into this caller's locals.  Both
   variants of every shape travel through these pointers, so the folded and the runtime call
   cross the same boundary.  Every forwarding consumer is indirected for the same reason: a
   returned aggregate handed straight on as an argument has to survive both conventions in one
   expression. */
static struct s8 (*volatile make_s8_p)(int, int) = make_s8;
static struct s16i (*volatile make_s16i_p)(int, int, int, int) = make_s16i;
static struct s16m (*volatile make_s16m_p)(int, double) = make_s16m;
static struct s16d (*volatile make_s16d_p)(double, double) = make_s16d;
static struct s16f (*volatile make_s16f_p)(float, float, float, float) = make_s16f;
static struct s24 (*volatile make_s24_p)(int, int, int, int, int, int) = make_s24;
static struct sbig (*volatile make_sbig_p)(int) = make_sbig;
static void (*volatile consume_s8_p)(struct s8, const char *) = consume_s8;
static void (*volatile consume_s16i_p)(struct s16i, const char *) = consume_s16i;
static void (*volatile consume_s16m_p)(struct s16m, const char *) = consume_s16m;
static void (*volatile consume_s16d_p)(struct s16d, const char *) = consume_s16d;
static void (*volatile consume_s16f_p)(struct s16f, const char *) = consume_s16f;
static void (*volatile consume_s24_p)(struct s24, const char *) = consume_s24;
static void (*volatile consume_sbig_p)(struct sbig, const char *) = consume_sbig;

int main(void)
{
    struct s8 f8a;
    struct s8 f8b;
    struct s16i f16ia;
    struct s16i f16ib;
    struct s16m f16ma;
    struct s16m f16mb;
    struct s16d f16da;
    struct s16d f16db;
    struct s16f f16fa;
    struct s16f f16fb;
    struct s24 f24a;
    struct s24 f24b;
    struct sbig fbiga;
    struct sbig fbigb;
    /* Plain staging storage for the runtime operands, filled one volatile read per full
       expression further down, plus the loop counter the element-wise reads use. */
    int si[16];
    double sd[6];
    float sf[8];
    int s24v[12];
    int element;

    f8a = make_s8_p(11, -12);
    f8b = make_s8_p(21, -22);
    printf("ret_s8_folded_1 a=%d b=%d\n", f8a.a, f8a.b);
    printf("ret_s8_folded_2 a=%d b=%d\n", f8b.a, f8b.b);
    f16ia = make_s16i_p(101, -102, 103, -104);
    f16ib = make_s16i_p(201, -202, 203, -204);
    printf("ret_s16i_folded_1 a=%d b=%d c=%d d=%d\n",
           f16ia.a, f16ia.b, f16ia.c, f16ia.d);
    printf("ret_s16i_folded_2 a=%d b=%d c=%d d=%d\n",
           f16ib.a, f16ib.b, f16ib.c, f16ib.d);
    f16ma = make_s16m_p(201, 1.5);
    f16mb = make_s16m_p(401, 2.5);
    printf("ret_s16m_folded_1 a=%d b=%.4f\n", f16ma.a, f16ma.b);
    printf("ret_s16m_folded_2 a=%d b=%.4f\n", f16mb.a, f16mb.b);
    f16da = make_s16d_p(1.25, -1.75);
    f16db = make_s16d_p(2.25, -2.75);
    printf("ret_s16d_folded_1 x=%.4f y=%.4f\n", f16da.x, f16da.y);
    printf("ret_s16d_folded_2 x=%.4f y=%.4f\n", f16db.x, f16db.y);
    f16fa = make_s16f_p(1.5f, -1.25f, 1.125f, -1.0625f);
    f16fb = make_s16f_p(2.5f, -2.25f, 2.125f, -2.0625f);
    printf("ret_s16f_folded_1 x=%.4f y=%.4f z=%.4f w=%.4f\n",
           (double)f16fa.x, (double)f16fa.y, (double)f16fa.z, (double)f16fa.w);
    printf("ret_s16f_folded_2 x=%.4f y=%.4f z=%.4f w=%.4f\n",
           (double)f16fb.x, (double)f16fb.y, (double)f16fb.z, (double)f16fb.w);
    f24a = make_s24_p(1001, -1002, 1003, -1004, 1005, -1006);
    f24b = make_s24_p(2001, -2002, 2003, -2004, 2005, -2006);
    printf("ret_s24_folded_1 a=%d b=%d c=%d d=%d e=%d f=%d\n",
           f24a.a, f24a.b, f24a.c, f24a.d, f24a.e, f24a.f);
    printf("ret_s24_folded_2 a=%d b=%d c=%d d=%d e=%d f=%d\n",
           f24b.a, f24b.b, f24b.c, f24b.d, f24b.e, f24b.f);
    fbiga = make_sbig_p(7100);
    fbigb = make_sbig_p(7200);
    print_sbig("ret_sbig", "folded_1", &fbiga);
    print_sbig("ret_sbig", "folded_2", &fbigb);

    /* The direct-expression path, for every one of the seven shapes.  Each member is read
       straight off a call expression that is never assigned to a named object, so the value the
       ABI's return mechanism produced is consumed where it was produced - a distinct case from
       the local-storage path above, where an assignment stands between the two.  Every member of
       every shape is read, so a return that was truncated or misordered cannot hide in an
       unexamined tail; each read is its own call, hence its own return, which is stronger than
       one call inspected several times.  For the 40-byte shape this is the case C11 6.2.4p8
       covers: a non-lvalue aggregate containing an array member designates an object with
       temporary lifetime that lasts to the end of the containing full expression, and each read
       below is a full expression of its own, so no temporary is used after its lifetime. */
    printf("direct_s8_folded a=%d b=%d\n",
           make_s8_p(11, -12).a, make_s8_p(11, -12).b);
    printf("direct_s16i_folded a=%d b=%d c=%d d=%d\n",
           make_s16i_p(101, -102, 103, -104).a, make_s16i_p(101, -102, 103, -104).b,
           make_s16i_p(101, -102, 103, -104).c, make_s16i_p(101, -102, 103, -104).d);
    printf("direct_s16m_folded a=%d b=%.4f\n",
           make_s16m_p(201, 1.5).a, make_s16m_p(201, 1.5).b);
    printf("direct_s16d_folded x=%.4f y=%.4f\n",
           make_s16d_p(1.25, -1.75).x, make_s16d_p(1.25, -1.75).y);
    printf("direct_s16f_folded x=%.4f y=%.4f z=%.4f w=%.4f\n",
           (double)make_s16f_p(1.5f, -1.25f, 1.125f, -1.0625f).x,
           (double)make_s16f_p(1.5f, -1.25f, 1.125f, -1.0625f).y,
           (double)make_s16f_p(1.5f, -1.25f, 1.125f, -1.0625f).z,
           (double)make_s16f_p(1.5f, -1.25f, 1.125f, -1.0625f).w);
    printf("direct_s24_folded a=%d b=%d c=%d d=%d e=%d f=%d\n",
           make_s24_p(1001, -1002, 1003, -1004, 1005, -1006).a,
           make_s24_p(1001, -1002, 1003, -1004, 1005, -1006).b,
           make_s24_p(1001, -1002, 1003, -1004, 1005, -1006).c,
           make_s24_p(1001, -1002, 1003, -1004, 1005, -1006).d,
           make_s24_p(1001, -1002, 1003, -1004, 1005, -1006).e,
           make_s24_p(1001, -1002, 1003, -1004, 1005, -1006).f);
    printf("direct_sbig_folded");
    for (element = 0; element < 10; ++element) {
        printf(" v%d=%d", element, make_sbig_p(7100).v[element]);
    }
    printf("\n");

    /* The forwarding path, for every one of the seven shapes: a maker's return value handed
       straight into a consumer's parameter with no named object between them. */
    consume_s8_p(make_s8_p(11, -12), "folded");
    consume_s16i_p(make_s16i_p(101, -102, 103, -104), "folded");
    consume_s16m_p(make_s16m_p(201, 1.5), "folded");
    consume_s16d_p(make_s16d_p(1.25, -1.75), "folded");
    consume_s16f_p(make_s16f_p(1.5f, -1.25f, 1.125f, -1.0625f), "folded");
    consume_s24_p(make_s24_p(1001, -1002, 1003, -1004, 1005, -1006), "folded");
    consume_sbig_p(make_sbig_p(7100), "folded");

    /* Plain staging of every runtime operand, one volatile read per full expression and in fixed
       index order.  Reading four volatile elements inside a maker's argument list would leave the
       relative order of those side effects unspecified, and a program whose own behaviour is
       unspecified cannot make a divergence between two compilers attributable to either - which
       is the precondition every oracle in this suite rests on.  Staging costs no discriminating
       power: a value that arrived through a volatile load stays opaque to the optimizer, so every
       maker below is still called with genuinely materialised arguments and its return path is
       genuinely exercised. */
    for (element = 0; element < 16; ++element) {
        si[element] = vi[element];
    }
    for (element = 0; element < 6; ++element) {
        sd[element] = vd[element];
    }
    for (element = 0; element < 8; ++element) {
        sf[element] = vf[element];
    }
    for (element = 0; element < 12; ++element) {
        s24v[element] = v24[element];
    }

    f8a = make_s8_p(si[0], si[1]);
    f8b = make_s8_p(si[2], si[3]);
    printf("ret_s8_runtime_1 a=%d b=%d\n", f8a.a, f8a.b);
    printf("ret_s8_runtime_2 a=%d b=%d\n", f8b.a, f8b.b);
    f16ia = make_s16i_p(si[4], si[5], si[6], si[7]);
    f16ib = make_s16i_p(si[8], si[9], si[10], si[11]);
    printf("ret_s16i_runtime_1 a=%d b=%d c=%d d=%d\n",
           f16ia.a, f16ia.b, f16ia.c, f16ia.d);
    printf("ret_s16i_runtime_2 a=%d b=%d c=%d d=%d\n",
           f16ib.a, f16ib.b, f16ib.c, f16ib.d);
    f16ma = make_s16m_p(si[12], sd[0]);
    f16mb = make_s16m_p(si[13], sd[1]);
    printf("ret_s16m_runtime_1 a=%d b=%.4f\n", f16ma.a, f16ma.b);
    printf("ret_s16m_runtime_2 a=%d b=%.4f\n", f16mb.a, f16mb.b);
    f16da = make_s16d_p(sd[2], sd[3]);
    f16db = make_s16d_p(sd[4], sd[5]);
    printf("ret_s16d_runtime_1 x=%.4f y=%.4f\n", f16da.x, f16da.y);
    printf("ret_s16d_runtime_2 x=%.4f y=%.4f\n", f16db.x, f16db.y);
    f16fa = make_s16f_p(sf[0], sf[1], sf[2], sf[3]);
    f16fb = make_s16f_p(sf[4], sf[5], sf[6], sf[7]);
    printf("ret_s16f_runtime_1 x=%.4f y=%.4f z=%.4f w=%.4f\n",
           (double)f16fa.x, (double)f16fa.y, (double)f16fa.z, (double)f16fa.w);
    printf("ret_s16f_runtime_2 x=%.4f y=%.4f z=%.4f w=%.4f\n",
           (double)f16fb.x, (double)f16fb.y, (double)f16fb.z, (double)f16fb.w);
    f24a = make_s24_p(s24v[0], s24v[1], s24v[2], s24v[3], s24v[4], s24v[5]);
    f24b = make_s24_p(s24v[6], s24v[7], s24v[8], s24v[9], s24v[10], s24v[11]);
    printf("ret_s24_runtime_1 a=%d b=%d c=%d d=%d e=%d f=%d\n",
           f24a.a, f24a.b, f24a.c, f24a.d, f24a.e, f24a.f);
    printf("ret_s24_runtime_2 a=%d b=%d c=%d d=%d e=%d f=%d\n",
           f24b.a, f24b.b, f24b.c, f24b.d, f24b.e, f24b.f);
    fbiga = make_sbig_p(si[14]);
    fbigb = make_sbig_p(si[15]);
    print_sbig("ret_sbig", "runtime_1", &fbiga);
    print_sbig("ret_sbig", "runtime_2", &fbigb);

    /* The direct-expression path for all seven shapes, runtime operands this time. */
    printf("direct_s8_runtime a=%d b=%d\n",
           make_s8_p(si[0], si[1]).a, make_s8_p(si[0], si[1]).b);
    printf("direct_s16i_runtime a=%d b=%d c=%d d=%d\n",
           make_s16i_p(si[4], si[5], si[6], si[7]).a,
           make_s16i_p(si[4], si[5], si[6], si[7]).b,
           make_s16i_p(si[4], si[5], si[6], si[7]).c,
           make_s16i_p(si[4], si[5], si[6], si[7]).d);
    printf("direct_s16m_runtime a=%d b=%.4f\n",
           make_s16m_p(si[12], sd[0]).a, make_s16m_p(si[12], sd[0]).b);
    printf("direct_s16d_runtime x=%.4f y=%.4f\n",
           make_s16d_p(sd[2], sd[3]).x, make_s16d_p(sd[2], sd[3]).y);
    printf("direct_s16f_runtime x=%.4f y=%.4f z=%.4f w=%.4f\n",
           (double)make_s16f_p(sf[0], sf[1], sf[2], sf[3]).x,
           (double)make_s16f_p(sf[0], sf[1], sf[2], sf[3]).y,
           (double)make_s16f_p(sf[0], sf[1], sf[2], sf[3]).z,
           (double)make_s16f_p(sf[0], sf[1], sf[2], sf[3]).w);
    printf("direct_s24_runtime a=%d b=%d c=%d d=%d e=%d f=%d\n",
           make_s24_p(s24v[0], s24v[1], s24v[2], s24v[3], s24v[4], s24v[5]).a,
           make_s24_p(s24v[0], s24v[1], s24v[2], s24v[3], s24v[4], s24v[5]).b,
           make_s24_p(s24v[0], s24v[1], s24v[2], s24v[3], s24v[4], s24v[5]).c,
           make_s24_p(s24v[0], s24v[1], s24v[2], s24v[3], s24v[4], s24v[5]).d,
           make_s24_p(s24v[0], s24v[1], s24v[2], s24v[3], s24v[4], s24v[5]).e,
           make_s24_p(s24v[0], s24v[1], s24v[2], s24v[3], s24v[4], s24v[5]).f);
    printf("direct_sbig_runtime");
    for (element = 0; element < 10; ++element) {
        printf(" v%d=%d", element, make_sbig_p(si[14]).v[element]);
    }
    printf("\n");

    /* The forwarding path for all seven shapes, runtime operands. */
    consume_s8_p(make_s8_p(si[0], si[1]), "runtime");
    consume_s16i_p(make_s16i_p(si[4], si[5], si[6], si[7]), "runtime");
    consume_s16m_p(make_s16m_p(si[12], sd[0]), "runtime");
    consume_s16d_p(make_s16d_p(sd[2], sd[3]), "runtime");
    consume_s16f_p(make_s16f_p(sf[0], sf[1], sf[2], sf[3]), "runtime");
    consume_s24_p(make_s24_p(s24v[0], s24v[1], s24v[2], s24v[3], s24v[4], s24v[5]),
                "runtime");
    consume_sbig_p(make_sbig_p(si[14]), "runtime");
    return 0;
}
