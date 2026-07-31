/* Every loop bound is read from volatile storage, so no iteration count below is
 * available for compile-time substitution. */

int printf(const char *, ...);

int main(void)
{
    volatile int vlimit;
    int n;
    int i;
    int sum;
    int count;

    vlimit = 0;
    n = vlimit;
    count = 0;
    while (count < n)
        count++;
    printf("while_zero=%d\n", count);

    vlimit = 1;
    n = vlimit;
    count = 0;
    while (count < n)
        count++;
    printf("while_one=%d\n", count);

    vlimit = 10;
    n = vlimit;
    sum = 0;
    i = 1;
    while (i <= n) {
        sum += i;
        i++;
    }
    printf("while_sum=%d\n", sum);

    vlimit = 0;
    n = vlimit;
    count = 0;
    do {
        count++;
    } while (count < n);
    printf("do_while_false_cond=%d\n", count);

    vlimit = 1;
    n = vlimit;
    count = 0;
    do {
        count++;
    } while (count < n);
    printf("do_while_one=%d\n", count);

    vlimit = 5;
    n = vlimit;
    sum = 0;
    i = 0;
    do {
        i++;
        sum += i * i;
    } while (i < n);
    printf("do_while_squares=%d\n", sum);

    vlimit = 0;
    n = vlimit;
    count = 0;
    for (i = 0; i < n; i++)
        count++;
    printf("for_zero=%d i_after=%d\n", count, i);

    vlimit = 1;
    n = vlimit;
    count = 0;
    for (i = 0; i < n; i++)
        count++;
    printf("for_one=%d i_after=%d\n", count, i);

    count = 0;
    for (;;) {
        count++;
        if (count == 4)
            break;
    }
    printf("for_forever_break=%d\n", count);

    vlimit = 3;
    n = vlimit;
    count = 0;
    for (i = 0; i < n;) {
        count += 2;
        i++;
    }
    printf("for_no_increment=%d\n", count);

    /* continue skips the remainder of the body but still runs the increment. */
    vlimit = 10;
    n = vlimit;
    sum = 0;
    for (i = 0; i < n; i++) {
        if (i % 2 != 0)
            continue;
        sum += i;
    }
    printf("for_continue_even_sum=%d\n", sum);

    /* continue in a while loop: the increment must be done before it. */
    vlimit = 10;
    n = vlimit;
    sum = 0;
    i = 0;
    while (i < n) {
        i++;
        if (i % 3 == 0)
            continue;
        sum += i;
    }
    printf("while_continue_sum=%d\n", sum);

    vlimit = 4;
    n = vlimit;
    sum = 0;
    count = 0;
    for (i = 0; i < n; i++) {
        int j;

        for (j = 0; j < n; j++) {
            if (j == 2)
                break;
            sum += 1;
        }
        count++;
    }
    printf("nested_break=%d outer_iterations=%d\n", sum, count);

    /* continue affects only the innermost loop. */
    sum = 0;
    for (i = 0; i < n; i++) {
        int j;

        for (j = 0; j < n; j++) {
            if (j == 1)
                continue;
            sum += 1;
        }
    }
    printf("nested_continue=%d\n", sum);

    /* A do-while nested inside a for, with the inner bound re-read from
       volatile storage on every test. */
    sum = 0;
    for (i = 0; i < 3; i++) {
        int k = 0;

        vlimit = i;
        do {
            sum += 1;
            k++;
        } while (k <= vlimit);
    }
    printf("for_do_while_mix=%d\n", sum);

    vlimit = 0;
    n = vlimit;
    count = 0;
    for (i = n; i > 0; i--)
        count++;
    printf("for_countdown_zero=%d\n", count);
    vlimit = 3;
    n = vlimit;
    count = 0;
    for (i = n; i > 0; i--)
        count++;
    printf("for_countdown_three=%d\n", count);
    return 0;
}
