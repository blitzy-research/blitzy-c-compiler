/* Area 14 - ABI and calling convention.
 * 006_nested_calls_callee_saved: a bounded chain of nested calls, nine frames
 * deep - EIGHT PRESSURE FRAMES ABOVE ONE LEAF.  Each of the eight pressure frames
 * keeps ten INDEPENDENT values live across its nested call and prints them only
 * AFTER that call has returned.  That is what detects a clobbered callee-saved
 * register: if the callee fails to restore a register the caller was using, the
 * caller's printed value is wrong, and the report names each frame whose line
 * differs -- one frame where a single register is clobbered, and every affected
 * frame where the failure is systemic.
 *
 * The leaf is the ninth frame and is counted as one, but it is deliberately NOT a
 * pressure frame: it makes no call, so it has nothing to hold values across, and
 * giving it a row of live values would read as coverage while providing none.  Its
 * job is to be the innermost frame the eight above it must survive.
 *
 * THE PRESERVED REGISTER SETS, STATED IN FULL.  What follows is what each ABI
 * REQUIRES a callee to preserve; it is not a claim about how any compiler chooses
 * to allocate, which is the compiler's business and not something a test may
 * assume.
 *   System V AMD64  rbx, rbp, r12, r13, r14, r15 (and rsp).  There is NO
 *                   callee-saved SSE or x87 data register at all, so a
 *                   floating-point value that must survive a call cannot be left
 *                   in one; the x87 control word and the mxcsr control bits are
 *                   preserved, but no data register is.
 *   System V i386   ebx, esi, edi and EBP (and esp) - four, the narrowest set of
 *                   the four ABIs, and likewise no callee-saved floating-point
 *                   data register.
 *   AAPCS64         x19-x28, plus x29 as the frame pointer (x30 is the link
 *                   register, saved by the caller's own prologue).  For the
 *                   SIMD/FP file the obligation is PARTIAL and precise: only the
 *                   LOW 64 BITS of v8-v15 - that is, d8-d15 - are preserved, and
 *                   the upper bits of those registers, like the whole of
 *                   v0-v7 and v16-v31, are not.
 *   LP64D           s0-s11 integer, where s0 doubles as the frame pointer, and
 *                   fs0-fs11 floating.
 * Each of the eight pressure frames keeps six ints, three doubles and one pointer
 * live ACROSS its nested call and prints them only after that call returns.  Ten
 * live values is more than the narrowest of those sets can hold, so on every
 * target some of them must survive the call somewhere other than a preserved
 * register - but which values go where is the compiler's choice, and this program
 * does not assert it.  What it asserts is the OBSERVABLE consequence: if any of the
 * ten fails to survive the call, the printed line for that frame changes, so the
 * divergence is localised to the frames it actually reaches rather than summarised.
 *
 * WHY THE LIVE VALUES COME FROM VOLATILE STORAGE, WHICH IS THE WHOLE POINT.
 * Live-across-a-call is a property of the generated code, not of the source, and
 * a value that can be RECOMPUTED after the call need not be kept anywhere: a
 * frame whose ten values were all derived from its own two parameters - seed + 1,
 * seed + 2, fseed + 0.25 and so on - lets the optimizer rematerialize every one
 * of them from the parameters afterwards, hold nothing across the call, and
 * still print the right answers.  The program would then assert ten values while
 * requiring the backend to preserve none of them, and a callee that clobbered
 * every callee-saved register it touched would pass.
 *
 * A value read from a volatile object cannot be rematerialized, because reading
 * it again would be a second observable access that the abstract machine does not
 * perform.  Each PRESSURE frame therefore reads its ten values out of its OWN row
 * of volatile tables before the nested call and consumes them only after the call
 * returns, which leaves the backend no choice: ten independent quantities must
 * survive the call, in callee-saved registers as far as they reach and spilled
 * beyond that.  There are eight such rows for the eight such frames.  The rows differ per frame and every value in the tables is
 * distinct, so a value restored from the wrong slot prints a value belonging to
 * another frame and names both frames at once.
 *
 * THE CALL BOUNDARY IS ENFORCED, NOT HOPED FOR.  Every level is reached through a
 * volatile-qualified function pointer rather than by name.  An ordinary static
 * callee may legally be inlined at -O1 and -O2, and an inlined callee neither
 * saves nor restores anything: the nested chain would collapse into one frame and
 * there would be no call for a value to be live across.  Measured with gcc 13.4.0
 * at -O2 before the indirection was added: only level1 survived in each variant -
 * levels 2 through 8 and the leaf were all inlined, so a nine-frame chain was
 * testing a single frame.  ISO C requires a volatile pointer to be re-read at each
 * point of call and control to go to whatever function that load produced
 * (C11 5.1.2.3p2 and p6, 6.7.3p7), so the designated function is opaque to the
 * optimizer and the call is genuinely indirect.  It does NOT forbid a specialised
 * clone or a guarded devirtualization that could still elide a frame, so the
 * nine-frame chain is verified rather than assumed: with gcc 13.4.0 at -O2 on all
 * four reference drivers no symbol carries .constprop, .isra or .part. and the ten
 * boundaries are reached through ten genuine indirect transfers per target.
 * 001_many_integer_parameters.expected states the residual risk that leaves and
 * what would close it.  The mechanism is pure ISO C: a function attribute would have
 * been shorter, but the documented attribute set for the compiler under test is
 * packed, aligned, section, unused, deprecated, visibility and format
 * (docs/technical-specifications.md line 506), so an inlining attribute would
 * risk a divergence caused by the test and would import an extension into an area
 * whose subject is the calling convention.
 *
 * ONE VOLATILE READ PER FULL STATEMENT.  Each of the ten reads in a frame is its
 * own statement, and no argument list anywhere in the program contains a volatile
 * lvalue: the runtime entry point stages its three operands into plain locals
 * first.  An access to a volatile object is an observable side effect and the
 * order of side effects within one argument list is unspecified, so reading
 * several volatile objects inside one call expression would leave the program with
 * more than one defined behaviour - which is exactly what a differential oracle
 * cannot tolerate, since it must be able to say that a divergence is a defect.
 *
 * AT WHICH OPTIMIZATION LEVEL THE CHAIN IS REALLY NINE FRAMES DEEP.  At the source
 * level it always is; as emitted code it depends on inlining, which is checked by
 * counting this program's own static function labels in the reference compiler's
 * assembly rather than assumed.  With the volatile call boundary in place, ALL NINE
 * FRAMES ARE EMITTED at -O0, -O1 and -O2 on x86-64, i686, AArch64 and RISC-V 64
 * alike, with no .constprop and no .isra clone anywhere.  So the eight-frame
 * pressure and the save/restore discipline it is designed to stress are exercised as
 * real frames at every one of the twelve cells, which is the property the boundary
 * exists to guarantee.  A different compiler may inline differently, which is
 * precisely why the boundary is expressed in the source rather than left to any
 * implementation's judgement.
 *
 * Depth is bounded at nine nested frames plus main, and every call is a distinct
 * function, so there is no recursion and no unbounded growth.  Peak stack use for
 * the program's own frames was MEASURED from the reference compiler's -O0
 * prologues on each target, rather than estimated from the ten scalars each frame
 * holds:
 *   x86-64      eight 96-byte frames, one 48 and one 32, plus ten saved return
 *               addresses and frame pointers at 8 bytes each  ~= 1.0 KiB
 *   i686        eight 84-byte frames, one 36 and one 32, plus ten at 4 bytes
 *               each                                          ~= 0.8 KiB
 *   AArch64     eight 128-byte frames, one 64 and one 48      ~= 1.1 KiB
 *   RISC-V 64   eight 144-byte frames, one 64 and one 48      ~= 1.2 KiB
 * The C library's own printf frame is on top of that and is not counted here.  So
 * the honest figure is on the order of one to two kibibytes, rather than the few
 * hundred bytes the ten-scalar count alone would suggest.  It remains roughly three
 * orders of magnitude below the
 * 8 MiB default stack these binaries run under, both natively (ulimit -s reports
 * 8192 KiB) and under QEMU user-mode emulation (QEMU_STACK_SIZE defaults to
 * 8388608 bytes).
 *
 * Determinism and width normalisation: only int and double values are printed,
 * both the same width on all four targets, at the fixed precision the area rule
 * requires.  The live pointer is never printed as an address - only the int it
 * points at is - so nothing depends on where anything was placed.  No plain char
 * appears, no long, no size_t.  main returns 0, inside the permitted 0-125 range.
 */

int printf(const char *, ...);

static int leaf_mix(int seed, double fseed, const char *tag);
static int level8(int seed, double fseed, const char *tag);
static int level7(int seed, double fseed, const char *tag);
static int level6(int seed, double fseed, const char *tag);
static int level5(int seed, double fseed, const char *tag);
static int level4(int seed, double fseed, const char *tag);
static int level3(int seed, double fseed, const char *tag);
static int level2(int seed, double fseed, const char *tag);
static int level1(int seed, double fseed, const char *tag);


static volatile int vseed = 100;
static volatile double vfseed = 1.0;
static const char *volatile vtag = "runtime";

/* The independent live values, ONE ROW PER PRESSURE FRAME -- eight rows for the
   eight frames that hold values across a nested call, and none for the leaf, which
   makes no call and therefore holds nothing across one.  Every one of the
   forty-eight integers, twenty-four doubles and eight pointed-to integers is
   distinct, so a value restored from the wrong slot prints a quantity that belongs
   to a different frame or a different position and names both at once.  Reading
   them through volatile storage is what makes them independent of the frame's
   parameters and therefore genuinely live across its nested call: they cannot be
   recomputed afterwards, because reading a volatile object twice is not what the
   program says.

   The table is sized to what is CONSUMED.  A ninth row would be initialized and
   never read, which reads as coverage while providing none: the leaf has no call to
   survive, so a row belonging to it could not be live across anything. */
static volatile int vlive_i[8][6] = {
    { 3101, -3102, 3103, -3104, 3105, -3106 },
    { 3201, -3202, 3203, -3204, 3205, -3206 },
    { 3301, -3302, 3303, -3304, 3305, -3306 },
    { 3401, -3402, 3403, -3404, 3405, -3406 },
    { 3501, -3502, 3503, -3504, 3505, -3506 },
    { 3601, -3602, 3603, -3604, 3605, -3606 },
    { 3701, -3702, 3703, -3704, 3705, -3706 },
    { 3801, -3802, 3803, -3804, 3805, -3806 }
};

static volatile double vlive_d[8][3] = {
    { 1.25, -2.5, 3.75 },
    { 4.25, -5.5, 6.75 },
    { 7.25, -8.5, 9.75 },
    { 10.25, -11.5, 12.75 },
    { 13.25, -14.5, 15.75 },
    { 16.25, -17.5, 18.75 },
    { 19.25, -20.5, 21.75 },
    { 22.25, -23.5, 24.75 }
};

/* The pointed-to objects for each frame's live pointer.  Only the int a pointer
   designates is ever printed, never the pointer itself, so the output cannot
   depend on an address. */
static const int plive_target[8] = {
    5101, 5202, 5303, 5404, 5505, 5606, 5707, 5808
};

static const int *volatile vlive_p[8] = {
    &plive_target[0], &plive_target[1], &plive_target[2],
    &plive_target[3], &plive_target[4], &plive_target[5],
    &plive_target[6], &plive_target[7]
};

/* THE CALL BARRIER, AND WHY THE NINE-FRAME CHAIN WOULD OTHERWISE NOT EXIST.  A
 * callee-saved obligation is only under test if a call actually happens.  With
 * direct calls the reference compiler at -O2 leaves only level1 standing: levels 2
 * through 8 and leaf_mix are all inlined into it, so the nine frames the program
 * describes would collapse to one and the property under test would not exist at
 * that optimization level.  Each level therefore reaches the next
 * through a FILE-SCOPE volatile FUNCTION POINTER.  A volatile lvalue must be
 * re-read on every access, so no conforming compiler may assume which function
 * the pointer designates: it can neither inline nor clone the callee, and every
 * frame in the chain survives at every optimization level.  This is plain
 * standard C rather than a compiler attribute, so both sides of oracle (a) honour
 * it for the same reason.  Verified in the generated assembly of all four targets
 * at -O2: all nine functions are emitted unmodified, with no .constprop and no
 * .isra clone.
 *
 * The pointer is also the ONLY volatile access in each call expression, which is
 * what keeps the corpus rule of at most one side-effecting argument per call. */

/* The enforced call boundary, one volatile-qualified pointer per level.  Each is
   re-read at its call site, so every one of the nine calls in the chain is
   indirect at every optimization level and no frame can be inlined away.  Both
   variants traverse the same nine pointers. */
static int (*volatile leaf_mix_p)(int, double, const char *) = leaf_mix;
static int (*volatile level8_p)(int, double, const char *) = level8;
static int (*volatile level7_p)(int, double, const char *) = level7;
static int (*volatile level6_p)(int, double, const char *) = level6;
static int (*volatile level5_p)(int, double, const char *) = level5;
static int (*volatile level4_p)(int, double, const char *) = level4;
static int (*volatile level3_p)(int, double, const char *) = level3;
static int (*volatile level2_p)(int, double, const char *) = level2;
static int (*volatile level1_p)(int, double, const char *) = level1;

/* The leaf.  It holds nothing across a call because it makes none; its job is to
   be the innermost frame that the eight callers above it must survive. */
static int leaf_mix(int seed, double fseed, const char *tag)
{
    int r = seed + (int)(fseed * 4.0);
    printf("nest_%s_L9 seed=%d fseed=%.4f r=%d\n", tag, seed, fseed, r);
    return r;
}

/* Frames 8 down to 1.  Every one has the same shape: ten volatile reads, each in
   a statement of its own; the nested call; then the printf that consumes all ten
   values, which is what forces them to be live across the call rather than merely
   computed near it.  Each frame reads its own row, so no two frames share a
   value. */
static int level8(int seed, double fseed, const char *tag)
{
    int a = vlive_i[7][0];
    int b = vlive_i[7][1];
    int c = vlive_i[7][2];
    int d = vlive_i[7][3];
    int e = vlive_i[7][4];
    int f = vlive_i[7][5];
    double x = vlive_d[7][0];
    double y = vlive_d[7][1];
    double z = vlive_d[7][2];
    const int *p = vlive_p[7];
    int inner = leaf_mix_p(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L8 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f"
           " p=%d inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, *p, inner);
    return inner + a + b + c + d + e + f;
}

static int level7(int seed, double fseed, const char *tag)
{
    int a = vlive_i[6][0];
    int b = vlive_i[6][1];
    int c = vlive_i[6][2];
    int d = vlive_i[6][3];
    int e = vlive_i[6][4];
    int f = vlive_i[6][5];
    double x = vlive_d[6][0];
    double y = vlive_d[6][1];
    double z = vlive_d[6][2];
    const int *p = vlive_p[6];
    int inner = level8_p(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L7 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f"
           " p=%d inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, *p, inner);
    return inner + a + b + c + d + e + f;
}

static int level6(int seed, double fseed, const char *tag)
{
    int a = vlive_i[5][0];
    int b = vlive_i[5][1];
    int c = vlive_i[5][2];
    int d = vlive_i[5][3];
    int e = vlive_i[5][4];
    int f = vlive_i[5][5];
    double x = vlive_d[5][0];
    double y = vlive_d[5][1];
    double z = vlive_d[5][2];
    const int *p = vlive_p[5];
    int inner = level7_p(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L6 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f"
           " p=%d inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, *p, inner);
    return inner + a + b + c + d + e + f;
}

static int level5(int seed, double fseed, const char *tag)
{
    int a = vlive_i[4][0];
    int b = vlive_i[4][1];
    int c = vlive_i[4][2];
    int d = vlive_i[4][3];
    int e = vlive_i[4][4];
    int f = vlive_i[4][5];
    double x = vlive_d[4][0];
    double y = vlive_d[4][1];
    double z = vlive_d[4][2];
    const int *p = vlive_p[4];
    int inner = level6_p(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L5 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f"
           " p=%d inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, *p, inner);
    return inner + a + b + c + d + e + f;
}

static int level4(int seed, double fseed, const char *tag)
{
    int a = vlive_i[3][0];
    int b = vlive_i[3][1];
    int c = vlive_i[3][2];
    int d = vlive_i[3][3];
    int e = vlive_i[3][4];
    int f = vlive_i[3][5];
    double x = vlive_d[3][0];
    double y = vlive_d[3][1];
    double z = vlive_d[3][2];
    const int *p = vlive_p[3];
    int inner = level5_p(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L4 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f"
           " p=%d inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, *p, inner);
    return inner + a + b + c + d + e + f;
}

static int level3(int seed, double fseed, const char *tag)
{
    int a = vlive_i[2][0];
    int b = vlive_i[2][1];
    int c = vlive_i[2][2];
    int d = vlive_i[2][3];
    int e = vlive_i[2][4];
    int f = vlive_i[2][5];
    double x = vlive_d[2][0];
    double y = vlive_d[2][1];
    double z = vlive_d[2][2];
    const int *p = vlive_p[2];
    int inner = level4_p(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L3 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f"
           " p=%d inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, *p, inner);
    return inner + a + b + c + d + e + f;
}

static int level2(int seed, double fseed, const char *tag)
{
    int a = vlive_i[1][0];
    int b = vlive_i[1][1];
    int c = vlive_i[1][2];
    int d = vlive_i[1][3];
    int e = vlive_i[1][4];
    int f = vlive_i[1][5];
    double x = vlive_d[1][0];
    double y = vlive_d[1][1];
    double z = vlive_d[1][2];
    const int *p = vlive_p[1];
    int inner = level3_p(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L2 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f"
           " p=%d inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, *p, inner);
    return inner + a + b + c + d + e + f;
}

static int level1(int seed, double fseed, const char *tag)
{
    int a = vlive_i[0][0];
    int b = vlive_i[0][1];
    int c = vlive_i[0][2];
    int d = vlive_i[0][3];
    int e = vlive_i[0][4];
    int f = vlive_i[0][5];
    double x = vlive_d[0][0];
    double y = vlive_d[0][1];
    double z = vlive_d[0][2];
    const int *p = vlive_p[0];
    int inner = level2_p(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L1 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f"
           " p=%d inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, *p, inner);
    return inner + a + b + c + d + e + f;
}

int main(void)
{
    /* Plain destinations for the runtime entry, staged one volatile read per full
       statement so the runtime call's argument list holds no side effect at all
       while its operands still arrive through loads the optimizer cannot fold. */
    int plain_seed;
    double plain_fseed;
    const char *plain_tag;
    int folded_total;
    int runtime_total;

    folded_total = level1_p(100, 1.0, "folded");
    printf("nest_folded_total=%d\n", folded_total);

    plain_seed = vseed;
    plain_fseed = vfseed;
    plain_tag = vtag;
    runtime_total = level1_p(plain_seed, plain_fseed, plain_tag);
    printf("nest_runtime_total=%d\n", runtime_total);
    return 0;
}
