int printf(const char *, ...);

int main(void)
{
    /* The casts are explicit because an *implicit* out-of-range constant
     * conversion to a signed narrow type trips -Woverflow, which this
     * program's recorded warning-gate deviation does not drop. */
    unsigned char  fold_u8  = (unsigned char)(-56);
    signed char    fold_i8  = (signed char)200;
    unsigned short fold_u16 = (unsigned short)(-200);

    /* The same three narrowings as *implicit* assignment conversions from
     * volatile int sources, so the converted values are read at run time
     * rather than substituted at compile time.  The implicit form is what
     * -Wconversion and -Wsign-conversion object to, which is why this program
     * carries the recorded deviation for both. */
    volatile int src_u8  = -56;
    volatile int src_i8  = 200;
    volatile int src_u16 = -200;

    unsigned char  run_u8;
    signed char    run_i8;
    unsigned short run_u16;

    run_u8  = src_u8;
    run_i8  = src_i8;
    run_u16 = src_u16;

    printf("narrow_u8=%d narrow_i8=%d narrow_u16=%d\n",
           fold_u8, fold_i8, fold_u16);
    printf("runtime_u8=%d runtime_i8=%d runtime_u16=%d\n",
           run_u8, run_i8, run_u16);
    return 0;
}
