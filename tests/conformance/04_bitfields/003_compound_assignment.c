int printf(const char *, ...);

struct acc {
    unsigned int u6 : 6;
    signed   int s7 : 7;
    signed   int s9 : 9;
};

static struct acc folded = { 5u, 3, -100 };
static volatile struct acc runtime = { 5u, 3, -100 };

int main(void)
{
    folded.u6 += 3u;   printf("folded_u6_add=%u\n", (unsigned)folded.u6);
    folded.u6 -= 1;    printf("folded_u6_sub=%u\n", (unsigned)folded.u6);
    folded.u6 |= 0x18u; printf("folded_u6_or=%u\n", (unsigned)folded.u6);
    folded.u6 &= 0x1Eu; printf("folded_u6_and=%u\n", (unsigned)folded.u6);
    folded.u6 ^= 0x05u; printf("folded_u6_xor=%u\n", (unsigned)folded.u6);
    folded.u6 <<= 1;   printf("folded_u6_shl=%u\n", (unsigned)folded.u6);
    folded.u6 >>= 2;   printf("folded_u6_shr=%u\n", (unsigned)folded.u6);

    /* The signed field is non-negative at every shift, so no negative value is
     * ever shifted, and every right-hand constant lies inside the field's
     * representable range. */
    folded.s7 += 5;    printf("folded_s7_add=%d\n", (int)folded.s7);
    folded.s7 -= 2;    printf("folded_s7_sub=%d\n", (int)folded.s7);
    folded.s7 |= 0x11;  printf("folded_s7_or=%d\n", (int)folded.s7);
    folded.s7 &= 0x1F;  printf("folded_s7_and=%d\n", (int)folded.s7);
    folded.s7 ^= 0x02;  printf("folded_s7_xor=%d\n", (int)folded.s7);
    folded.s7 <<= 1;   printf("folded_s7_shl=%d\n", (int)folded.s7);
    folded.s7 >>= 3;   printf("folded_s7_shr=%d\n", (int)folded.s7);

    folded.s9 += 40;   printf("folded_s9_add=%d\n", (int)folded.s9);
    folded.s9 -= 25;   printf("folded_s9_sub=%d\n", (int)folded.s9);
    folded.s9 -= 171;  printf("folded_s9_min=%d\n", (int)folded.s9);
    folded.s9 += 255;  printf("folded_s9_up1=%d\n", (int)folded.s9);
    folded.s9 += 200;  printf("folded_s9_up2=%d\n", (int)folded.s9);
    folded.s9 += 56;   printf("folded_s9_max=%d\n", (int)folded.s9);

    /* The identical operator sequence through volatile storage, so each
     * read-modify-write happens at run time rather than being folded. */
    runtime.u6 += 3u;   printf("runtime_u6_add=%u\n", (unsigned)runtime.u6);
    runtime.u6 -= 1;    printf("runtime_u6_sub=%u\n", (unsigned)runtime.u6);
    runtime.u6 |= 0x18u; printf("runtime_u6_or=%u\n", (unsigned)runtime.u6);
    runtime.u6 &= 0x1Eu; printf("runtime_u6_and=%u\n", (unsigned)runtime.u6);
    runtime.u6 ^= 0x05u; printf("runtime_u6_xor=%u\n", (unsigned)runtime.u6);
    runtime.u6 <<= 1;   printf("runtime_u6_shl=%u\n", (unsigned)runtime.u6);
    runtime.u6 >>= 2;   printf("runtime_u6_shr=%u\n", (unsigned)runtime.u6);

    runtime.s7 += 5;    printf("runtime_s7_add=%d\n", (int)runtime.s7);
    runtime.s7 -= 2;    printf("runtime_s7_sub=%d\n", (int)runtime.s7);
    runtime.s7 |= 0x11;  printf("runtime_s7_or=%d\n", (int)runtime.s7);
    runtime.s7 &= 0x1F;  printf("runtime_s7_and=%d\n", (int)runtime.s7);
    runtime.s7 ^= 0x02;  printf("runtime_s7_xor=%d\n", (int)runtime.s7);
    runtime.s7 <<= 1;   printf("runtime_s7_shl=%d\n", (int)runtime.s7);
    runtime.s7 >>= 3;   printf("runtime_s7_shr=%d\n", (int)runtime.s7);

    runtime.s9 += 40;   printf("runtime_s9_add=%d\n", (int)runtime.s9);
    runtime.s9 -= 25;   printf("runtime_s9_sub=%d\n", (int)runtime.s9);
    runtime.s9 -= 171;  printf("runtime_s9_min=%d\n", (int)runtime.s9);
    runtime.s9 += 255;  printf("runtime_s9_up1=%d\n", (int)runtime.s9);
    runtime.s9 += 200;  printf("runtime_s9_up2=%d\n", (int)runtime.s9);
    runtime.s9 += 56;   printf("runtime_s9_max=%d\n", (int)runtime.s9);

    return 0;
}
