/* break inside a switch ends the switch and not the enclosing loop, whereas
 * continue inside a switch continues that loop and skips the rest of its body -
 * the two are easy to conflate, so every case below prints which one happened.
 * Each controlling expression is read from volatile storage, so no dispatch is
 * available for compile-time substitution. */

int printf(const char *, ...);

int main(void)
{
    volatile int vsel;
    int i;
    int sum;
    int hits;
    int loops;

    /* break inside a switch ends the switch only; the loop still completes all
       of its iterations and the statement after the switch still runs. */
    sum = 0;
    loops = 0;
    for (i = 0; i < 6; i++) {
        vsel = i;
        switch (vsel) {
        case 0:
            sum += 1;
            break;
        case 1:
            sum += 2;
            break;
        default:
            sum += 4;
            break;
        }
        loops++;
    }
    printf("switch_break_in_for sum=%d loops=%d\n", sum, loops);

    /* continue inside a switch continues the enclosing loop, so the statement
       after the switch is skipped for the values that take that arm. */
    sum = 0;
    hits = 0;
    for (i = 0; i < 6; i++) {
        vsel = i;
        switch (vsel) {
        case 2:
        case 4:
            continue;
        default:
            sum += vsel;
            break;
        }
        hits++;
    }
    printf("switch_continue_in_for sum=%d hits=%d\n", sum, hits);

    /* A loop nested inside a switch case: its break ends only that loop, and
       the following statement in the same case still runs. */
    sum = 0;
    vsel = 1;
    switch (vsel) {
    case 1:
        for (i = 0; i < 10; i++) {
            if (i == 3)
                break;
            sum += 1;
        }
        sum += 100;
        break;
    default:
        sum = -1;
        break;
    }
    printf("loop_break_inside_switch=%d\n", sum);

    sum = 0;
    hits = 0;
    i = 0;
    while (i < 8) {
        vsel = i % 3;
        i++;
        switch (vsel) {
        case 0:
            sum += 1;
            continue;
        case 1:
            sum += 10;
            break;
        default:
            sum += 100;
            break;
        }
        hits++;
    }
    printf("while_switch sum=%d hits=%d\n", sum, hits);

    /* A switch inside a do-while: the switch's break must not end the loop. */
    sum = 0;
    i = 0;
    do {
        vsel = i;
        switch (vsel) {
        case 0:
            sum += 7;
            break;
        default:
            sum += 1;
            break;
        }
        i++;
    } while (i < 4);
    printf("do_while_switch=%d\n", sum);

    /* continue inside a switch inside a do-while: the loop's own controlling
       expression is still evaluated, so the loop terminates normally. */
    sum = 0;
    hits = 0;
    i = 0;
    do {
        vsel = i;
        i++;
        switch (vsel) {
        case 1:
            continue;
        default:
            sum += 1;
            break;
        }
        hits++;
    } while (i < 5);
    printf("do_while_switch_continue sum=%d hits=%d\n", sum, hits);

    sum = 0;
    for (i = 0; i < 3; i++) {
        int j;

        for (j = 0; j < 3; j++) {
            vsel = j;
            switch (i) {
            case 0:
                switch (vsel) {
                case 0:
                    sum += 1;
                    break;
                default:
                    sum += 2;
                    break;
                }
                break;
            case 1:
                sum += 4;
                break;
            default:
                sum += 8;
                break;
            }
        }
    }
    printf("nested_switch=%d\n", sum);

    /* A switch guarding a nested loop's continue: continue inside the inner
       loop but inside a switch continues the INNER loop. */
    sum = 0;
    for (i = 0; i < 3; i++) {
        int j;

        for (j = 0; j < 4; j++) {
            vsel = j;
            switch (vsel) {
            case 1:
                continue;
            default:
                break;
            }
            sum += 1;
        }
        sum += 10;
    }
    printf("inner_continue_from_switch=%d\n", sum);
    return 0;
}
