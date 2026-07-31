/* 002_loop_invariance.c -- Area 09, differential conformance suite.
 *
 * Claim under test: loop results are IDENTICAL at -O0, -O1 and -O2, whether or
 * not the compiler performs loop-invariant code motion, strength reduction or
 * unrolling.  bcc documents none of unrolling, vectorization or IPA, while the
 * reference compiler performs all of them; that asymmetry must not change any
 * printed value.  Every bound and factor is read from volatile storage so no
 * loop can be folded away into a constant.
 */

int printf(const char *, ...);

int main(void)
{
    /* Bounds and factors come from volatile storage so no loop can be folded
       away into a constant.  All bounds are small and every accumulator stays
       far inside the int range, so no signed overflow is possible. */
    volatile int v_n = 10;
    volatile int v_k = 3;
    volatile int v_m = 7;
    volatile int v_big = 1000;

    int n = v_n, k = v_k, m = v_m, big = v_big;
    int i, j;

    /* strength reduction candidate: multiply by a loop-invariant factor */
    int sum_mul = 0;
    for (i = 0; i < n; i++) {
        sum_mul += i * k;
    }

    /* loop-invariant code motion candidate: k * m never changes */
    int sum_inv = 0;
    for (i = 0; i < n; i++) {
        sum_inv += k * m;
    }

    /* power-of-two multiply: classic strength reduction to a shift */
    int sum_pow2 = 0;
    for (i = 0; i < n; i++) {
        sum_pow2 += i * 8;
    }

    /* larger trip count, still far inside range: 999*1000/2 = 499500 */
    int sum_big = 0;
    for (i = 0; i < big; i++) {
        sum_big += i;
    }

    /* while loop with a decrementing induction variable */
    int down = 0;
    i = n;
    while (i > 0) {
        down += i;
        i--;
    }

    /* do-while executes at least once even when the bound is zero */
    int once = 0;
    i = 0;
    do {
        once += 5;
        i++;
    } while (i < 0);

    /* nested loops: 10 * 7 iterations, product accumulated */
    int nested = 0;
    for (i = 0; i < n; i++) {
        for (j = 0; j < m; j++) {
            nested += (i + 1) * (j + 1);
        }
    }

    /* loop with an early break and a continue */
    int guarded = 0;
    for (i = 0; i < big; i++) {
        if (i % 3 == 0) {
            continue;
        }
        if (i > 20) {
            break;
        }
        guarded += i;
    }

    printf("sum_mul=%d\n", sum_mul);
    printf("sum_inv=%d\n", sum_inv);
    printf("sum_pow2=%d\n", sum_pow2);
    printf("sum_big=%d\n", sum_big);
    printf("down=%d\n", down);
    printf("once=%d\n", once);
    printf("nested=%d\n", nested);
    printf("guarded=%d\n", guarded);
    return 0;
}
