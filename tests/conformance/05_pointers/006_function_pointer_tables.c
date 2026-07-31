/* No function address is printed: which entry a call reached is observed through
 * equality comparisons, through index differences cast to long long, and through
 * the returned values. */

int printf(const char *, ...);

typedef int (*binop)(int, int);
typedef int (*unop)(int);
typedef int (*nilop)(void);

static int op_add(int x, int y)
{
    return x + y;
}

static int op_sub(int x, int y)
{
    return x - y;
}

static int op_mul(int x, int y)
{
    return x * y;
}

static int op_min(int x, int y)
{
    return x < y ? x : y;
}

static int op_max(int x, int y)
{
    return x > y ? x : y;
}

static int op_first(int x, int y)
{
    (void)y;
    return x;
}

static int un_neg(int x)
{
    return -x;
}

static int un_dbl(int x)
{
    return x * 2;
}

static int un_sq(int x)
{
    return x * x;
}

static int nil_one(void)
{
    return 1;
}

static int nil_two(void)
{
    return 2;
}

static int nil_three(void)
{
    return 3;
}

static const binop bin_table[6] = {
    op_add, op_sub, op_mul, op_min, op_max, op_first
};

static const unop un_table[3] = { un_neg, un_dbl, un_sq };

static const nilop nil_table[3] = { nil_one, nil_two, nil_three };

static const binop grid[2][3] = {
    { op_add, op_sub, op_mul },
    { op_min, op_max, op_first }
};

static int dispatch(const binop *tab, int idx, int x, int y)
{
    return tab[idx](x, y);
}

int main(void)
{
    volatile int vidx;
    int k;
    int i;
    int j;
    int folded_sweep_acc = 0;

    printf("bin0=%d\n", bin_table[0](12, 5));
    printf("bin1=%d\n", bin_table[1](12, 5));
    printf("bin2=%d\n", bin_table[2](12, 5));
    printf("bin3=%d\n", bin_table[3](12, 5));
    printf("bin4=%d\n", bin_table[4](12, 5));
    printf("bin5=%d\n", bin_table[5](12, 5));

    printf("bin0_deref=%d\n", (*bin_table[0])(20, 4));
    printf("bin1_deref=%d\n", (*bin_table[1])(20, 4));

    printf("bin_identity0=%d\n", bin_table[0] == op_add);
    printf("bin_identity5=%d\n", bin_table[5] == op_first);
    printf("bin_distinct=%d\n", bin_table[0] != bin_table[1]);

    printf("un0=%d\n", un_table[0](9));
    printf("un1=%d\n", un_table[1](9));
    printf("un2=%d\n", un_table[2](9));
    printf("nil0=%d\n", nil_table[0]());
    printf("nil1=%d\n", nil_table[1]());
    printf("nil2=%d\n", nil_table[2]());

    printf("grid00=%d\n", grid[0][0](7, 3));
    printf("grid01=%d\n", grid[0][1](7, 3));
    printf("grid02=%d\n", grid[0][2](7, 3));
    printf("grid10=%d\n", grid[1][0](7, 3));
    printf("grid11=%d\n", grid[1][1](7, 3));
    printf("grid12=%d\n", grid[1][2](7, 3));

    {
        const binop *tp = bin_table;
        printf("decay_first=%d\n", (*tp)(30, 6));
        ++tp;
        printf("decay_second=%d\n", (*tp)(30, 6));
        tp += 2;
        printf("decay_fourth=%d\n", (*tp)(30, 6));
        printf("decay_index=%lld\n", (long long)(tp - bin_table));
        --tp;
        printf("decay_third=%d\n", (*tp)(30, 6));
        printf("decay_span=%lld\n", (long long)((bin_table + 6) - bin_table));
        /* grid is an array of two rows, so grid and grid + 2 point into one
         * and the same array object and their difference is well defined.
         * The distance from the first cell of row 0 to the first cell of
         * row 1 is NOT obtained as &grid[1][0] - &grid[0][0]: those two
         * pointers designate elements of two different inner arrays, and
         * pointer subtraction is defined only within a single array object.
         * The contiguity of the enclosing two-dimensional array does not
         * extend that permission.  The distance is therefore built from a
         * legal row difference scaled by the row width taken from the type. */
        printf("decay_row_span=%lld\n", (long long)((grid + 2) - grid));
        /* The distance from one row's first cell to the next row's first cell,
         * expressed as a difference between ROW pointers -- which are elements
         * of the same array object `grid` -- multiplied by the column count.
         * Subtracting &grid[0][0] from &grid[1][0] instead would be undefined:
         * those two pointers belong to two distinct row-array objects, and the
         * rows being laid out contiguously does not make a difference across
         * them meaningful. */
        printf("decay_cell_span=%lld\n",
               (long long)((grid + 1) - grid)
                   * (long long)(sizeof grid[0] / sizeof grid[0][0]));
    }

    printf("dispatch_param0=%d\n", dispatch(bin_table, 0, 40, 8));
    printf("dispatch_param2=%d\n", dispatch(bin_table, 2, 40, 8));
    printf("dispatch_param_row1=%d\n", dispatch(grid[1], 1, 40, 8));

    for (i = 0; i < 6; ++i) {
        folded_sweep_acc += bin_table[i](10, 4);
    }
    printf("folded_sweep=%d\n", folded_sweep_acc);

    /* Runtime variant: each index is read from volatile storage, so which table
     * entry a call reaches is not decidable before the program runs. */
    vidx = 0;
    k = vidx;
    printf("runtime_bin=%d\n", bin_table[k](12, 5));
    printf("runtime_bin_identity=%d\n", bin_table[k] == op_add);

    vidx = 3;
    k = vidx;
    printf("runtime_bin_min=%d\n", bin_table[k](12, 5));
    printf("runtime_bin_deref=%d\n", (*bin_table[k])(12, 5));

    vidx = 5;
    k = vidx;
    printf("runtime_bin_last=%d\n", bin_table[k](12, 5));
    printf("runtime_bin_index=%lld\n",
           (long long)(&bin_table[k] - bin_table));

    vidx = 2;
    k = vidx;
    printf("runtime_un=%d\n", un_table[k](9));
    printf("runtime_nil=%d\n", nil_table[k]());

    vidx = 1;
    k = vidx;
    printf("runtime_grid=%d\n", grid[k][k](7, 3));
    printf("runtime_grid_other=%d\n", grid[k - 1][k + 1](7, 3));
    printf("runtime_dispatch=%d\n", dispatch(grid[k], k, 40, 8));
    printf("runtime_dispatch_table=%d\n", dispatch(bin_table, k, 40, 8));

    {
        int acc = 0;
        int n;
        vidx = 6;
        n = vidx;
        for (i = 0; i < n; ++i) {
            acc += bin_table[i](10, 4);
        }
        printf("runtime_sweep=%d\n", acc);
        printf("runtime_sweep_matches_folded=%d\n", acc == folded_sweep_acc);
    }

    {
        int acc = 0;
        int rows;
        int cols;
        vidx = 2;
        rows = vidx;
        vidx = 3;
        cols = vidx;
        for (i = 0; i < rows; ++i) {
            for (j = 0; j < cols; ++j) {
                acc += grid[i][j](6, 2);
            }
        }
        printf("runtime_grid_sweep=%d\n", acc);
    }

    {
        binop chosen;
        vidx = 4;
        k = vidx;
        chosen = bin_table[k];
        printf("runtime_copied=%d\n", chosen(12, 5));
        printf("runtime_copied_is_max=%d\n", chosen == op_max);
        printf("runtime_copied_from_table=%d\n", chosen == bin_table[4]);
    }

    return 0;
}
