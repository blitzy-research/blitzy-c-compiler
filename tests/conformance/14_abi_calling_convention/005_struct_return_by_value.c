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
 * Three consumption paths are exercised for each variant: storing the returned
 * aggregate in a local and reading its members, reading a member directly off
 * the call expression without storing it, and forwarding the returned aggregate
 * straight into another function as an argument.
 *
 * Padding discipline: sizeof is never printed and no aggregate is ever memcmp'd.
 * Only named members are read back.
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
static void consume_s16i(struct s16i s, const char *t);
static void consume_s16m(struct s16m s, const char *t);

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

static void consume_s16i(struct s16i s, const char *t)
{
    printf("fwd_s16i_%s a=%d b=%d c=%d d=%d\n", t, s.a, s.b, s.c, s.d);
}

static void consume_s16m(struct s16m s, const char *t)
{
    printf("fwd_s16m_%s a=%d b=%.4f\n", t, s.a, s.b);
}

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

    f8a = make_s8(11, -12);
    f8b = make_s8(21, -22);
    printf("ret_s8_folded_1 a=%d b=%d\n", f8a.a, f8a.b);
    printf("ret_s8_folded_2 a=%d b=%d\n", f8b.a, f8b.b);
    f16ia = make_s16i(101, -102, 103, -104);
    f16ib = make_s16i(201, -202, 203, -204);
    printf("ret_s16i_folded_1 a=%d b=%d c=%d d=%d\n",
           f16ia.a, f16ia.b, f16ia.c, f16ia.d);
    printf("ret_s16i_folded_2 a=%d b=%d c=%d d=%d\n",
           f16ib.a, f16ib.b, f16ib.c, f16ib.d);
    f16ma = make_s16m(201, 1.5);
    f16mb = make_s16m(401, 2.5);
    printf("ret_s16m_folded_1 a=%d b=%.4f\n", f16ma.a, f16ma.b);
    printf("ret_s16m_folded_2 a=%d b=%.4f\n", f16mb.a, f16mb.b);
    f16da = make_s16d(1.25, -1.75);
    f16db = make_s16d(2.25, -2.75);
    printf("ret_s16d_folded_1 x=%.4f y=%.4f\n", f16da.x, f16da.y);
    printf("ret_s16d_folded_2 x=%.4f y=%.4f\n", f16db.x, f16db.y);
    f16fa = make_s16f(1.5f, -1.25f, 1.125f, -1.0625f);
    f16fb = make_s16f(2.5f, -2.25f, 2.125f, -2.0625f);
    printf("ret_s16f_folded_1 x=%.4f y=%.4f z=%.4f w=%.4f\n",
           (double)f16fa.x, (double)f16fa.y, (double)f16fa.z, (double)f16fa.w);
    printf("ret_s16f_folded_2 x=%.4f y=%.4f z=%.4f w=%.4f\n",
           (double)f16fb.x, (double)f16fb.y, (double)f16fb.z, (double)f16fb.w);
    f24a = make_s24(1001, -1002, 1003, -1004, 1005, -1006);
    f24b = make_s24(2001, -2002, 2003, -2004, 2005, -2006);
    printf("ret_s24_folded_1 a=%d b=%d c=%d d=%d e=%d f=%d\n",
           f24a.a, f24a.b, f24a.c, f24a.d, f24a.e, f24a.f);
    printf("ret_s24_folded_2 a=%d b=%d c=%d d=%d e=%d f=%d\n",
           f24b.a, f24b.b, f24b.c, f24b.d, f24b.e, f24b.f);
    fbiga = make_sbig(7100);
    fbigb = make_sbig(7200);
    printf("ret_sbig_folded_1 v0=%d v4=%d v9=%d\n",
           fbiga.v[0], fbiga.v[4], fbiga.v[9]);
    printf("ret_sbig_folded_2 v0=%d v4=%d v9=%d\n",
           fbigb.v[0], fbigb.v[4], fbigb.v[9]);
    printf("direct_s8_folded a=%d\n", make_s8(11, -12).a);
    printf("direct_s16m_folded b=%.4f\n", make_s16m(201, 1.5).b);
    consume_s16i(make_s16i(101, -102, 103, -104), "folded");
    consume_s16m(make_s16m(201, 1.5), "folded");

    f8a = make_s8(vi[0], vi[1]);
    f8b = make_s8(vi[2], vi[3]);
    printf("ret_s8_runtime_1 a=%d b=%d\n", f8a.a, f8a.b);
    printf("ret_s8_runtime_2 a=%d b=%d\n", f8b.a, f8b.b);
    f16ia = make_s16i(vi[4], vi[5], vi[6], vi[7]);
    f16ib = make_s16i(vi[8], vi[9], vi[10], vi[11]);
    printf("ret_s16i_runtime_1 a=%d b=%d c=%d d=%d\n",
           f16ia.a, f16ia.b, f16ia.c, f16ia.d);
    printf("ret_s16i_runtime_2 a=%d b=%d c=%d d=%d\n",
           f16ib.a, f16ib.b, f16ib.c, f16ib.d);
    f16ma = make_s16m(vi[12], vd[0]);
    f16mb = make_s16m(vi[13], vd[1]);
    printf("ret_s16m_runtime_1 a=%d b=%.4f\n", f16ma.a, f16ma.b);
    printf("ret_s16m_runtime_2 a=%d b=%.4f\n", f16mb.a, f16mb.b);
    f16da = make_s16d(vd[2], vd[3]);
    f16db = make_s16d(vd[4], vd[5]);
    printf("ret_s16d_runtime_1 x=%.4f y=%.4f\n", f16da.x, f16da.y);
    printf("ret_s16d_runtime_2 x=%.4f y=%.4f\n", f16db.x, f16db.y);
    f16fa = make_s16f(vf[0], vf[1], vf[2], vf[3]);
    f16fb = make_s16f(vf[4], vf[5], vf[6], vf[7]);
    printf("ret_s16f_runtime_1 x=%.4f y=%.4f z=%.4f w=%.4f\n",
           (double)f16fa.x, (double)f16fa.y, (double)f16fa.z, (double)f16fa.w);
    printf("ret_s16f_runtime_2 x=%.4f y=%.4f z=%.4f w=%.4f\n",
           (double)f16fb.x, (double)f16fb.y, (double)f16fb.z, (double)f16fb.w);
    f24a = make_s24(v24[0], v24[1], v24[2], v24[3], v24[4], v24[5]);
    f24b = make_s24(v24[6], v24[7], v24[8], v24[9], v24[10], v24[11]);
    printf("ret_s24_runtime_1 a=%d b=%d c=%d d=%d e=%d f=%d\n",
           f24a.a, f24a.b, f24a.c, f24a.d, f24a.e, f24a.f);
    printf("ret_s24_runtime_2 a=%d b=%d c=%d d=%d e=%d f=%d\n",
           f24b.a, f24b.b, f24b.c, f24b.d, f24b.e, f24b.f);
    fbiga = make_sbig(vi[14]);
    fbigb = make_sbig(vi[15]);
    printf("ret_sbig_runtime_1 v0=%d v4=%d v9=%d\n",
           fbiga.v[0], fbiga.v[4], fbiga.v[9]);
    printf("ret_sbig_runtime_2 v0=%d v4=%d v9=%d\n",
           fbigb.v[0], fbigb.v[4], fbigb.v[9]);
    printf("direct_s8_runtime a=%d\n", make_s8(vi[0], vi[1]).a);
    printf("direct_s16m_runtime b=%.4f\n", make_s16m(vi[12], vd[0]).b);
    consume_s16i(make_s16i(vi[4], vi[5], vi[6], vi[7]), "runtime");
    consume_s16m(make_s16m(vi[12], vd[0]), "runtime");
    return 0;
}
