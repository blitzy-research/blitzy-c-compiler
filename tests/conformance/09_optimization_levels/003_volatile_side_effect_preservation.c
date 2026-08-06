/* Each counter is written and then read back, and every value read is printed
 * immediately, so the printed trace records the individual accesses rather than
 * only a final result.  That trace is the assertion: it must be the same at -O0,
 * -O1 and -O2. */

int printf(const char *, ...);

/* Both counters are volatile and at file scope, and every access to them below is
   written as its own full expression, so the printed trace pins down the order in
   which the accesses happen and not just the value they end on. */
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
