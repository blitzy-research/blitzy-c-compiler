/* Every field carries an explicit signed int or unsigned int base type, so its
 * signedness is stated by the program rather than chosen by the implementation as
 * it would be for a field declared on unqualified int. */
int printf(const char *, ...);

struct sfields {
    signed int s2  : 2;
    signed int s3  : 3;
    signed int s5  : 5;
    signed int s8  : 8;
    signed int s16 : 16;
    signed int s31 : 31;
    signed int s32 : 32;
};

struct ufields {
    unsigned int u1  : 1;
    unsigned int u2  : 2;
    unsigned int u5  : 5;
    unsigned int u8  : 8;
    unsigned int u16 : 16;
    unsigned int u31 : 31;
    unsigned int u32 : 32;
};

static struct sfields s_min = { -2, -4, -16, -128, -32768,
                                -1073741823 - 1, -2147483647 - 1 };
static struct sfields s_max = { 1, 3, 15, 127, 32767,
                                1073741823, 2147483647 };
static struct ufields u_max = { 1u, 3u, 31u, 255u, 65535u,
                                2147483647u, 4294967295u };

/* Runtime variant: volatile storage, so every write and read below happens at run
 * time rather than being folded. */
static volatile struct sfields vs;
static volatile struct ufields vu;

/* Opaque seed: reading a volatile object stops the optimizer from folding the
 * derived extremes into compile-time constants. */
static volatile int seed = 1;

int main(void)
{
    int v;

    printf("folded_smin s2=%d s3=%d s5=%d s8=%d s16=%d s31=%d s32=%d\n",
           (int)s_min.s2, (int)s_min.s3, (int)s_min.s5, (int)s_min.s8,
           (int)s_min.s16, (int)s_min.s31, (int)s_min.s32);
    printf("folded_smax s2=%d s3=%d s5=%d s8=%d s16=%d s31=%d s32=%d\n",
           (int)s_max.s2, (int)s_max.s3, (int)s_max.s5, (int)s_max.s8,
           (int)s_max.s16, (int)s_max.s31, (int)s_max.s32);
    printf("folded_umax u1=%u u2=%u u5=%u u8=%u u16=%u u31=%u u32=%u\n",
           (unsigned)u_max.u1, (unsigned)u_max.u2, (unsigned)u_max.u5,
           (unsigned)u_max.u8, (unsigned)u_max.u16, (unsigned)u_max.u31,
           (unsigned)u_max.u32);

    vs.s2 = -2; vs.s3 = -4; vs.s5 = -16; vs.s8 = -128; vs.s16 = -32768;
    vs.s31 = -1073741823 - 1; vs.s32 = -2147483647 - 1;
    printf("runtime_smin s2=%d s3=%d s5=%d s8=%d s16=%d s31=%d s32=%d\n",
           (int)vs.s2, (int)vs.s3, (int)vs.s5, (int)vs.s8,
           (int)vs.s16, (int)vs.s31, (int)vs.s32);

    vs.s2 = 1; vs.s3 = 3; vs.s5 = 15; vs.s8 = 127; vs.s16 = 32767;
    vs.s31 = 1073741823; vs.s32 = 2147483647;
    printf("runtime_smax s2=%d s3=%d s5=%d s8=%d s16=%d s31=%d s32=%d\n",
           (int)vs.s2, (int)vs.s3, (int)vs.s5, (int)vs.s8,
           (int)vs.s16, (int)vs.s31, (int)vs.s32);

    vu.u1 = 1u; vu.u2 = 3u; vu.u5 = 31u; vu.u8 = 255u; vu.u16 = 65535u;
    vu.u31 = 2147483647u; vu.u32 = 4294967295u;
    printf("runtime_umax u1=%u u2=%u u5=%u u8=%u u16=%u u31=%u u32=%u\n",
           (unsigned)vu.u1, (unsigned)vu.u2, (unsigned)vu.u5,
           (unsigned)vu.u8, (unsigned)vu.u16, (unsigned)vu.u31,
           (unsigned)vu.u32);

    /* Each extreme is derived from the opaque seed rather than stored as a
     * literal.  Signed destinations select between two in-range literals and
     * unsigned destinations are masked to the exact field width, which is what
     * keeps every store provably value-preserving for the conversion warnings. */
    v = seed;
    vs.s2  = (v != 0) ? -2 : 1;
    vs.s5  = (v != 0) ? -16 : 15;
    vs.s8  = (v != 0) ? -128 : 127;
    vs.s16 = (v != 0) ? -32768 : 32767;
    printf("derived_smin s2=%d s5=%d s8=%d s16=%d\n",
           (int)vs.s2, (int)vs.s5, (int)vs.s8, (int)vs.s16);

    vu.u1  = (unsigned)v & 1u;
    vu.u5  = (unsigned)(v * 31) & 31u;
    vu.u8  = (unsigned)(v * 255) & 255u;
    vu.u16 = (unsigned)(v * 65535) & 65535u;
    printf("derived_umax u1=%u u5=%u u8=%u u16=%u\n",
           (unsigned)vu.u1, (unsigned)vu.u5, (unsigned)vu.u8,
           (unsigned)vu.u16);

    vs.s5 = -15;
    vu.u5 = 30u;
    printf("near_extreme s5=%d u5=%u\n", (int)vs.s5, (unsigned)vu.u5);

    return 0;
}
