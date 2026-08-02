/* No header is brought in and no preprocessor directive appears anywhere in this
 * file.  bcc ships no stdio.h: its bundled set is the nine required freestanding
 * headers plus a bonus stdatomic.h, ten files in all (docs/project-guide.md line
 * 212), and no standard I/O header is among them, so naming one would fail against
 * bcc while succeeding against the reference compiler.  printf is declared by hand
 * instead.
 *
 * Every bitfield below is declared on an explicit unsigned int or signed int base
 * type, because a field declared on unqualified int has implementation-defined
 * signedness and using it would manufacture a divergence that says nothing about
 * code generation.  Fixing the signedness is all the base type fixes: the size,
 * alignment and byte images printed here are measured expectations for the four
 * ABIs under test, not values the standard or the base type guarantees -- see the
 * note above struct packed3.
 *
 * A byte image is printed only for a struct whose every bit belongs to a named
 * field.  Zero-filling an overlay before storing into it buys freedom from reading
 * uninitialized storage and nothing more: a store to a bitfield may leave the
 * padding bits of its allocation unit unspecified afterwards, so a struct with
 * unnamed bits could legitimately read back with those zeroes discarded.  Full bit
 * coverage, not the zero fill, is what makes an image determinate.
 *
 * Every sizeof and _Alignof result is cast to unsigned and printed with %u, so no
 * type whose width varies by target and no long-width length modifier ever reaches
 * a format string.  Nothing address-valued is printed.  Nothing outside this file is
 * read, so every cell is reproducible from this source and its expectation record
 * alone.  The program compiles clean under the suite's default warning gate with no
 * sanctioned deviation. */
int printf(const char *, ...);

/* How much storage a bitfield sequence is allocated, whether a field may straddle a
 * storage-unit boundary and in what order fields are placed are all
 * implementation-defined.  Every layout value printed below is therefore a tested
 * ABI expectation for the four targets under test, not a guarantee the standard
 * makes.  Here 3 + 5 + 9 = 17 bits are expected to share one four-byte unit, with
 * a and b sharing byte 0 and c crossing the byte 1 / byte 2 boundary, so sizeof 4
 * observes allocation-unit rounding rather than restating arithmetic.  Fifteen bits
 * of the unit belong to no named field, so this struct is observed through sizeof,
 * _Alignof and named-member read-back alone, never as a byte image; the fully named
 * 3 + 5 + 9 + 15 partition is covered by 005_straddling_and_zero_width.c. */
struct packed3 { unsigned int a : 3; unsigned int b : 5; unsigned int c : 9; };

/* Two 16-bit halves expected to fill one unit exactly, leaving no padding bits. */
struct oneword { unsigned int lo : 16; unsigned int hi : 16; };

/* A signed and an unsigned narrow field: 8 bits of payload, but the allocation
 * unit rather than the payload is expected to set sizeof. */
struct withsigned { signed int s : 4; unsigned int u : 4; };

/* 60 bits of payload, which exceeds one four-byte unit, so q is expected in a
 * second unit while the alignment stays that of the declared type. */
struct spanning { unsigned int p : 30; unsigned int q : 30; };

/* Exactly 1 + 14 + 7 + 10 = 32 bits, so every bit of the allocation unit belongs to
 * a named field and the byte image carries no unspecified padding bit.  Every
 * internal byte boundary is crossed by some field -- b spans bits 1..14, c spans
 * 15..21 and d spans 22..31 -- which makes the image a stronger check on insert
 * code generation than a byte-aligned partition would be, and it is a different
 * partition from the one 005_straddling_and_zero_width.c uses, so the two programs
 * cover different placements rather than repeating one. */
struct covered32 {
    unsigned int a : 1;
    unsigned int b : 14;
    unsigned int c : 7;
    unsigned int d : 10;
};

/* A union overlay is the well-defined route to a struct's byte image: an
 * `unsigned char` array member may alias the object representation.  Each array
 * length comes from sizeof the struct, so an overlay can never be shorter than the
 * object it inspects.  Only the two fully covered structs have one, because only
 * their representations are specified once their named fields have been assigned. */
union imagew { struct oneword f; unsigned char bytes[sizeof(struct oneword)]; };
union imagec { struct covered32 f; unsigned char bytes[sizeof(struct covered32)]; };

/* Constant variant: a file-scope object with a constant initializer, so the whole
 * read-back is available to a constant folder. */
static struct packed3 p3 = { 5u, 21u, 300u };

/* Runtime variant.  The destinations are volatile, so every store must emit a real
 * insert and every read a real extract; the operands arrive from volatile sources,
 * so no value below can be folded into an immediate either.  All three objects have
 * static storage duration and therefore begin zeroed. */
static volatile struct packed3 vp3;
static volatile union imagew   viw;
static volatile union imagec   vic;

static volatile unsigned int src_a3 = 5u;       /* -> packed3.a    (3 bits)  */
static volatile unsigned int src_b5 = 21u;      /* -> packed3.b    (5 bits)  */
static volatile unsigned int src_c9 = 300u;     /* -> packed3.c    (9 bits)  */
static volatile unsigned int src_lo = 0x1234u;  /* -> oneword.lo   (16 bits) */
static volatile unsigned int src_hi = 0x5678u;  /* -> oneword.hi   (16 bits) */
static volatile unsigned int src_a1 = 1u;       /* -> covered32.a  (1 bit)   */
static volatile unsigned int src_b14 = 12345u;  /* -> covered32.b  (14 bits) */
static volatile unsigned int src_c7 = 101u;     /* -> covered32.c  (7 bits)  */
static volatile unsigned int src_d10 = 678u;    /* -> covered32.d  (10 bits) */

int main(void)
{
    union imagew iw;
    union imagec ic;
    unsigned int k;

    /* Ten compile-time layout facts, one printed line each. Each result is
     * cast to `unsigned` so that no target-width-dependent type and no
     * long-width length modifier reaches the format string. */
    printf("sizeof_packed3=%u\n", (unsigned)sizeof(struct packed3));
    printf("alignof_packed3=%u\n", (unsigned)_Alignof(struct packed3));
    printf("sizeof_oneword=%u\n", (unsigned)sizeof(struct oneword));
    printf("alignof_oneword=%u\n", (unsigned)_Alignof(struct oneword));
    printf("sizeof_withsigned=%u\n", (unsigned)sizeof(struct withsigned));
    printf("alignof_withsigned=%u\n", (unsigned)_Alignof(struct withsigned));
    printf("sizeof_spanning=%u\n", (unsigned)sizeof(struct spanning));
    printf("alignof_spanning=%u\n", (unsigned)_Alignof(struct spanning));
    printf("sizeof_covered32=%u\n", (unsigned)sizeof(struct covered32));
    printf("alignof_covered32=%u\n", (unsigned)_Alignof(struct covered32));

    /* Constant variant, under-covered struct: read-back only.  Each value fills its
     * destination field's full width -- 5 fills 3 bits, 21 fills 5 bits, and 300
     * needs 9 bits and so crosses a byte boundary -- and each fits its field, which
     * is what keeps the conversion warnings quiet without a mask. */
    printf("packed3_readback a=%u b=%u c=%u\n",
           (unsigned)p3.a, (unsigned)p3.b, (unsigned)p3.c);

    /* Constant variant, fully covered structs: read-back and byte image.  Each
     * overlay is byte-written in full before any byte of it is read. */
    for (k = 0; k < (unsigned)sizeof iw.bytes; ++k) { iw.bytes[k] = 0u; }
    iw.f.lo = 0x1234u; iw.f.hi = 0x5678u;
    printf("oneword_readback lo=%u hi=%u\n", (unsigned)iw.f.lo, (unsigned)iw.f.hi);
    printf("oneword_image");
    for (k = 0; k < (unsigned)sizeof iw.bytes; ++k) { printf(" %02x", (unsigned)iw.bytes[k]); }
    printf("\n");

    for (k = 0; k < (unsigned)sizeof ic.bytes; ++k) { ic.bytes[k] = 0u; }
    ic.f.a = 1u; ic.f.b = 12345u; ic.f.c = 101u; ic.f.d = 678u;
    printf("covered32_readback a=%u b=%u c=%u d=%u\n",
           (unsigned)ic.f.a, (unsigned)ic.f.b, (unsigned)ic.f.c, (unsigned)ic.f.d);
    /* MEASURED image, not a derived one.  73 e0 b2 a9 is what all four targets were
     * observed to produce, and the whole point of printing it is that nothing in the
     * standard predicts it.  C11 6.7.2.1p11 makes the order in which bitfields are
     * allocated within a storage unit implementation-defined, and 6.7.2.1p12 does the
     * same for whether a field may straddle a unit boundary, so no amount of reasoning
     * from field widths and byte order can produce this row: each supported target's
     * psABI fixes it, and the four psABIs happen to agree here.  The bytes correspond
     * to allocating a at the least-significant end of the unit and each later field
     * above the previous one -- 1 | (12345 << 1) | (101 << 15) | (678 << 22), reading
     * back as 0xa9b2e073 -- and that correspondence is a description of the observed
     * layout, not a prediction of it.  A target whose psABI allocated from the most
     * significant end would print a different, equally conforming row; that would be an
     * implementation-defined divergence to record with a narrowed target list, never a
     * defect in a backend that did nothing wrong. */
    printf("covered32_image");
    for (k = 0; k < (unsigned)sizeof ic.bytes; ++k) { printf(" %02x", (unsigned)ic.bytes[k]); }
    printf("\n");

    /* Runtime variant: identical facts, but every operand arrives from volatile
     * storage and every destination is volatile, so each store below is a real
     * insert and each read a real extract at every optimization level. Each
     * value is masked to its destination field's exact width, which makes the
     * conversion provably value-preserving for the strict warning gate. */
    vp3.a = src_a3 & 7u; vp3.b = src_b5 & 31u; vp3.c = src_c9 & 511u;
    printf("volatile_packed3_readback a=%u b=%u c=%u\n",
           (unsigned)vp3.a, (unsigned)vp3.b, (unsigned)vp3.c);

    viw.f.lo = src_lo & 0xffffu; viw.f.hi = src_hi & 0xffffu;
    printf("volatile_oneword_readback lo=%u hi=%u\n",
           (unsigned)viw.f.lo, (unsigned)viw.f.hi);
    printf("volatile_oneword_image");
    for (k = 0; k < (unsigned)sizeof viw.bytes; ++k) {
        printf(" %02x", (unsigned)viw.bytes[k]);
    }
    printf("\n");

    vic.f.a = src_a1 & 1u; vic.f.b = src_b14 & 16383u;
    vic.f.c = src_c7 & 127u; vic.f.d = src_d10 & 1023u;
    printf("volatile_covered32_readback a=%u b=%u c=%u d=%u\n",
           (unsigned)vic.f.a, (unsigned)vic.f.b, (unsigned)vic.f.c, (unsigned)vic.f.d);
    printf("volatile_covered32_image");
    for (k = 0; k < (unsigned)sizeof vic.bytes; ++k) {
        printf(" %02x", (unsigned)vic.bytes[k]);
    }
    printf("\n");
    return 0;
}
