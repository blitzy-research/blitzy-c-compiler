/* Byte images are printed only for structs whose every bit is covered by a named
 * field, so no unspecified padding bit is ever observed.  Structs that necessarily
 * contain padding - the zero-width and unnamed-field cases - are observed through
 * sizeof, _Alignof and named-member read-back only.
 *
 * Where a field is placed inside its allocation unit is implementation-defined, so
 * every bit position named below is the placement expected on the four targets
 * under test rather than a guarantee the standard makes. */

int printf(const char *, ...);

/* Exactly 32 bits, fully covered, with c and d expected to straddle a byte
 * boundary inside the storage unit. */
struct cover32 {
    unsigned int a : 3;
    unsigned int b : 5;
    unsigned int c : 9;
    unsigned int d : 15;
};

struct cover64 {
    unsigned int a : 3;
    unsigned int b : 5;
    unsigned int c : 9;
    unsigned int d : 15;
    unsigned int e : 11;
    unsigned int f : 21;
};

/* Each field is too wide to finish inside the unit its predecessor occupies, so
 * each is expected in a fresh unit - observable through sizeof. */
struct nostraddle {
    unsigned int a : 20;
    unsigned int b : 20;
    unsigned int c : 24;
};

/* Zero-width separator: b is forced to the next storage unit. */
struct nozero { unsigned int a : 5; unsigned int b : 5; };
struct onezero { unsigned int a : 5; unsigned int : 0; unsigned int b : 5; };
struct twozero {
    unsigned int a : 7;
    unsigned int : 0;
    unsigned int b : 7;
    unsigned int : 0;
    unsigned int c : 7;
};

struct unnamedpad {
    unsigned int a : 3;
    unsigned int : 4;
    unsigned int b : 9;
};

/* Signed straddling field, observed through read-back rather than byte image. */
struct signedstraddle {
    signed int a : 9;
    signed int b : 13;
    signed int c : 10;
};

union image32 { struct cover32 f; unsigned char bytes[sizeof(struct cover32)]; };
union image64 { struct cover64 f; unsigned char bytes[sizeof(struct cover64)]; };

static void zero32(union image32 *p)
{
    unsigned int k;
    for (k = 0; k < (unsigned)sizeof p->bytes; ++k) { p->bytes[k] = 0u; }
}

static void zero64(union image64 *p)
{
    unsigned int k;
    for (k = 0; k < (unsigned)sizeof p->bytes; ++k) { p->bytes[k] = 0u; }
}

static void show32(const union image32 *p, const char *tag)
{
    unsigned int k;
    printf("%s", tag);
    for (k = 0; k < (unsigned)sizeof p->bytes; ++k) {
        printf(" %02x", (unsigned)p->bytes[k]);
    }
    printf("\n");
}

static void show64(const union image64 *p, const char *tag)
{
    unsigned int k;
    printf("%s", tag);
    for (k = 0; k < (unsigned)sizeof p->bytes; ++k) {
        printf(" %02x", (unsigned)p->bytes[k]);
    }
    printf("\n");
}

static struct nostraddle      ns  = { 703710u, 74565u, 6785451u };
static struct nozero          nz  = { 21u, 10u };
static struct onezero         oz  = { 21u, 10u };
static struct twozero         tz  = { 100u, 101u, 102u };
static struct unnamedpad      up  = { 5u, 300u };
static struct signedstraddle  ss  = { -256, -4096, -512 };

static volatile struct cover32        vc32;
static volatile struct signedstraddle vss;

int main(void)
{
    union image32 x;
    union image64 y;

    printf("size_cover32=%u align_cover32=%u\n",
           (unsigned)sizeof(struct cover32), (unsigned)_Alignof(struct cover32));
    printf("size_cover64=%u align_cover64=%u\n",
           (unsigned)sizeof(struct cover64), (unsigned)_Alignof(struct cover64));
    printf("size_nostraddle=%u align_nostraddle=%u\n",
           (unsigned)sizeof(struct nostraddle), (unsigned)_Alignof(struct nostraddle));
    printf("size_nozero=%u size_onezero=%u size_twozero=%u\n",
           (unsigned)sizeof(struct nozero), (unsigned)sizeof(struct onezero),
           (unsigned)sizeof(struct twozero));
    printf("size_unnamedpad=%u align_unnamedpad=%u\n",
           (unsigned)sizeof(struct unnamedpad), (unsigned)_Alignof(struct unnamedpad));
    printf("size_signedstraddle=%u align_signedstraddle=%u\n",
           (unsigned)sizeof(struct signedstraddle),
           (unsigned)_Alignof(struct signedstraddle));

    zero32(&x);
    x.f.a = 5u; x.f.b = 21u; x.f.c = 300u; x.f.d = 20000u;
    printf("cover32_read a=%u b=%u c=%u d=%u\n",
           (unsigned)x.f.a, (unsigned)x.f.b, (unsigned)x.f.c, (unsigned)x.f.d);
    show32(&x, "cover32_image");

    x.f.c = 511u;
    printf("cover32_after_c a=%u b=%u c=%u d=%u\n",
           (unsigned)x.f.a, (unsigned)x.f.b, (unsigned)x.f.c, (unsigned)x.f.d);
    show32(&x, "cover32_image_after_c");

    x.f.c = 0u;
    printf("cover32_c_zero a=%u b=%u c=%u d=%u\n",
           (unsigned)x.f.a, (unsigned)x.f.b, (unsigned)x.f.c, (unsigned)x.f.d);
    show32(&x, "cover32_image_c_zero");

    zero64(&y);
    y.f.a = 7u; y.f.b = 31u; y.f.c = 511u; y.f.d = 32767u;
    y.f.e = 2047u; y.f.f = 2097151u;
    printf("cover64_read a=%u b=%u c=%u d=%u e=%u f=%u\n",
           (unsigned)y.f.a, (unsigned)y.f.b, (unsigned)y.f.c,
           (unsigned)y.f.d, (unsigned)y.f.e, (unsigned)y.f.f);
    show64(&y, "cover64_all_ones");

    zero64(&y);
    y.f.a = 5u; y.f.b = 21u; y.f.c = 300u; y.f.d = 20000u;
    y.f.e = 1365u; y.f.f = 1398101u;
    printf("cover64_mixed a=%u b=%u c=%u d=%u e=%u f=%u\n",
           (unsigned)y.f.a, (unsigned)y.f.b, (unsigned)y.f.c,
           (unsigned)y.f.d, (unsigned)y.f.e, (unsigned)y.f.f);
    show64(&y, "cover64_image");

    printf("nostraddle_read a=%u b=%u c=%u\n",
           (unsigned)ns.a, (unsigned)ns.b, (unsigned)ns.c);
    ns.b = 1048575u;
    printf("nostraddle_after_b a=%u b=%u c=%u\n",
           (unsigned)ns.a, (unsigned)ns.b, (unsigned)ns.c);

    printf("nozero_read a=%u b=%u\n", (unsigned)nz.a, (unsigned)nz.b);
    printf("onezero_read a=%u b=%u\n", (unsigned)oz.a, (unsigned)oz.b);
    printf("twozero_read a=%u b=%u c=%u\n",
           (unsigned)tz.a, (unsigned)tz.b, (unsigned)tz.c);
    oz.a = 31u;
    printf("onezero_after_a a=%u b=%u\n", (unsigned)oz.a, (unsigned)oz.b);
    tz.b = 127u;
    printf("twozero_after_b a=%u b=%u c=%u\n",
           (unsigned)tz.a, (unsigned)tz.b, (unsigned)tz.c);

    printf("unnamedpad_read a=%u b=%u\n", (unsigned)up.a, (unsigned)up.b);
    up.b = 511u;
    printf("unnamedpad_after_b a=%u b=%u\n", (unsigned)up.a, (unsigned)up.b);

    printf("signedstraddle_read a=%d b=%d c=%d\n",
           (int)ss.a, (int)ss.b, (int)ss.c);
    ss.b = 4095;
    printf("signedstraddle_after_b a=%d b=%d c=%d\n",
           (int)ss.a, (int)ss.b, (int)ss.c);

    /* Volatile runtime variant: every access below happens at run time rather than
     * being folded. */
    vc32.a = 5u; vc32.b = 21u; vc32.c = 300u; vc32.d = 20000u;
    printf("volatile_cover32 a=%u b=%u c=%u d=%u\n",
           (unsigned)vc32.a, (unsigned)vc32.b, (unsigned)vc32.c, (unsigned)vc32.d);
    vc32.c = 511u;
    printf("volatile_cover32_after_c a=%u b=%u c=%u d=%u\n",
           (unsigned)vc32.a, (unsigned)vc32.b, (unsigned)vc32.c, (unsigned)vc32.d);

    vss.a = -256; vss.b = -4096; vss.c = -512;
    printf("volatile_signedstraddle a=%d b=%d c=%d\n",
           (int)vss.a, (int)vss.b, (int)vss.c);
    vss.b = 4095;
    printf("volatile_signedstraddle_after_b a=%d b=%d c=%d\n",
           (int)vss.a, (int)vss.b, (int)vss.c);

    return 0;
}
