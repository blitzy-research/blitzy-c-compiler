/* Area 08 - GCC extensions: __builtin_* intrinsics with deterministic results. */
int printf(const char *, ...);

struct off_s { int x; int y; double z; };

int main(void)
{
    char dst[16];
    char src[8];
    int i;
    int cmp_lt, cmp_gt, cmp_eq;
    int memcmp_ok;
    volatile unsigned int vu = 0xF0F0F0F0u;
    volatile unsigned long long vull = 0x0102030405060708ULL;
    volatile int vneg = -12345;

    for (i = 0; i < 8; i++) { src[i] = (char)('a' + i); }
    __builtin_memset(dst, 0, sizeof dst);
    __builtin_memcpy(dst, src, 8);
    memcmp_ok = (__builtin_memcmp(dst, src, 8) == 0) && (dst[8] == 0) && (dst[15] == 0);

    cmp_lt = __builtin_strcmp("abc", "abd");
    cmp_gt = __builtin_strcmp("abd", "abc");
    cmp_eq = __builtin_strcmp("abc", "abc");

    printf("bi_popcount32=%d\n", __builtin_popcount(0xF0F0F0F0u));
    printf("bi_popcount64=%d\n", __builtin_popcountll(0x0102030405060708ULL));
    printf("bi_clz_one=%d\n", __builtin_clz(1u));
    printf("bi_clz_top=%d\n", __builtin_clz(0x80000000u));
    printf("bi_ctz_eight=%d\n", __builtin_ctz(8u));
    printf("bi_clzll_one=%d\n", __builtin_clzll(1ULL));
    printf("bi_ctzll_bit40=%d\n", __builtin_ctzll(1ULL << 40));
    printf("bi_bswap16=%u\n", (unsigned)__builtin_bswap16(0x1234u));
    printf("bi_bswap32=%u\n", (unsigned)__builtin_bswap32(0x11223344u));
    printf("bi_bswap64=%llu\n", (unsigned long long)__builtin_bswap64(0x0102030405060708ULL));
    printf("bi_abs=%d\n", __builtin_abs(-12345));
    printf("bi_strlen=%u\n", (unsigned)__builtin_strlen("conformance"));
    printf("bi_strcmp_sign=%d %d %d\n",
           (cmp_lt < 0) ? -1 : (cmp_lt > 0), (cmp_gt < 0) ? -1 : (cmp_gt > 0),
           (cmp_eq < 0) ? -1 : (cmp_eq > 0));
    printf("bi_mem_roundtrip=%d\n", memcmp_ok);
    printf("bi_offsetof=%u %u %u\n",
           (unsigned)__builtin_offsetof(struct off_s, x),
           (unsigned)__builtin_offsetof(struct off_s, y),
           (unsigned)__builtin_offsetof(struct off_s, z));
    printf("bi_expect_taken=%d\n", __builtin_expect(1, 1) ? 7 : 8);
    printf("bi_expect_value=%d\n", (int)__builtin_expect(5L, 0L));
    printf("bi_runtime_popcount32=%d\n", __builtin_popcount(vu));
    printf("bi_runtime_popcount64=%d\n", __builtin_popcountll(vull));
    printf("bi_runtime_clz=%d\n", __builtin_clz(vu));
    printf("bi_runtime_ctz=%d\n", __builtin_ctz(vu));
    printf("bi_runtime_bswap32=%u\n", (unsigned)__builtin_bswap32((unsigned)vu));
    printf("bi_runtime_abs=%d\n", __builtin_abs(vneg));
    return 0;
}
