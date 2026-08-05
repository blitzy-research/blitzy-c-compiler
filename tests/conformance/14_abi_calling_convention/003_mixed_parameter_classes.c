/* Area 14 - ABI and calling convention.
 * 003_mixed_parameter_classes: interleaved integer, floating, pointer and
 * aggregate parameters.  Twenty-six parameters arranged as five repetitions of
 * (int, double, double, pointer, 8-byte aggregate) followed by an integer tag.
 * That is sixteen integer-class consumers and ten floating-class consumers, so
 * BOTH register files are exhausted on System V AMD64, AAPCS64 and LP64D, while
 * System V i386 cdecl marshals all twenty-six on the stack.  Interleaving is
 * the point: it forces each ABI to advance both of its allocators through one
 * parameter list, which is exactly where the four conventions disagree - and
 * they disagree on the allocators' relationship, not merely on their contents.
 * On System V AMD64 and AAPCS64 the two advance INDEPENDENTLY: exhausting the
 * floating file has no effect on which integer register comes next, and the
 * reverse.  LP64D does not work that way.  There the two are COUPLED in one
 * direction: once fa0-fa7 are spent, a further floating argument is passed in an
 * INTEGER argument register if one is still free, and only falls to the stack when
 * none is.  So on RISC-V 64 a floating parameter can consume an integer slot that a
 * later integer parameter then does not get, and the crossover point has to be
 * exactly right.
 *
 * What that means for THIS program was measured rather than assumed -
 * `riscv64-linux-gnu-gcc -O0 -S -o - 003_mixed_parameter_classes.c`, reading
 * take_mixed26's prologue.  Sixteen integer-class parameters fill a0-a7 and ten
 * floating parameters fill fa0-fa7, so both files are already exhausted when the
 * ninth and tenth doubles are allocated: they arrive on the stack, and the prologue
 * contains no fmv.d.x at all.  The coupling itself - excess doubles crossing into
 * a0-a5 with the trailing tag pushed to a6 - is exercised by
 * 002_many_float_parameters.c, which leaves integer registers free precisely so
 * that it can be.  This program's job is the interleaving: both allocators are
 * driven to exhaustion through one parameter list, and every parameter is printed
 * back individually, so a backend that advanced one counter for both, or crossed
 * over one slot early, prints a wrong value rather than merely emitting different
 * instructions.  The documented rows are docs/technical-specifications.md line 546
 * (x86-64), line 553 (i686 cdecl), line 559 (AAPCS64) and line 565 (LP64D).
 *
 * THE CALL BARRIER, AND WHY THE ABI BOUNDARY WOULD OTHERWISE NOT EXIST.  A
 * twenty-six parameter boundary is only under test if the call actually happens.
 * A direct call to a static function whose arguments are all known lets the
 * optimizer specialise the callee: at -O2 the reference compiler emits only
 * take_mixed26.constprop.0, a clone with the constants folded in, so the
 * documented boundary would never be crossed at that level and the program would
 * silently test less than it claims at two of its three optimization levels.
 * Every call below therefore goes through a FILE-SCOPE volatile FUNCTION POINTER.
 * A volatile lvalue must be re-read on every access, so no conforming compiler
 * may establish at translation time which function the pointer designates, and
 * the transfer is therefore a genuine indirect call.  What that buys is an
 * OBSTACLE rather than a prohibition, and the two are kept apart here because
 * conflating them would overstate what this program proves: ISO C does NOT
 * forbid a specialised clone, a scalarised copy of the body or a guarded
 * devirtualization, each of which would honour the loaded value and still
 * dissolve the classification under test.  Their absence is therefore MEASURED
 * rather than asserted - with gcc 13.4.0 at -O2 on all four reference drivers
 * take_mixed26 is emitted unmodified, no symbol carries .constprop, .isra or
 * .part., and both call sites are genuine indirect transfers, two per target.
 * Those are artifacts of the REFERENCE compiler, so the boundary this program
 * claims to cover holds for a compiler under test only as far as the same
 * artifact inspection shows that it does; the residual risk that leaves, and the
 * per-cell static check that would close it, are recorded in
 * 001_many_integer_parameters.expected.  The mechanism is plain standard C
 * rather than a compiler attribute, so both sides of oracle (a) honour it for
 * the same reason and neither needs to support an extension for the barrier to
 * hold.  The full account, including the standard citations, is the section
 * "THE CALL BOUNDARY IS ENFORCED, NOT HOPED FOR" below; this paragraph states
 * only why the indirection is here at all.
 *
 * EVALUATION-ORDER DISCIPLINE.  An access to a volatile object is an observable
 * side effect and argument evaluation order is unspecified, so every volatile
 * datum is read exactly once, in a statement of its own, and only the snapshots
 * are passed -- at most one side-effecting argument per call, which here means
 * the function-pointer load and nothing else.  The snapshots do not weaken the
 * runtime variant: each is itself a load from volatile storage the optimizer may
 * not fold, so the callee still receives values materialised at run time.
 *
 * Pointer discipline: pointers are passed but no pointer value is ever printed.
 * Only the pointed-to int is printed.  Aggregate discipline: only named members
 * are read back; no padding byte is printed, compared or memcmp'd.
 *
 * THE CALL BOUNDARY IS ENFORCED, NOT HOPED FOR.  The callee is reached through a
 * volatile-qualified function pointer rather than by name.  An ordinary static
 * callee may legally be inlined at -O1 and -O2, and an inlined callee marshals
 * nothing: the interleaved allocator advance this program exists to exercise
 * would then never happen, and its assertions would describe code that was never
 * emitted.  That is not hypothetical: measured with gcc 13.4.0 at -O2, a by-name
 * callee in this area is inlined into its caller unless it happens to exceed the
 * inliner's size budget, so survival by name would be an accident of a heuristic
 * rather than a property of the test.  ISO C requires a
 * volatile pointer to be re-read at each point of call and control to go to
 * whatever function that load produced (C11 5.1.2.3p2 and p6, 6.7.3p7), so the
 * designated function is opaque to the optimizer and the call is genuinely
 * indirect.  It does NOT forbid a specialised clone, a scalarised copy of the
 * body, or a guarded devirtualization, each of which could honour the loaded value
 * and still dissolve exactly the classification this program is testing - so the
 * absence of them is measured rather than assumed: with gcc 13.4.0 at -O2 on all
 * four reference drivers no symbol carries .constprop, .isra or .part., and both
 * variants reach the callee through a genuine indirect transfer, two per target.
 * 001_many_integer_parameters.expected states the residual risk that leaves and
 * what would close it.  The mechanism is pure ISO C: a
 * function attribute would have been shorter, but the documented attribute set
 * for the compiler under test is packed, aligned, section, unused, deprecated,
 * visibility and format (docs/technical-specifications.md line 506), so an
 * inlining attribute would risk a divergence caused by the test and would import
 * an extension into an area whose subject is the calling convention.
 *
 * BOTH VARIANTS ARE GENUINELY TWO VARIANTS, POINTERS INCLUDED.  The folded call
 * passes literal integers, literal doubles, addresses of a static const array and
 * aggregates built from constants.  The runtime call passes values that all
 * originate in volatile storage - the pointers among them.  A pointer taken
 * directly as &ptgt[i] would be a link-time constant that the compiler can
 * materialise without ever loading anything, so the pointer class would have a
 * folded spelling and no runtime spelling at all: the one argument class whose
 * transport was never actually exercised.  The runtime pointers are therefore
 * read out of a volatile array of pointers, so each one arrives through a load
 * the optimizer cannot fold, and the pointer-class slot of the ABI is marshalled
 * for real.
 *
 * ONE VOLATILE READ PER FULL STATEMENT.  No argument list contains a volatile
 * lvalue.  Every runtime value - integer, double, pointer and aggregate member -
 * is copied out of volatile storage into a plain local in a statement of its own,
 * and the call then reads only those plain locals.  The order in which a compiler
 * evaluates the arguments of a call is unspecified, so an argument list holding
 * twenty-odd volatile reads would have an unspecified order of side effects, and
 * a suite whose premise is that a divergence means a defect needs the program to
 * have exactly one defined behaviour.
 */

int printf(const char *, ...);

struct pair2 {
    int lo;
    int hi;
};

static const char *variant_tag(int variant);
static void take_mixed26(int i1, double d1, double e1, const int *p1,
                         struct pair2 s1,
                         int i2, double d2, double e2, const int *p2,
                         struct pair2 s2,
                         int i3, double d3, double e3, const int *p3,
                         struct pair2 s3,
                         int i4, double d4, double e4, const int *p4,
                         struct pair2 s4,
                         int i5, double d5, double e5, const int *p5,
                         struct pair2 s5,
                         int variant);

static const int ptgt[5] = { 3001, -3002, 3003, -3004, 3005 };

static volatile int vint[5] = { 201, -202, 203, -204, 205 };
static volatile double vdbl[5] = { 1.5, -2.25, 3.125, -4.0625, 5.5 };
static volatile double vdbe[5] = { -6.75, 7.875, -8.1875, 9.25, -10.5 };
static volatile int vslo[5] = { 41, 43, 45, 47, 49 };
static volatile int vshi[5] = { -42, -44, -46, -48, -50 };
static volatile int vtag = 1;

/* Runtime sources for the pointer class.  The elements are the same five
   addresses the folded call passes as &ptgt[i], but reaching them through
   volatile storage means each one arrives at the call through a load the
   optimizer may not fold, so the pointer class has a genuine runtime spelling
   instead of a second folded one.  The pointed-to objects are the same static
   const array, so what is dereferenced is identical in both variants and only
   the transport differs. */
static const int *volatile vptr[5] = {
    &ptgt[0], &ptgt[1], &ptgt[2], &ptgt[3], &ptgt[4]
};

/* The call boundary: a volatile-qualified pointer to the callee, so the call is
   indirect at every optimization level and the aggregate parameters are opaque to
   scalar replacement.  Measured on the reference toolchain at -O2: no clone and no
   scalarised copy appears.  Both variants travel through it. */
static void (*volatile take_mixed26_p)(int, double, double, const int *,
                                       struct pair2,
                                       int, double, double, const int *,
                                       struct pair2,
                                       int, double, double, const int *,
                                       struct pair2,
                                       int, double, double, const int *,
                                       struct pair2,
                                       int, double, double, const int *,
                                       struct pair2,
                                       int) = take_mixed26;

static const char *variant_tag(int variant)
{
    return (variant == 0) ? "folded" : "runtime";
}

static void take_mixed26(int i1, double d1, double e1, const int *p1,
                         struct pair2 s1,
                         int i2, double d2, double e2, const int *p2,
                         struct pair2 s2,
                         int i3, double d3, double e3, const int *p3,
                         struct pair2 s3,
                         int i4, double d4, double e4, const int *p4,
                         struct pair2 s4,
                         int i5, double d5, double e5, const int *p5,
                         struct pair2 s5,
                         int variant)
{
    const char *t = variant_tag(variant);
    printf("mix_%s_i1=%d\n", t, i1);
    printf("mix_%s_d1=%.4f\n", t, d1);
    printf("mix_%s_e1=%.4f\n", t, e1);
    printf("mix_%s_p1_deref=%d\n", t, *p1);
    printf("mix_%s_s1_lo=%d s1_hi=%d\n", t, s1.lo, s1.hi);
    printf("mix_%s_i2=%d\n", t, i2);
    printf("mix_%s_d2=%.4f\n", t, d2);
    printf("mix_%s_e2=%.4f\n", t, e2);
    printf("mix_%s_p2_deref=%d\n", t, *p2);
    printf("mix_%s_s2_lo=%d s2_hi=%d\n", t, s2.lo, s2.hi);
    printf("mix_%s_i3=%d\n", t, i3);
    printf("mix_%s_d3=%.4f\n", t, d3);
    printf("mix_%s_e3=%.4f\n", t, e3);
    printf("mix_%s_p3_deref=%d\n", t, *p3);
    printf("mix_%s_s3_lo=%d s3_hi=%d\n", t, s3.lo, s3.hi);
    printf("mix_%s_i4=%d\n", t, i4);
    printf("mix_%s_d4=%.4f\n", t, d4);
    printf("mix_%s_e4=%.4f\n", t, e4);
    printf("mix_%s_p4_deref=%d\n", t, *p4);
    printf("mix_%s_s4_lo=%d s4_hi=%d\n", t, s4.lo, s4.hi);
    printf("mix_%s_i5=%d\n", t, i5);
    printf("mix_%s_d5=%.4f\n", t, d5);
    printf("mix_%s_e5=%.4f\n", t, e5);
    printf("mix_%s_p5_deref=%d\n", t, *p5);
    printf("mix_%s_s5_lo=%d s5_hi=%d\n", t, s5.lo, s5.hi);
}

int main(void)
{
    struct pair2 c1 = { 41, -42 };
    struct pair2 c2 = { 43, -44 };
    struct pair2 c3 = { 45, -46 };
    struct pair2 c4 = { 47, -48 };
    struct pair2 c5 = { 49, -50 };
    struct pair2 r1;
    struct pair2 r2;
    struct pair2 r3;
    struct pair2 r4;
    struct pair2 r5;
    /* Plain destinations for every runtime operand: integers, both double
       columns, the pointers and the tag.  Only these are read at the runtime call
       site, so its argument list holds no side effect at all. */
    int plain_int[5];
    double plain_dbl[5];
    double plain_dbe[5];
    const int *plain_ptr[5];
    int plain_tag;
    int k;

    take_mixed26_p(201, 1.5, -6.75, &ptgt[0], c1,
                   -202, -2.25, 7.875, &ptgt[1], c2,
                   203, 3.125, -8.1875, &ptgt[2], c3,
                   -204, -4.0625, 9.25, &ptgt[3], c4,
                   205, 5.5, -10.5, &ptgt[4], c5,
                   0);

    /* One volatile read per full statement, each separated from the next by a
       sequence point.  The loop body performs five reads in five statements, and
       the aggregate members are staged the same way, so no two volatile accesses
       share an expression and nothing depends on an unspecified order. */
    for (k = 0; k < 5; k++) {
        plain_int[k] = vint[k];
        plain_dbl[k] = vdbl[k];
        plain_dbe[k] = vdbe[k];
        plain_ptr[k] = vptr[k];
    }
    plain_tag = vtag;


    r1.lo = vslo[0];
    r1.hi = vshi[0];
    r2.lo = vslo[1];
    r2.hi = vshi[1];
    r3.lo = vslo[2];
    r3.hi = vshi[2];
    r4.lo = vslo[3];
    r4.hi = vshi[3];
    r5.lo = vslo[4];
    r5.hi = vshi[4];

    take_mixed26_p(plain_int[0], plain_dbl[0], plain_dbe[0], plain_ptr[0], r1,
                   plain_int[1], plain_dbl[1], plain_dbe[1], plain_ptr[1], r2,
                   plain_int[2], plain_dbl[2], plain_dbe[2], plain_ptr[2], r3,
                   plain_int[3], plain_dbl[3], plain_dbe[3], plain_ptr[3], r4,
                   plain_int[4], plain_dbl[4], plain_dbe[4], plain_ptr[4], r5,
                   plain_tag);
    return 0;
}
