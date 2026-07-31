/* 002_read_write.c -- read and write of every bitfield, folded and volatile runtime variants.
 * Area 04 (bitfields).  No header is included; printf is declared by hand.
 * Every bitfield carries an explicit signed int / unsigned int base type. */

int printf(const char *, ...);

struct bits {
    unsigned int u3  : 3;
    signed   int s5  : 5;
    unsigned int u9  : 9;
    signed   int s12 : 12;
    unsigned int u1  : 1;
};

/* Folded variant: file-scope object with a constant initializer.  Every value
 * below is inside its field's representable range. */
static struct bits folded = { 6u, -13, 400u, -1500, 1u };

/* Runtime variant: volatile-qualified, so each read and write must emit real
 * extract / insert code instead of being folded to an immediate. */
static volatile struct bits runtime;

int main(void)
{
    printf("folded_read u3=%u s5=%d u9=%u s12=%d u1=%u\n",
           (unsigned)folded.u3, (int)folded.s5, (unsigned)folded.u9,
           (int)folded.s12, (unsigned)folded.u1);

    folded.u3 = 1u;
    folded.s5 = 15;
    folded.u9 = 511u;
    folded.s12 = 2047;
    folded.u1 = 0u;
    printf("folded_write u3=%u s5=%d u9=%u s12=%d u1=%u\n",
           (unsigned)folded.u3, (int)folded.s5, (unsigned)folded.u9,
           (int)folded.s12, (unsigned)folded.u1);

    folded.s5 = -16;
    folded.s12 = -2048;
    printf("folded_min s5=%d s12=%d\n", (int)folded.s5, (int)folded.s12);

    runtime.u3 = 6u;
    runtime.s5 = -13;
    runtime.u9 = 400u;
    runtime.s12 = -1500;
    runtime.u1 = 1u;
    printf("runtime_read u3=%u s5=%d u9=%u s12=%d u1=%u\n",
           (unsigned)runtime.u3, (int)runtime.s5, (unsigned)runtime.u9,
           (int)runtime.s12, (unsigned)runtime.u1);

    /* Read-modify-write through volatile storage.  Unsigned destinations are
     * masked to their exact field width so the conversion is provably
     * value-preserving; signed destinations receive an in-range result. */
    runtime.u3  = (unsigned)((int)runtime.u3 + 1) & 7u;
    runtime.s5  = (signed)((int)runtime.s5 + 1);
    runtime.u9  = (unsigned)((int)runtime.u9 + 1) & 511u;
    runtime.s12 = (signed)((int)runtime.s12 + 1);
    runtime.u1  = (unsigned)(1u - (unsigned)runtime.u1) & 1u;
    printf("runtime_write u3=%u s5=%d u9=%u s12=%d u1=%u\n",
           (unsigned)runtime.u3, (int)runtime.s5, (unsigned)runtime.u9,
           (int)runtime.s12, (unsigned)runtime.u1);

    runtime.u3 = 7u;
    runtime.u9 = 511u;
    runtime.u1 = 1u;
    printf("runtime_max u3=%u u9=%u u1=%u\n",
           (unsigned)runtime.u3, (unsigned)runtime.u9, (unsigned)runtime.u1);

    runtime.u3 = 0u;
    runtime.u9 = 0u;
    runtime.u1 = 0u;
    printf("runtime_zero u3=%u u9=%u u1=%u\n",
           (unsigned)runtime.u3, (unsigned)runtime.u9, (unsigned)runtime.u1);

    return 0;
}
