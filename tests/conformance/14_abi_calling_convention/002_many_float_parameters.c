/* Area 14 - ABI and calling convention.
 * 002_many_float_parameters: floating parameter counts deliberately sized to
 * exhaust the floating-point argument registers of every supported ABI and
 * continue past the threshold.  The widest floating register file across the
 * four ABIs is eight (System V AMD64 xmm0-xmm7, AAPCS64 v0-v7, LP64D fa0-fa7);
 * System V i386 cdecl passes every argument on the stack.  Fourteen floating
 * parameters plus a trailing integer tag therefore exhaust the floating argument
 * registers of all four ABIs and continue past that threshold, and on the three
 * 64-bit targets they also put the trailing integer parameter through the
 * integer allocator while the floating allocator is already full.
 *
 * Where the six excess floating values actually go differs between the four, and
 * the difference is measured rather than assumed - see the table below.  Three of
 * the four spill them to the stack; RISC-V LP64D instead passes them in the
 * INTEGER argument registers, which is a distinct and even more error-prone path.
 *
 * Determinism: every literal is a dyadic rational with denominator at most 16,
 * so it is exactly representable in both float and double and prints exactly at
 * four decimal places with no rounding tie.  This is what makes the output
 * byte-identical even on i686, whose reference compiler evaluates with x87
 * 80-bit intermediates (FLT_EVAL_METHOD == 2).
 *
 * THE AREA RULE, which organises the whole file: size the parameter counts
 * against the WIDEST ABI, not the narrowest, and print every parameter back
 * individually and in order.  Never a checksum - a checksum can mask two
 * compensating errors, so one argument delivered wrongly and another delivered
 * wrongly in the opposite direction would still total correctly, and a pair of
 * transposed arguments would not show at all.  Fourteen is chosen because eight
 * is the widest documented floating argument file and fourteen clears it by six,
 * so no target under test can satisfy either call from its floating argument
 * registers alone.  Where the six that do not fit are then placed, and where the
 * trailing tag lands, was MEASURED by reading the reference compiler's own
 * assembly for this exact program on each target:
 *
 *   x86-64   System V AMD64  8 in xmm0-xmm7  6 on the STACK        tag in edi
 *   i686     System V cdecl  no arg registers at all, all 15 on the STACK
 *   AArch64  AAPCS64         8 in d0-d7      6 on the STACK        tag in w0
 *   RISC-V   LP64D           8 in fa0-fa7    6 in INTEGER a0-a5    tag in a6
 *
 * The RISC-V row is the interesting one and it is why "spilled to the stack" is
 * not said of all four: the LP64D convention passes a floating argument that no
 * longer fits the floating file in an INTEGER argument register before it resorts
 * to the stack, so six doubles cross into a0-a5 as raw bit patterns and the
 * trailing int tag is pushed along to a6.  Far from being a wrinkle, that makes
 * this program a sharper test on RISC-V than on the other three: the two register
 * files are deliberately COUPLED there, so a backend must get the crossover point
 * exactly right, and an off-by-one would silently shift six values and the tag.
 *
 * The documented ABIs those rows were checked against, so that the measurement is
 * confirmation rather than the only authority: see
 * docs/technical-specifications.md, which records "rdi, rsi, rdx, rcx, r8, r9
 * for integers; xmm0-xmm7 for floats" for x86-64, "all arguments passed on stack
 * (right-to-left push order)" for i686 cdecl, "x0-x7 integer argument registers,
 * v0-v7 SIMD/FP argument registers" for AAPCS64 and "a0-a7 integer argument
 * registers, fa0-fa7 float argument registers" for LP64D; the same four rows
 * appear in the component table of docs/project-guide.md.
 *
 * The trailing int tag is deliberately LAST, and it is not decoration.  On the
 * three 64-bit targets it lands in an INTEGER argument register even though all
 * fourteen preceding parameters are floating, and the measurement above shows the
 * three of them answering that in two genuinely different ways:
 *
 *   - on x86-64 and AArch64 the two allocators advance INDEPENDENTLY, so the
 *     integer file is still untouched when the floating file is full and the tag
 *     takes the very first integer register (edi, w0).  A backend that advanced
 *     one counter for both would place it past the first slot and print a wrong
 *     variant name in every line of the group;
 *   - on RISC-V the two are deliberately COUPLED, six excess doubles having
 *     already consumed a0-a5, so the correct home for the tag is a6 and nothing
 *     else.  Here a backend that kept the files independent would put it in a0
 *     and overwrite the sixth floating argument - the opposite mistake, caught by
 *     the same single parameter.
 *
 * On i686 the same parameter is simply one more stack slot, arriving after
 * fourteen eight-byte or four-byte floating slots.  One parameter therefore
 * discriminates between two opposite allocator errors on three targets and
 * exercises the plain stack path on the fourth, which is why it is present at all
 * and why it must stay last.
 *
 * Both a double call and a float call are made, because single and double
 * precision occupy the same register file differently on each target: AAPCS64
 * views v0-v7 as s0-s7 or as d0-d7, LP64D uses fa0-fa7 in either precision, and
 * System V AMD64 packs both in xmm0-xmm7.  A backend can be right about one
 * width and wrong about the other, so testing only one width would leave half of
 * the floating argument path unvisited.
 *
 * The two-variant rule.  Each of the two calls is made TWICE: once over literal
 * arguments, which the constant folder may place at translation time, and once
 * over arguments that originate in volatile-qualified storage and so must be
 * re-read at run time.  Without the runtime variant the optimizer would
 * substitute the folder's answer for the backend's, and an argument-passing
 * defect would escape detection entirely - the suite would report PASS while
 * never having asked the backend to marshal anything.
 *
 * How the runtime variant is staged, and why it is staged rather than read in
 * place.  Each volatile array is copied element by element into a plain array in
 * a loop of its own, and only the plain array is read at the call site.  An
 * access to a volatile object is an observable side effect, and the relative
 * order of side effects within one argument list is unspecified, so reading
 * fourteen volatile elements inside the call expression would make the order of
 * fourteen side effects depend on the unspecified order of argument evaluation -
 * which requirement 1 of this suite's brief forbids.  Copying first separates
 * every volatile access from the next by a sequence point and leaves the
 * argument list free of side effects altogether, while costing the test nothing:
 * the copies are loads from volatile storage, so the values are still not
 * available for compile-time substitution and the call is still fed genuine
 * runtime operands.  This is the same staging that 07_variadics/001_int_args.c,
 * 07_variadics/006_many_args_stack_spill.c and
 * 11_literals_and_strings/004_format_specifier_conformance.c use, and for this
 * reason.  Every element of every volatile array is initialized in its own
 * declaration and written nowhere afterwards, so every element is initialized
 * before it is read.
 *
 * No header is named.  bcc ships no stdio.h - its bundled set is the nine
 * required freestanding headers plus a bonus stdatomic.h
 * (docs/technical-specifications.md line 19 and lines 202-214) - so an
 * #include <stdio.h> would fail against bcc while succeeding against the
 * reference compiler, manufacturing a divergence caused by the test rather than
 * by the compiler.  printf is therefore hand-declared, and float.h is not
 * included either, no macro from it being needed.  Area 14 takes neither of the
 * suite's two sanctioned header exceptions.
 *
 * Freedom from undefined behaviour, which is the precondition that makes the
 * differential oracle sound at all.  There is no arithmetic here beyond the one
 * equality comparison and the two loop counters that carry the structure, so
 * there is no signed overflow, no division, no shift, no conversion that can
 * lose a value, and no operation that can produce a NaN or an infinity: every
 * floating value is copied from a literal or from an initialized volatile object
 * into a parameter and then printed.  Each float is widened to double at the
 * printf call site, which is exact in every case because binary32 is a subset of
 * binary64.  Both loop counters run from 0 to 13 over arrays of fourteen
 * elements, so no index is ever out of range and no pointer past the end of an
 * object is formed at all.  No pointer arithmetic is performed; the only
 * pointers are the format strings and the const char * returned by variant_tag,
 * which points into a string literal with static storage duration and so stays
 * valid for the lifetime of the program.  No object is modified twice between
 * sequence points, and no argument to any call has a side effect.  Nothing
 * depends on padding bytes, on the relative addresses of unrelated objects, or
 * on any representation detail.
 *
 * Nothing representation-dependent is printed: no long double, no plain long, no
 * size_t, no pointer value, and no plain-char value - char appears only as
 * const char *.  The format set is exactly %s and %.4f.  main returns 0, inside
 * the permitted 0-125 range.
 *
 * This program carries no expected-divergence marker and no target restriction,
 * and it declares no warning-gate deviation.  The four calling conventions
 * differ by design and that difference is exactly what is under test; the
 * OBSERVABLE RESULTS must nevertheless agree, so a cross-backend difference here
 * is a genuine finding rather than an implementation-defined one, and the
 * program is compared on all four targets at all three optimization levels.
 * Because the full default gate applies, every float literal carries the f
 * suffix and every float passed to the variadic printf carries an explicit
 * (double) cast, so no narrowing and no implicit promotion is left implied.
 *
 * Deliberately elsewhere, so this program duplicates none of it: integer
 * parameter exhaustion belongs to 001_many_integer_parameters.c and the
 * interleaving of the two classes to 003_mixed_parameter_classes.c; aggregate
 * argument passing and aggregate return to 004 and 005; callee-saved
 * preservation across nested calls to 006.  The widest floating type's
 * cross-target representation difference belongs to area 13, which handles it
 * with a marker and a scoped oracle exclusion - which is why long double is
 * absent here, since using it would import a divergence that has nothing to do
 * with argument passing.  Variadic argument retrieval belongs to area 07; every
 * call this program makes is to a fully prototyped fixed-arity function, so the
 * arguments travel the ordinary parameter path rather than the variadic one.
 */

int printf(const char *, ...);

static const char *variant_tag(int variant);
static void take_double15(double d01, double d02, double d03, double d04,
                          double d05, double d06, double d07, double d08,
                          double d09, double d10, double d11, double d12,
                          double d13, double d14, int variant);
static void take_float15(float f01, float f02, float f03, float f04,
                         float f05, float f06, float f07, float f08,
                         float f09, float f10, float f11, float f12,
                         float f13, float f14, int variant);

static volatile double vdbl[14] = {
    1.5, -2.25, 3.125, -4.0625, 5.5, -6.75, 7.875,
    -8.1875, 9.25, -10.5, 11.125, -12.3125, 13.75, -14.9375
};

static volatile float vflt[14] = {
    0.5f, -1.25f, 2.375f, -3.0625f, 4.5f, -5.75f, 6.875f,
    -7.1875f, 8.25f, -9.5f, 10.125f, -11.3125f, 12.75f, -13.9375f
};

static volatile int vtag = 1;

/* Names the variant a group of lines belongs to.  Taking the tag as a parameter
   rather than reading a global is what puts the trailing int parameter on the
   wire in the first place, and returning a pointer into a string literal keeps
   the value valid for the whole program without any storage of its own. */
static const char *variant_tag(int variant)
{
    return (variant == 0) ? "folded" : "runtime";
}

/* Fourteen double parameters and a trailing int tag.  Every parameter is printed
   on a line of its own, in declaration order, so a single divergent line names
   the exact parameter that was mishandled.  The tag is consumed through
   variant_tag, so no parameter is unused. */
static void take_double15(double d01, double d02, double d03, double d04,
                          double d05, double d06, double d07, double d08,
                          double d09, double d10, double d11, double d12,
                          double d13, double d14, int variant)
{
    const char *t = variant_tag(variant);
    printf("dbl_%s_d01=%.4f\n", t, d01);
    printf("dbl_%s_d02=%.4f\n", t, d02);
    printf("dbl_%s_d03=%.4f\n", t, d03);
    printf("dbl_%s_d04=%.4f\n", t, d04);
    printf("dbl_%s_d05=%.4f\n", t, d05);
    printf("dbl_%s_d06=%.4f\n", t, d06);
    printf("dbl_%s_d07=%.4f\n", t, d07);
    printf("dbl_%s_d08=%.4f\n", t, d08);
    printf("dbl_%s_d09=%.4f\n", t, d09);
    printf("dbl_%s_d10=%.4f\n", t, d10);
    printf("dbl_%s_d11=%.4f\n", t, d11);
    printf("dbl_%s_d12=%.4f\n", t, d12);
    printf("dbl_%s_d13=%.4f\n", t, d13);
    printf("dbl_%s_d14=%.4f\n", t, d14);
}

/* The single-precision counterpart.  Each parameter carries an explicit (double)
   cast where it is handed to printf below: the default argument promotion would
   widen it anyway, but stating the conversion keeps it visible and keeps the
   program clean under the audit gate's -Wconversion, from which area 14
   sanctions no deviation. */
static void take_float15(float f01, float f02, float f03, float f04,
                         float f05, float f06, float f07, float f08,
                         float f09, float f10, float f11, float f12,
                         float f13, float f14, int variant)
{
    const char *t = variant_tag(variant);
    printf("flt_%s_f01=%.4f\n", t, (double)f01);
    printf("flt_%s_f02=%.4f\n", t, (double)f02);
    printf("flt_%s_f03=%.4f\n", t, (double)f03);
    printf("flt_%s_f04=%.4f\n", t, (double)f04);
    printf("flt_%s_f05=%.4f\n", t, (double)f05);
    printf("flt_%s_f06=%.4f\n", t, (double)f06);
    printf("flt_%s_f07=%.4f\n", t, (double)f07);
    printf("flt_%s_f08=%.4f\n", t, (double)f08);
    printf("flt_%s_f09=%.4f\n", t, (double)f09);
    printf("flt_%s_f10=%.4f\n", t, (double)f10);
    printf("flt_%s_f11=%.4f\n", t, (double)f11);
    printf("flt_%s_f12=%.4f\n", t, (double)f12);
    printf("flt_%s_f13=%.4f\n", t, (double)f13);
    printf("flt_%s_f14=%.4f\n", t, (double)f14);
}

int main(void)
{
    /* Plain destinations for the runtime variants.  Only these are read at the
       two runtime call sites, so neither argument list contains a side effect,
       while the values themselves remain unfoldable because they arrive from
       volatile storage. */
    double plain_d[14];
    float plain_f[14];
    int plain_tag;
    int k_d;
    int k_f;

    /* Folded variants: literal arguments, which the constant folder is free to
       place at translation time. */
    take_double15(1.5, -2.25, 3.125, -4.0625, 5.5, -6.75, 7.875,
                  -8.1875, 9.25, -10.5, 11.125, -12.3125, 13.75, -14.9375, 0);
    take_float15(0.5f, -1.25f, 2.375f, -3.0625f, 4.5f, -5.75f, 6.875f,
                 -7.1875f, 8.25f, -9.5f, 10.125f, -11.3125f, 12.75f,
                 -13.9375f, 0);

    /* One volatile read per statement, each separated from the next by a
       sequence point.  The tag is read once and used by both runtime calls. */
    plain_tag = vtag;

    for (k_d = 0; k_d < 14; k_d++) {
        plain_d[k_d] = vdbl[k_d];
    }
    take_double15(plain_d[0], plain_d[1], plain_d[2], plain_d[3], plain_d[4],
                  plain_d[5], plain_d[6], plain_d[7], plain_d[8], plain_d[9],
                  plain_d[10], plain_d[11], plain_d[12], plain_d[13],
                  plain_tag);

    for (k_f = 0; k_f < 14; k_f++) {
        plain_f[k_f] = vflt[k_f];
    }
    take_float15(plain_f[0], plain_f[1], plain_f[2], plain_f[3], plain_f[4],
                 plain_f[5], plain_f[6], plain_f[7], plain_f[8], plain_f[9],
                 plain_f[10], plain_f[11], plain_f[12], plain_f[13],
                 plain_tag);

    return 0;
}
