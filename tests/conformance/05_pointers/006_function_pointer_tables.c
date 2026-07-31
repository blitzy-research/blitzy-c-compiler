/* Area 05 - Pointer arithmetic and function pointers
 * 006_function_pointer_tables: dispatch through an array of function
 * pointers.  The tables are file-scope "static const" and the runtime
 * variant indexes them with a volatile index, so the indirect branch
 * cannot be devirtualized into a direct call.
 *
 * This is the highest-yield program in the area: indirect dispatch through
 * a table is the construct most likely to interact with a backend's
 * indirect-branch lowering.  The suite passes no hardening flag, so the
 * default lowering is what is exercised.
 *
 * No header is included; printf is hand-declared.  No function address is
 * ever printed; table facts appear only as equality comparisons, as index
 * differences cast to long long, and as the results of the calls.
 */

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

/* the dispatch tables: file scope, static, const */
static const binop bin_table[6] = {
    op_add, op_sub, op_mul, op_min, op_max, op_first
};

static const unop un_table[3] = { un_neg, un_dbl, un_sq };

static const nilop nil_table[3] = { nil_one, nil_two, nil_three };

/* a two-dimensional table of function pointers */
static const binop grid[2][3] = {
    { op_add, op_sub, op_mul },
    { op_min, op_max, op_first }
};

/* a table reached through a pointer parameter */
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

    /* ---- folded variant: constant table indices ---- */
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

    /* the table decays to a pointer, and the pointer is stepped */
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
        printf("decay_row_span=%lld\n", (long long)((grid + 2) - grid));
        printf("decay_cell_span=%lld\n",
               (long long)(&grid[1][0] - &grid[0][0]));
    }

    printf("dispatch_param0=%d\n", dispatch(bin_table, 0, 40, 8));
    printf("dispatch_param2=%d\n", dispatch(bin_table, 2, 40, 8));
    printf("dispatch_param_row1=%d\n", dispatch(grid[1], 1, 40, 8));

    /* a full sweep of the table with a constant loop bound */
    for (i = 0; i < 6; ++i) {
        folded_sweep_acc += bin_table[i](10, 4);
    }
    printf("folded_sweep=%d\n", folded_sweep_acc);

    /* ---- runtime variant: a volatile index forces a real indirect call ---- */
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

    /* a full sweep with a volatile bound: every call is indirect */
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

    /* a nested sweep over the two-dimensional table, both bounds volatile */
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

    /* a table entry copied into a variable, then called */
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
