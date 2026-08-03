/* Area 14 - ABI and calling convention.
 * 001_many_integer_parameters: integer parameter counts deliberately sized to
 * exhaust the integer argument registers of every supported ABI and continue
 * past the threshold, so that every target genuinely marshals arguments on the
 * stack.  Widest register file across the four ABIs is eight integer registers
 * (AAPCS64 x0-x7, LP64D a0-a7); System V AMD64 has only six (rdi, rsi, rdx,
 * rcx, r8, r9); System V i386 cdecl passes everything on the stack.  Fifteen
 * integer-class parameters therefore spill on all four.
 *
 * Two variants: a folded variant whose arguments are compile-time constants,
 * and a runtime variant whose arguments are read from volatile storage so the
 * values are genuinely materialised and marshalled rather than folded into the
 * callee.  The runtime variant is staged: each volatile element is copied into a
 * plain array in a full expression of its own, and only the plain array is read
 * at the call site, so no argument list contains a side effect whose order
 * relative to another is unspecified.  Staging costs the test nothing, because a
 * load from volatile storage is still not available for compile-time
 * substitution.
 *
 * THE CALL BOUNDARY IS ENFORCED, NOT HOPED FOR.  Every call below goes through
 * a volatile-qualified function pointer rather than naming the callee
 * directly. That is what makes this program an ABI test at every optimization
 * level: an ordinary static callee may legally be inlined at -O1 and -O2, and
 * a callee that is inlined marshals nothing, so the parameter passing the
 * program claims to exercise would simply not happen.  Measured with gcc
 * 13.4.0 at -O2 across this area before the indirection was added: whole
 * groups of callees disappeared into their callers, and the ones that survived
 * did so only because they happened to be too large for the inliner's budget -
 * an accident of a heuristic, not a property of the test.
 *
 * A volatile-qualified pointer removes the accident.  The value must be
 * re-read from memory at the point of call, so the compiler may not assume
 * which function it designates and must emit a genuine indirect call; and
 * because the callee's address escapes into storage, its signature may not be
 * cloned or scalarised either.  Verified at instruction level on all four
 * targets at -O2: an indirect call through the pointer, with the argument list
 * marshalled exactly as the ABI requires.
 *
 * The mechanism is deliberately pure ISO C - a volatile function pointer,
 * nothing more.  A function attribute would have been the shorter spelling,
 * but the documented attribute set for the compiler under test is packed,
 * aligned, section, unused, deprecated, visibility and format
 * (docs/technical-specifications.md line 506), so an inlining attribute would
 * risk a divergence caused by the test rather than by the compiler, and it
 * would import an extension into an area whose subject is the calling
 * convention.
 *
 * ONE VOLATILE READ PER FULL STATEMENT.  No argument list contains a volatile
 * lvalue.  Every runtime value is copied out of volatile storage into a plain
 * local first, one read per statement, and the call then reads only those
 * plain locals.  The order in which a compiler evaluates the arguments of a
 * call is unspecified, so a list containing several volatile reads would have
 * an unspecified order of side effects - and this suite's whole premise is
 * that a divergence means a defect, which requires the program to have exactly
 * one defined behaviour.
 *
 * EVALUATION-ORDER DISCIPLINE, and why the runtime variant looks the way it
 * does.  An access to a volatile object IS an observable side effect, and the
 * order in which a call's arguments are evaluated is UNSPECIFIED, so an argument
 * list that read fifteen volatile objects would leave the sequence of fifteen
 * side effects unspecified -- and the corpus rule is at most ONE side-effecting
 * argument per call.  Each volatile element is therefore read exactly once, in a
 * statement of its own, and only the resulting snapshots are passed.  The
 * snapshots cost nothing in coverage: each is itself a load from volatile
 * storage that the optimizer may not fold or elide, so the callee still receives
 * fifteen values that were genuinely materialised at run time and marshalled
 * through the ABI.  What changes is only that the fifteen loads are now
 * SEQUENCED rather than unordered.
 *
 * Volatile staging discipline, which the runtime variant depends on for its
 * meaning: every volatile element is read into a plain staging array in a
 * statement of its own before any call, and the calls then name only plain
 * values.  Reading several volatile objects inside one argument list would leave
 * the relative order of those side effects unspecified, and a program whose own
 * behaviour is unspecified cannot make a divergence between two compilers
 * attributable to either - which is exactly the precondition the suite's
 * undefined-and-unspecified-behaviour rule protects.  No volatile lvalue appears
 * in any argument list in this file.
 *
 * Width normalisation: only int and long long are passed or printed.  No long,
 * no size_t, no pointer value, no plain char.
 */

int printf(const char *, ...);

static const char *variant_tag(int variant);
static void take_int15(int a01, int a02, int a03, int a04, int a05,
                       int a06, int a07, int a08, int a09, int a10,
                       int a11, int a12, int a13, int a14, int variant);
static void take_llong15(long long b01, long long b02, long long b03,
                         long long b04, long long b05, long long b06,
                         long long b07, long long b08, long long b09,
                         long long b10, long long b11, long long b12,
                         long long b13, long long b14, int variant);

static volatile int vint[15] = {
    1001, -1002, 1003, -1004, 1005, -1006, 1007,
    -1008, 1009, -1010, 1011, -1012, 1013, -1014, 1
};

static volatile long long vllong[15] = {
    5000000001LL, -5000000002LL, 5000000003LL, -5000000004LL,
    5000000005LL, -5000000006LL, 5000000007LL, -5000000008LL,
    5000000009LL, -5000000010LL, 5000000011LL, -5000000012LL,
    5000000013LL, -5000000014LL, 1LL
};

/* The enforced call boundary.  Each pointer is volatile-qualified, so it is
   re-read at every call site and the callee it designates is unknown to the
   optimizer: the call is indirect, the callee's body is not inlined, and its
   signature is not cloned.  Both variants of both groups travel through these,
   so the folded and the runtime call cross the same boundary. */
static void (*volatile take_int15_p)(int, int, int, int, int, int, int, int,
                                     int, int, int, int, int, int, int) =
    take_int15;
static void (*volatile take_llong15_p)(long long, long long, long long,
                                       long long, long long, long long,
                                       long long, long long, long long,
                                       long long, long long, long long,
                                       long long, long long, int) =
    take_llong15;

static const char *variant_tag(int variant)
{
    return (variant == 0) ? "folded" : "runtime";
}

static void take_int15(int a01, int a02, int a03, int a04, int a05,
                       int a06, int a07, int a08, int a09, int a10,
                       int a11, int a12, int a13, int a14, int variant)
{
    const char *t = variant_tag(variant);
    printf("int_%s_a01=%d\n", t, a01);
    printf("int_%s_a02=%d\n", t, a02);
    printf("int_%s_a03=%d\n", t, a03);
    printf("int_%s_a04=%d\n", t, a04);
    printf("int_%s_a05=%d\n", t, a05);
    printf("int_%s_a06=%d\n", t, a06);
    printf("int_%s_a07=%d\n", t, a07);
    printf("int_%s_a08=%d\n", t, a08);
    printf("int_%s_a09=%d\n", t, a09);
    printf("int_%s_a10=%d\n", t, a10);
    printf("int_%s_a11=%d\n", t, a11);
    printf("int_%s_a12=%d\n", t, a12);
    printf("int_%s_a13=%d\n", t, a13);
    printf("int_%s_a14=%d\n", t, a14);
}

static void take_llong15(long long b01, long long b02, long long b03,
                         long long b04, long long b05, long long b06,
                         long long b07, long long b08, long long b09,
                         long long b10, long long b11, long long b12,
                         long long b13, long long b14, int variant)
{
    const char *t = variant_tag(variant);
    printf("llong_%s_b01=%lld\n", t, b01);
    printf("llong_%s_b02=%lld\n", t, b02);
    printf("llong_%s_b03=%lld\n", t, b03);
    printf("llong_%s_b04=%lld\n", t, b04);
    printf("llong_%s_b05=%lld\n", t, b05);
    printf("llong_%s_b06=%lld\n", t, b06);
    printf("llong_%s_b07=%lld\n", t, b07);
    printf("llong_%s_b08=%lld\n", t, b08);
    printf("llong_%s_b09=%lld\n", t, b09);
    printf("llong_%s_b10=%lld\n", t, b10);
    printf("llong_%s_b11=%lld\n", t, b11);
    printf("llong_%s_b12=%lld\n", t, b12);
    printf("llong_%s_b13=%lld\n", t, b13);
    printf("llong_%s_b14=%lld\n", t, b14);
}

int main(void)
{
    /* Plain destinations for the runtime variants.  Only these are read at the
       runtime call sites, so neither argument list contains a side effect,
       while the values themselves remain unfoldable because they arrive from
       volatile storage. */
    int plain_int[15];
    long long plain_llong[15];
    int k_int;
    int k_llong;

    /* Folded variants: literal arguments, which the constant folder is free to
       place at translation time - but which still cross the enforced boundary,
       because the pointer is volatile and the callee is therefore unknown. */
    take_int15_p(1001, -1002, 1003, -1004, 1005, -1006, 1007,
                 -1008, 1009, -1010, 1011, -1012, 1013, -1014, 0);
    take_llong15_p(5000000001LL, -5000000002LL, 5000000003LL, -5000000004LL,
                   5000000005LL, -5000000006LL, 5000000007LL, -5000000008LL,
                   5000000009LL, -5000000010LL, 5000000011LL, -5000000012LL,
                   5000000013LL, -5000000014LL, 0);

    /* One volatile read per statement, each separated from the next by a
       sequence point.  An access to a volatile object is an observable side
       effect, and the relative order of side effects within one argument list
       is unspecified, so reading fifteen volatile elements inside the call
       expression would make the order of fifteen side effects depend on the
       unspecified order of argument evaluation - which requirement 1 of this
       suite's brief forbids.  Copying first costs the test nothing: the copies
       are loads from volatile storage, so the values are still not available
       for compile-time substitution and each call is still fed genuine runtime
       operands that every ABI must marshal for real.  No argument list holds
       more than one side effect - in fact it holds none. */
    for (k_int = 0; k_int < 15; k_int++) {
        plain_int[k_int] = vint[k_int];
    }
    take_int15_p(plain_int[0], plain_int[1], plain_int[2], plain_int[3],
                 plain_int[4], plain_int[5], plain_int[6], plain_int[7],
                 plain_int[8], plain_int[9], plain_int[10], plain_int[11],
                 plain_int[12], plain_int[13], plain_int[14]);

    for (k_llong = 0; k_llong < 15; k_llong++) {
        plain_llong[k_llong] = vllong[k_llong];
    }
    take_llong15_p(plain_llong[0], plain_llong[1], plain_llong[2],
                   plain_llong[3], plain_llong[4], plain_llong[5],
                   plain_llong[6], plain_llong[7], plain_llong[8],
                   plain_llong[9], plain_llong[10], plain_llong[11],
                   plain_llong[12], plain_llong[13], (int)plain_llong[14]);
    return 0;
}
