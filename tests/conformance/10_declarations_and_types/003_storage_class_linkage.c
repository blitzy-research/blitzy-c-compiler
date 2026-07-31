int printf(const char *, ...);

extern int shared_counter;
int shared_counter = 41;

static int file_static = 5;
static int zero_init_probe;

static int internal_triple(int v) { return v * 3; }

static int accumulate(int delta)
{
    static int total = 100;
    total = total + delta;
    return total;
}

int main(void)
{
    volatile int seed = 2;
    int first;
    int second;
    int third;

    shared_counter = shared_counter + 1;
    file_static = file_static + 2;

    first = accumulate(1);
    second = accumulate(seed);
    third = accumulate(0);

    printf("file_static=%d\n", file_static);
    printf("extern_defined_here=%d\n", shared_counter);
    printf("block_static_accum=%d %d %d\n", first, second, third);
    printf("static_fn=%d\n", internal_triple(7));
    printf("zero_init=%d\n", zero_init_probe);
    return 0;
}
