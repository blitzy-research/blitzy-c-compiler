/* The printed results must not depend on which loop transformations, if any, a
 * compiler applies.  The two compilers being compared do not document the same
 * optimization set, and that difference must not be observable in any value. */

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

    int sum_mul = 0;
    for (i = 0; i < n; i++) {
        sum_mul += i * k;
    }

    int sum_inv = 0;
    for (i = 0; i < n; i++) {
        sum_inv += k * m;
    }

    int sum_pow2 = 0;
    for (i = 0; i < n; i++) {
        sum_pow2 += i * 8;
    }

    int sum_big = 0;
    for (i = 0; i < big; i++) {
        sum_big += i;
    }

    int down = 0;
    i = n;
    while (i > 0) {
        down += i;
        i--;
    }

    int once = 0;
    i = 0;
    do {
        once += 5;
        i++;
    } while (i < 0);

    int nested = 0;
    for (i = 0; i < n; i++) {
        for (j = 0; j < m; j++) {
            nested += (i + 1) * (j + 1);
        }
    }

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
