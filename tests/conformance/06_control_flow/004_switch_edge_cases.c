/* 004_switch_edge_cases -- switch statement edge cases, with every label mapping
 * observed individually.
 *
 * Eight switch shapes are compiled here: an empty body, a switch whose default
 * comes first, one with no default at all, one with a single label, one whose
 * labels are written out of numeric order, a DENSE set of sixteen consecutive
 * labels, a dense set OFFSET away from zero, and a SPARSE set spanning from
 * -1000000 to 1000000.  Label density and label origin are both things a compiler
 * may use when choosing how to lower a switch -- a jump table indexed from the
 * lowest label, a jump table after subtracting an offset, a binary search over
 * ranges, or a chain of comparisons -- so a corpus containing only one shape could
 * leave a whole lowering strategy unvisited.  The observable result must be the
 * same whichever strategy is chosen, and that is what the printed values check.
 *
 * EVERY LABEL MAPPING IS PRINTED ON ITS OWN LINE, and that is the property that
 * makes this program able to discriminate at all.  A switch lowered through a jump
 * table fails in a characteristic way: two entries are transposed, or the whole
 * table is displaced by one, so that input i selects the arm belonging to input j
 * and input j selects i's.  A test that only printed the SUM over the label set
 * would see 100 + 101 where it expected 101 + 100 and report agreement, because
 * the two errors cancel exactly.  So each of the sixteen dense labels, each of the
 * eight offset labels, and each of the six sparse labels is printed under its own
 * stable key naming the input that produced it -- const_dense[7]=107,
 * runtime_sparse[65535]=5 -- together with at least one input that must fall
 * through to default, or past a switch that has no default.  A transposition now
 * changes exactly two lines and names both inputs involved.  The two sweep totals
 * at the end are kept only as REDUNDANT confirmation of the per-label lines above
 * them; nothing in this program is observable through a total alone.
 *
 * Each shape is exercised twice, which is the suite's two-variant rule.  The
 * const_ lines pass integer constants, so the compiler is free to resolve the
 * switch entirely at translation time and fold the whole call to an immediate --
 * verified at -O2, where these arms leave no branch behind at all.  The runtime_
 * lines pass a value that has just been read back through a volatile int, which
 * the compiler may not assume anything about, so the switch must be lowered into
 * real dispatch code and the backend's chosen strategy actually runs.  Without the
 * volatile half a folding defect and a lowering defect would be indistinguishable;
 * without the constant half the folding path would never be reached.
 *
 * Portability and determinism: every value computed and printed is an int and %d
 * is the only conversion specifier, so nothing whose width differs between the
 * 32-bit target and the other three is declared, computed or printed, and no
 * character data appears, so plain-char signedness is unreachable.  No address is
 * printed.  Iteration order is fixed by literal bounds and every input comes from
 * a literal in this file.
 *
 * Freedom from undefined behaviour: no case label is duplicated in any switch, so
 * no constraint is violated; the sparse labels -1000000 through 1000000 are all
 * representable in int on every conforming target; the largest accumulated value
 * is the dense sweep total of 1720, far inside INT_MAX; every subscript is
 * strictly inside its array; no pointer is formed; nothing is shifted, divided or
 * cast; each volatile store and the read that follows it are separate full
 * expressions, so no object is modified twice between sequence points and each
 * printf call receives arguments free of side effects. */

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
    /* Every label of each density-sensitive switch, followed by the inputs that
     * must miss it.  The tables drive the runtime_ half; the const_ half spells the
     * same inputs out as integer constants, because a constant is exactly what
     * lets the compiler fold the switch away, and a value loaded from a table
     * would not. */
    static const int dense_in[18] = {
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, -1
    };
    static const int dense_off_in[9] = {
        200, 201, 202, 203, 204, 205, 206, 207, 208
    };
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

    /* All sixteen consecutive labels, then one input above the range and one
     * below it, each on its own line.  A jump table displaced by one entry, or
     * with two entries transposed, changes exactly the lines whose inputs it
     * mismapped and leaves the rest alone. */
    printf("const_dense[0]=%d\n", dense(0));
    printf("const_dense[1]=%d\n", dense(1));
    printf("const_dense[2]=%d\n", dense(2));
    printf("const_dense[3]=%d\n", dense(3));
    printf("const_dense[4]=%d\n", dense(4));
    printf("const_dense[5]=%d\n", dense(5));
    printf("const_dense[6]=%d\n", dense(6));
    printf("const_dense[7]=%d\n", dense(7));
    printf("const_dense[8]=%d\n", dense(8));
    printf("const_dense[9]=%d\n", dense(9));
    printf("const_dense[10]=%d\n", dense(10));
    printf("const_dense[11]=%d\n", dense(11));
    printf("const_dense[12]=%d\n", dense(12));
    printf("const_dense[13]=%d\n", dense(13));
    printf("const_dense[14]=%d\n", dense(14));
    printf("const_dense[15]=%d\n", dense(15));
    printf("const_dense[16]=%d\n", dense(16));
    printf("const_dense[-1]=%d\n", dense(-1));
    for (i = 0; i < 18; i++) {
        vin = dense_in[i];
        out[0] = dense(vin);
        printf("runtime_dense[%d]=%d\n", dense_in[i], out[0]);
    }

    /* All eight labels of the offset set, then one input past the top, each on its
     * own line.  This is the shape a compiler lowers by subtracting the lowest
     * label before indexing, so an off-by-one in that subtraction shifts every
     * mapping by one and is visible here as eight wrong lines rather than as a
     * total that still adds up. */
    printf("const_dense_offset[200]=%d\n", dense_offset(200));
    printf("const_dense_offset[201]=%d\n", dense_offset(201));
    printf("const_dense_offset[202]=%d\n", dense_offset(202));
    printf("const_dense_offset[203]=%d\n", dense_offset(203));
    printf("const_dense_offset[204]=%d\n", dense_offset(204));
    printf("const_dense_offset[205]=%d\n", dense_offset(205));
    printf("const_dense_offset[206]=%d\n", dense_offset(206));
    printf("const_dense_offset[207]=%d\n", dense_offset(207));
    printf("const_dense_offset[208]=%d\n", dense_offset(208));
    for (i = 0; i < 9; i++) {
        vin = dense_off_in[i];
        out[0] = dense_offset(vin);
        printf("runtime_dense_offset[%d]=%d\n", dense_off_in[i], out[0]);
    }

    /* All six sparse labels, then two inputs that must reach default -- one inside
     * the span between labels and one above the highest -- each on its own line.
     * A sparse set is lowered by comparison chain or binary search rather than by
     * table, so the failure to catch here is a mis-ordered or mis-signed
     * comparison, which sends one specific input to the wrong arm; naming the
     * input in the key is what identifies which comparison went wrong. */
    printf("const_sparse[-1000000]=%d\n", sparse(-1000000));
    printf("const_sparse[3]=%d\n", sparse(3));
    printf("const_sparse[97]=%d\n", sparse(97));
    printf("const_sparse[1000]=%d\n", sparse(1000));
    printf("const_sparse[65535]=%d\n", sparse(65535));
    printf("const_sparse[1000000]=%d\n", sparse(1000000));
    printf("const_sparse[0]=%d\n", sparse(0));
    printf("const_sparse[999999]=%d\n", sparse(999999));
    for (i = 0; i < 8; i++) {
        vin = sparse_in[i];
        out[0] = sparse(vin);
        printf("runtime_sparse[%d]=%d\n", sparse_in[i], out[0]);
    }

    /* The two totals are REDUNDANT confirmation of the per-label lines above, kept
     * because they exercise the switch inside a loop -- a context in which a
     * compiler may hoist or restructure the dispatch differently from a
     * straight-line call -- and never as the only evidence for any mapping.  Every
     * input either total covers has already been printed on a line of its own, so
     * a pair of transposed table entries that these sums cannot see is still caught
     * above. */
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
