/* The dense and the sparse label set are both present because label density is
 * one of the things a compiler may use when choosing how to lower a switch, so a
 * corpus with only dense switches could leave one of those paths unvisited.  The
 * observable result is the same either way, which is what the printed values
 * check. */

int printf(const char *, ...);

static int empty_body(int v)
{
    int r = 5;

    switch (v) {
    }
    return r;
}

static int default_first(int v)
{
    int r;

    switch (v) {
    default:
        r = 1;
        break;
    case 10:
        r = 2;
        break;
    case 20:
        r = 3;
        break;
    }
    return r;
}

static int default_absent(int v)
{
    int r = 0;

    switch (v) {
    case 1:
        r = 11;
        break;
    case 2:
        r = 22;
        break;
    }
    r += 100;
    return r;
}

static int single_label(int v)
{
    int r = 0;

    switch (v) {
    case 42:
        r = 1;
        break;
    }
    return r;
}

static int unordered_labels(int v)
{
    int r;

    switch (v) {
    case 30:
        r = 3;
        break;
    case 10:
        r = 1;
        break;
    case 20:
        r = 2;
        break;
    default:
        r = 0;
        break;
    }
    return r;
}

static int dense(int v)
{
    int r;

    switch (v) {
    case 0: r = 100; break;
    case 1: r = 101; break;
    case 2: r = 102; break;
    case 3: r = 103; break;
    case 4: r = 104; break;
    case 5: r = 105; break;
    case 6: r = 106; break;
    case 7: r = 107; break;
    case 8: r = 108; break;
    case 9: r = 109; break;
    case 10: r = 110; break;
    case 11: r = 111; break;
    case 12: r = 112; break;
    case 13: r = 113; break;
    case 14: r = 114; break;
    case 15: r = 115; break;
    default: r = 999; break;
    }
    return r;
}

static int dense_offset(int v)
{
    int r;

    switch (v) {
    case 200: r = 1; break;
    case 201: r = 2; break;
    case 202: r = 3; break;
    case 203: r = 4; break;
    case 204: r = 5; break;
    case 205: r = 6; break;
    case 206: r = 7; break;
    case 207: r = 8; break;
    default: r = 0; break;
    }
    return r;
}

static int sparse(int v)
{
    int r;

    switch (v) {
    case -1000000: r = 1; break;
    case 3: r = 2; break;
    case 97: r = 3; break;
    case 1000: r = 4; break;
    case 65535: r = 5; break;
    case 1000000: r = 6; break;
    default: r = 0; break;
    }
    return r;
}

int main(void)
{
    static const int dense_in[6] = { 0, 1, 7, 15, 16, -1 };
    static const int dense_off_in[4] = { 200, 204, 207, 208 };
    static const int sparse_in[8] = {
        -1000000, 3, 97, 1000, 65535, 1000000, 0, 999999
    };
    volatile int vin;
    int out[8];
    int i;

    printf("const_empty_body=%d %d\n", empty_body(3), empty_body(0));
    vin = 3;
    out[0] = empty_body(vin);
    vin = 0;
    out[1] = empty_body(vin);
    printf("runtime_empty_body=%d %d\n", out[0], out[1]);

    printf("const_default_first=%d %d %d\n",
           default_first(10), default_first(20), default_first(99));
    vin = 10;
    out[0] = default_first(vin);
    vin = 20;
    out[1] = default_first(vin);
    vin = 99;
    out[2] = default_first(vin);
    printf("runtime_default_first=%d %d %d\n", out[0], out[1], out[2]);

    printf("const_default_absent=%d %d %d\n",
           default_absent(1), default_absent(2), default_absent(3));
    vin = 1;
    out[0] = default_absent(vin);
    vin = 2;
    out[1] = default_absent(vin);
    vin = 3;
    out[2] = default_absent(vin);
    printf("runtime_default_absent=%d %d %d\n", out[0], out[1], out[2]);

    printf("const_single_label=%d %d\n", single_label(42), single_label(41));
    vin = 42;
    out[0] = single_label(vin);
    vin = 41;
    out[1] = single_label(vin);
    printf("runtime_single_label=%d %d\n", out[0], out[1]);

    printf("const_unordered_labels=%d %d %d %d\n",
           unordered_labels(10), unordered_labels(20),
           unordered_labels(30), unordered_labels(40));
    vin = 10;
    out[0] = unordered_labels(vin);
    vin = 20;
    out[1] = unordered_labels(vin);
    vin = 30;
    out[2] = unordered_labels(vin);
    vin = 40;
    out[3] = unordered_labels(vin);
    printf("runtime_unordered_labels=%d %d %d %d\n",
           out[0], out[1], out[2], out[3]);

    printf("const_dense=%d %d %d %d %d %d\n",
           dense(0), dense(1), dense(7), dense(15), dense(16), dense(-1));
    for (i = 0; i < 6; i++) {
        vin = dense_in[i];
        out[i] = dense(vin);
    }
    printf("runtime_dense=%d %d %d %d %d %d\n",
           out[0], out[1], out[2], out[3], out[4], out[5]);

    printf("const_dense_offset=%d %d %d %d\n",
           dense_offset(200), dense_offset(204),
           dense_offset(207), dense_offset(208));
    for (i = 0; i < 4; i++) {
        vin = dense_off_in[i];
        out[i] = dense_offset(vin);
    }
    printf("runtime_dense_offset=%d %d %d %d\n",
           out[0], out[1], out[2], out[3]);

    printf("const_sparse=%d %d %d %d %d %d %d %d\n",
           sparse(-1000000), sparse(3), sparse(97), sparse(1000),
           sparse(65535), sparse(1000000), sparse(0), sparse(999999));
    for (i = 0; i < 8; i++) {
        vin = sparse_in[i];
        out[i] = sparse(vin);
    }
    printf("runtime_sparse=%d %d %d %d %d %d %d %d\n",
           out[0], out[1], out[2], out[3], out[4], out[5], out[6], out[7]);

    out[0] = 0;
    for (i = 0; i < 16; i++) {
        vin = i;
        out[0] += dense(vin);
    }
    printf("dense_sweep_total=%d\n", out[0]);

    out[1] = 0;
    for (i = 0; i < 8; i++) {
        vin = sparse_in[i];
        out[1] += sparse(vin);
    }
    printf("sparse_sweep_total=%d\n", out[1]);
    return 0;
}
