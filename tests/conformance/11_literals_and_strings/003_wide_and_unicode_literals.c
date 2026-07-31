/* No header is included here, so none of the three C11 character type names is
   written: every literal is indexed in place and every element is cast explicitly
   to long long or unsigned long long.  The source file is pure US-ASCII, and all
   non-ASCII characters appear only as universal-character-name escapes. */
int printf(const char *, ...);

#define WIDE_LIT  L"A\u0024\u00A9\u0100\u07FF\u7FFFz"
#define U8_LIT   u8"A\u0024\u00A9\u0100\u07FF\u7FFFz"
#define U16_LIT   u"A\u0024\u00A9\u0100\u07FF\u7FFFz"
#define U32_LIT   U"A\u0024\u00A9\u0100\u07FF\u7FFFz"

#define WIDE_COUNT ((int)(sizeof WIDE_LIT / sizeof WIDE_LIT[0]))
#define U8_COUNT   ((int)(sizeof U8_LIT   / sizeof U8_LIT[0]))
#define U16_COUNT  ((int)(sizeof U16_LIT  / sizeof U16_LIT[0]))
#define U32_COUNT  ((int)(sizeof U32_LIT  / sizeof U32_LIT[0]))

static volatile int zero = 0;

int main(void)
{
    int i;

    printf("wide_count=%d\n", WIDE_COUNT);
    for (i = 0; i < WIDE_COUNT; i++) {
        printf("wide[%d]=%lld\n", i, (long long)WIDE_LIT[i]);
    }

    printf("u8_count=%d\n", U8_COUNT);
    for (i = 0; i < U8_COUNT; i++) {
        printf("u8[%d]=%llu\n", i,
               (unsigned long long)(unsigned char)U8_LIT[i]);
    }

    printf("u16_count=%d\n", U16_COUNT);
    for (i = 0; i < U16_COUNT; i++) {
        printf("u16[%d]=%llu\n", i, (unsigned long long)U16_LIT[i]);
    }

    printf("u32_count=%d\n", U32_COUNT);
    for (i = 0; i < U32_COUNT; i++) {
        printf("u32[%d]=%llu\n", i, (unsigned long long)U32_LIT[i]);
    }

    printf("wide_char_A=%lld\n", (long long)L'A');
    printf("wide_char_dollar=%lld\n", (long long)L'\u0024');
    printf("wide_char_copy=%lld\n", (long long)L'\u00A9');
    printf("wide_char_high=%lld\n", (long long)L'\u7FFF');
    printf("u16_char_A=%llu\n", (unsigned long long)u'A');
    printf("u16_char_copy=%llu\n", (unsigned long long)u'\u00A9');
    printf("u16_char_high=%llu\n", (unsigned long long)u'\u7FFF');
    printf("u32_char_A=%llu\n", (unsigned long long)U'A');
    printf("u32_char_copy=%llu\n", (unsigned long long)U'\u00A9');
    printf("u32_char_high=%llu\n", (unsigned long long)U'\u7FFF');

    printf("wide_terminator=%lld\n", (long long)WIDE_LIT[WIDE_COUNT - 1]);
    printf("u16_terminator=%llu\n", (unsigned long long)U16_LIT[U16_COUNT - 1]);
    printf("u32_terminator=%llu\n", (unsigned long long)U32_LIT[U32_COUNT - 1]);
    printf("u8_terminator=%llu\n",
           (unsigned long long)(unsigned char)U8_LIT[U8_COUNT - 1]);

    printf("runtime_wide0=%lld\n", (long long)WIDE_LIT[zero]);
    printf("runtime_u16_0=%llu\n", (unsigned long long)U16_LIT[zero]);
    printf("runtime_u32_0=%llu\n", (unsigned long long)U32_LIT[zero]);
    printf("runtime_u8_0=%llu\n",
           (unsigned long long)(unsigned char)U8_LIT[zero]);
    return 0;
}
