/* Area 01 / program 004 - narrowing integer conversions at the destination
 * range edge, in both a compile-time-folded and a volatile-runtime variant.
 *
 * Self-contained by mandate: no header is included and the only libc
 * prototype is hand-declared, because bcc ships no <stdio.h>.
 */
int printf(const char *, ...);

int main(void)
{
    /* Folded variant: the narrowing is applied to constant expressions, so a
     * correct compiler resolves all three values in the constant folder.
     * The casts are explicit because an *implicit* out-of-range constant
     * conversion to a signed narrow type trips -Woverflow, which the
     * sanctioned deviation for this program does not drop.
     */
    unsigned char  fold_u8  = (unsigned char)(-56);
    signed char    fold_i8  = (signed char)200;
    unsigned short fold_u16 = (unsigned short)(-200);

    /* Runtime variant: the same three narrowings, expressed as *implicit*
     * assignment conversions from volatile int sources.  volatile defeats
     * constant propagation, so the backend must emit real truncations, and
     * the implicit form is precisely what -Wconversion / -Wsign-conversion
     * object to - which is why this program carries the recorded deviation.
     */
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
