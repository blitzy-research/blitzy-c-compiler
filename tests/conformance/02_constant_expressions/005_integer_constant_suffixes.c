/* Suffix-driven type selection at the boundary where a literal no longer fits
 * the default type.  This is the constant-expression area's probe for the
 * numeric literal parser and its `u`, `l`, `ll` and `f` suffixes
 * (docs/technical-specifications.md line 498), and for the type the resulting
 * constant then carries into expressions.
 *
 * The rule under test (C11 6.4.4.1p5): an integer constant has the FIRST type
 * in its list that can represent its value, and the list depends on how the
 * constant is spelled.  A decimal constant with no suffix walks
 * int -> long -> long long, every candidate SIGNED.  A hexadecimal or octal
 * constant with no suffix walks int -> unsigned int -> long -> unsigned long ->
 * long long -> unsigned long long, so it becomes UNSIGNED at a value where a
 * decimal constant of exactly the same magnitude never does.  A suffix deletes
 * the smaller candidates from the list: u/U keeps only unsigned types, l/L
 * starts at long, ll/LL starts at long long.
 *
 * Every claim is printed as OBSERVABLE BEHAVIOUR -- a _Generic selection naming a
 * TYPE, or the value an expression computes -- never as a printed width, and that
 * distinction is load-bearing here.  Because `long` is 4 bytes on i686 and 8 on the
 * other three targets, the unsuffixed decimal constant 4294967295 legitimately has
 * type `long` on the 64-bit targets and `long long` on i686, so printing a width --
 * or asking _Generic to distinguish those two types -- would diverge on i686 through
 * no fault of the compiler.  Asking instead whether the constant is unsigned, or is
 * one of the two wider SIGNED types, yields one answer everywhere while still
 * proving the typing rule was applied.
 *
 * Which spelling each printed field probes:
 *   u_suffix, suffix_upper_u                       1u    1U          unsigned int
 *   suffix_lower_l, suffix_upper_l                 1l    1L          long
 *   suffix_lower_ll, ll_suffix                     1ll   1LL         long long
 *   suffix_ul, suffix_upper_ul, suffix_lu          1ul   1UL   1lu   unsigned long
 *   suffix_ull, suffix_upper_ull, suffix_llu       1ull  1ULL  1LLU  unsigned long long
 *   f_suffix, float_upper_f                        1.0f  1.0F        float
 *   double_no_suffix                               1.0               double
 *   long_double_lower_l, long_double_upper_l       1.0l  1.0L        long double
 * The ill-formed mixed-case integer forms lL and Ll are deliberately absent.
 *
 * Two-variant rule: every arithmetic consequence is printed twice -- once folded
 * from constants (fold_*) and once recomputed at run time from volatile operands
 * holding the same literals (runtime_*).  _Generic selection is resolved entirely
 * at translation time and therefore has no runtime twin by construction, which is
 * correct rather than an omission.
 *
 * Freedom from undefined behaviour: the only wrapping arithmetic here is UNSIGNED,
 * which C11 6.2.5p9 defines as reduction modulo 2^N, so 0u - 1u, 0xFFFFFFFF + 1 and
 * 0ULL - 1ULL are fully defined rather than overflowing.  No signed expression ever
 * leaves its range -- the widest signed intermediate is 4294967296, formed only in
 * long long -- and both shift counts (16 and 32) are strictly below the width of
 * their 32- and 64-bit unsigned left operands.  Limits are spelled as literals
 * because no header is named: bcc ships no stdio.h, its bundled set being the nine
 * required freestanding headers plus a bonus stdatomic.h, ten files in all
 * (docs/project-guide.md, "9 bundled freestanding headers").
 */
int printf(const char *, ...);

/* Compile-time corroboration of the same rules the printed lines assert at run
 * time.  Every condition is target-invariant, which is essential: a
 * _Static_assert that failed on one target would break that target's build and
 * destroy the four-way comparison rather than reporting a divergence.  The
 * two-argument form is used throughout because the message-less form is C23 and
 * -pedantic rejects it. */
_Static_assert(_Generic(0xFFFFFFFF, unsigned int : 1, default : 0) == 1,
               "an unsuffixed hexadecimal constant above INT_MAX has type unsigned int");
_Static_assert(_Generic(4294967295, unsigned int : 1, default : 0) == 0,
               "an unsuffixed decimal constant is never given an unsigned type");
_Static_assert(_Generic(4294967295, long : 1, long long : 1, default : 0) == 1,
               "the decimal constant above INT_MAX takes the next wider signed type");
_Static_assert(_Generic(4294967296u, unsigned long : 1, unsigned long long : 1, default : 0) == 1,
               "a u-suffixed constant above UINT_MAX stays unsigned and widens");
_Static_assert(_Generic(0xFFFFFFFFFFFFFFFF, unsigned long : 1, unsigned long long : 1,
                        default : 0) == 1,
               "the hexadecimal list reaches a wider unsigned type at 2^64 - 1");
_Static_assert(_Generic(1.0L, long double : 1, default : 0) == 1,
               "the L suffix on a floating constant selects long double");
_Static_assert(0xFFFFFFFFu + 1u == 0u, "unsigned int arithmetic wraps modulo 2^32");
_Static_assert(0u - 1u == 4294967295u, "unsigned zero minus one is the unsigned int maximum");
_Static_assert(0ULL - 1ULL == 18446744073709551615ULL,
               "unsigned long long arithmetic wraps modulo 2^64");
_Static_assert(4294967295 + 1 == 4294967296LL,
               "the decimal constant's signed type is wide enough not to wrap");
_Static_assert(100000LL * 100000LL == 10000000000LL,
               "the LL suffix gives a product that does not fit in 32 bits");
_Static_assert(0177 == 127, "a leading zero makes the constant octal, so 0177 is 127");
_Static_assert(010 == 8, "010 is octal eight, not decimal ten");

int main(void)
{
    /* Runtime operands for the two-variant rule.  Each holds exactly the
     * literal its folded twin uses, and each is volatile so its value is read
     * at run time instead of substituted at translation time. */
    volatile unsigned int u_zero = 0u;
    volatile unsigned int u_max = 4294967295u;
    volatile unsigned long ul_max32 = 0xFFFFFFFFuL;
    volatile unsigned long long ull_zero = 0ULL;
    volatile unsigned long long ull_max = 18446744073709551615ULL;
    volatile long long ll_dec_max32 = 4294967295;
    volatile long long ll_factor = 100000LL;
    volatile int i_neg_one = -1;
    volatile long long ll_neg_one = -1;
    volatile int i_octal_0177 = 0177;
    volatile int i_octal_010 = 010;

    /* The boundary case, and the heart of this program: one magnitude spelled
     * two ways.  4294967295 exceeds INT_MAX, so both spellings must move beyond
     * int -- but only the hexadecimal list contains unsigned int, so only the
     * hexadecimal constant becomes unsigned.  The decimal constant lands on
     * long or long long according to the target's long width, which is why the
     * questions asked are "is it unsigned" and "is it one of the two wider
     * signed types" rather than which of the two it is. */
    printf("hex_ffffffff_is_uint=%d\n", _Generic(0xFFFFFFFF, unsigned int : 1, default : 0));
    printf("dec_4294967295_is_uint=%d\n", _Generic(4294967295, unsigned int : 1, default : 0));
    printf("dec_4294967295_is_wider_signed=%d\n",
           _Generic(4294967295, long : 1, long long : 1, default : 0));

    /* Boundaries that stay inside int and unsigned int on every target, so the
     * step from one to the other is observable without any width entering the
     * output: 2147483647 and 0x7FFFFFFF are the largest values int represents,
     * and one more takes the decimal constant to a wider signed type while
     * taking the hexadecimal constant to unsigned int. */
    printf("dec_2147483647_is_int=%d hex_7fffffff_is_int=%d\n",
           _Generic(2147483647, int : 1, default : 0),
           _Generic(0x7FFFFFFF, int : 1, default : 0));
    printf("dec_2147483648_is_uint=%d hex_80000000_is_uint=%d\n",
           _Generic(2147483648, unsigned int : 1, default : 0),
           _Generic(0x80000000, unsigned int : 1, default : 0));

    /* The u suffix restricts the list to unsigned types, so a u-suffixed
     * decimal constant becomes unsigned int where it fits and widens to a
     * wider unsigned type where it does not -- never to a signed type. */
    printf("u_dec_2147483648_is_uint=%d u_dec_4294967296_is_uint=%d"
           " u_dec_4294967296_is_wider_unsigned=%d\n",
           _Generic(2147483648u, unsigned int : 1, default : 0),
           _Generic(4294967296u, unsigned int : 1, default : 0),
           _Generic(4294967296u, unsigned long : 1, unsigned long long : 1, default : 0));
    printf("hex_max64_is_wider_unsigned=%d\n",
           _Generic(0xFFFFFFFFFFFFFFFF, unsigned long : 1, unsigned long long : 1, default : 0));

    /* Every suffix spelling the parser must accept, including the case-only
     * variants and the two orderings of a combined suffix.  Each association
     * names a type rather than a width, so each answer is identical on all four
     * targets even where the selected type's width is not. */
    printf("u_suffix=%d ll_suffix=%d f_suffix=%d\n",
           _Generic(1u, unsigned int : 1, default : 0),
           _Generic(1LL, long long : 1, default : 0),
           _Generic(1.0f, float : 1, default : 0));
    printf("suffix_upper_u=%d suffix_lower_l=%d suffix_upper_l=%d\n",
           _Generic(1U, unsigned int : 1, default : 0),
           _Generic(1l, long : 1, default : 0),
           _Generic(1L, long : 1, default : 0));
    printf("suffix_lower_ll=%d suffix_ul=%d suffix_upper_ul=%d suffix_lu=%d\n",
           _Generic(1ll, long long : 1, default : 0),
           _Generic(1ul, unsigned long : 1, default : 0),
           _Generic(1UL, unsigned long : 1, default : 0),
           _Generic(1lu, unsigned long : 1, default : 0));
    printf("suffix_ull=%d suffix_upper_ull=%d suffix_llu=%d\n",
           _Generic(1ull, unsigned long long : 1, default : 0),
           _Generic(1ULL, unsigned long long : 1, default : 0),
           _Generic(1LLU, unsigned long long : 1, default : 0));
    printf("float_upper_f=%d double_no_suffix=%d long_double_lower_l=%d"
           " long_double_upper_l=%d\n",
           _Generic(1.0F, float : 1, default : 0),
           _Generic(1.0, double : 1, default : 0),
           _Generic(1.0l, long double : 1, default : 0),
           _Generic(1.0L, long double : 1, default : 0));

    /* Small octal and decimal constants share the same first candidate, so the
     * base affects the value rather than the type. */
    printf("octal_0177_is_int=%d octal_010_is_int=%d dec_1_is_int=%d\n",
           _Generic(0177, int : 1, default : 0),
           _Generic(010, int : 1, default : 0),
           _Generic(1, int : 1, default : 0));

    /* Folded consequences: the values the constants' own types produce. */
    printf("fold_uwrap=%d fold_ullwrap=%d\n",
           (int)((0u - 1u) == 4294967295u),
           (int)((0ULL - 1ULL) == 18446744073709551615ULL));

    /* The typing rule made visible in arithmetic rather than in a type probe.
     * 0xFFFFFFFF is unsigned int, so adding one reduces modulo 2^32 and gives
     * zero; the same magnitude written as 4294967295 is a wider signed type, so
     * adding one gives 4294967296 and the comparison against zero is false.
     * One expression wraps and the other does not, entirely because of how the
     * constant was spelled. */
    printf("fold_hex_max_plus_one_is_zero=%d fold_dec_max_plus_one_is_zero=%d\n",
           (int)((0xFFFFFFFF + 1) == 0u),
           (int)((4294967295 + 1) == 0));

    /* A signedness-sensitive comparison, which is the sharpest observable
     * consequence of the typing rule: -1 is below the decimal 2147483648
     * because that constant is signed, and NOT below the hexadecimal
     * 0x80000000 because that constant is unsigned int, so the usual arithmetic
     * conversions turn -1 into 4294967295.  The cast to unsigned int performs
     * exactly the conversion the comparison would perform implicitly -- both
     * forms were measured to print 0 -- and is written explicitly because
     * -Wsign-compare (enabled by -Wextra, fatal under -Werror) refuses the
     * implicit mixing, and this area sanctions no gate deviation. */
    printf("fold_neg_below_hex_boundary=%d fold_neg_below_dec_boundary=%d\n",
           (int)((unsigned int)(-1) < 0x80000000),
           (int)(-1 < 2147483648));
    printf("fold_ll_product=%lld\n", 100000LL * 100000LL);
    printf("fold_ul_shift=%d fold_ull_shift=%d\n",
           (int)((0xFFFFFFFFuL >> 16) == 65535uL),
           (int)((0xFFFFFFFFFFFFFFFFULL >> 32) == 4294967295ULL));
    printf("fold_octal_0177_is_127=%d fold_octal_010_is_8=%d\n",
           (int)(0177 == 127),
           (int)(010 == 8));

    /* Division is where the u suffix is most visible: 0u - 1u is the unsigned
     * int maximum, so dividing it by three gives 1431655765 rather than the
     * zero that signed -1 / 3 would give. */
    printf("fold_udiv=%d\n", (int)(((0u - 1u) / 3u) == 1431655765u));

    /* Runtime twins, in the same order and computing the same values from the
     * volatile operands declared above. */
    printf("runtime_uwrap=%d runtime_ullwrap=%d\n",
           (int)((u_zero - 1u) == 4294967295u),
           (int)((ull_zero - 1ULL) == 18446744073709551615ULL));
    printf("runtime_hex_max_plus_one_is_zero=%d runtime_dec_max_plus_one_is_zero=%d\n",
           (int)((u_max + 1u) == 0u),
           (int)((ll_dec_max32 + 1) == 0));
    printf("runtime_neg_below_hex_boundary=%d runtime_neg_below_dec_boundary=%d\n",
           (int)((unsigned int)i_neg_one < 0x80000000),
           (int)(ll_neg_one < 2147483648));
    printf("runtime_ll_product=%lld\n", ll_factor * ll_factor);
    printf("runtime_ul_shift=%d runtime_ull_shift=%d\n",
           (int)((ul_max32 >> 16) == 65535uL),
           (int)((ull_max >> 32) == 4294967295ULL));
    printf("runtime_octal_0177_is_127=%d runtime_octal_010_is_8=%d\n",
           (int)(i_octal_0177 == 127),
           (int)(i_octal_010 == 8));
    printf("runtime_udiv=%d\n", (int)(((u_zero - 1u) / 3u) == 1431655765u));
    return 0;
}
