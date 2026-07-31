/* 009_recursion_call_chains.c -- Area 06 control flow.
 * Direct recursion, mutual recursion, and a recursion that keeps several values
 * live across the recursive call so that callee-saved registers must genuinely
 * be spilled and restored.  Every depth is bounded and deliberately shallow:
 * the deepest chain in this program is 31 frames.  That is deep enough to force
 * real call-frame work on every target, and shallow enough that no target can
 * come close to overflowing its stack under user-mode emulation -- the
 * repository's own runtime-verified recursion depths are single digit
 * (factorial(5) and fib(7)), so 31 frames stays in the same order of magnitude.
 *
 * All arithmetic stays well inside the range of a 32-bit int: the largest value
 * produced is 3628800 (10 factorial), so no signed overflow can occur.  Each
 * chain is driven once by a compile-time constant and once by an argument read
 * from volatile storage, so the recursion cannot be folded away.
 */

int printf(const char *, ...);

/* 10! = 3628800, comfortably inside 32-bit int; depth 10. */
static int fact(int n)
{
    if (n <= 1)
        return 1;
    return n * fact(n - 1);
}

/* fib(15) = 610; recursion depth 15, and a branching call tree. */
static int fib(int n)
{
    if (n < 2)
        return n;
    return fib(n - 1) + fib(n - 2);
}

/* Tail-shaped recursion, depth 30 -- the deepest chain here. */
static int sum_to(int n)
{
    if (n <= 0)
        return 0;
    return n + sum_to(n - 1);
}

/* Mutual recursion between two functions, depth n + 1. */
static int is_odd(int n);

static int is_even(int n)
{
    if (n == 0)
        return 1;
    return is_odd(n - 1);
}

static int is_odd(int n)
{
    if (n == 0)
        return 0;
    return is_even(n - 1);
}

/* Mutual recursion where both partners contribute to the result. */
static int pong(int n);

static int ping(int n)
{
    if (n <= 0)
        return 0;
    return 1 + pong(n - 1);
}

static int pong(int n)
{
    if (n <= 0)
        return 0;
    return 2 + ping(n - 1);
}

/* Two values stay live across the recursive call, forcing callee-saved
 * register preservation.  chain(n) = chain(n - 1) + n - 5, chain(0) = 0.
 */
static int chain(int n)
{
    int a;
    int b;
    int deeper;

    if (n == 0)
        return 0;
    a = n * 2;
    b = n + 5;
    deeper = chain(n - 1);
    return deeper + a - b;
}

/* A local array is fully written before any read, and stays live across the
 * recursive call, forcing a real stack frame at every level.
 */
static int frames(int n)
{
    int local[8];
    int i;
    int s;

    for (i = 0; i < 8; i++)
        local[i] = n + i;
    if (n <= 0)
        return 0;
    s = frames(n - 1);
    for (i = 0; i < 8; i++)
        s += local[i];
    return s;
}

/* Recursion whose dispatch goes through a switch at every level. */
static int switched(int n)
{
    int r;

    switch (n) {
    case 0:
        r = 0;
        break;
    case 1:
        r = 1;
        break;
    default:
        r = n + switched(n - 2);
        break;
    }
    return r;
}

int main(void)
{
    volatile int vin;
    int r;

    printf("const_fact=%d %d %d\n", fact(1), fact(5), fact(10));
    vin = 1;
    r = fact(vin);
    printf("runtime_fact_one=%d\n", r);
    vin = 10;
    r = fact(vin);
    printf("runtime_fact_ten=%d\n", r);

    printf("const_fib=%d %d %d %d\n", fib(0), fib(1), fib(7), fib(15));
    vin = 7;
    r = fib(vin);
    printf("runtime_fib_seven=%d\n", r);
    vin = 15;
    r = fib(vin);
    printf("runtime_fib_fifteen=%d\n", r);

    printf("const_sum_to=%d %d %d\n", sum_to(0), sum_to(10), sum_to(30));
    vin = 0;
    r = sum_to(vin);
    printf("runtime_sum_to_zero=%d\n", r);
    vin = 30;
    r = sum_to(vin);
    printf("runtime_sum_to_thirty=%d\n", r);

    printf("const_mutual_parity=%d %d %d %d\n",
           is_even(0), is_even(20), is_odd(20), is_odd(21));
    vin = 20;
    r = is_even(vin);
    printf("runtime_is_even_twenty=%d\n", r);
    vin = 21;
    r = is_odd(vin);
    printf("runtime_is_odd_twentyone=%d\n", r);

    printf("const_ping_pong=%d %d %d\n", ping(0), ping(10), pong(10));
    vin = 10;
    r = ping(vin);
    printf("runtime_ping_ten=%d\n", r);
    vin = 10;
    r = pong(vin);
    printf("runtime_pong_ten=%d\n", r);

    printf("const_chain=%d %d %d\n", chain(0), chain(1), chain(10));
    vin = 10;
    r = chain(vin);
    printf("runtime_chain_ten=%d\n", r);

    printf("const_frames=%d %d\n", frames(0), frames(5));
    vin = 5;
    r = frames(vin);
    printf("runtime_frames_five=%d\n", r);

    printf("const_switched=%d %d %d\n", switched(0), switched(1), switched(10));
    vin = 10;
    r = switched(vin);
    printf("runtime_switched_ten=%d\n", r);
    return 0;
}
