/* 001_arithmetic_invariance.c -- Area 09, differential conformance suite.
 *
 * Claim under test: the OBSERVABLE RESULT of ordinary integer arithmetic is
 * identical at -O0, -O1 and -O2.  Every expression appears twice: once built
 * entirely from literal constants (so the constant folder may evaluate it at
 * compile time) and once from volatile-qualified storage (so the backend must
 * emit real instructions).  Both spellings must print the same value.
 */

int printf(const char *, ...);

int main(void)
{
    /* ---- folded variant: literal operands, foldable at compile time ---- */
    int f_add = 1000 + 337;
    int f_sub = 2000 - 663;
    int f_mul = 191 * 7;
    int f_div = 9359 / 7;
    int f_mod = 9362 % 7;
    int f_neg = -1337;
    unsigned int f_shl = 167u << 3;
    unsigned int f_shr = 5348u >> 2;
    unsigned int f_and = 0xF0F0F0F0u & 0x0FF00FF0u;
    unsigned int f_or  = 0xF0F0F0F0u | 0x0FF00FF0u;
    unsigned int f_xor = 0xF0F0F0F0u ^ 0x0FF00FF0u;
    unsigned int f_not = ~0x0000FFFFu;
    long long f_wide = 1234567891011LL + 9876543210LL;

    /* ---- runtime variant: volatile operands, not foldable -------------- */
    volatile int va = 1000, vb = 337, vc = 2000, vd = 663;
    volatile int ve = 191, vf = 7, vg = 9359, vh = 9362, vi = 1337;
    volatile unsigned int vs = 167u, vt = 5348u;
    volatile unsigned int vm = 0xF0F0F0F0u, vn = 0x0FF00FF0u, vp = 0x0000FFFFu;
    volatile long long vw = 1234567891011LL, vx = 9876543210LL;

    int r_add = va + vb;
    int r_sub = vc - vd;
    int r_mul = ve * vf;
    int r_div = vg / vf;
    int r_mod = vh % vf;
    int r_neg = -vi;
    unsigned int r_shl = vs << 3;
    unsigned int r_shr = vt >> 2;
    unsigned int r_and = vm & vn;
    unsigned int r_or  = vm | vn;
    unsigned int r_xor = vm ^ vn;
    unsigned int r_not = ~vp;
    long long r_wide = vw + vx;

    printf("add folded=%d runtime=%d equal=%d\n", f_add, r_add, f_add == r_add);
    printf("sub folded=%d runtime=%d equal=%d\n", f_sub, r_sub, f_sub == r_sub);
    printf("mul folded=%d runtime=%d equal=%d\n", f_mul, r_mul, f_mul == r_mul);
    printf("div folded=%d runtime=%d equal=%d\n", f_div, r_div, f_div == r_div);
    printf("mod folded=%d runtime=%d equal=%d\n", f_mod, r_mod, f_mod == r_mod);
    printf("neg folded=%d runtime=%d equal=%d\n", f_neg, r_neg, f_neg == r_neg);
    printf("shl folded=%u runtime=%u equal=%d\n", f_shl, r_shl, f_shl == r_shl);
    printf("shr folded=%u runtime=%u equal=%d\n", f_shr, r_shr, f_shr == r_shr);
    printf("and folded=%u runtime=%u equal=%d\n", f_and, r_and, f_and == r_and);
    printf("or  folded=%u runtime=%u equal=%d\n", f_or, r_or, f_or == r_or);
    printf("xor folded=%u runtime=%u equal=%d\n", f_xor, r_xor, f_xor == r_xor);
    printf("not folded=%u runtime=%u equal=%d\n", f_not, r_not, f_not == r_not);
    printf("wide folded=%lld runtime=%lld equal=%d\n",
           f_wide, r_wide, f_wide == r_wide);
    return 0;
}
