/* Every union access reads back the member that was last written.  No union here is
 * written through one member and read through another, and no padding byte or
 * padding bit is ever observed. */

int printf(const char *, ...);

/* The three views are never mixed: each one is written and read on its own, so
 * every read observes the member that was last stored. */
union views {
    struct { unsigned int a : 3; unsigned int b : 5;
             unsigned int c : 9; unsigned int d : 15; } packed;
    struct { unsigned int lo : 16; unsigned int hi : 16; } halves;
    unsigned int whole;
};

union onebit {
    unsigned int u5 : 5;
    signed   int s5 : 5;
};

struct inner { unsigned int p : 6; signed int q : 10; };

struct outer {
    struct inner head;
    unsigned int r : 4;
    struct inner tail[2];
    signed int s : 12;
};

struct tagged {
    unsigned int tag : 4;
    union {
        unsigned int flat : 7;
        struct { unsigned int m : 3; unsigned int n : 4; } pair;
    } body;
    unsigned int trailer : 5;
};

static union views  vw;
static union onebit ob;
static struct outer ot = { { 40u, -300 }, 9u, { { 1u, 2 }, { 63u, -512 } }, -2000 };
static struct tagged tg;

static volatile union views  vvw;
static volatile struct outer vot;
static volatile struct tagged vtg;

int main(void)
{
    printf("size_views=%u align_views=%u\n",
           (unsigned)sizeof(union views), (unsigned)_Alignof(union views));
    printf("size_onebit=%u align_onebit=%u\n",
           (unsigned)sizeof(union onebit), (unsigned)_Alignof(union onebit));
    printf("size_inner=%u align_inner=%u\n",
           (unsigned)sizeof(struct inner), (unsigned)_Alignof(struct inner));
    printf("size_outer=%u align_outer=%u\n",
           (unsigned)sizeof(struct outer), (unsigned)_Alignof(struct outer));
    printf("size_tagged=%u align_tagged=%u\n",
           (unsigned)sizeof(struct tagged), (unsigned)_Alignof(struct tagged));

    vw.packed.a = 5u; vw.packed.b = 21u;
    vw.packed.c = 300u; vw.packed.d = 20000u;
    printf("views_packed a=%u b=%u c=%u d=%u\n",
           (unsigned)vw.packed.a, (unsigned)vw.packed.b,
           (unsigned)vw.packed.c, (unsigned)vw.packed.d);
    vw.packed.c = 511u;
    printf("views_packed_after_c a=%u b=%u c=%u d=%u\n",
           (unsigned)vw.packed.a, (unsigned)vw.packed.b,
           (unsigned)vw.packed.c, (unsigned)vw.packed.d);

    vw.halves.lo = 4660u; vw.halves.hi = 22136u;
    printf("views_halves lo=%u hi=%u\n",
           (unsigned)vw.halves.lo, (unsigned)vw.halves.hi);
    vw.halves.lo = 65535u;
    printf("views_halves_after_lo lo=%u hi=%u\n",
           (unsigned)vw.halves.lo, (unsigned)vw.halves.hi);

    vw.whole = 3735928559u;
    printf("views_whole=%u\n", vw.whole);

    ob.u5 = 31u;
    printf("onebit_u5=%u\n", (unsigned)ob.u5);
    ob.u5 = 17u;
    printf("onebit_u5_again=%u\n", (unsigned)ob.u5);
    ob.s5 = -16;
    printf("onebit_s5=%d\n", (int)ob.s5);
    ob.s5 = 15;
    printf("onebit_s5_again=%d\n", (int)ob.s5);

    printf("outer_head p=%u q=%d\n", (unsigned)ot.head.p, (int)ot.head.q);
    printf("outer_r=%u\n", (unsigned)ot.r);
    printf("outer_tail0 p=%u q=%d\n",
           (unsigned)ot.tail[0].p, (int)ot.tail[0].q);
    printf("outer_tail1 p=%u q=%d\n",
           (unsigned)ot.tail[1].p, (int)ot.tail[1].q);
    printf("outer_s=%d\n", (int)ot.s);

    ot.head.p = 63u;
    ot.tail[0].q = -512;
    ot.tail[1].p = 0u;
    ot.s = 2047;
    printf("outer_modified head_p=%u head_q=%d r=%u t0_p=%u t0_q=%d "
           "t1_p=%u t1_q=%d s=%d\n",
           (unsigned)ot.head.p, (int)ot.head.q, (unsigned)ot.r,
           (unsigned)ot.tail[0].p, (int)ot.tail[0].q,
           (unsigned)ot.tail[1].p, (int)ot.tail[1].q, (int)ot.s);

    tg.tag = 12u;
    tg.trailer = 21u;
    tg.body.flat = 100u;
    printf("tagged_flat tag=%u flat=%u trailer=%u\n",
           (unsigned)tg.tag, (unsigned)tg.body.flat, (unsigned)tg.trailer);
    tg.body.flat = 127u;
    printf("tagged_flat_max tag=%u flat=%u trailer=%u\n",
           (unsigned)tg.tag, (unsigned)tg.body.flat, (unsigned)tg.trailer);

    tg.body.pair.m = 5u;
    tg.body.pair.n = 9u;
    printf("tagged_pair tag=%u m=%u n=%u trailer=%u\n",
           (unsigned)tg.tag, (unsigned)tg.body.pair.m,
           (unsigned)tg.body.pair.n, (unsigned)tg.trailer);
    tg.body.pair.m = 7u;
    printf("tagged_pair_after_m tag=%u m=%u n=%u trailer=%u\n",
           (unsigned)tg.tag, (unsigned)tg.body.pair.m,
           (unsigned)tg.body.pair.n, (unsigned)tg.trailer);

    vvw.packed.a = 5u; vvw.packed.b = 21u;
    vvw.packed.c = 300u; vvw.packed.d = 20000u;
    printf("volatile_views_packed a=%u b=%u c=%u d=%u\n",
           (unsigned)vvw.packed.a, (unsigned)vvw.packed.b,
           (unsigned)vvw.packed.c, (unsigned)vvw.packed.d);
    vvw.halves.lo = 4660u; vvw.halves.hi = 22136u;
    printf("volatile_views_halves lo=%u hi=%u\n",
           (unsigned)vvw.halves.lo, (unsigned)vvw.halves.hi);

    vot.head.p = 40u; vot.head.q = -300; vot.r = 9u;
    vot.tail[0].p = 1u; vot.tail[0].q = 2;
    vot.tail[1].p = 63u; vot.tail[1].q = -512;
    vot.s = -2000;
    printf("volatile_outer head_p=%u head_q=%d r=%u t0_p=%u t0_q=%d "
           "t1_p=%u t1_q=%d s=%d\n",
           (unsigned)vot.head.p, (int)vot.head.q, (unsigned)vot.r,
           (unsigned)vot.tail[0].p, (int)vot.tail[0].q,
           (unsigned)vot.tail[1].p, (int)vot.tail[1].q, (int)vot.s);

    vtg.tag = 12u; vtg.trailer = 21u; vtg.body.flat = 100u;
    printf("volatile_tagged_flat tag=%u flat=%u trailer=%u\n",
           (unsigned)vtg.tag, (unsigned)vtg.body.flat, (unsigned)vtg.trailer);
    vtg.body.pair.m = 5u; vtg.body.pair.n = 9u;
    printf("volatile_tagged_pair tag=%u m=%u n=%u trailer=%u\n",
           (unsigned)vtg.tag, (unsigned)vtg.body.pair.m,
           (unsigned)vtg.body.pair.n, (unsigned)vtg.trailer);

    return 0;
}
