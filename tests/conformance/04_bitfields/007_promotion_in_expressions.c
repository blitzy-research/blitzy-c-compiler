/* A bitfield narrower than int promotes to int even when its declared base type is
 * unsigned, so subtracting below zero yields a negative int rather than a large
 * unsigned value.  A 32-bit-wide unsigned bitfield promotes to unsigned int
 * instead and wraps modularly.  That contrast is the whole point of the file. */

int printf(const char *, ...);

struct fields {
    unsigned int u5  : 5;
    unsigned int u16 : 16;
    unsigned int u31 : 31;
    unsigned int u32 : 32;
    signed   int s5  : 5;
    signed   int s16 : 16;
};

static struct fields folded = { 0u, 0u, 0u, 0u, -16, -1000 };
static volatile struct fields runtime;

static void report(const char *tag, const struct fields *p)
{
    printf("%s_sizeof_promoted_u5=%u\n", tag, (unsigned)sizeof(p->u5 + 0));
    printf("%s_sizeof_promoted_s5=%u\n", tag, (unsigned)sizeof(p->s5 + 0));
    printf("%s_u5_minus_one=%d\n", tag, (int)(p->u5 - 1));
    printf("%s_u16_minus_one=%d\n", tag, (int)(p->u16 - 1));
    printf("%s_u31_minus_one=%d\n", tag, (int)(p->u31 - 1));
    printf("%s_u32_minus_one=%u\n", tag, (unsigned)(p->u32 - 1u));
    printf("%s_not_u5=%d\n", tag, (int)(!p->u5));
    printf("%s_complement_u5=%d\n", tag, (int)(~p->u5));
    printf("%s_negate_s5=%d\n", tag, (int)(-p->s5));
    printf("%s_s5_times_two=%d\n", tag, (int)(p->s5 * 2));
    printf("%s_s5_minus_one=%d\n", tag, (int)(p->s5 - 1));
    printf("%s_s16_times_hundred=%d\n", tag, (int)(p->s16 * 100));
}

static void report_volatile(const char *tag, volatile struct fields *p)
{
    printf("%s_sizeof_promoted_u5=%u\n", tag, (unsigned)sizeof(p->u5 + 0));
    printf("%s_sizeof_promoted_s5=%u\n", tag, (unsigned)sizeof(p->s5 + 0));
    printf("%s_u5_minus_one=%d\n", tag, (int)(p->u5 - 1));
    printf("%s_u16_minus_one=%d\n", tag, (int)(p->u16 - 1));
    printf("%s_u31_minus_one=%d\n", tag, (int)(p->u31 - 1));
    printf("%s_u32_minus_one=%u\n", tag, (unsigned)(p->u32 - 1u));
    printf("%s_not_u5=%d\n", tag, (int)(!p->u5));
    printf("%s_complement_u5=%d\n", tag, (int)(~p->u5));
    printf("%s_negate_s5=%d\n", tag, (int)(-p->s5));
    printf("%s_s5_times_two=%d\n", tag, (int)(p->s5 * 2));
    printf("%s_s5_minus_one=%d\n", tag, (int)(p->s5 - 1));
    printf("%s_s16_times_hundred=%d\n", tag, (int)(p->s16 * 100));
}

int main(void)
{
    report("folded", &folded);

    folded.u5 = 31u; folded.u16 = 65535u;
    folded.u31 = 2147483647u; folded.u32 = 4294967295u;
    folded.s5 = 15; folded.s16 = 32767;
    printf("folded_max_u5_squared=%d\n", (int)(folded.u5 * folded.u5));
    printf("folded_max_u5_shifted=%d\n", (int)(folded.u5 << 20));
    printf("folded_max_u16_squared=%d\n", (int)((folded.u16 >> 8) * 4));
    printf("folded_max_u32_plus_one=%u\n", (unsigned)(folded.u32 + 1u));
    printf("folded_max_s5_squared=%d\n", (int)(folded.s5 * folded.s5));
    printf("folded_max_s16_div=%d\n", (int)(folded.s16 / 3));

    folded.u5 = 1u;
    printf("folded_signed_div=%d\n", (int)((folded.u5 - 2) / 2));
    printf("folded_mixed_unsigned=%u\n", (unsigned)(folded.u5 + 1u));
    printf("folded_conditional=%d\n", (int)(folded.u5 ? 10 : 20));
    folded.u5 = 0u;
    printf("folded_conditional_zero=%d\n", (int)(folded.u5 ? 10 : 20));

    folded.u5 = 21u; folded.s5 = -11; folded.u32 = 3000000000u;
    printf("folded_nocast_u5=%d\n", folded.u5);
    printf("folded_nocast_s5=%d\n", folded.s5);
    printf("folded_nocast_u32=%u\n", folded.u32);

    runtime.u5 = 0u; runtime.u16 = 0u; runtime.u31 = 0u; runtime.u32 = 0u;
    runtime.s5 = -16; runtime.s16 = -1000;
    report_volatile("runtime", &runtime);

    runtime.u5 = 31u; runtime.u16 = 65535u;
    runtime.u31 = 2147483647u; runtime.u32 = 4294967295u;
    runtime.s5 = 15; runtime.s16 = 32767;
    printf("runtime_max_u5_squared=%d\n", (int)(runtime.u5 * runtime.u5));
    printf("runtime_max_u5_shifted=%d\n", (int)(runtime.u5 << 20));
    printf("runtime_max_u16_squared=%d\n", (int)((runtime.u16 >> 8) * 4));
    printf("runtime_max_u32_plus_one=%u\n", (unsigned)(runtime.u32 + 1u));
    printf("runtime_max_s5_squared=%d\n", (int)(runtime.s5 * runtime.s5));
    printf("runtime_max_s16_div=%d\n", (int)(runtime.s16 / 3));

    runtime.u5 = 1u;
    printf("runtime_signed_div=%d\n", (int)((runtime.u5 - 2) / 2));
    printf("runtime_mixed_unsigned=%u\n", (unsigned)(runtime.u5 + 1u));
    printf("runtime_conditional=%d\n", (int)(runtime.u5 ? 10 : 20));
    runtime.u5 = 0u;
    printf("runtime_conditional_zero=%d\n", (int)(runtime.u5 ? 10 : 20));

    runtime.u5 = 21u; runtime.s5 = -11; runtime.u32 = 3000000000u;
    printf("runtime_nocast_u5=%d\n", runtime.u5);
    printf("runtime_nocast_s5=%d\n", runtime.s5);
    printf("runtime_nocast_u32=%u\n", runtime.u32);

    return 0;
}
