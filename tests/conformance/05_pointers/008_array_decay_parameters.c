/* Area 05 - Pointer arithmetic and function pointers
 * 008_array_decay_parameters: array-to-pointer decay in parameter
 * position, including multidimensional arrays.
 *
 * A parameter declared as an array is adjusted to a pointer to the element
 * type.  For a multidimensional array the element type is itself an array,
 * so the parameter becomes a pointer to an array and stepping it advances a
 * whole row.  All of that is made observable through element values and
 * element-count differences.
 *
 * sizeof of a decayed parameter is pointer width and therefore differs
 * between i686 and the other three targets, so it is never printed.  Only
 * element counts and element-count differences are printed, and every
 * difference is cast to long long.  No address or pointer value is printed.
 */

int printf(const char *, ...);

/* one dimension: [] and * spellings must behave identically */
static int sum_brackets(const int a[], int n)
{
    int total = 0;
    int i;
    for (i = 0; i < n; ++i) {
        total += a[i];
    }
    return total;
}

static int sum_pointer(const int *a, int n)
{
    int total = 0;
    int i;
    for (i = 0; i < n; ++i) {
        total += a[i];
    }
    return total;
}

/* a bound written in the parameter is ignored: it still decays */
static int sum_sized(const int a[8], int n)
{
    int total = 0;
    int i;
    for (i = 0; i < n; ++i) {
        total += a[i];
    }
    return total;
}

/* the parameter is a genuine pointer, so it can be advanced */
static int first_after_step(const int a[], int step)
{
    a += step;
    return *a;
}

/* two dimensions: parameter becomes a pointer to an array of 4 ints */
static int sum_2d_brackets(const int a[][4], int rows)
{
    int total = 0;
    int i;
    int j;
    for (i = 0; i < rows; ++i) {
        for (j = 0; j < 4; ++j) {
            total += a[i][j];
        }
    }
    return total;
}

static int sum_2d_pointer(const int (*a)[4], int rows)
{
    int total = 0;
    int i;
    int j;
    for (i = 0; i < rows; ++i) {
        for (j = 0; j < 4; ++j) {
            total += a[i][j];
        }
    }
    return total;
}

/* stepping the 2-D parameter advances a whole row */
static int row_first(const int (*a)[4], int row)
{
    a += row;
    return (*a)[0];
}

static long long row_stride(const int (*a)[4])
{
    return (long long)((a + 1) - a);
}

static long long row_element_span(const int (*a)[4])
{
    return (long long)(&(*a)[4] - &(*a)[0]);
}

/* three dimensions: parameter becomes a pointer to a 3-by-2 array */
static int sum_3d(const int a[][3][2], int planes)
{
    int total = 0;
    int i;
    int j;
    int m;
    for (i = 0; i < planes; ++i) {
        for (j = 0; j < 3; ++j) {
            for (m = 0; m < 2; ++m) {
                total += a[i][j][m];
            }
        }
    }
    return total;
}

/* indexing a 3-D array once yields a 2-D array, which decays to a pointer
 * to an array of 2 ints */
static int sum_plane(const int (*a)[2], int rows)
{
    int total = 0;
    int i;
    int j;
    for (i = 0; i < rows; ++i) {
        for (j = 0; j < 2; ++j) {
            total += a[i][j];
        }
    }
    return total;
}

/* an array of pointers decays to a pointer to pointer */
static int sum_via_table(const int *const tab[], int n)
{
    int total = 0;
    int i;
    for (i = 0; i < n; ++i) {
        total += *tab[i];
    }
    return total;
}

/* a qualified parameter still decays */
static int sum_const_ptr(const int *const a, int n)
{
    int total = 0;
    int i;
    for (i = 0; i < n; ++i) {
        total += a[i];
    }
    return total;
}

static int flat_index(const int (*a)[4], int row, int col)
{
    const int *base = &a[0][0];
    const int *cell = &a[row][col];
    return (int)(cell - base);
}

static const int one_d[8] = { 1, 2, 3, 4, 5, 6, 7, 8 };
static const int two_d[3][4] = {
    { 10, 11, 12, 13 },
    { 20, 21, 22, 23 },
    { 30, 31, 32, 33 }
};
static const int three_d[2][3][2] = {
    { { 1, 2 }, { 3, 4 }, { 5, 6 } },
    { { 7, 8 }, { 9, 10 }, { 11, 12 } }
};
static const int *const ptr_tab[4] = {
    &one_d[0], &one_d[2], &one_d[4], &one_d[6]
};

int main(void)
{
    volatile int vidx;
    int k;

    /* element counts computed at the caller, where the array type is intact */
    printf("one_d_count=%d\n", (int)(sizeof one_d / sizeof one_d[0]));
    printf("two_d_rows=%d\n", (int)(sizeof two_d / sizeof two_d[0]));
    printf("two_d_cols=%d\n", (int)(sizeof two_d[0] / sizeof two_d[0][0]));
    printf("three_d_planes=%d\n", (int)(sizeof three_d / sizeof three_d[0]));

    /* ---- one dimension: all three spellings agree ---- */
    printf("sum_brackets=%d\n", sum_brackets(one_d, 8));
    printf("sum_pointer=%d\n", sum_pointer(one_d, 8));
    printf("sum_sized=%d\n", sum_sized(one_d, 8));
    printf("sum_const_ptr=%d\n", sum_const_ptr(one_d, 8));
    {
        int via_brackets = sum_brackets(one_d, 8);
        int via_pointer = sum_pointer(one_d, 8);
        int via_sized = sum_sized(one_d, 8);
        printf("spellings_agree=%d\n",
               (via_brackets == via_pointer) && (via_pointer == via_sized));
    }

    /* passing an interior element address, and a partial range */
    printf("sum_from_interior=%d\n", sum_brackets(&one_d[4], 4));
    printf("sum_partial=%d\n", sum_brackets(one_d, 3));
    printf("sum_empty=%d\n", sum_brackets(one_d, 0));
    printf("step_param=%d\n", first_after_step(one_d, 5));
    printf("step_param_zero=%d\n", first_after_step(one_d, 0));

    /* ---- two dimensions ---- */
    printf("sum_2d_brackets=%d\n", sum_2d_brackets(two_d, 3));
    printf("sum_2d_pointer=%d\n", sum_2d_pointer(two_d, 3));
    printf("sum_2d_partial=%d\n", sum_2d_brackets(two_d, 2));
    printf("sum_2d_one_row=%d\n", sum_2d_pointer(&two_d[1], 1));
    printf("row_first0=%d\n", row_first(two_d, 0));
    printf("row_first2=%d\n", row_first(two_d, 2));
    printf("row_stride=%lld\n", row_stride(two_d));
    printf("row_element_span=%lld\n", row_element_span(two_d));
    printf("flat_index_00=%d\n", flat_index(two_d, 0, 0));
    printf("flat_index_12=%d\n", flat_index(two_d, 1, 2));
    printf("flat_index_23=%d\n", flat_index(two_d, 2, 3));

    /* a row of a 2-D array decays to a pointer to int */
    printf("row_as_1d=%d\n", sum_brackets(two_d[1], 4));
    printf("row_as_1d_last=%d\n", sum_pointer(two_d[2], 4));

    /* ---- three dimensions ---- */
    printf("sum_3d=%d\n", sum_3d(three_d, 2));
    printf("sum_3d_one_plane=%d\n", sum_3d(three_d, 1));
    printf("sum_3d_second_plane=%d\n", sum_3d(&three_d[1], 1));
    printf("plane0_as_2d=%d\n", sum_plane(three_d[0], 3));
    printf("plane1_as_2d=%d\n", sum_plane(three_d[1], 3));
    printf("plane_row_as_1d=%d\n", sum_brackets(three_d[1][2], 2));

    /* ---- an array of pointers decays to a pointer to pointer ---- */
    printf("sum_via_table=%d\n", sum_via_table(ptr_tab, 4));
    printf("sum_via_table_partial=%d\n", sum_via_table(ptr_tab, 2));
    printf("table_count=%d\n", (int)(sizeof ptr_tab / sizeof ptr_tab[0]));

    /* ---- runtime variant: volatile bounds and indices ---- */
    vidx = 8;
    k = vidx;
    printf("runtime_sum_1d=%d\n", sum_brackets(one_d, k));
    printf("runtime_sum_pointer=%d\n", sum_pointer(one_d, k));

    vidx = 3;
    k = vidx;
    printf("runtime_sum_2d=%d\n", sum_2d_brackets(two_d, k));
    printf("runtime_row_first=%d\n", row_first(two_d, k - 1));
    printf("runtime_flat_index=%d\n", flat_index(two_d, k - 1, k));
    printf("runtime_row_as_1d=%d\n", sum_brackets(two_d[k - 2], 4));

    vidx = 2;
    k = vidx;
    printf("runtime_sum_3d=%d\n", sum_3d(three_d, k));
    printf("runtime_step=%d\n", first_after_step(one_d, k));
    printf("runtime_from_interior=%d\n", sum_brackets(&one_d[k], k));
    printf("runtime_table=%d\n", sum_via_table(ptr_tab, k));
    printf("runtime_one_row=%d\n", sum_2d_pointer(&two_d[k], 1));
    printf("runtime_plane=%d\n", sum_plane(three_d[k - 1], 3));
    printf("runtime_plane_row=%d\n", sum_brackets(three_d[k - 2][k], 2));

    return 0;
}
