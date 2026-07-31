int printf(const char *, ...);

/* Unary + applies the integer promotions, and a _Generic controlling
 * expression is unevaluated, so these probes report the promoted type
 * without performing any conversion the strict warning gate objects to.
 */
static const char *promo_uchar(void)
{
    volatile unsigned char v = 200;
    return _Generic(+v, int: "int", unsigned int: "unsigned int", default: "other");
}

static const char *promo_schar(void)
{
    volatile signed char v = -100;
    return _Generic(+v, int: "int", unsigned int: "unsigned int", default: "other");
}

static const char *promo_ushort(void)
{
    volatile unsigned short v = 60000;
    return _Generic(+v, int: "int", unsigned int: "unsigned int", default: "other");
}

static const char *promo_short(void)
{
    volatile short v = -30000;
    return _Generic(+v, int: "int", unsigned int: "unsigned int", default: "other");
}

static const char *promo_bool(void)
{
    volatile _Bool v = 1;
    return _Generic(+v, int: "int", unsigned int: "unsigned int", default: "other");
}

static const char *promo_product(void)
{
    volatile unsigned char a = 200;
    volatile unsigned char b = 200;
    return _Generic(a * b, int: "int", unsigned int: "unsigned int", default: "other");
}

int main(void)
{
    volatile unsigned char  uc = 200;
    volatile signed char    sc = -128;
    volatile unsigned short us = 300;
    volatile short          sh = -300;
    volatile unsigned char  zero = 0;

    printf("fold_uchar_product=%d\n", (unsigned char)200 * (unsigned char)200);
    printf("fold_ushort_product=%d\n", (unsigned short)300 * (unsigned short)300);
    printf("fold_uchar_complement=%d\n", ~(unsigned char)0);
    printf("fold_schar_negate=%d\n", -(signed char)(-128));
    printf("fold_short_negate=%d\n", -(short)(-300));
    printf("fold_uchar_shift=%d\n", (unsigned char)1 << 20);
    printf("fold_schar_widen=%d\n", (signed char)(-128) - 1);

    /* Runtime variant: volatile operands are read at run time, so none of these
     * values is available for compile-time substitution. */
    printf("run_uchar_product=%d\n", uc * uc);
    printf("run_ushort_product=%d\n", us * us);
    printf("run_uchar_complement=%d\n", ~zero);
    printf("run_schar_negate=%d\n", -sc);
    printf("run_short_negate=%d\n", -sh);
    printf("run_uchar_shift=%d\n", (zero + 1) << 20);
    printf("run_schar_widen=%d\n", sc - 1);

    printf("promo_uchar=%s\n", promo_uchar());
    printf("promo_schar=%s\n", promo_schar());
    printf("promo_ushort=%s\n", promo_ushort());
    printf("promo_short=%s\n", promo_short());
    printf("promo_bool=%s\n", promo_bool());
    printf("promo_product=%s\n", promo_product());
    return 0;
}
