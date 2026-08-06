/* The same computation is spelled twice - once over literal constants, once over
 * volatile-qualified operands - and both spellings must print the same value at
 * every optimization level. */

int printf(const char *, ...);

/* Each computation is written once, as a macro, and instantiated twice: with
   literal constants, and with volatile-qualified operands whose values are read
   at run time and so are not available for compile-time substitution.  Using one
   macro for both instantiations is what makes the two variants the same
   computation.  Every macro parameter appears exactly once in every macro body,
   so no operand is evaluated more than once and no volatile object is read twice
   within a single expression. */

#define MIX_INT(A, B, C, D, E)     ((((A) * (B)) + ((C) << 2) - ((D) / (E))) % 97)
#define MIX_UNS(A, B, C)           ((((A) ^ (B)) + ((C) >> 3)) & 0x00FFFFFFu)
#define MIX_WIDE(A, B, C)          (((A) * (B)) - (C))
#define MIX_REL(A, B, C, D)        ((((A) < (B)) + ((C) >= (D))) * 10)
#define MIX_COND(A, B, C, D, E)    ((A) > (B) ? (C) - (D) : (E))

int main(void)
{
    int          f_int  = MIX_INT(191, 7, 13, 100, 3);
    unsigned int f_uns  = MIX_UNS(0xF0F0F0F0u, 0x0FF00FF0u, 0xABCDEF00u);
    long long    f_wide = MIX_WIDE(1234567LL, 89101LL, 987654321LL);
    int          f_rel  = MIX_REL(3, 9, 9, 9);
    int          f_cond = MIX_COND(41, 17, 41, 17, 5);

    volatile int          va = 191, vb = 7, vc = 13, vd = 100, ve = 3;
    volatile unsigned int ua = 0xF0F0F0F0u, ub = 0x0FF00FF0u, uc = 0xABCDEF00u;
    volatile long long    wa = 1234567LL, wb = 89101LL, wc = 987654321LL;
    volatile int          ra = 3, rb = 9, rc = 9, rd = 9;
    volatile int          ca = 41, cb = 17, cc = 41, cd = 17, ce = 5;

    int          r_int  = MIX_INT(va, vb, vc, vd, ve);
    unsigned int r_uns  = MIX_UNS(ua, ub, uc);
    long long    r_wide = MIX_WIDE(wa, wb, wc);
    int          r_rel  = MIX_REL(ra, rb, rc, rd);
    int          r_cond = MIX_COND(ca, cb, cc, cd, ce);

    int all_equal = (f_int == r_int) && (f_uns == r_uns)
                 && (f_wide == r_wide) && (f_rel == r_rel)
                 && (f_cond == r_cond);

    printf("int  folded=%d runtime=%d equal=%d\n", f_int, r_int, f_int == r_int);
    printf("uns  folded=%u runtime=%u equal=%d\n", f_uns, r_uns, f_uns == r_uns);
    printf("wide folded=%lld runtime=%lld equal=%d\n",
           f_wide, r_wide, f_wide == r_wide);
    printf("rel  folded=%d runtime=%d equal=%d\n", f_rel, r_rel, f_rel == r_rel);
    printf("cond folded=%d runtime=%d equal=%d\n",
           f_cond, r_cond, f_cond == r_cond);
    printf("all_equal=%d\n", all_equal);
    return 0;
}
