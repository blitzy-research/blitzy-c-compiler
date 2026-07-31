/* Alignment is observed through residues and 0/1 relations, and no address is
 * ever printed.  Only the char, short, int and float alignments are printed raw,
 * because those four are identical on all four targets; they are printed as
 * values rather than asserted here, so a target that disagreed would show up as a
 * divergence on that line.
 *
 * EXACT EXPECTATIONS, NOT A TOLERATED RANGE.  The alignments of double, long
 * long, long and pointers do differ per target (4 on i686 versus 8 on the other
 * three), but they are still asserted EXACTLY rather than bounded.  The exact
 * value is selected by the architecture macro the compiler predefines, in the
 * same style as 08_gcc_extensions/008_inline_asm_per_target.c, so every printed
 * predicate is an equality against one specific number and the output stays
 * byte-identical across all four targets.  A permissive range such as
 * "4 <= align <= 8" must not be used as normalization: it accepts an alignment of
 * 5, 6 or 7, and it accepts the wrong choice between 4 and 8, so a real
 * code-generation defect would still print 1 and both the reference oracle and
 * the golden record would agree with it.  The expectation is deliberately keyed
 * to the ARCHITECTURE macro rather than to __SIZEOF_POINTER__: the architecture
 * is a categorical fact the harness fixes with --target, whereas a pointer-width
 * macro is a numeric claim by the compiler under test, and asserting that claim
 * against itself would be circular.
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

/* The exact alignment, in bytes, that double, long long, long and every object
 * pointer must have on the target being compiled, and the exact size of a pointer
 * and of long.  On all four supported targets these five quantities are one and
 * the same number, so a single expectation pins all of them.  Each branch states
 * the number outright, which is what makes the predicates below exact. */
#if defined(__x86_64__) || defined(__amd64__)
#define EXPECTED_WIDE_ALIGN 8u
#elif defined(__i386__) || defined(__i386)
#define EXPECTED_WIDE_ALIGN 4u
#elif defined(__aarch64__) || defined(__arm64__)
#define EXPECTED_WIDE_ALIGN 8u
#elif defined(__riscv) || defined(__riscv__)
#define EXPECTED_WIDE_ALIGN 8u
#else
#error "007_alignof_alignas: no alignment expectation selected; the target architecture predefined macro (__x86_64__ / __i386__ / __aarch64__ / __riscv) was not recognized"
#endif

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
    /* Exact equality against the one number the target must use.  An alignment of
     * 5, 6 or 7, or the wrong choice between 4 and 8, changes the printed digit
     * from 1 to 0 and so changes stdout. */
    printf("alignof_exact=%d %d %d %d\n",
           (int)(_Alignof(double) == EXPECTED_WIDE_ALIGN),
           (int)(_Alignof(long long) == EXPECTED_WIDE_ALIGN),
           (int)(_Alignof(long) == EXPECTED_WIDE_ALIGN),
           (int)(_Alignof(void *) == EXPECTED_WIDE_ALIGN));
    /* The same exact number also pins the width-varying sizes, and the two sizes
     * that are 8 on every target are pinned to 8 outright.  Without these, a
     * compiler that got both the alignment and the width wrong in the same
     * direction could still satisfy the line above. */
    printf("sizeof_exact=%d %d %d %d\n",
           (int)(sizeof(void *) == EXPECTED_WIDE_ALIGN),
           (int)(sizeof(long) == EXPECTED_WIDE_ALIGN),
           (int)(sizeof(double) == 8u),
           (int)(sizeof(long long) == 8u));
    printf("alignas_struct=%d %d\n",
           (int)_Alignof(struct padded), (int)sizeof(struct padded));
    printf("residues=%d %d %d %d\n",
           (int)(a_buf % 16ULL), (int)(a_wide % 32ULL),
           (int)(a_local % 8ULL), (int)(a_member % 8ULL));
    printf("readback=%d %d %d\n", (int)local8[0], (int)p.c, p.aligned_member);
    return 0;
}
