/* Area 08 - GCC extensions: __attribute__((packed)) and __attribute__((aligned)). */
int printf(const char *, ...);

struct plain_s  { signed char a; int b; short c; };
struct __attribute__((packed))  packed_s { signed char a; int b; short c; };
struct __attribute__((aligned(16))) aligned_s { signed char a; int b; };
struct __attribute__((packed, aligned(8))) both_s { signed char a; int b; short c; };

static int aligned_obj __attribute__((aligned(16)));
static struct packed_s packed_obj;

int main(void)
{
    struct packed_s ps;
    struct plain_s pl;
    unsigned long addr;

    ps.a = (signed char)65;
    ps.b = 123456789;
    ps.c = (short)-321;

    pl.a = (signed char)-7;
    pl.b = -2000000000;
    pl.c = (short)32767;

    packed_obj.a = (signed char)1;
    packed_obj.b = 2;
    packed_obj.c = (short)3;

    aligned_obj = 99;
    addr = (unsigned long)(void *)&aligned_obj;

    printf("attr_plain_size=%u\n",   (unsigned)sizeof(struct plain_s));
    printf("attr_plain_align=%u\n",  (unsigned)_Alignof(struct plain_s));
    printf("attr_plain_off=%u %u %u\n",
           (unsigned)__builtin_offsetof(struct plain_s, a),
           (unsigned)__builtin_offsetof(struct plain_s, b),
           (unsigned)__builtin_offsetof(struct plain_s, c));
    printf("attr_packed_size=%u\n",  (unsigned)sizeof(struct packed_s));
    printf("attr_packed_align=%u\n", (unsigned)_Alignof(struct packed_s));
    printf("attr_packed_off=%u %u %u\n",
           (unsigned)__builtin_offsetof(struct packed_s, a),
           (unsigned)__builtin_offsetof(struct packed_s, b),
           (unsigned)__builtin_offsetof(struct packed_s, c));
    printf("attr_aligned_size=%u\n",  (unsigned)sizeof(struct aligned_s));
    printf("attr_aligned_align=%u\n", (unsigned)_Alignof(struct aligned_s));
    printf("attr_aligned_off=%u %u\n",
           (unsigned)__builtin_offsetof(struct aligned_s, a),
           (unsigned)__builtin_offsetof(struct aligned_s, b));
    printf("attr_both_size=%u\n",  (unsigned)sizeof(struct both_s));
    printf("attr_both_align=%u\n", (unsigned)_Alignof(struct both_s));
    printf("attr_both_off=%u %u %u\n",
           (unsigned)__builtin_offsetof(struct both_s, a),
           (unsigned)__builtin_offsetof(struct both_s, b),
           (unsigned)__builtin_offsetof(struct both_s, c));
    printf("attr_packed_saves=%d\n",
           (int)((unsigned)sizeof(struct plain_s) - (unsigned)sizeof(struct packed_s)));
    printf("attr_packed_roundtrip=%d %d %d\n", (int)ps.a, ps.b, (int)ps.c);
    printf("attr_plain_roundtrip=%d %d %d\n", (int)pl.a, pl.b, (int)pl.c);
    printf("attr_static_packed=%d %d %d\n",
           (int)packed_obj.a, packed_obj.b, (int)packed_obj.c);
    printf("attr_var_align16=%d\n", (addr % 16UL) == 0UL);
    printf("attr_var_value=%d\n", aligned_obj);
    return 0;
}
