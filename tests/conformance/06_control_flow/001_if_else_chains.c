/* 001_if_else_chains.c -- Area 06 control flow.
 * Deep if/else-if chains, three-level nested conditions, an else attached to a
 * nested if, first-match-wins arm selection, and an if with no else -- each in
 * both a constant-folded and a volatile-runtime variant.
 */

int printf(const char *, ...);

static int classify(int v)
{
    if (v < -100)
        return 1;
    else if (v < -10)
        return 2;
    else if (v < 0)
        return 3;
    else if (v == 0)
        return 4;
    else if (v < 10)
        return 5;
    else if (v < 100)
        return 6;
    else
        return 7;
}

static int nested(int a, int b, int c)
{
    int r;

    if (a > 0) {
        if (b > 0) {
            if (c > 0)
                r = 111;
            else
                r = 110;
        } else {
            if (c > 0)
                r = 101;
            else
                r = 100;
        }
    } else {
        if (b > 0) {
            if (c > 0)
                r = 11;
            else
                r = 10;
        } else {
            if (c > 0)
                r = 1;
            else
                r = 0;
        }
    }
    return r;
}

/* The else belongs to the inner if: when a <= 0 neither arm runs at all. */
static int nested_else(int a, int b)
{
    int r = -1;

    if (a > 0) {
        if (b > 0)
            r = 2;
        else
            r = 1;
    }
    return r;
}

/* Only the first arm whose condition holds runs, even when later ones also
 * hold: 6 satisfies both the %2 and the %3 test and must select arm 1.
 */
static int first_match(int v)
{
    int r;

    if (v % 2 == 0)
        r = 1;
    else if (v % 3 == 0)
        r = 2;
    else if (v % 5 == 0)
        r = 3;
    else
        r = 4;
    return r;
}

static int no_else(int a)
{
    int r = 9;

    if (a > 0)
        r = 4;
    return r;
}

int main(void)
{
    static const int samples[7] = { -200, -50, -5, 0, 5, 50, 500 };
    static const int triples[8][3] = {
        { 1, 1, 1 }, { 1, 1, -1 }, { 1, -1, 1 }, { 1, -1, -1 },
        { -1, 1, 1 }, { -1, 1, -1 }, { -1, -1, 1 }, { -1, -1, -1 }
    };
    static const int pairs[4][2] = { { 1, 1 }, { 1, -1 }, { -1, 1 }, { -1, -1 } };
    static const int match_in[5] = { 6, 9, 25, 7, 30 };
    volatile int vin;
    int out[8];
    int i;

    printf("const_classify=%d %d %d %d %d %d %d\n",
           classify(-200), classify(-50), classify(-5), classify(0),
           classify(5), classify(50), classify(500));

    for (i = 0; i < 7; i++) {
        vin = samples[i];
        out[i] = classify(vin);
    }
    printf("runtime_classify=%d %d %d %d %d %d %d\n",
           out[0], out[1], out[2], out[3], out[4], out[5], out[6]);

    printf("const_nested=%d %d %d %d %d %d %d %d\n",
           nested(1, 1, 1), nested(1, 1, -1), nested(1, -1, 1),
           nested(1, -1, -1), nested(-1, 1, 1), nested(-1, 1, -1),
           nested(-1, -1, 1), nested(-1, -1, -1));

    for (i = 0; i < 8; i++) {
        int a;
        int b;
        int c;

        vin = triples[i][0];
        a = vin;
        vin = triples[i][1];
        b = vin;
        vin = triples[i][2];
        c = vin;
        out[i] = nested(a, b, c);
    }
    printf("runtime_nested=%d %d %d %d %d %d %d %d\n",
           out[0], out[1], out[2], out[3], out[4], out[5], out[6], out[7]);

    printf("const_nested_else=%d %d %d %d\n",
           nested_else(1, 1), nested_else(1, -1),
           nested_else(-1, 1), nested_else(-1, -1));

    for (i = 0; i < 4; i++) {
        int a;
        int b;

        vin = pairs[i][0];
        a = vin;
        vin = pairs[i][1];
        b = vin;
        out[i] = nested_else(a, b);
    }
    printf("runtime_nested_else=%d %d %d %d\n", out[0], out[1], out[2], out[3]);

    printf("const_first_match=%d %d %d %d %d\n",
           first_match(6), first_match(9), first_match(25),
           first_match(7), first_match(30));

    for (i = 0; i < 5; i++) {
        vin = match_in[i];
        out[i] = first_match(vin);
    }
    printf("runtime_first_match=%d %d %d %d %d\n",
           out[0], out[1], out[2], out[3], out[4]);

    printf("const_no_else=%d %d\n", no_else(1), no_else(-1));
    vin = 1;
    out[0] = no_else(vin);
    vin = -1;
    out[1] = no_else(vin);
    printf("runtime_no_else=%d %d\n", out[0], out[1]);
    return 0;
}
