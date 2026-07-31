/* Alignment is observed through residues and 0/1 relations, and no address is
 * ever printed.  Only the char, short, int and float alignments are printed raw,
 * because those four are identical on all four targets; they are printed as
 * values rather than asserted here, so a target that disagreed would show up as a
 * divergence on that line.  The alignments of double, long long, long and
 * pointers do differ (4 on i686 versus 8 elsewhere) and so appear only as bounded
 * relations.
 *
 * WIDTH DISCIPLINE.  Each address is converted in two steps: first to unsigned
 * long, which is exactly pointer width on all four targets (8 on x86-64, AArch64
 * and RISC-V 64; 4 on i686), and only then widened to unsigned long long, which
 * is 8 bytes everywhere and so is always wide enough to hold the residue
 * arithmetic below.  Casting a pointer straight to unsigned long long would cross
 * a width boundary on i686 and is diagnosed as -Wpointer-to-int-cast, which the
 * mandatory audit gate escalates to an error; the two-step form never crosses
 * one.  This matches the carrier discipline used by
 * 05_pointers/007_casts_roundtrip.c.  uintptr_t is not used because it would
 * require stdint.h and no header may be included. */

int printf(const char *, ...);

struct padded { char c; _Alignas(8) int aligned_member; };

static _Alignas(16) unsigned char buf16[32];
static _Alignas(32) int wide_aligned = 7;

int main(void)
{
    _Alignas(8) unsigned char local8[8];
    struct padded p;
    unsigned long long a_buf =
        (unsigned long long)(unsigned long)(const void *)buf16;
    unsigned long long a_wide =
        (unsigned long long)(unsigned long)(const void *)&wide_aligned;
    unsigned long long a_local =
        (unsigned long long)(unsigned long)(const void *)local8;
    unsigned long long a_member =
        (unsigned long long)(unsigned long)(const void *)&p.aligned_member;

    local8[0] = 1;
    p.c = 2;
    p.aligned_member = 3;

    printf("alignof_stable=%d %d %d %d\n",
           (int)_Alignof(char), (int)_Alignof(short),
           (int)_Alignof(int), (int)_Alignof(float));
    printf("alignof_bounded=%d %d %d %d\n",
           (int)(_Alignof(double) >= 4u && _Alignof(double) <= 8u),
           (int)(_Alignof(long long) >= 4u && _Alignof(long long) <= 8u),
           (int)(_Alignof(long) >= 4u && _Alignof(long) <= 8u),
           (int)(_Alignof(void *) >= 4u && _Alignof(void *) <= 8u));
    printf("alignas_struct=%d %d\n",
           (int)_Alignof(struct padded), (int)sizeof(struct padded));
    printf("residues=%d %d %d %d\n",
           (int)(a_buf % 16ULL), (int)(a_wide % 32ULL),
           (int)(a_local % 8ULL), (int)(a_member % 8ULL));
    printf("readback=%d %d %d\n", (int)local8[0], (int)p.c, p.aligned_member);
    return 0;
}
