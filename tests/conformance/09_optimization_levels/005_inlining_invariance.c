/* 005_inlining_invariance.c -- Area 09, differential conformance suite.
 *
 * Claim under test: small-function results are IDENTICAL at -O0, -O1 and -O2,
 * whether or not the compiler inlines the call.  bcc documents no inlining pass
 * and lists interprocedural optimization as not supported, while the reference
 * compiler inlines small static functions freely; that asymmetry must not change
 * any printed value.  Arguments come from volatile storage so the calls are
 * genuinely materialized at -O0 and cannot be constant-folded at -O1 or -O2.
 */

int printf(const char *, ...);

/* Small static functions: strong inlining candidates at -O1 and -O2, real
   calls at -O0.  The observable result must not depend on which happened. */
static int add3(int a, int b, int c)
{
    return a + b + c;
}

static int scale(int x, int factor)
{
    return x * factor;
}

static int clamp(int x, int lo, int hi)
{
    if (x < lo) {
        return lo;
    }
    if (x > hi) {
        return hi;
    }
    return x;
}

static int accumulate(int n)
{
    int total = 0;
    int i;
    for (i = 1; i <= n; i++) {
        total += i;
    }
    return total;
}

/* A recursive function, which an inliner may partially expand but cannot
   eliminate.  Bounded depth keeps every intermediate far inside int range. */
static int factorial(int n)
{
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

/* Nested static calls: inlining one may expose the other. */
static int combine(int a, int b)
{
    return add3(a, b, scale(a, 2));
}

int main(void)
{
    /* Arguments are sourced from volatile storage so the calls are genuinely
       materialized at -O0 and cannot be constant-folded at -O1 or -O2. */
    volatile int v_a = 4;
    volatile int v_b = 9;
    volatile int v_c = 11;
    volatile int v_n = 7;
    volatile int v_lo = 10;
    volatile int v_hi = 20;

    int a = v_a, b = v_b, c = v_c, n = v_n, lo = v_lo, hi = v_hi;

    printf("add3=%d\n", add3(a, b, c));
    printf("scale=%d\n", scale(a, b));
    printf("clamp_low=%d\n", clamp(a, lo, hi));
    printf("clamp_mid=%d\n", clamp(c + 4, lo, hi));
    printf("clamp_high=%d\n", clamp(c * 3, lo, hi));
    printf("accumulate=%d\n", accumulate(n));
    printf("factorial=%d\n", factorial(n));
    printf("combine=%d\n", combine(a, b));
    printf("nested_calls=%d\n", add3(add3(a, b, c), scale(b, 2), accumulate(n)));
    return 0;
}
