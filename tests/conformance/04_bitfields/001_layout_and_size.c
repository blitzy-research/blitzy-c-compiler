/* 001_layout_and_size.c
 *
 * Differential conformance suite -- Area 04 (Bitfields), program 001.
 * Property under test: sizeof, _Alignof and the observable byte image of
 * bitfield sequences.
 *
 * ROLE IN THE AREA
 *   This program is the area's LAYOUT BASELINE. Its eight compile-time
 *   size/alignment facts and its two byte images are the ground truth that every
 *   other program in 04_bitfields builds on. It is compiled by bcc for four
 *   targets (x86_64-linux-gnu, i686-linux-gnu, aarch64-linux-gnu,
 *   riscv64-linux-gnu) at three optimization levels (-O0, -O1, -O2) and by a
 *   reference C compiler, and its output is compared byte-for-byte under all
 *   three oracles:
 *     (a) bcc against the reference compiler, same target and same -O level;
 *     (b) each non-baseline target against the x86-64 baseline at the same level;
 *     (c) every cell against the expected output recorded in the sibling
 *         001_layout_and_size.expected record.
 *   No target restriction and no expected-divergence marker apply: all four
 *   targets, all three optimization levels and all three oracles are enabled.
 *   A divergence here is therefore a GENUINE FINDING in layout (src/sema/) or in
 *   bitfield extract/insert code generation (src/codegen/); no documented
 *   bitfield limitation exists anywhere in this repository that could classify
 *   such a divergence as expected.
 *
 * WHY EVERY PRINTED VALUE IS TARGET-INVARIANT BY DERIVATION
 *   The four supported targets differ only in `long` width, pointer width and
 *   ELF class: i686 is 4/4/ELF32 while the other three are 8/8/ELF64. `int` is
 *   four bytes on all four and all four are little-endian. Every bitfield below
 *   is declared on an explicit `unsigned int` or `signed int` base type, so the
 *   allocation unit is four bytes everywhere and the resulting size, alignment
 *   and byte image cannot vary by target. The invariance is a derivation rather
 *   than a coincidence, which is exactly what makes a divergence diagnostic.
 *
 * AUTHORING RULES HONOURED
 *   - No header is brought in, and no preprocessor directive appears anywhere in
 *     this file. bcc ships nine freestanding headers and no hosted standard
 *     input/output header, so naming one would fail against bcc while succeeding
 *     against the reference compiler, manufacturing a divergence caused by the
 *     test rather than by the compiler. printf is declared by hand instead.
 *   - Every bitfield carries an explicit `unsigned int` or `signed int` base
 *     type. A bitfield declared on unqualified `int` has implementation-defined
 *     signedness and would manufacture a spurious divergence.
 *   - The byte image is inspected through the `unsigned char` array member of a
 *     union, which is the well-defined aliasing route, and never through an
 *     incompatible pointer type.
 *   - One printed line per property claimed, so a single divergent line
 *     localizes the defect to one construct rather than to the program.
 *   - Every sizeof and _Alignof result is cast to `unsigned` and printed with
 *     the %u conversion, so neither the implementation's unsigned size type nor
 *     any long-width length modifier reaches a format string, and nothing whose
 *     width varies by target is ever printed.
 *   - Nothing address-valued is printed. No narrow character type of
 *     implementation-defined signedness is used as a value type. There is no
 *     clock reading, no nondeterministic source, no locale-sensitive
 *     formatting, and no file or network access. Nothing outside this source
 *     file is ever read: not the ambient process configuration, not the command
 *     line, not the standard input stream. Every input is a literal in this
 *     file, so the program is hermetic and reproducible from this source plus
 *     its expectation record alone.
 *
 * UNDEFINED-BEHAVIOUR FREEDOM
 *   No signed arithmetic overflow: the only signed bitfield is declared for its
 *   layout contribution and is never assigned. No shift operators appear. No
 *   aliasing violation. No read of uninitialized storage: both unions are
 *   written in full before any byte of either is read. No object is modified
 *   twice between sequence points, because each bitfield store is its own full
 *   expression. No dependence on padding bytes, on the relative addresses of
 *   unrelated objects, or on unspecified evaluation order. The exit status is 0,
 *   inside the mandated 0-125 range.
 *
 * PADDING-BIT HAZARD -- WHY THE ZEROING LOOPS EXIST
 *   A bitfield struct carries unnamed padding bits whose values remain
 *   unspecified even after every named field has been assigned. Because this
 *   program reads the byte image, each union is byte-zeroed through its
 *   `unsigned char` array member BEFORE any bitfield is stored. Without that
 *   step the bits of `packed3` not covered by a, b or c would be unspecified
 *   storage, and the printed image could differ between compilers for a reason
 *   having nothing to do with bitfield correctness.
 *
 * STRICT WARNING GATE
 *   Compiles clean under the suite's default gate, with no sanctioned
 *   deviation: -Wall -Wextra -pedantic -Wconversion -Wsign-conversion -Wshadow
 *   -Werror. Every bitfield store is a constant that fits its destination field,
 *   which is the one narrow-bitfield write form the conversion warnings accept
 *   without masking or a conditional select of in-range literals.
 *
 * EXPECTED OUTPUT -- 12 lines, 266 bytes, exactly one trailing newline
 *   sizeof_packed3=4
 *   alignof_packed3=4
 *   sizeof_oneword=4
 *   alignof_oneword=4
 *   sizeof_withsigned=4
 *   alignof_withsigned=4
 *   sizeof_spanning=8
 *   alignof_spanning=4
 *   packed3_readback a=5 b=21 c=300
 *   packed3_image ad 2c 01 00
 *   oneword_readback lo=4660 hi=22136
 *   oneword_image 34 12 78 56
 */
int printf(const char *, ...);

/* The canonical straddling sequence: 3 + 5 + 9 = 17 bits, so all three fields
 * share one four-byte allocation unit (sizeof 4, _Alignof 4). Fields a and b
 * share byte 0, and field c crosses the byte 1 / byte 2 boundary, which is what
 * turns the byte image into a meaningful check on insert code generation. */
struct packed3 { unsigned int a : 3; unsigned int b : 5; unsigned int c : 9; };

/* Two 16-bit halves exactly filling one unit with no padding bits at all; its
 * byte image is a direct little-endian placement check on every target. */
struct oneword { unsigned int lo : 16; unsigned int hi : 16; };

/* A signed and an unsigned narrow field sharing one unit: 4 + 4 = 8 bits still
 * occupy a single four-byte allocation unit, so sizeof is 4 and not 1. */
struct withsigned { signed int s : 4; unsigned int u : 4; };

/* 30 + 30 = 60 bits cannot share one four-byte unit, and a bitfield may not
 * cross the alignment boundary of its declared type, so q is placed in a second
 * unit: sizeof 8 with _Alignof still 4. A non-trivial, valuable layout fact. */
struct spanning { unsigned int p : 30; unsigned int q : 30; };

/* A union overlay is the only well-defined route to a struct's byte image: an
 * `unsigned char` array member may alias the object representation. Each array
 * length is taken from sizeof the struct itself, so an overlay can never be
 * shorter than the object it inspects. */
union image3 { struct packed3 f; unsigned char bytes[sizeof(struct packed3)]; };
union imagew  { struct oneword f; unsigned char bytes[sizeof(struct oneword)]; };

int main(void)
{
    union image3 i3;
    union imagew iw;
    unsigned int k;

    /* Eight compile-time layout facts, one printed line each. Each result is
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

    /* Mandatory padding-bit safety: write every byte of the overlay before any
     * bitfield is stored, so the bits not covered by a, b or c are known zero
     * rather than unspecified when the image is printed below. */
    for (k = 0; k < (unsigned)sizeof i3.bytes; ++k) { i3.bytes[k] = 0u; }
    /* Constants chosen to exercise the full width of each field: 5 fills 3 bits,
     * 21 fills 5 bits, and 300 needs 9 bits and so crosses a byte boundary. Each
     * value fits its destination field, keeping the conversion gate clean. */
    i3.f.a = 5u; i3.f.b = 21u; i3.f.c = 300u;
    printf("packed3_readback a=%u b=%u c=%u\n",
           (unsigned)i3.f.a, (unsigned)i3.f.b, (unsigned)i3.f.c);
    /* Derived image: byte 0 = (21 << 3) | 5 = 0xad, byte 1 = 300 & 0xff = 0x2c,
     * byte 2 = 300 >> 8 = 0x01, byte 3 = 0x00 (padding, zeroed above). */
    printf("packed3_image");
    for (k = 0; k < (unsigned)sizeof i3.bytes; ++k) { printf(" %02x", (unsigned)i3.bytes[k]); }
    printf("\n");

    /* The same padding discipline for the second overlay. This struct has no
     * padding bits, but zeroing first keeps the two cases uniform and keeps the
     * property under test independent of any read-modify-write detail in a
     * backend's bitfield insert sequence. */
    for (k = 0; k < (unsigned)sizeof iw.bytes; ++k) { iw.bytes[k] = 0u; }
    iw.f.lo = 0x1234u; iw.f.hi = 0x5678u;
    printf("oneword_readback lo=%u hi=%u\n", (unsigned)iw.f.lo, (unsigned)iw.f.hi);
    /* Derived image on every (little-endian) target: 34 12 78 56. */
    printf("oneword_image");
    for (k = 0; k < (unsigned)sizeof iw.bytes; ++k) { printf(" %02x", (unsigned)iw.bytes[k]); }
    printf("\n");
    return 0;
}
