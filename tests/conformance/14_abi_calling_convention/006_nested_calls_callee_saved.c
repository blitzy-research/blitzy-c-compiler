/* Area 14 - ABI and calling convention.
 * 006_nested_calls_callee_saved: a bounded chain of nested calls, nine frames
 * deep, in which every frame keeps ten values live across its nested call and
 * prints them only AFTER that call has returned.  That is what detects a
 * clobbered callee-saved register: if the callee fails to restore a register the
 * caller was using, the caller's printed value is wrong and the divergence
 * localises to the exact frame.
 *
 * Callee-saved register sets stressed:
 *   System V AMD64  rbx, r12-r15, rbp - and NO callee-saved SSE register, so the
 *                   three live doubles must be spilled to the stack frame
 *   System V i386   ebx, esi, edi - only three, so most values spill
 *   AAPCS64         x19-x28 integer, d8-d15 floating
 *   LP64D           s0-s11 integer, fs0-fs11 floating
 * Each frame holds six live ints, three live doubles and one live pointer across
 * its nested call - deliberately more than the narrowest set can hold, so every
 * target must both use its callee-saved registers and spill.
 *
 * Depth is bounded at nine nested frames plus main.  Each frame holds ten
 * scalars, so peak stack use is a few hundred bytes on every target, far below
 * any stack limit under QEMU user-mode emulation.  There is no recursion and no
 * unbounded growth.
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

static int leaf_mix(int seed, double fseed, const char *tag)
{
    int r = seed + (int)(fseed * 4.0);
    printf("nest_%s_L9 seed=%d fseed=%.4f r=%d\n", tag, seed, fseed, r);
    return r;
}

static int level8(int seed, double fseed, const char *tag)
{
    int a = seed + 1;
    int b = seed + 2;
    int c = seed + 3;
    int d = seed + 4;
    int e = seed + 5;
    int f = seed + 6;
    double x = fseed + 0.25;
    double y = fseed + 0.50;
    double z = fseed + 0.75;
    int inner = leaf_mix(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L8 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, inner);
    return inner + a + b + c + d + e + f;
}

static int level7(int seed, double fseed, const char *tag)
{
    int a = seed + 1;
    int b = seed + 2;
    int c = seed + 3;
    int d = seed + 4;
    int e = seed + 5;
    int f = seed + 6;
    double x = fseed + 0.25;
    double y = fseed + 0.50;
    double z = fseed + 0.75;
    int inner = level8(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L7 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, inner);
    return inner + a + b + c + d + e + f;
}

static int level6(int seed, double fseed, const char *tag)
{
    int a = seed + 1;
    int b = seed + 2;
    int c = seed + 3;
    int d = seed + 4;
    int e = seed + 5;
    int f = seed + 6;
    double x = fseed + 0.25;
    double y = fseed + 0.50;
    double z = fseed + 0.75;
    int inner = level7(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L6 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, inner);
    return inner + a + b + c + d + e + f;
}

static int level5(int seed, double fseed, const char *tag)
{
    int a = seed + 1;
    int b = seed + 2;
    int c = seed + 3;
    int d = seed + 4;
    int e = seed + 5;
    int f = seed + 6;
    double x = fseed + 0.25;
    double y = fseed + 0.50;
    double z = fseed + 0.75;
    int inner = level6(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L5 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, inner);
    return inner + a + b + c + d + e + f;
}

static int level4(int seed, double fseed, const char *tag)
{
    int a = seed + 1;
    int b = seed + 2;
    int c = seed + 3;
    int d = seed + 4;
    int e = seed + 5;
    int f = seed + 6;
    double x = fseed + 0.25;
    double y = fseed + 0.50;
    double z = fseed + 0.75;
    int inner = level5(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L4 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, inner);
    return inner + a + b + c + d + e + f;
}

static int level3(int seed, double fseed, const char *tag)
{
    int a = seed + 1;
    int b = seed + 2;
    int c = seed + 3;
    int d = seed + 4;
    int e = seed + 5;
    int f = seed + 6;
    double x = fseed + 0.25;
    double y = fseed + 0.50;
    double z = fseed + 0.75;
    int inner = level4(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L3 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, inner);
    return inner + a + b + c + d + e + f;
}

static int level2(int seed, double fseed, const char *tag)
{
    int a = seed + 1;
    int b = seed + 2;
    int c = seed + 3;
    int d = seed + 4;
    int e = seed + 5;
    int f = seed + 6;
    double x = fseed + 0.25;
    double y = fseed + 0.50;
    double z = fseed + 0.75;
    int inner = level3(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L2 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, inner);
    return inner + a + b + c + d + e + f;
}

static int level1(int seed, double fseed, const char *tag)
{
    int a = seed + 1;
    int b = seed + 2;
    int c = seed + 3;
    int d = seed + 4;
    int e = seed + 5;
    int f = seed + 6;
    double x = fseed + 0.25;
    double y = fseed + 0.50;
    double z = fseed + 0.75;
    int inner = level2(seed + 10, fseed + 1.0, tag);
    printf("nest_%s_L1 a=%d b=%d c=%d d=%d e=%d f=%d x=%.4f y=%.4f z=%.4f inner=%d\n",
           tag, a, b, c, d, e, f, x, y, z, inner);
    return inner + a + b + c + d + e + f;
}

int main(void)
{
    int folded_total = level1(100, 1.0, "folded");
    int runtime_total;
    printf("nest_folded_total=%d\n", folded_total);
    runtime_total = level1(vseed, vfseed, vtag);
    printf("nest_runtime_total=%d\n", runtime_total);
    return 0;
}
