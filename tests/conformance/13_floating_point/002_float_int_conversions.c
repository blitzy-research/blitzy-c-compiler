/* Conversions in both directions between the floating types and the integer
 * types, at REPRESENTABLE boundaries, folded and again at run time.
 *
 * Area 13 program 2 of 4.  Area 02's 008_float_constant_folding.c defers
 * "general float <-> int conversion boundaries" here by name, so this program
 * owns them and no other program duplicates them: double -> int, double ->
 * long long, double -> unsigned long long, double -> unsigned int, double ->
 * short / unsigned short / signed char / unsigned char, the same integer
 * destinations again from float, the reverse direction out of int, unsigned
 * int, long long, unsigned long long, signed char and unsigned char into both
 * float and double, and four round trips that go out to a floating type and
 * back.  Through the runtime half of every line it reaches the four backends'
 * conversion instruction families -- SSE cvttsd2si / cvtsi2sd on x86-64, x87
 * together with SSE on i686, fcvtzs / scvtf on AArch64 and fcvt.w.d / fcvt.d.w
 * on RISC-V 64 -- which is where a conversion defect would live.
 *
 * THE RANGE DISCIPLINE, which is the whole reason this program is shaped the
 * way it is.  Converting a floating value to an integer type is UNDEFINED
 * unless the value TRUNCATED toward zero is representable in the destination
 * (C11 6.3.1.4p1).  Undefined behaviour here would not merely weaken the
 * differential oracle, it would destroy it: with undefined behaviour present,
 * both compilers are permitted to do anything, so a divergence between them
 * would be evidence about neither.  Every floating source below is therefore
 * chosen so that its truncated value sits STRICTLY INSIDE the destination
 * range with room to spare, never at the edge.  Every destination width is the
 * same on all four supported targets -- int and unsigned int 32 bits, short
 * and unsigned short 16, signed char and unsigned char 8, long long and
 * unsigned long long 64 -- so one range table holds everywhere:
 *
 *     int          -2147483648 .. 2147483647   unsigned int         0 .. 4294967295
 *     short             -32768 .. 32767        unsigned short       0 .. 65535
 *     signed char         -128 .. 127          unsigned char        0 .. 255
 *     long long        about -9.22e18 .. 9.22e18
 *     unsigned long long             0 .. about 1.84e19
 *
 * and every floating source, with the value it truncates to and the distance
 * from that value to the nearer bound of its destination, is:
 *
 *     source            format  destination        truncates to  margin
 *     1000000.5         double  int                     1000000  vast
 *     -1000000.5        double  int                    -1000000  vast
 *     0.75              double  int                           0  vast
 *     -0.75             double  int                           0  vast
 *     2147480000.0      double  int                  2147480000  3647 below the maximum
 *     -2147480000.0     double  int                 -2147480000  3648 above the minimum
 *     1234567890123.5   double  long long         1234567890123  vast
 *     4000000000000.75  double  unsigned long long 4000000000000 vast
 *     4000000000.0      double  unsigned int         4000000000  294967295 below the maximum
 *     32000.5           double  short                     32000  767 below the maximum
 *     65000.5           double  unsigned short            65000  535 below the maximum
 *     100.75            double  signed char                 100  27 below the maximum
 *     200.25            double  unsigned char               200  55 below the maximum
 *     16777216.0f       float   int                    16777216  vast
 *     3.75f             float   int                           3  vast
 *     1000000.0f        float   unsigned int            1000000  vast
 *     -32000.0f         float   short                    -32000  768 above the minimum
 *     250.75f           float   unsigned char               250  5 below the maximum
 *     -0.75             double  unsigned int                  0  exactly at the minimum
 *     -0.5f             float   unsigned char                 0  exactly at the minimum
 *     -0.25             double  unsigned long long            0  exactly at the minimum
 *
 * The last three rows are the deliberate negative-to-unsigned cases.  They are
 * defined, not borderline-undefined: 6.3.1.4p1 turns on the representability of
 * the INTEGRAL PART, and a value strictly between -1 and 0 has integral part 0,
 * which every unsigned type represents.  See the asymmetry note below.
 *
 * The four round trips add four more conversions in this direction, on their
 * inward leg, and each is inside its destination by an equally wide margin:
 * 12345.0 as a double to int, 1024.0 as a float to int, 1234567890123.0 as a
 * double to long long, and 4000000000.0 as a double to unsigned int, the last
 * of which is the same conversion the table's ninth row already records.  The
 * narrowest margin anywhere in the program is therefore the 5 that separates
 * 250 from unsigned char's maximum, and the narrowest on a signed destination
 * is the 27 that separates 100 from signed char's.
 *
 * TRUNCATION IS TOWARD ZERO, not toward negative infinity, and the negative
 * cases are here to pin that down: -1000000.5 gives -1000000 rather than
 * -1000001, and -0.75 gives 0 rather than -1.  A backend that rounded instead
 * of truncating, or that truncated the wrong way, changes those two fields and
 * nothing else, which is what localizes the defect.
 *
 * THE ASYMMETRY THAT IS EASIEST TO GET WRONG.  An out-of-range conversion
 * between INTEGER types is well defined for an unsigned destination -- the
 * value is reduced modulo one plus the destination maximum (C11 6.3.1.3p2) --
 * but that modular rule does NOT extend to a FLOATING source.  C11 6.3.1.4p1
 * states the floating-to-integer rule in two steps, and both steps matter: the
 * fractional part is discarded first, and the behaviour is undefined only if
 * THE VALUE OF THE INTEGRAL PART, the value that survives that truncation,
 * cannot be represented by the destination type.  Nothing is reduced modulo
 * anything.
 *
 * Reading that rule precisely is what keeps this program's own analysis honest,
 * because the two steps do not draw the line where a hasty reading would put
 * it.  A negative source is NOT automatically undefined against an unsigned
 * destination: any value strictly between -1 and 0 truncates to a zero that
 * every unsigned type represents, so the conversion is defined and yields
 * zero.  What is undefined is a negative value of magnitude one or more,
 * whose integral part is negative and therefore outside every unsigned range;
 * and, symmetrically, a positive value whose integral part exceeds the
 * destination maximum, which is undefined for a signed and an unsigned
 * destination alike.  The dangerous region is "integral part out of range",
 * not "source negative".
 *
 * This program therefore exercises that narrow defined case ON PURPOSE rather
 * than steering around it.  THREE NEGATIVE SOURCES ARE CONVERTED TO UNSIGNED
 * DESTINATIONS -- -0.75 to unsigned int, -0.5f to unsigned char and -0.25 to
 * unsigned long long -- and every one of them lies strictly between -1 and 0, so
 * every integral part is 0 and every conversion is fully defined and must yield
 * 0.  Each is written twice, once as a constant expression and once through a
 * volatile operand, so the constant folder and the backend are held to the same
 * answer.  The case earns its place because it is where an implementation is
 * likeliest to substitute a modular reinterpretation or a saturating conversion
 * for truncation toward zero, and either of those prints something other than 0.
 * What is absent is the UNDEFINED region on both sides: no negative source of
 * magnitude one or more reaches an unsigned destination, and no source of either
 * sign has an integral part outside its destination's range.  Every other source
 * feeding unsigned int, unsigned short, unsigned char or unsigned long long is
 * non-negative, and every integral part converted anywhere sits inside its
 * destination by the margins tabulated earlier.
 *
 * INTEGER -> FLOATING is the safer direction but has one hazard of its own: an
 * integer not exactly representable in the destination floating type is
 * converted with implementation-defined rounding, which the standard permits
 * to pick either adjacent value, so two targets could legitimately disagree.
 * Every integer converted below is therefore EXACTLY representable in the
 * format it is converted to and no rounding decision is ever taken.  binary32
 * carries 24 significand bits, so integers up to 2^24 = 16777216 are exact;
 * binary64 carries 53, so integers up to 2^53 = 9007199254740992 are exact.
 * 12345, 1024, 16777216, 250 and -125 are exact in both formats; 4000000000 is
 * exact in both as well (it is 2^11 * 5^9, needing 21 significand bits);
 * 9007199254740992 is exactly 2^53, 4000000000000 is 2^14 * 5^12 and so needs
 * only the 28 significand bits its odd part 5^12 = 244140625 occupies, and
 * 1234567890123 is odd and needs 41, so all three are exact in binary64.  The
 * count that matters for exactness is the width of the ODD PART, not the width of
 * the whole integer: trailing factors of two are absorbed by the exponent and
 * cost no significand at all.  Neither 4000000000000 nor 1234567890123 fits
 * binary32's 24 bits, and precisely for that reason neither is ever converted to
 * float, only to double.
 *
 * EVERY FLOATING VALUE HERE IS A DYADIC RATIONAL -- an integer, or an integer
 * plus 1/4, 1/2 or 3/4 -- so it is exactly representable in binary32 and
 * binary64 alike, and every printed floating value is printed with an explicit
 * %.6f whose digits are exact with room to spare.  That is what makes the
 * twelve cells of this program's matrix comparable byte for byte even though
 * i686 evaluates floating expressions in x87 extended precision
 * (FLT_EVAL_METHOD 2) while the other three targets do not: excess precision
 * cannot perturb a value that is already exact in every format involved.  No
 * value whose decimal expansion fails to terminate well within six fractional
 * digits appears, no division appears at all, and no NaN, infinity, negative
 * zero or division by zero is ever produced -- converting a NaN or an infinity
 * to an integer type would be undefined, so none is brought into existence.
 *
 * TYPE WIDTHS ARE NORMALIZED.  sizeof(long) and sizeof(void *) were measured
 * as 4 on i686 and 8 on the other three targets, so long, unsigned long,
 * size_t, ptrdiff_t and intptr_t are never named: a destination whose range
 * differed per target could not carry one golden record.  Only int, unsigned
 * int, short, unsigned short, signed char, unsigned char, long long and
 * unsigned long long appear, every one of them the same width on all four
 * targets.  The widest floating type is likewise absent, its representation
 * having been measured to differ across the targets; it belongs to
 * 004_long_double_target_restricted.c, which handles it with a recorded
 * oracle-(b) exclusion and no marker -- that program's record disables the
 * cross-backend arm alone and carries the measurement as its recorded reason,
 * which is the mechanism EXPECTED_DIVERGENCES.md section 4.2 provisions for it.
 * Plain char never appears either, its signedness having been measured as signed on
 * x86-64 and i686 and unsigned on AArch64 and RISC-V 64, so the two character
 * conversions here name signed char and unsigned char explicitly.  No address
 * or pointer value is printed.
 *
 * THE TWO-VARIANT RULE.  Every conversion is performed twice: once over
 * constants, which the folder evaluates at translation time, and once over
 * volatile-qualified operands, which must be re-read from memory at run time
 * and so cannot be folded away.  Without the runtime half the backend would
 * never be asked to convert anything and a conversion defect would escape
 * entirely, the suite reporting a pass on the strength of the folder's answer
 * alone.  The two halves are kept in step structurally: the folded section and
 * the runtime section perform the same conversions in the same order under
 * mirrored names, fold_X against rt_X, printed on mirrored lines, and the
 * final folded_matches_runtime line asserts that each mirrored pair of lines
 * agrees field by field.  That line is a summary on top of the per-property
 * lines rather than a substitute for them, since every field is also printed
 * individually above it.  Each conversion is written out explicitly rather
 * than hidden behind a macro, deliberately, so that the range argument for a
 * site sits beside the cast that performs it.
 *
 * EVERY CONVERSION SITE CARRIES AN EXPLICIT CAST.  Area 13 sanctions no
 * deviation from the audit gate, so -Wconversion and -Wsign-conversion remain
 * in force under -Werror and an implicit narrowing or sign change would fail
 * the gate rather than being tested.  Every float widened at a variadic call
 * carries an explicit (double), stating the default argument promotion printf
 * would otherwise perform silently, and every value of a type narrower than
 * int is widened with an explicit (int) before it is printed, so no length
 * modifier is needed and the promotion is visible at the call.
 *
 * FREEDOM FROM UNDEFINED BEHAVIOUR.  The one undefined case specific to this
 * program -- a floating value whose truncation escapes its destination -- is
 * foreclosed by the range table above, site by site, and the sanitizer gate's
 * float-cast-overflow check would fire on any lapse.  There is no signed
 * overflow: no arithmetic is performed on any integer at all, only conversion.
 * There is no shift, so no shift count can leave its range.  Nothing is
 * type-punned through a pointer or a union, so no aliasing rule is violated
 * and nothing depends on a representation or on padding bytes.  Every object,
 * the volatile ones among them, is initialized at its declaration, so no
 * uninitialized storage is read.  No object is modified twice between sequence
 * points -- nothing is assigned after its initializer -- and no argument to
 * any call has a side effect, every value being computed into its own named
 * variable first.  Each volatile object is read exactly once, into a plain
 * local of its own type, before any conversion is applied to it, so no printed
 * value can depend on the order in which two reads happen.  No address of any
 * object is taken, so nothing depends on the relative addresses of unrelated
 * objects.
 *
 * NO HEADER IS NAMED, and no preprocessor directive appears anywhere below.
 * bcc ships no stdio.h -- its bundled set is the nine required freestanding
 * headers plus a bonus stdatomic.h, ten files in all (docs/project-guide.md
 * line 212) -- so naming it here would fail against bcc while succeeding
 * against the reference compiler, a divergence caused by the test rather than
 * by the compiler.  float.h and limits.h are bundled and are still not named:
 * every boundary value above is written out as a literal in this file, and the
 * bundled set is exercised by 12_preprocessor/003_bundled_header_inclusion.c,
 * whose subject that is.  The single libc prototype this program needs is
 * hand-declared below.
 *
 * DELIBERATELY ELSEWHERE, so this program does not duplicate it: floating
 * arithmetic and folding belong to 001_float_double_arithmetic.c and
 * 02_constant_expressions/008_float_constant_folding.c; finite ordering and
 * comparison breadth to 003_finite_comparisons.c; the widest floating type to
 * 004_long_double_target_restricted.c; _Bool conversions to
 * 01_integer_conversions/005_bool_conversions.c; and narrowing between integer
 * types to 01_integer_conversions/004_narrowing_conversions.c.  This program
 * converts, and does nothing else.
 */

int printf(const char *, ...);

int main(void)
{
    /* ----------------------------------------------------------------------
     * Folded variant.  Every operand below is a constant, so each initializer
     * is a constant expression the folder evaluates at translation time.
     * ---------------------------------------------------------------------- */

    /* double -> int, with a fractional part in both signs so that truncation
       toward zero is pinned down rather than assumed, and with two values
       whose truncation is zero from either side. */
    int fold_d2i_pos = (int)1000000.5;                  /* -> 1000000        */
    int fold_d2i_neg = (int)(-1000000.5);               /* -> -1000000       */
    int fold_d2i_trunc_pos = (int)0.75;                 /* -> 0              */
    int fold_d2i_trunc_neg = (int)(-0.75);              /* -> 0, not -1      */

    /* double -> int near, but strictly inside, int's bounds: 3647 to spare
       below the maximum and 3648 above the minimum.  A boundary test with
       margin, never a test at the edge, because a source whose truncation
       reached the edge exactly would leave no room for a rounding surprise
       and an overshoot there would be undefined rather than wrong. */
    int fold_d2i_bound_pos = (int)2147480000.0;         /* -> 2147480000     */
    int fold_d2i_bound_neg = (int)(-2147480000.0);      /* -> -2147480000    */

    /* double -> the 64-bit integer types.  Both truncated values sit far
       inside their destinations, and both sources are exact in binary64:
       1234567890123.5 needs 42 significand bits, 4000000000000.75 needs 44,
       against the 53 available.  The unsigned source is non-negative. */
    long long fold_d2ll = (long long)1234567890123.5;   /* -> 1234567890123  */
    unsigned long long fold_d2ull =
        (unsigned long long)4000000000000.75;           /* -> 4000000000000  */

    /* double -> unsigned int and the narrow integer types.  Every source here
       is non-negative, which the asymmetry note in the header explains, and
       each truncated value clears its destination maximum by the margin
       recorded there. */
    unsigned int fold_d2u = (unsigned int)4000000000.0; /* -> 4000000000     */
    short fold_d2short = (short)32000.5;                /* -> 32000          */
    unsigned short fold_d2ushort = (unsigned short)65000.5;  /* -> 65000     */
    signed char fold_d2schar = (signed char)100.75;     /* -> 100            */
    unsigned char fold_d2uchar = (unsigned char)200.25; /* -> 200            */

    /* NEGATIVE floating source -> unsigned destination, in the one range where
       C11 6.3.1.4p1 defines it: strictly between -1 and 0, whose integral part
       is zero and is representable everywhere.  Three destination widths, so a
       defect confined to one width cannot hide behind the others.  A saturating
       conversion would print the destination maximum, the modular integer rule
       applied by mistake would print it too, and rounding away from zero would
       print 1; only truncation toward zero prints 0. */
    unsigned int fold_negfrac2u = (unsigned int)(-0.75);          /* -> 0     */
    unsigned char fold_negfrac2uchar = (unsigned char)(-0.5f);    /* -> 0     */
    unsigned long long fold_negfrac2ull =
        (unsigned long long)(-0.25);                              /* -> 0     */

    /* float -> integer.  Every source is exact in binary32: 16777216 is
       exactly 2^24, the largest integer for which every integer below it is
       representable, and 3.75, 1000000, -32000 and 250.75 all need far fewer
       than 24 significand bits.  The two unsigned destinations are fed
       non-negative sources. */
    int fold_f2i = (int)16777216.0f;                    /* -> 16777216       */
    int fold_f2i_trunc = (int)3.75f;                    /* -> 3              */
    unsigned int fold_f2u = (unsigned int)1000000.0f;   /* -> 1000000        */
    short fold_f2short = (short)(-32000.0f);            /* -> -32000         */
    unsigned char fold_f2uchar = (unsigned char)250.75f;    /* -> 250        */

    /* integer -> floating, out of the four integer widths.  Every source is
       exactly representable in its destination format, so no rounding
       decision is taken and no implementation-defined choice arises. */
    float fold_i2f = (float)12345;                      /* exact in binary32 */
    double fold_i2d = (double)(-12345);                 /* exact in binary64 */
    double fold_u2d = (double)4000000000u;              /* 21 bits, exact    */
    double fold_ll2d = (double)9007199254740992LL;      /* exactly 2^53      */

    /* integer -> floating again, from a power of two at binary32's exactness
       limit and from the two explicitly signed character types, whose values
       are well inside both formats. */
    float fold_i2f_pow2 = (float)16777216;              /* exactly 2^24      */
    double fold_sc2d = (double)(signed char)(-125);     /* exact in binary64 */
    float fold_uc2f = (float)(unsigned char)250;        /* exact in binary32 */
    double fold_ull2d = (double)4000000000000ULL;       /* 28 bits, exact    */

    /* Round trips: out to a floating type and back to the integer type it
       came from.  Each source is exact in the floating type it passes
       through, so the original value must return unchanged, and the recovered
       value is printed rather than a boolean so a wrong answer is visible
       rather than merely reported as unequal. */
    int fold_roundtrip_i_d_i = (int)(double)12345;      /* -> 12345          */
    int fold_roundtrip_i_f_i = (int)(float)1024;        /* -> 1024           */
    long long fold_roundtrip_ll_d_ll =
        (long long)(double)1234567890123LL;             /* -> 1234567890123  */
    unsigned int fold_roundtrip_u_d_u =
        (unsigned int)(double)4000000000u;              /* -> 4000000000     */

    /* ----------------------------------------------------------------------
     * Runtime variant.  The same conversions in the same order, over
     * volatile-qualified operands that must be re-read from memory, so the
     * backend has to emit a genuine conversion instruction for each one
     * instead of the folder substituting an answer.  Every volatile object is
     * initialized at its declaration and is read exactly once, into a plain
     * local of its own type, before any conversion touches it.
     * ---------------------------------------------------------------------- */

    /* Runtime sources for double -> int.  Same four values as the folded
       group, so the printed pairs must agree field for field. */
    volatile double vol_d2i_pos = 1000000.5;
    volatile double vol_d2i_neg = -1000000.5;
    volatile double vol_d2i_trunc_pos = 0.75;
    volatile double vol_d2i_trunc_neg = -0.75;
    double snap_d2i_pos = vol_d2i_pos;
    double snap_d2i_neg = vol_d2i_neg;
    double snap_d2i_trunc_pos = vol_d2i_trunc_pos;
    double snap_d2i_trunc_neg = vol_d2i_trunc_neg;
    int rt_d2i_pos = (int)snap_d2i_pos;                 /* -> 1000000        */
    int rt_d2i_neg = (int)snap_d2i_neg;                 /* -> -1000000       */
    int rt_d2i_trunc_pos = (int)snap_d2i_trunc_pos;     /* -> 0              */
    int rt_d2i_trunc_neg = (int)snap_d2i_trunc_neg;     /* -> 0, not -1      */

    /* Runtime sources for the in-margin int bounds and the 64-bit integer
       destinations. */
    volatile double vol_d2i_bound_pos = 2147480000.0;
    volatile double vol_d2i_bound_neg = -2147480000.0;
    volatile double vol_d2ll = 1234567890123.5;
    volatile double vol_d2ull = 4000000000000.75;
    double snap_d2i_bound_pos = vol_d2i_bound_pos;
    double snap_d2i_bound_neg = vol_d2i_bound_neg;
    double snap_d2ll = vol_d2ll;
    double snap_d2ull = vol_d2ull;
    int rt_d2i_bound_pos = (int)snap_d2i_bound_pos;     /* -> 2147480000     */
    int rt_d2i_bound_neg = (int)snap_d2i_bound_neg;     /* -> -2147480000    */
    long long rt_d2ll = (long long)snap_d2ll;           /* -> 1234567890123  */
    unsigned long long rt_d2ull =
        (unsigned long long)snap_d2ull;                 /* -> 4000000000000  */

    /* Runtime sources for double -> unsigned int and the narrow integer
       types.  Every one is non-negative, exactly as in the folded group. */
    volatile double vol_d2u = 4000000000.0;
    volatile double vol_d2short = 32000.5;
    volatile double vol_d2ushort = 65000.5;
    volatile double vol_d2schar = 100.75;
    volatile double vol_d2uchar = 200.25;
    double snap_d2u = vol_d2u;
    double snap_d2short = vol_d2short;
    double snap_d2ushort = vol_d2ushort;
    double snap_d2schar = vol_d2schar;
    double snap_d2uchar = vol_d2uchar;
    unsigned int rt_d2u = (unsigned int)snap_d2u;       /* -> 4000000000     */
    short rt_d2short = (short)snap_d2short;             /* -> 32000          */
    unsigned short rt_d2ushort = (unsigned short)snap_d2ushort;  /* -> 65000 */
    signed char rt_d2schar = (signed char)snap_d2schar; /* -> 100            */
    unsigned char rt_d2uchar = (unsigned char)snap_d2uchar;      /* -> 200   */

    /* Runtime sources for the three negative-fraction conversions.  The folder
       cannot answer these, so the backend has to emit the genuine
       floating-to-unsigned conversion for a negative operand -- which is the
       instruction sequence the folded group above can never reach, and the one
       where a saturating or wrongly-rounded lowering actually lives.  The float
       source is held in float so the single-precision form is selected too. */
    volatile double vol_negfrac2u = -0.75;
    volatile float vol_negfrac2uchar = -0.5f;
    volatile double vol_negfrac2ull = -0.25;
    double snap_negfrac2u = vol_negfrac2u;
    float snap_negfrac2uchar = vol_negfrac2uchar;
    double snap_negfrac2ull = vol_negfrac2ull;
    unsigned int rt_negfrac2u = (unsigned int)snap_negfrac2u;     /* -> 0     */
    unsigned char rt_negfrac2uchar =
        (unsigned char)snap_negfrac2uchar;                        /* -> 0     */
    unsigned long long rt_negfrac2ull =
        (unsigned long long)snap_negfrac2ull;                     /* -> 0     */

    /* Runtime sources for float -> integer.  Held in float rather than
       double, so the backend must select the single-precision conversion
       instruction rather than the double-precision one. */
    volatile float vol_f2i = 16777216.0f;
    volatile float vol_f2i_trunc = 3.75f;
    volatile float vol_f2u = 1000000.0f;
    volatile float vol_f2short = -32000.0f;
    volatile float vol_f2uchar = 250.75f;
    float snap_f2i = vol_f2i;
    float snap_f2i_trunc = vol_f2i_trunc;
    float snap_f2u = vol_f2u;
    float snap_f2short = vol_f2short;
    float snap_f2uchar = vol_f2uchar;
    int rt_f2i = (int)snap_f2i;                         /* -> 16777216       */
    int rt_f2i_trunc = (int)snap_f2i_trunc;             /* -> 3              */
    unsigned int rt_f2u = (unsigned int)snap_f2u;       /* -> 1000000        */
    short rt_f2short = (short)snap_f2short;             /* -> -32000         */
    unsigned char rt_f2uchar = (unsigned char)snap_f2uchar;      /* -> 250   */

    /* Runtime sources for integer -> floating, out of the four integer
       widths.  Each value is exactly representable in its destination format,
       so the conversion instruction has no rounding decision to take. */
    volatile int vol_i2f = 12345;
    volatile int vol_i2d = -12345;
    volatile unsigned int vol_u2d = 4000000000u;
    volatile long long vol_ll2d = 9007199254740992LL;
    int snap_i2f = vol_i2f;
    int snap_i2d = vol_i2d;
    unsigned int snap_u2d = vol_u2d;
    long long snap_ll2d = vol_ll2d;
    float rt_i2f = (float)snap_i2f;                     /* -> 12345.0        */
    double rt_i2d = (double)snap_i2d;                   /* -> -12345.0       */
    double rt_u2d = (double)snap_u2d;                   /* -> 4000000000.0   */
    double rt_ll2d = (double)snap_ll2d;                 /* -> 2^53           */

    /* Runtime sources for the power of two at binary32's exactness limit, the
       two explicitly signed character types, and the 64-bit unsigned width. */
    volatile int vol_i2f_pow2 = 16777216;
    volatile signed char vol_sc2d = (signed char)(-125);
    volatile unsigned char vol_uc2f = (unsigned char)250;
    volatile unsigned long long vol_ull2d = 4000000000000ULL;
    int snap_i2f_pow2 = vol_i2f_pow2;
    signed char snap_sc2d = vol_sc2d;
    unsigned char snap_uc2f = vol_uc2f;
    unsigned long long snap_ull2d = vol_ull2d;
    float rt_i2f_pow2 = (float)snap_i2f_pow2;           /* -> 2^24           */
    double rt_sc2d = (double)snap_sc2d;                 /* -> -125.0         */
    float rt_uc2f = (float)snap_uc2f;                   /* -> 250.0          */
    double rt_ull2d = (double)snap_ull2d;           /* -> 4000000000000.0    */

    /* Runtime sources for the four round trips.  Each goes out to a floating
       type in which it is exact and comes back to the integer type it started
       in, so the recovered value must equal the source. */
    volatile int vol_roundtrip_i_d_i = 12345;
    volatile int vol_roundtrip_i_f_i = 1024;
    volatile long long vol_roundtrip_ll_d_ll = 1234567890123LL;
    volatile unsigned int vol_roundtrip_u_d_u = 4000000000u;
    int snap_roundtrip_i_d_i = vol_roundtrip_i_d_i;
    int snap_roundtrip_i_f_i = vol_roundtrip_i_f_i;
    long long snap_roundtrip_ll_d_ll = vol_roundtrip_ll_d_ll;
    unsigned int snap_roundtrip_u_d_u = vol_roundtrip_u_d_u;
    int rt_roundtrip_i_d_i =
        (int)(double)snap_roundtrip_i_d_i;               /* -> 12345         */
    int rt_roundtrip_i_f_i =
        (int)(float)snap_roundtrip_i_f_i;                /* -> 1024          */
    long long rt_roundtrip_ll_d_ll =
        (long long)(double)snap_roundtrip_ll_d_ll;       /* -> 1234567890123 */
    unsigned int rt_roundtrip_u_d_u =
        (unsigned int)(double)snap_roundtrip_u_d_u;      /* -> 4000000000    */

    /* ----------------------------------------------------------------------
     * The folded and runtime spellings of every conversion must agree.  One
     * flag per mirrored pair of printed lines, each the conjunction of that
     * line's field comparisons, in the order the lines are printed.  Every
     * field is also printed individually above, so these eight flags are a
     * summary on top of the per-property lines rather than a substitute for
     * them: a divergence shows up on its own field first and here second.
     * Each comparison is between two values of the same type, so no usual
     * arithmetic conversion is involved and no comparison mixes signedness.
     * ---------------------------------------------------------------------- */
    int agree_d2i = (fold_d2i_pos == rt_d2i_pos)
        && (fold_d2i_neg == rt_d2i_neg)
        && (fold_d2i_trunc_pos == rt_d2i_trunc_pos)
        && (fold_d2i_trunc_neg == rt_d2i_trunc_neg);
    int agree_wide = (fold_d2i_bound_pos == rt_d2i_bound_pos)
        && (fold_d2i_bound_neg == rt_d2i_bound_neg)
        && (fold_d2ll == rt_d2ll)
        && (fold_d2ull == rt_d2ull);
    int agree_d2narrow = (fold_d2u == rt_d2u)
        && (fold_d2short == rt_d2short)
        && (fold_d2ushort == rt_d2ushort)
        && (fold_d2schar == rt_d2schar)
        && (fold_d2uchar == rt_d2uchar);
    int agree_negfrac = (fold_negfrac2u == rt_negfrac2u)
        && (fold_negfrac2uchar == rt_negfrac2uchar)
        && (fold_negfrac2ull == rt_negfrac2ull);
    int agree_f2int = (fold_f2i == rt_f2i)
        && (fold_f2i_trunc == rt_f2i_trunc)
        && (fold_f2u == rt_f2u)
        && (fold_f2short == rt_f2short)
        && (fold_f2uchar == rt_f2uchar);
    int agree_int2fp = (fold_i2f == rt_i2f)
        && (fold_i2d == rt_i2d)
        && (fold_u2d == rt_u2d)
        && (fold_ll2d == rt_ll2d);
    int agree_int2fp_wide = (fold_i2f_pow2 == rt_i2f_pow2)
        && (fold_sc2d == rt_sc2d)
        && (fold_uc2f == rt_uc2f)
        && (fold_ull2d == rt_ull2d);
    int agree_roundtrip = (fold_roundtrip_i_d_i == rt_roundtrip_i_d_i)
        && (fold_roundtrip_i_f_i == rt_roundtrip_i_f_i)
        && (fold_roundtrip_ll_d_ll == rt_roundtrip_ll_d_ll)
        && (fold_roundtrip_u_d_u == rt_roundtrip_u_d_u);

    /* ----------------------------------------------------------------------
     * Output.  Eight folded lines, then their eight runtime mirrors in the
     * same order and with the same field order, then the agreement summary.
     * Values narrower than int are widened with an explicit (int) and floats
     * with an explicit (double), so every default argument promotion is
     * stated at the call rather than left implicit.
     * ---------------------------------------------------------------------- */

    printf("d2i_pos=%d d2i_neg=%d d2i_trunc_pos=%d d2i_trunc_neg=%d\n",
           fold_d2i_pos, fold_d2i_neg, fold_d2i_trunc_pos,
           fold_d2i_trunc_neg);
    printf("d2i_bound_pos=%d d2i_bound_neg=%d d2ll=%lld d2ull=%llu\n",
           fold_d2i_bound_pos, fold_d2i_bound_neg, fold_d2ll, fold_d2ull);
    printf("d2u=%u d2short=%d d2ushort=%d d2schar=%d d2uchar=%d\n",
           fold_d2u, (int)fold_d2short, (int)fold_d2ushort,
           (int)fold_d2schar, (int)fold_d2uchar);
    printf("negfrac2u=%u negfrac2uchar=%d negfrac2ull=%llu\n",
           fold_negfrac2u, (int)fold_negfrac2uchar, fold_negfrac2ull);
    printf("f2i=%d f2i_trunc=%d f2u=%u f2short=%d f2uchar=%d\n",
           fold_f2i, fold_f2i_trunc, fold_f2u, (int)fold_f2short,
           (int)fold_f2uchar);
    printf("i2f=%.6f i2d=%.6f u2d=%.6f ll2d=%.6f\n",
           (double)fold_i2f, fold_i2d, fold_u2d, fold_ll2d);
    printf("i2f_pow2=%.6f sc2d=%.6f uc2f=%.6f ull2d=%.6f\n",
           (double)fold_i2f_pow2, fold_sc2d, (double)fold_uc2f, fold_ull2d);
    printf("roundtrip_i_d_i=%d roundtrip_i_f_i=%d roundtrip_ll_d_ll=%lld"
           " roundtrip_u_d_u=%u\n",
           fold_roundtrip_i_d_i, fold_roundtrip_i_f_i,
           fold_roundtrip_ll_d_ll, fold_roundtrip_u_d_u);

    printf("rt_d2i_pos=%d rt_d2i_neg=%d rt_d2i_trunc_pos=%d"
           " rt_d2i_trunc_neg=%d\n",
           rt_d2i_pos, rt_d2i_neg, rt_d2i_trunc_pos, rt_d2i_trunc_neg);
    printf("rt_d2i_bound_pos=%d rt_d2i_bound_neg=%d rt_d2ll=%lld"
           " rt_d2ull=%llu\n",
           rt_d2i_bound_pos, rt_d2i_bound_neg, rt_d2ll, rt_d2ull);
    printf("rt_d2u=%u rt_d2short=%d rt_d2ushort=%d rt_d2schar=%d"
           " rt_d2uchar=%d\n",
           rt_d2u, (int)rt_d2short, (int)rt_d2ushort, (int)rt_d2schar,
           (int)rt_d2uchar);
    printf("rt_negfrac2u=%u rt_negfrac2uchar=%d rt_negfrac2ull=%llu\n",
           rt_negfrac2u, (int)rt_negfrac2uchar, rt_negfrac2ull);
    printf("rt_f2i=%d rt_f2i_trunc=%d rt_f2u=%u rt_f2short=%d"
           " rt_f2uchar=%d\n",
           rt_f2i, rt_f2i_trunc, rt_f2u, (int)rt_f2short, (int)rt_f2uchar);
    printf("rt_i2f=%.6f rt_i2d=%.6f rt_u2d=%.6f rt_ll2d=%.6f\n",
           (double)rt_i2f, rt_i2d, rt_u2d, rt_ll2d);
    printf("rt_i2f_pow2=%.6f rt_sc2d=%.6f rt_uc2f=%.6f rt_ull2d=%.6f\n",
           (double)rt_i2f_pow2, rt_sc2d, (double)rt_uc2f, rt_ull2d);
    printf("rt_roundtrip_i_d_i=%d rt_roundtrip_i_f_i=%d"
           " rt_roundtrip_ll_d_ll=%lld rt_roundtrip_u_d_u=%u\n",
           rt_roundtrip_i_d_i, rt_roundtrip_i_f_i, rt_roundtrip_ll_d_ll,
           rt_roundtrip_u_d_u);

    printf("folded_matches_runtime=%d %d %d %d %d %d %d %d\n",
           agree_d2i, agree_wide, agree_d2narrow, agree_negfrac, agree_f2int,
           agree_int2fp, agree_int2fp_wide, agree_roundtrip);
    return 0;
}
