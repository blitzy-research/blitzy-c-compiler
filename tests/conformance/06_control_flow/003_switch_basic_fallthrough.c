/* Every deliberate fallthrough below carries the canonical "fall through" comment
 * that GCC's -Wimplicit-fallthrough recognises, which is what marks it as
 * intentional rather than a missing break.  Removing one of those annotations
 * makes the strict warning gate fail. */

int printf(const char *, ...);

static int stacked(int v)
{
    int r;

    switch (v) {
    case 1:
    case 2:
    case 3:
        r = 10;
        break;
    case 4:
    case 5:
        r = 20;
        break;
    default:
        r = 30;
        break;
    }
    return r;
}

static int accumulate(int v)
{
    int r = 0;

    switch (v) {
    case 0:
        r += 1;
        /* fall through */
    case 1:
        r += 2;
        /* fall through */
    case 2:
        r += 4;
        /* fall through */
    case 3:
        r += 8;
        break;
    case 4:
        r += 16;
        break;
    default:
        r += 32;
        break;
    }
    return r;
}

static int through_default(int v)
{
    int r = 0;

    switch (v) {
    case 7:
        r += 1;
        /* fall through */
    default:
        r += 2;
        /* fall through */
    case 8:
        r += 4;
        break;
    case 9:
        r += 8;
        break;
    }
    return r;
}

static int signed_labels(int v)
{
    int r;

    switch (v) {
    case -2:
        r = 100;
        break;
    case -1:
        r = 200;
        break;
    case 0:
        r = 300;
        break;
    default:
        r = 400;
        break;
    }
    return r;
}

static int break_from_nested_block(int v)
{
    int r = 0;

    switch (v) {
    case 1:
        {
            r += 1;
            if (v == 1)
                break;
            r += 100;
        }
        /* fall through */
    case 2:
        r += 1000;
        break;
    default:
        r += 7;
        break;
    }
    return r;
}

int main(void)
{
    static const int in6[6] = { 0, 1, 2, 3, 4, 9 };
    static const int in4[4] = { -2, -1, 0, 5 };
    volatile int vin;
    int out[6];
    int i;

    printf("const_stacked=%d %d %d %d %d %d\n",
           stacked(1), stacked(2), stacked(3), stacked(4), stacked(5), stacked(6));
    for (i = 0; i < 6; i++) {
        vin = i + 1;
        out[i] = stacked(vin);
    }
    printf("runtime_stacked=%d %d %d %d %d %d\n",
           out[0], out[1], out[2], out[3], out[4], out[5]);

    printf("const_accumulate=%d %d %d %d %d %d\n",
           accumulate(0), accumulate(1), accumulate(2),
           accumulate(3), accumulate(4), accumulate(9));
    for (i = 0; i < 6; i++) {
        vin = in6[i];
        out[i] = accumulate(vin);
    }
    printf("runtime_accumulate=%d %d %d %d %d %d\n",
           out[0], out[1], out[2], out[3], out[4], out[5]);

    printf("const_through_default=%d %d %d %d\n",
           through_default(7), through_default(8),
           through_default(9), through_default(99));
    vin = 7;
    out[0] = through_default(vin);
    vin = 8;
    out[1] = through_default(vin);
    vin = 9;
    out[2] = through_default(vin);
    vin = 99;
    out[3] = through_default(vin);
    printf("runtime_through_default=%d %d %d %d\n", out[0], out[1], out[2], out[3]);

    printf("const_signed_labels=%d %d %d %d\n",
           signed_labels(-2), signed_labels(-1), signed_labels(0), signed_labels(5));
    for (i = 0; i < 4; i++) {
        vin = in4[i];
        out[i] = signed_labels(vin);
    }
    printf("runtime_signed_labels=%d %d %d %d\n", out[0], out[1], out[2], out[3]);

    printf("const_break_from_nested_block=%d %d %d\n",
           break_from_nested_block(1), break_from_nested_block(2),
           break_from_nested_block(3));
    vin = 1;
    out[0] = break_from_nested_block(vin);
    vin = 2;
    out[1] = break_from_nested_block(vin);
    vin = 3;
    out[2] = break_from_nested_block(vin);
    printf("runtime_break_from_nested_block=%d %d %d\n", out[0], out[1], out[2]);
    return 0;
}
