/* Area 14 - ABI and calling convention.
 * 001_many_integer_parameters: integer parameter counts deliberately sized to
 * exhaust the integer argument registers of every supported ABI and continue
 * past the threshold, so that every target genuinely marshals arguments on the
 * stack.  Widest register file across the four ABIs is eight integer registers
 * (AAPCS64 x0-x7, LP64D a0-a7); System V AMD64 has only six (rdi, rsi, rdx,
 * rcx, r8, r9); System V i386 cdecl passes everything on the stack.  Fifteen
 * integer-class parameters therefore spill on all four.
 *
 * Two variants: a folded variant whose arguments are compile-time constants,
 * and a runtime variant whose arguments are read from volatile storage so the
 * values are genuinely materialised and marshalled rather than folded into the
 * callee.
 *
 * Width normalisation: only int and long long are passed or printed.  No long,
 * no size_t, no pointer value, no plain char.
 */

int printf(const char *, ...);

static const char *variant_tag(int variant);
static void take_int15(int a01, int a02, int a03, int a04, int a05,
                       int a06, int a07, int a08, int a09, int a10,
                       int a11, int a12, int a13, int a14, int variant);
static void take_llong15(long long b01, long long b02, long long b03,
                         long long b04, long long b05, long long b06,
                         long long b07, long long b08, long long b09,
                         long long b10, long long b11, long long b12,
                         long long b13, long long b14, int variant);

static volatile int vint[15] = {
    1001, -1002, 1003, -1004, 1005, -1006, 1007,
    -1008, 1009, -1010, 1011, -1012, 1013, -1014, 1
};

static volatile long long vllong[15] = {
    5000000001LL, -5000000002LL, 5000000003LL, -5000000004LL,
    5000000005LL, -5000000006LL, 5000000007LL, -5000000008LL,
    5000000009LL, -5000000010LL, 5000000011LL, -5000000012LL,
    5000000013LL, -5000000014LL, 1LL
};

static const char *variant_tag(int variant)
{
    return (variant == 0) ? "folded" : "runtime";
}

static void take_int15(int a01, int a02, int a03, int a04, int a05,
                       int a06, int a07, int a08, int a09, int a10,
                       int a11, int a12, int a13, int a14, int variant)
{
    const char *t = variant_tag(variant);
    printf("int_%s_a01=%d\n", t, a01);
    printf("int_%s_a02=%d\n", t, a02);
    printf("int_%s_a03=%d\n", t, a03);
    printf("int_%s_a04=%d\n", t, a04);
    printf("int_%s_a05=%d\n", t, a05);
    printf("int_%s_a06=%d\n", t, a06);
    printf("int_%s_a07=%d\n", t, a07);
    printf("int_%s_a08=%d\n", t, a08);
    printf("int_%s_a09=%d\n", t, a09);
    printf("int_%s_a10=%d\n", t, a10);
    printf("int_%s_a11=%d\n", t, a11);
    printf("int_%s_a12=%d\n", t, a12);
    printf("int_%s_a13=%d\n", t, a13);
    printf("int_%s_a14=%d\n", t, a14);
}

static void take_llong15(long long b01, long long b02, long long b03,
                         long long b04, long long b05, long long b06,
                         long long b07, long long b08, long long b09,
                         long long b10, long long b11, long long b12,
                         long long b13, long long b14, int variant)
{
    const char *t = variant_tag(variant);
    printf("llong_%s_b01=%lld\n", t, b01);
    printf("llong_%s_b02=%lld\n", t, b02);
    printf("llong_%s_b03=%lld\n", t, b03);
    printf("llong_%s_b04=%lld\n", t, b04);
    printf("llong_%s_b05=%lld\n", t, b05);
    printf("llong_%s_b06=%lld\n", t, b06);
    printf("llong_%s_b07=%lld\n", t, b07);
    printf("llong_%s_b08=%lld\n", t, b08);
    printf("llong_%s_b09=%lld\n", t, b09);
    printf("llong_%s_b10=%lld\n", t, b10);
    printf("llong_%s_b11=%lld\n", t, b11);
    printf("llong_%s_b12=%lld\n", t, b12);
    printf("llong_%s_b13=%lld\n", t, b13);
    printf("llong_%s_b14=%lld\n", t, b14);
}

int main(void)
{
    take_int15(1001, -1002, 1003, -1004, 1005, -1006, 1007,
               -1008, 1009, -1010, 1011, -1012, 1013, -1014, 0);
    take_llong15(5000000001LL, -5000000002LL, 5000000003LL, -5000000004LL,
                 5000000005LL, -5000000006LL, 5000000007LL, -5000000008LL,
                 5000000009LL, -5000000010LL, 5000000011LL, -5000000012LL,
                 5000000013LL, -5000000014LL, 0);
    take_int15(vint[0], vint[1], vint[2], vint[3], vint[4], vint[5], vint[6],
               vint[7], vint[8], vint[9], vint[10], vint[11], vint[12],
               vint[13], vint[14]);
    take_llong15(vllong[0], vllong[1], vllong[2], vllong[3], vllong[4],
                 vllong[5], vllong[6], vllong[7], vllong[8], vllong[9],
                 vllong[10], vllong[11], vllong[12], vllong[13],
                 (int)vllong[14]);
    return 0;
}
