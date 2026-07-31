/* Only well-defined jumps are used: no jump enters the scope of a
 * variably-modified type, since no variable-length array is declared anywhere,
 * and no jump bypasses an initialisation that is later read - every object is
 * declared at the top of its function and given its value by an ordinary
 * assignment rather than by an initialiser a jump could skip. */

int printf(const char *, ...);

static int cleanup_path(int fail_at)
{
    int stage;
    int code;

    stage = 0;
    code = 0;

    stage = 1;
    if (fail_at == 1) {
        code = 11;
        goto cleanup;
    }
    stage = 2;
    if (fail_at == 2) {
        code = 22;
        goto cleanup;
    }
    stage = 3;
cleanup:
    return stage * 100 + code;
}

static int switch_goto(int v)
{
    int r;

    r = 0;
    switch (v) {
    case 1:
        r = 1;
        goto out;
    case 2:
        r = 2;
        break;
    default:
        r = 3;
        goto out;
    }
    r += 10;
out:
    return r;
}

static int goto_loop(int n)
{
    int i;
    int sum;

    i = 0;
    sum = 0;
top:
    if (i >= n)
        goto bottom;
    i++;
    sum += i;
    goto top;
bottom:
    return sum;
}

int main(void)
{
    volatile int vin;
    int i;
    int sum;
    int steps;
    int r;

    r = 1;
    goto skip;
    r = 999;
skip:
    printf("forward_goto=%d\n", r);

    i = 0;
    sum = 0;
again:
    i++;
    sum += i;
    if (i < 5)
        goto again;
    printf("backward_goto sum=%d i=%d\n", sum, i);

    vin = 4;
    sum = 0;
    steps = 0;
    for (i = 0; i < 10; i++) {
        int j;

        for (j = 0; j < 10; j++) {
            {
                steps++;
                if (i * 10 + j >= vin)
                    goto escaped;
                sum += 1;
            }
        }
    }
escaped:
    printf("goto_out_of_nested sum=%d steps=%d\n", sum, steps);

    /* goto into a deeper block.  The block declares no object, so no
       initialisation is bypassed and no variably-modified type is entered. */
    vin = 1;
    r = 0;
    if (vin > 0)
        goto inner;
    r = 500;
    {
        r += 1;
inner:
        r += 10;
    }
    printf("goto_into_block_taken=%d\n", r);

    vin = 0;
    r = 0;
    if (vin > 0)
        goto inner2;
    r = 500;
    {
        r += 1;
inner2:
        r += 10;
    }
    printf("goto_into_block_not_taken=%d\n", r);

    /* A label at the end of a compound statement must be followed by a
       statement; the null statement serves. */
    vin = 1;
    r = 0;
    if (vin > 0)
        goto done_label;
    r = 7;
done_label:
    ;
    printf("null_statement_label=%d\n", r);

    vin = 6;
    sum = 0;
    i = 0;
loop_top:
    if (i >= vin)
        goto loop_end;
    if (i % 2 != 0)
        goto loop_next;
    sum += i;
loop_next:
    i++;
    goto loop_top;
loop_end:
    printf("goto_as_continue sum=%d\n", sum);

    printf("cleanup_path=%d %d %d\n",
           cleanup_path(1), cleanup_path(2), cleanup_path(0));
    vin = 1;
    r = cleanup_path(vin);
    printf("runtime_cleanup_path_first=%d\n", r);
    vin = 2;
    r = cleanup_path(vin);
    printf("runtime_cleanup_path_second=%d\n", r);
    vin = 0;
    r = cleanup_path(vin);
    printf("runtime_cleanup_path_none=%d\n", r);

    printf("switch_goto=%d %d %d\n", switch_goto(1), switch_goto(2), switch_goto(9));
    vin = 1;
    r = switch_goto(vin);
    printf("runtime_switch_goto_one=%d\n", r);
    vin = 2;
    r = switch_goto(vin);
    printf("runtime_switch_goto_two=%d\n", r);
    vin = 9;
    r = switch_goto(vin);
    printf("runtime_switch_goto_default=%d\n", r);

    printf("goto_loop=%d %d %d\n", goto_loop(0), goto_loop(1), goto_loop(10));
    vin = 0;
    r = goto_loop(vin);
    printf("runtime_goto_loop_zero=%d\n", r);
    vin = 10;
    r = goto_loop(vin);
    printf("runtime_goto_loop_ten=%d\n", r);
    return 0;
}
