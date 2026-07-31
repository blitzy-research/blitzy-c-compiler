/* 003_volatile_side_effect_preservation.c -- Area 09, conformance suite.
 *
 * Claim under test: the sequence of observable side effects on volatile objects
 * is IDENTICAL at -O0, -O1 and -O2.  This is the only program in the area whose
 * correctness depends on the optimizer NOT doing something: volatile accesses
 * may not be elided, may not be reordered relative to one another, and may not
 * be coalesced.  The printed sequence is therefore the assertion.
 */

int printf(const char *, ...);

/* A genuine volatile object at file scope.  Every read and every write below
   is an observable side effect that no optimization level may elide, reorder
   relative to another volatile access, or coalesce with another access. */
static volatile int g_counter = 0;
static volatile int g_sink = 0;

static int bump(void)
{
    g_counter = g_counter + 1;
    return g_counter;
}

int main(void)
{
    int step1, step2, step3, loop_total, i;
    volatile int v_iterations = 5;
    int iterations = v_iterations;

    printf("counter_initial=%d\n", g_counter);

    step1 = bump();
    printf("after_step1 returned=%d counter=%d\n", step1, g_counter);

    step2 = bump();
    printf("after_step2 returned=%d counter=%d\n", step2, g_counter);

    step3 = bump();
    printf("after_step3 returned=%d counter=%d\n", step3, g_counter);

    /* A sequence of volatile stores that a non-volatile object would allow the
       optimizer to collapse into the final store alone.  Each store here is
       observable, and the value read back after each one is printed. */
    g_sink = 11;
    printf("sink_a=%d\n", g_sink);
    g_sink = 22;
    printf("sink_b=%d\n", g_sink);
    g_sink = 33;
    printf("sink_c=%d\n", g_sink);

    /* A counted loop over a volatile object: the loop cannot be replaced by a
       single add, because each iteration performs an observable read/write. */
    loop_total = 0;
    for (i = 0; i < iterations; i++) {
        g_counter = g_counter + 1;
        loop_total = g_counter;
        printf("loop_step=%d counter=%d\n", i, loop_total);
    }

    printf("counter_final=%d\n", g_counter);
    return 0;
}
