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

/* THE ONE IMPLEMENTATION-DEFINED PROPERTY THIS PROGRAM DEPENDS ON, VERIFIED RATHER THAN
   ASSUMED.  Every wide (L) value printed below is a member of the extended execution
   character set, and C11 leaves the mapping of a universal-character-name to that set
   implementation-defined.  A conforming implementation could therefore map \u00A9 to
   something other than 0xA9, and if it did, comparing this program's wide values across
   backends would report a divergence that is NOT a compiler defect -- which is precisely
   how a conforming backend gets mislabelled.  The program does not assume the mapping: it
   requires it, and refuses to compile with a legible reason where it does not hold, so the
   precondition that makes cross-backend comparison sound is established at compile time
   instead of being discovered as a wrong value afterwards.

   The verification is done with _Static_assert on the values themselves, deliberately, and
   NOT by requiring __STDC_ISO_10646__.  That macro is the standard's mechanism for an
   implementation to DOCUMENT that wchar_t values are Unicode code points, and the reference
   toolchain defines it as 201706 on all four supported targets -- but it was measured that
   clang maps every universal-character-name below to its Unicode code point correctly while
   defining no such macro at all.  Gating on the advertisement rather than on the behaviour
   would therefore refuse a compiler that is doing exactly the right thing, and would refuse
   the compiler under test for a property the suite never needed it to announce.  The asserts
   below test precisely what the printed values depend on, so they accept any implementation
   whose mapping is right and reject any whose mapping is wrong, which is the only
   distinction that matters here. */

/* Wide literals: implementation-defined mapping, so every value used below is pinned. */
_Static_assert(L'A'      == 0x41,   "wide execution character set: 'A' must map to 0x41");
_Static_assert(L'\u0024' == 0x24,   "wide execution character set: U+0024 must map to 0x24");
_Static_assert(L'\u00A9' == 0xA9,   "wide execution character set: U+00A9 must map to 0xA9");
_Static_assert(L'\u0100' == 0x100,  "wide execution character set: U+0100 must map to 0x100");
_Static_assert(L'\u07FF' == 0x7FF,  "wide execution character set: U+07FF must map to 0x7FF");
_Static_assert(L'\u7FFF' == 0x7FFF, "wide execution character set: U+7FFF must map to 0x7FFF");
_Static_assert(L'z'      == 0x7A,   "wide execution character set: 'z' must map to 0x7A");

/* char16_t and char32_t literals are UTF-16 and UTF-32 by C11 6.4.5, so these values are
   SPECIFIED rather than implementation-defined.  They are pinned anyway, so that a backend
   which encodes them incorrectly is refused at compile time with a reason a reader can act
   on, instead of producing a wrong number that the oracles would have to attribute later. */
_Static_assert(u'\u00A9' == 0xA9,   "char16_t literals are UTF-16: U+00A9 must be 0xA9");
_Static_assert(u'\u7FFF' == 0x7FFF, "char16_t literals are UTF-16: U+7FFF must be 0x7FFF");
_Static_assert(U'\u00A9' == 0xA9,   "char32_t literals are UTF-32: U+00A9 must be 0xA9");
_Static_assert(U'\u7FFF' == 0x7FFF, "char32_t literals are UTF-32: U+7FFF must be 0x7FFF");

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
