/* The restrict-qualified parameters are used truthfully: dest_data and source_data
 * are distinct file-scope arrays that never alias. */

int printf(const char *, ...);

static const int const_source = 42;

static int dest_data[4];
static const int source_data[4] = { 1, 2, 3, 4 };

static void scale_into(int *restrict dst, const int *restrict src, int n, int k)
{
    int i;
    for (i = 0; i < n; i++) {
        dst[i] = src[i] * k;
    }
}

int main(void)
{
    volatile int ticks = 0;
    int target = 17;
    int *const const_ptr_to_int = &target;
    const int *ptr_to_const = &const_source;
    int sum = 0;
    int i;

    ticks = ticks + 1;
    ticks = ticks + 2;
    ticks = ticks + 3;

    scale_into(dest_data, source_data, 4, 3);
    for (i = 0; i < 4; i++) {
        sum = sum + dest_data[i];
    }

    *const_ptr_to_int = 19;

    printf("const_read=%d\n", const_source);
    printf("volatile_seq=%d\n", ticks);
    printf("restrict_scaled=%d %d %d %d sum=%d\n",
           dest_data[0], dest_data[1], dest_data[2], dest_data[3], sum);
    printf("const_qualified_ptr_target=%d\n", *ptr_to_const);
    printf("const_pointer_write=%d\n", *const_ptr_to_int);
    return 0;
}
