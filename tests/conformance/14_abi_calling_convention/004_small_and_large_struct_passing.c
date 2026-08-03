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
 * WHICH REGISTER FILE EACH SHAPE ACTUALLY EXHAUSTS.  Stated per function rather
 * than as one blanket claim, because the shapes do not behave alike and a single
 * sentence about "exhausting the register files" would be false for two of them.
 * Slot counts below are for the aggregates only; the trailing int tag is counted
 * separately and named where it matters.  System V i386 cdecl marshals every
 * argument of every one of these calls on the stack, so it is not repeated per
 * row.
 *
 *   take_s8x10   10 x s8.  One integer slot each on all three 64-bit ABIs, so 10
 *                slots plus the tag = 11.  EXHAUSTS the integer file on all
 *                three (6 on AMD64, 8 on AAPCS64, 8 on LP64D) and spills.  No
 *                floating slot is used.
 *   take_s16ix5   5 x s16i.  Two integer slots each = 10, plus the tag = 11.
 *                EXHAUSTS the integer file on all three and spills.  No
 *                floating slot is used.
 *   take_s16mx5   5 x s16m.  The interesting one, and the reason a blanket claim
 *                would be wrong.  AMD64 classifies each as INTEGER + SSE, so 5
 *                integer slots plus the tag fills EXACTLY the six integer
 *                registers while using only 5 of 8 SSE registers -- the floating
 *                file is NOT exhausted.  AAPCS64 sees a non-homogeneous 16-byte
 *                aggregate and uses two integer slots each = 10 plus the tag, so
 *                the integer file IS exhausted and no floating-point ARGUMENT
 *                register is used at all.  LP64D splits each into one integer
 *                plus one floating slot, giving 6 of 8 integer registers with the
 *                tag and 5 of 8 floating, so on that target NEITHER file is
 *                exhausted -- what this call exercises there is the independent
 *                advance of the two allocators, which is the genuinely valuable
 *                property and is why the shape is kept.  All three readings were
 *                measured in the reference compiler's assembly at -O0.
 *   take_s16dx5   5 x s16d.  Two floating slots each = 10 on all three, so the
 *                FLOATING file is exhausted on every one of them.  What happens
 *                to the overflow differs, and that difference is part of the
 *                point: AMD64 and AAPCS64 place the fifth aggregate on the stack
 *                and use one integer register for the tag, whereas LP64D spills
 *                the two remaining doubles into INTEGER argument registers before
 *                the tag, so on that target the integer file carries floating
 *                data as well.  Measured in the reference compiler's assembly at
 *                -O0 for each target.
 *   take_s16fx5   5 x s16f.  AMD64 packs each into two SSE registers = 10, so
 *                the floating file is exhausted.  AAPCS64 treats each as a
 *                four-float HFA = 20 floating slots, exhausted far past the
 *                threshold.  LP64D declines the floating path for a 16-byte
 *                four-member aggregate and uses two integer slots each = 10 plus
 *                the tag, so on that target it is the INTEGER file that is
 *                exhausted.  The same source shape therefore exhausts different
 *                files on different targets, which is exactly the disagreement
 *                worth testing.
 *   take_s24x4    4 x s24, 24 bytes.  Memory on every target -- AMD64 by the
 *                MEMORY class, AAPCS64 and LP64D by an indirect reference -- so
 *                no file is exhausted.  What this row tests is the by-memory
 *                path itself, on the far side of every threshold.
 *   take_sbigx2   2 x sbig, 40 bytes.  Unambiguously memory everywhere, for the
 *                same reason and with the same purpose.
 *
 * THE CALL BARRIER, AND WHY THESE BOUNDARIES WOULD OTHERWISE NOT EXIST.  An
 * aggregate-passing boundary is only under test if the call actually happens.
 * Measured with gcc 13.4.0 at -O2, with direct calls to these static functions
 * the only one left standing was take_s8x10: the other six were inlined away and
 * six of the seven documented boundaries above were not crossed at all at that
 * level.  Every call below therefore goes through a FILE-SCOPE volatile FUNCTION
 * POINTER.  A volatile lvalue must be re-read on every access, so no conforming
 * compiler may assume which function the pointer designates -- it can neither
 * inline nor clone the callee, and it must marshal each aggregate exactly as the
 * ABI prescribes because it cannot know what will receive it.  This is plain
 * standard C rather than a compiler attribute, so both sides of oracle (a)
 * honour it for the same reason.  Verified in the generated assembly of all four
 * targets at -O2: all seven callees are emitted unmodified, with no .constprop
 * and no .isra clone.
 *
 * EVALUATION-ORDER DISCIPLINE.  An access to a volatile object is an observable
 * side effect and argument evaluation order is unspecified, so every volatile
 * aggregate and every volatile tag is copied into an ordinary local in a
 * statement of its own, and only those locals are passed -- at most one
 * side-effecting argument per call, which here means the function-pointer load
 * and nothing else.  The copies do not weaken the runtime variant: each is
 * itself a read of volatile storage the optimizer may not fold, so every
 * aggregate the callee receives was genuinely materialised at run time.
 *
 * Member coverage: EVERY named member of EVERY aggregate is read back on both
 * sides of both variants - all ten elements of the 40-byte shape included.  A
 * member the program never reads is a member the backend's aggregate copy is
 * never checked on, and for the two memory-passed shapes that copy is the whole
 * mechanism under test.
 *
 * Volatile staging discipline: copying a volatile aggregate accesses the whole
 * object, so each such copy is made in a full expression of its own, into plain
 * staging storage, before any runtime call; the calls pass only plain objects.
 * Several volatile accesses inside one argument list would leave the relative
 * order of those side effects unspecified, and an unspecified-behaviour program
 * cannot make a divergence attributable to either compiler.  No volatile lvalue
 * appears in any argument list in this file.
 *
 * WHAT THE TABLE ABOVE DESCRIBES, AND AT WHICH OPTIMIZATION LEVEL.  Every row is
 * a classification the SOURCE requests: seven aggregate shapes, each passed to a
 * separate static function, on both sides of every target's by-register /
 * by-memory threshold.  Whether a call survives to exercise that classification
 * at run time is a separate question, and it was measured rather than assumed.
 * Counting the program's own static helpers still emitted in the reference
 * compiler's assembly - `<driver> -O<n> -S -o -
 * 004_small_and_large_struct_passing.c`, then grepping for the helper labels:
 * with the volatile call boundary in place, ALL SEVEN HELPERS ARE EMITTED at -O0,
 * -O1 and -O2 on x86-64, i686, AArch64 and RISC-V 64 alike, with no .constprop
 * and no .isra clone anywhere.  Every argument-marshalling path in the table is
 * therefore exercised as a real indirect call at every one of the twelve cells.
 *
 * The measured BEFORE-STATE, recorded because it is the reason the boundary is
 * there rather than as a description of the program as it now stands.  With
 * DIRECT calls to these static helpers, the same counting - over all eight of the
 * program's static functions then, the seven callees plus the element printer -
 * gave:
 *
 *   -O0  all 8 functions, 21 calls  on x86-64, i686, AArch64 and RISC-V 64 alike
 *   -O1  5 functions / 10 calls on x86-64 and AArch64, 1 / 2 on RISC-V 64,
 *        and 0 / 0 on i686 - every one inlined away
 *   -O2  1 function / 2 calls on x86-64, AArch64 and RISC-V 64, 0 / 0 on i686
 *
 * so the threshold calls were exercised at -O0 and only partly above it, and on
 * i686 not at all from -O1 upward.  Those three lines describe a program that no
 * longer exists; they are kept because they are the measurement that justified
 * introducing the indirection, and deleting the evidence for a design decision
 * makes the decision unreviewable.  The compiler under test may inline
 * differently again; its own behaviour is not measured on this branch, since no
 * bcc binary is present.
 *
 * Padding discipline: sizeof is never printed and no aggregate is ever
 * memcmp'd.  Only named members are read back, so the fact that struct s16m is
 * 12 bytes on i686 (where double has four-byte alignment) and 16 bytes on the
 * other three targets cannot affect the output.
 *
 * EVERY MEMBER OF EVERY AGGREGATE IS READ BACK, not a sample of them.  The two
 * forty-byte aggregates print all ten of their members, in both variants,
 * exactly as the smaller shapes print all of theirs.  A sampled read-back -
 * first, middle, last - would pass while a by-memory copy corrupted,
 * transposed or dropped any of the other seven members, and a by-memory
 * aggregate is precisely the shape where a wrong copy length or a wrong
 * hidden-pointer offset shows up.  The cost of reading every member is a few
 * more printed fields; the cost of sampling is a defect that passes.
 *
 * THE CALL BOUNDARY IS ENFORCED, NOT HOPED FOR.  Every callee is reached
 * through a volatile-qualified function pointer rather than by name.  An
 * ordinary static callee may legally be inlined at -O1 and -O2, and an inlined
 * callee marshals nothing at all: worse, the aggregates it takes by value
 * become candidates for scalar replacement, which dissolves exactly the
 * by-register versus by-memory classification this program exists to test.
 * Measured with gcc 13.4.0 at -O2 before the indirection was added: of the
 * fourteen intended aggregate-passing boundaries only two survived, the other
 * twelve having been inlined away, and the two survivors survived only by
 * exceeding the inliner's size budget.
 *
 * A volatile pointer must be re-read at the point of call, so the designated
 * function is unknown and the call is genuinely indirect; and because the
 * address escapes into storage, the signature may not be cloned or scalarised
 * either.  The mechanism is pure ISO C: a function attribute would have been
 * shorter, but the documented attribute set for the compiler under test is
 * packed, aligned, section, unused, deprecated, visibility and format
 * (docs/technical-specifications.md line 506), so an inlining attribute would
 * risk a divergence caused by the test and would import an extension into an
 * area whose subject is the calling convention.
 *
 * ONE VOLATILE READ PER FULL STATEMENT.  No argument list contains a volatile
 * lvalue.  Each runtime aggregate is copied out of its volatile object into a
 * plain local of the same type in a statement of its own, and the tag
 * likewise, so the call reads only plain locals.  Copying a volatile-qualified
 * aggregate is an observable access, and the order of side effects within one
 * argument list is unspecified, so a list holding several such copies would
 * have an unspecified order of side effects - and a suite whose premise is
 * that a divergence means a defect needs the program to have exactly one
 * defined behaviour.
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

/* The enforced call boundary, one volatile-qualified pointer per aggregate
   shape.
   Each is re-read at its call site, so every call is indirect at every
   optimization level, no callee body is inlined, and no by-value aggregate
   parameter can be scalarised out of existence.  Both variants of every shape
   travel through these pointers, so the folded and the runtime call cross the
   same boundary. */
static void (*volatile take_s8x10_p)(struct s8, struct s8, struct s8, struct s8,
                                     struct s8, struct s8, struct s8, struct s8,
                                     struct s8, struct s8, int) = take_s8x10;
static void (*volatile take_s16ix5_p)(struct s16i, struct s16i, struct s16i,
                                      struct s16i, struct s16i, int) =
    take_s16ix5;
static void (*volatile take_s16mx5_p)(struct s16m, struct s16m, struct s16m,
                                      struct s16m, struct s16m, int) =
    take_s16mx5;
static void (*volatile take_s16dx5_p)(struct s16d, struct s16d, struct s16d,
                                      struct s16d, struct s16d, int) =
    take_s16dx5;
static void (*volatile take_s16fx5_p)(struct s16f, struct s16f, struct s16f,
                                      struct s16f, struct s16f, int) =
    take_s16fx5;
static void (*volatile take_s24x4_p)(struct s24, struct s24, struct s24,
                                     struct s24, int) = take_s24x4;
static void (*volatile take_sbigx2_p)(struct sbig, struct sbig, int) =
    take_sbigx2;

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

/* Every one of the ten members is printed, with its index as its label.
 *
 * A large aggregate is the one shape passed wholly through memory on all four targets, so
 * the copy the callee reads is produced by an explicit block copy the compiler emits.  A
 * defect in that copy - a wrong length, a wrong displacement, a partially overlapping
 * move - corrupts the MIDDLE of the object far more readily than its ends, and an earlier
 * form of this program observed only indices 0, 4 and 9.  Seven of the ten members per
 * value could therefore be arbitrary and every oracle still agreed, in all twelve cells.
 * The values are consecutive by construction, so a reader spots a break in the sequence at
 * a glance, and one line per struct value keeps the output bounded.
 */
static void take_sbigx2(struct sbig p1, struct sbig p2, int variant)
{
    const char *t = variant_tag(variant);
    printf("sbig_%s_p1 v0=%d v1=%d v2=%d v3=%d v4=%d v5=%d v6=%d v7=%d v8=%d v9=%d\n", t,
           p1.v[0], p1.v[1], p1.v[2], p1.v[3], p1.v[4], p1.v[5], p1.v[6], p1.v[7],
           p1.v[8], p1.v[9]);
    printf("sbig_%s_p2 v0=%d v1=%d v2=%d v3=%d v4=%d v5=%d v6=%d v7=%d v8=%d v9=%d\n", t,
           p2.v[0], p2.v[1], p2.v[2], p2.v[3], p2.v[4], p2.v[5], p2.v[6], p2.v[7],
           p2.v[8], p2.v[9]);
}

int main(void)
{
    /* Plain destinations for the runtime variants, one per aggregate the
       runtime calls pass.  Only these are read at the call sites, so no
       argument list holds a volatile access and none depends on an unspecified
       order. */
    struct s8 r8[10];
    struct s16i r16i[5];
    struct s16m r16m[5];
    struct s16d r16d[5];
    struct s16f r16f[5];
    struct s24 r24[4];
    struct sbig rbig[2];
    int plain_tag;
    int k;

    take_s8x10_p(c8[0], c8[1], c8[2], c8[3], c8[4], c8[5], c8[6], c8[7],
                 c8[8], c8[9], 0);
    take_s16ix5_p(c16i[0], c16i[1], c16i[2], c16i[3], c16i[4], 0);
    take_s16mx5_p(c16m[0], c16m[1], c16m[2], c16m[3], c16m[4], 0);
    take_s16dx5_p(c16d[0], c16d[1], c16d[2], c16d[3], c16d[4], 0);
    take_s16fx5_p(c16f[0], c16f[1], c16f[2], c16f[3], c16f[4], 0);
    take_s24x4_p(c24[0], c24[1], c24[2], c24[3], 0);
    take_sbigx2_p(cbig[0], cbig[1], 0);

    /* One volatile aggregate copy per full statement, each separated from the
       next by a sequence point.  The copies are still loads from volatile
       storage, so nothing becomes available for compile-time substitution and
       the runtime calls are still fed genuine runtime aggregates. */
    plain_tag = vtag;
    for (k = 0; k < 10; k++) {
        r8[k] = v8[k];
    }
    for (k = 0; k < 5; k++) {
        r16i[k] = v16i[k];
    }
    for (k = 0; k < 5; k++) {
        r16m[k] = v16m[k];
    }
    for (k = 0; k < 5; k++) {
        r16d[k] = v16d[k];
    }
    for (k = 0; k < 5; k++) {
        r16f[k] = v16f[k];
    }
    for (k = 0; k < 4; k++) {
        r24[k] = v24[k];
    }
    for (k = 0; k < 2; k++) {
        rbig[k] = vbig[k];
    }

    take_s8x10_p(r8[0], r8[1], r8[2], r8[3], r8[4], r8[5], r8[6], r8[7],
                 r8[8], r8[9], plain_tag);
    take_s16ix5_p(r16i[0], r16i[1], r16i[2], r16i[3], r16i[4], plain_tag);
    take_s16mx5_p(r16m[0], r16m[1], r16m[2], r16m[3], r16m[4], plain_tag);
    take_s16dx5_p(r16d[0], r16d[1], r16d[2], r16d[3], r16d[4], plain_tag);
    take_s16fx5_p(r16f[0], r16f[1], r16f[2], r16f[3], r16f[4], plain_tag);
    take_s24x4_p(r24[0], r24[1], r24[2], r24[3], plain_tag);
    take_sbigx2_p(rbig[0], rbig[1], plain_tag);
    return 0;
}
