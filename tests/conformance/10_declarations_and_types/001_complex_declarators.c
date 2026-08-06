int printf(const char *, ...);

typedef int (*int_fn_t)(int);
typedef const int *(*slot_fn_t)(int);

static const int slot_table[2] = { 11, 22 };
static int grid[3][3] = { { 1, 2, 3 }, { 4, 5, 6 }, { 7, 8, 9 } };

static int add_six(int v) { return v + 6; }
static int times_two(int v) { return v * 2; }

static const int *pick_slot(int i) { return &slot_table[i & 1]; }

static int_fn_t choose(int flag) { return flag ? add_six : times_two; }

int main(void)
{
    static int_fn_t table[2] = { add_six, times_two };
    static const int *(*direct_table[2])(int) = { pick_slot, pick_slot };
    slot_fn_t typedef_slot = pick_slot;
    int (*row_ptr)[3] = &grid[1];
    volatile int seed = 5;
    int folded = table[0](7) + table[1](7);
    int runtime = table[0](seed) + table[1](seed);
    int choose_on = choose(1)(5);
    int choose_off = choose(0)(5);

    printf("fn_table_folded=%d\n", folded);
    printf("fn_table_runtime=%d\n", runtime);
    printf("slot_values=%d %d\n", *typedef_slot(0), *typedef_slot(1));
    printf("direct_table=%d %d\n", *direct_table[0](0), *direct_table[1](1));
    printf("row_ptr=%d %d %d\n", (*row_ptr)[0], (*row_ptr)[1], (*row_ptr)[2]);
    printf("choose=%d %d\n", choose_on, choose_off);
    printf("sizes=%d %d\n", (int)sizeof grid, (int)sizeof slot_table);
    return 0;
}
