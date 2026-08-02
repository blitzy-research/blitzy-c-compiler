/* Area 14 - ABI and calling convention.
 * 004_small_and_large_struct_passing: aggregates on BOTH sides of every
 * target's by-register / by-memory threshold, plus the two aggregate shapes
 * where the four conventions disagree most.
 *
 *   struct s8    8 bytes, two ints          - one GPR on every 64-bit target
 *   struct s16i 16 bytes, four ints         - the exact boundary: two GPRs
 *   struct s16m mixed int + double          - System V AMD64 classifies it
 *                                             INTEGER + SSE, AAPCS64 sees a
 *                                             non-homogeneous 16-byte aggregate
 *                                             and uses two GPRs, LP64D splits
 *                                             it a0 + fa0.  Highest-yield
 *                                             aggregate shape in the corpus.
 *   struct s16d two doubles                 - two SSE on AMD64, a homogeneous
 *                                             float aggregate on AAPCS64,
 *                                             fa0 + fa1 on LP64D
 *   struct s16f four floats                 - an AAPCS64 HFA in four float
 *                                             registers, two packed SSE
 *                                             registers on AMD64, two GPRs on
 *                                             LP64D
 *   struct s24  24 bytes, six ints          - memory on every target
 *   struct sbig 40 bytes, ten ints          - unambiguously memory everywhere
 *
 * Instance counts are chosen so the aggregates alone exhaust the eight-register
 * files of AAPCS64 and LP64D and the six integer registers of System V AMD64.
 * System V i386 cdecl marshals everything on the stack.
 *
 * Padding discipline: sizeof is never printed and no aggregate is ever
 * memcmp'd.  Only named members are read back, so the fact that struct s16m is
 * 12 bytes on i686 (where double has four-byte alignment) and 16 bytes on the
 * other three targets cannot affect the output.
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

static const char *variant_tag(int variant);
static void take_s8x10(struct s8 p01, struct s8 p02, struct s8 p03,
                       struct s8 p04, struct s8 p05, struct s8 p06,
                       struct s8 p07, struct s8 p08, struct s8 p09,
                       struct s8 p10, int variant);
static void take_s16ix5(struct s16i p1, struct s16i p2, struct s16i p3,
                        struct s16i p4, struct s16i p5, int variant);
static void take_s16mx5(struct s16m p1, struct s16m p2, struct s16m p3,
                        struct s16m p4, struct s16m p5, int variant);
static void take_s16dx5(struct s16d p1, struct s16d p2, struct s16d p3,
                        struct s16d p4, struct s16d p5, int variant);
static void take_s16fx5(struct s16f p1, struct s16f p2, struct s16f p3,
                        struct s16f p4, struct s16f p5, int variant);
static void take_s24x4(struct s24 p1, struct s24 p2, struct s24 p3,
                       struct s24 p4, int variant);
static void take_sbigx2(struct sbig p1, struct sbig p2, int variant);

static const struct s8 c8[10] = {
    { 11, -12 }, { 21, -22 }, { 31, -32 }, { 41, -42 }, { 51, -52 },
    { 61, -62 }, { 71, -72 }, { 81, -82 }, { 91, -92 }, { 101, -102 }
};
static volatile struct s8 v8[10] = {
    { 11, -12 }, { 21, -22 }, { 31, -32 }, { 41, -42 }, { 51, -52 },
    { 61, -62 }, { 71, -72 }, { 81, -82 }, { 91, -92 }, { 101, -102 }
};

static const struct s16i c16i[5] = {
    { 101, -102, 103, -104 }, { 201, -202, 203, -204 },
    { 301, -302, 303, -304 }, { 401, -402, 403, -404 },
    { 501, -502, 503, -504 }
};
static volatile struct s16i v16i[5] = {
    { 101, -102, 103, -104 }, { 201, -202, 203, -204 },
    { 301, -302, 303, -304 }, { 401, -402, 403, -404 },
    { 501, -502, 503, -504 }
};

static const struct s16m c16m[5] = {
    { 201, 1.5 }, { 401, 2.5 }, { 601, 3.5 }, { 801, 4.5 }, { 1001, 5.5 }
};
static volatile struct s16m v16m[5] = {
    { 201, 1.5 }, { 401, 2.5 }, { 601, 3.5 }, { 801, 4.5 }, { 1001, 5.5 }
};

static const struct s16d c16d[5] = {
    { 1.25, -1.75 }, { 2.25, -2.75 }, { 3.25, -3.75 },
    { 4.25, -4.75 }, { 5.25, -5.75 }
};
static volatile struct s16d v16d[5] = {
    { 1.25, -1.75 }, { 2.25, -2.75 }, { 3.25, -3.75 },
    { 4.25, -4.75 }, { 5.25, -5.75 }
};

static const struct s16f c16f[5] = {
    { 1.5f, -1.25f, 1.125f, -1.0625f }, { 2.5f, -2.25f, 2.125f, -2.0625f },
    { 3.5f, -3.25f, 3.125f, -3.0625f }, { 4.5f, -4.25f, 4.125f, -4.0625f },
    { 5.5f, -5.25f, 5.125f, -5.0625f }
};
static volatile struct s16f v16f[5] = {
    { 1.5f, -1.25f, 1.125f, -1.0625f }, { 2.5f, -2.25f, 2.125f, -2.0625f },
    { 3.5f, -3.25f, 3.125f, -3.0625f }, { 4.5f, -4.25f, 4.125f, -4.0625f },
    { 5.5f, -5.25f, 5.125f, -5.0625f }
};

static const struct s24 c24[4] = {
    { 1001, -1002, 1003, -1004, 1005, -1006 },
    { 2001, -2002, 2003, -2004, 2005, -2006 },
    { 3001, -3002, 3003, -3004, 3005, -3006 },
    { 4001, -4002, 4003, -4004, 4005, -4006 }
};
static volatile struct s24 v24[4] = {
    { 1001, -1002, 1003, -1004, 1005, -1006 },
    { 2001, -2002, 2003, -2004, 2005, -2006 },
    { 3001, -3002, 3003, -3004, 3005, -3006 },
    { 4001, -4002, 4003, -4004, 4005, -4006 }
};

static const struct sbig cbig[2] = {
    { { 9100, 9101, 9102, 9103, 9104, 9105, 9106, 9107, 9108, 9109 } },
    { { 9200, 9201, 9202, 9203, 9204, 9205, 9206, 9207, 9208, 9209 } }
};
static volatile struct sbig vbig[2] = {
    { { 9100, 9101, 9102, 9103, 9104, 9105, 9106, 9107, 9108, 9109 } },
    { { 9200, 9201, 9202, 9203, 9204, 9205, 9206, 9207, 9208, 9209 } }
};

static volatile int vtag = 1;

static const char *variant_tag(int variant)
{
    return (variant == 0) ? "folded" : "runtime";
}

static void take_s8x10(struct s8 p01, struct s8 p02, struct s8 p03,
                       struct s8 p04, struct s8 p05, struct s8 p06,
                       struct s8 p07, struct s8 p08, struct s8 p09,
                       struct s8 p10, int variant)
{
    const char *t = variant_tag(variant);
    printf("s8_%s_p01 a=%d b=%d\n", t, p01.a, p01.b);
    printf("s8_%s_p02 a=%d b=%d\n", t, p02.a, p02.b);
    printf("s8_%s_p03 a=%d b=%d\n", t, p03.a, p03.b);
    printf("s8_%s_p04 a=%d b=%d\n", t, p04.a, p04.b);
    printf("s8_%s_p05 a=%d b=%d\n", t, p05.a, p05.b);
    printf("s8_%s_p06 a=%d b=%d\n", t, p06.a, p06.b);
    printf("s8_%s_p07 a=%d b=%d\n", t, p07.a, p07.b);
    printf("s8_%s_p08 a=%d b=%d\n", t, p08.a, p08.b);
    printf("s8_%s_p09 a=%d b=%d\n", t, p09.a, p09.b);
    printf("s8_%s_p10 a=%d b=%d\n", t, p10.a, p10.b);
}

static void take_s16ix5(struct s16i p1, struct s16i p2, struct s16i p3,
                        struct s16i p4, struct s16i p5, int variant)
{
    const char *t = variant_tag(variant);
    printf("s16i_%s_p1 a=%d b=%d c=%d d=%d\n", t, p1.a, p1.b, p1.c, p1.d);
    printf("s16i_%s_p2 a=%d b=%d c=%d d=%d\n", t, p2.a, p2.b, p2.c, p2.d);
    printf("s16i_%s_p3 a=%d b=%d c=%d d=%d\n", t, p3.a, p3.b, p3.c, p3.d);
    printf("s16i_%s_p4 a=%d b=%d c=%d d=%d\n", t, p4.a, p4.b, p4.c, p4.d);
    printf("s16i_%s_p5 a=%d b=%d c=%d d=%d\n", t, p5.a, p5.b, p5.c, p5.d);
}

static void take_s16mx5(struct s16m p1, struct s16m p2, struct s16m p3,
                        struct s16m p4, struct s16m p5, int variant)
{
    const char *t = variant_tag(variant);
    printf("s16m_%s_p1 a=%d b=%.4f\n", t, p1.a, p1.b);
    printf("s16m_%s_p2 a=%d b=%.4f\n", t, p2.a, p2.b);
    printf("s16m_%s_p3 a=%d b=%.4f\n", t, p3.a, p3.b);
    printf("s16m_%s_p4 a=%d b=%.4f\n", t, p4.a, p4.b);
    printf("s16m_%s_p5 a=%d b=%.4f\n", t, p5.a, p5.b);
}

static void take_s16dx5(struct s16d p1, struct s16d p2, struct s16d p3,
                        struct s16d p4, struct s16d p5, int variant)
{
    const char *t = variant_tag(variant);
    printf("s16d_%s_p1 x=%.4f y=%.4f\n", t, p1.x, p1.y);
    printf("s16d_%s_p2 x=%.4f y=%.4f\n", t, p2.x, p2.y);
    printf("s16d_%s_p3 x=%.4f y=%.4f\n", t, p3.x, p3.y);
    printf("s16d_%s_p4 x=%.4f y=%.4f\n", t, p4.x, p4.y);
    printf("s16d_%s_p5 x=%.4f y=%.4f\n", t, p5.x, p5.y);
}

static void take_s16fx5(struct s16f p1, struct s16f p2, struct s16f p3,
                        struct s16f p4, struct s16f p5, int variant)
{
    const char *t = variant_tag(variant);
    printf("s16f_%s_p1 x=%.4f y=%.4f z=%.4f w=%.4f\n", t,
           (double)p1.x, (double)p1.y, (double)p1.z, (double)p1.w);
    printf("s16f_%s_p2 x=%.4f y=%.4f z=%.4f w=%.4f\n", t,
           (double)p2.x, (double)p2.y, (double)p2.z, (double)p2.w);
    printf("s16f_%s_p3 x=%.4f y=%.4f z=%.4f w=%.4f\n", t,
           (double)p3.x, (double)p3.y, (double)p3.z, (double)p3.w);
    printf("s16f_%s_p4 x=%.4f y=%.4f z=%.4f w=%.4f\n", t,
           (double)p4.x, (double)p4.y, (double)p4.z, (double)p4.w);
    printf("s16f_%s_p5 x=%.4f y=%.4f z=%.4f w=%.4f\n", t,
           (double)p5.x, (double)p5.y, (double)p5.z, (double)p5.w);
}

static void take_s24x4(struct s24 p1, struct s24 p2, struct s24 p3,
                       struct s24 p4, int variant)
{
    const char *t = variant_tag(variant);
    printf("s24_%s_p1 a=%d b=%d c=%d d=%d e=%d f=%d\n", t,
           p1.a, p1.b, p1.c, p1.d, p1.e, p1.f);
    printf("s24_%s_p2 a=%d b=%d c=%d d=%d e=%d f=%d\n", t,
           p2.a, p2.b, p2.c, p2.d, p2.e, p2.f);
    printf("s24_%s_p3 a=%d b=%d c=%d d=%d e=%d f=%d\n", t,
           p3.a, p3.b, p3.c, p3.d, p3.e, p3.f);
    printf("s24_%s_p4 a=%d b=%d c=%d d=%d e=%d f=%d\n", t,
           p4.a, p4.b, p4.c, p4.d, p4.e, p4.f);
}

static void take_sbigx2(struct sbig p1, struct sbig p2, int variant)
{
    const char *t = variant_tag(variant);
    printf("sbig_%s_p1 v0=%d v4=%d v9=%d\n", t, p1.v[0], p1.v[4], p1.v[9]);
    printf("sbig_%s_p2 v0=%d v4=%d v9=%d\n", t, p2.v[0], p2.v[4], p2.v[9]);
}

int main(void)
{
    take_s8x10(c8[0], c8[1], c8[2], c8[3], c8[4], c8[5], c8[6], c8[7],
               c8[8], c8[9], 0);
    take_s16ix5(c16i[0], c16i[1], c16i[2], c16i[3], c16i[4], 0);
    take_s16mx5(c16m[0], c16m[1], c16m[2], c16m[3], c16m[4], 0);
    take_s16dx5(c16d[0], c16d[1], c16d[2], c16d[3], c16d[4], 0);
    take_s16fx5(c16f[0], c16f[1], c16f[2], c16f[3], c16f[4], 0);
    take_s24x4(c24[0], c24[1], c24[2], c24[3], 0);
    take_sbigx2(cbig[0], cbig[1], 0);

    take_s8x10(v8[0], v8[1], v8[2], v8[3], v8[4], v8[5], v8[6], v8[7],
               v8[8], v8[9], vtag);
    take_s16ix5(v16i[0], v16i[1], v16i[2], v16i[3], v16i[4], vtag);
    take_s16mx5(v16m[0], v16m[1], v16m[2], v16m[3], v16m[4], vtag);
    take_s16dx5(v16d[0], v16d[1], v16d[2], v16d[3], v16d[4], vtag);
    take_s16fx5(v16f[0], v16f[1], v16f[2], v16f[3], v16f[4], vtag);
    take_s24x4(v24[0], v24[1], v24[2], v24[3], vtag);
    take_sbigx2(vbig[0], vbig[1], vtag);
    return 0;
}
