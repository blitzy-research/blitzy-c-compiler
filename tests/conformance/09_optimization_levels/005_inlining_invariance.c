/* The printed results must not depend on whether a compiler inlines any of these
 * calls.  Every argument is read from volatile storage, so no input is known at
 * compile time and each result is computed from values observed at run time. */

int printf(const char *, ...);

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

/* Bounded depth keeps every intermediate far inside the int range. */
static int factorial(int n)
{
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

static int combine(int a, int b)
{
    return add3(a, b, scale(a, 2));
}

int main(void)
{
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
