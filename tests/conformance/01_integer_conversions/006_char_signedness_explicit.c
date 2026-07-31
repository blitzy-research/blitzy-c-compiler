/* Plain char is deliberately never used for a signedness-dependent value: its
 * signedness is implementation-defined, and it was measured signed on x86-64
 * and i686 but unsigned on AArch64 and RISC-V 64, so a plain-char value here
 * would diverge across backends for a reason unrelated to the compiler under
 * test.  Every character type below is spelled signed char or unsigned char.
 */
int printf(const char *, ...);

static const char *schar_promotes(void)
{
    volatile signed char v = -1;
    return _Generic(+v, int: "int", unsigned int: "unsigned int", default: "other");
}

static const char *uchar_promotes(void)
{
    volatile unsigned char v = 255;
    return _Generic(+v, int: "int", unsigned int: "unsigned int", default: "other");
}

int main(void)
{
    volatile signed char s_min = -128;
    volatile signed char s_max = 127;
    volatile signed char s_neg = -1;
    volatile unsigned char u_max = 255;
    volatile unsigned char u_200 = 200;
    volatile int i_200 = 200;
    volatile int i_neg = -1;

    printf("fold_schar_min=%d\n", (int)(signed char)(-128));
    printf("fold_schar_max=%d\n", (int)(signed char)127);
    printf("fold_uchar_max=%d\n", (int)(unsigned char)255);
    printf("fold_schar_to_uchar=%d\n", (int)(unsigned char)(signed char)(-1));
    printf("fold_uchar_to_schar=%d\n", (int)(signed char)(unsigned char)200);
    printf("fold_int_to_schar=%d\n", (int)(signed char)200);
    printf("fold_int_to_uchar=%d\n", (int)(unsigned char)(-1));
    printf("fold_uchar_add=%d\n", (unsigned char)255 + 1);
    printf("fold_schar_sub=%d\n", (signed char)(-128) - 1);
    printf("fold_schar_shift=%d\n", (signed char)(-1) >> 1);
    printf("fold_uchar_shift=%d\n", (unsigned char)200 >> 1);
    printf("fold_schar_uchar_diff=%d\n", (int)(signed char)0x7f - (int)(unsigned char)0x7f);

    /* Runtime variant: the operands are volatile, so each sign or zero extension
     * is applied to a value read at run time rather than to a constant. */
    printf("run_schar_min=%d\n", (int)s_min);
    printf("run_schar_max=%d\n", (int)s_max);
    printf("run_uchar_max=%d\n", (int)u_max);
    printf("run_schar_to_uchar=%d\n", (int)(unsigned char)s_neg);
    printf("run_uchar_to_schar=%d\n", (int)(signed char)u_200);
    printf("run_int_to_schar=%d\n", (int)(signed char)i_200);
    printf("run_int_to_uchar=%d\n", (int)(unsigned char)i_neg);
    printf("run_uchar_add=%d\n", u_max + 1);
    printf("run_schar_sub=%d\n", s_min - 1);
    printf("run_schar_shift=%d\n", s_neg >> 1);
    printf("run_uchar_shift=%d\n", u_200 >> 1);
    printf("run_schar_uchar_diff=%d\n", (int)s_max - (int)(unsigned char)127);

    printf("schar_promotes=%s\n", schar_promotes());
    printf("uchar_promotes=%s\n", uchar_promotes());
    return 0;
}
