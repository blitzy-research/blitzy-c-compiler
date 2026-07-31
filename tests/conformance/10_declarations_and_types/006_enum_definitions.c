/* Area 10 / 006 -- enumerator value assignment, gaps, negative enumerators and
 * the int-range edges. Every enumerator value is kept inside int range, because
 * the underlying type is otherwise implementation-defined. */

int printf(const char *, ...);

enum color { COLOR_RED, COLOR_GREEN, COLOR_BLUE };
enum sparse { S_A = 10, S_B, S_C = 20, S_D };
enum signed_vals { N_NEG = -5, N_ZERO = 0, N_POS = 7, N_NEXT };
enum bounded { B_LOW = -2147483647 - 1, B_HIGH = 2147483647 };

static int classify(enum sparse s)
{
    switch (s) {
    case S_A: return 100;
    case S_B: return 200;
    case S_C: return 300;
    case S_D: return 400;
    }
    return -1;
}

int main(void)
{
    volatile int selector = 20;
    int folded = classify(S_C);
    int runtime = classify((enum sparse)selector);
    int arith = (int)COLOR_BLUE + (int)S_B;

    printf("color=%d %d %d\n", (int)COLOR_RED, (int)COLOR_GREEN, (int)COLOR_BLUE);
    printf("sparse=%d %d %d %d\n", (int)S_A, (int)S_B, (int)S_C, (int)S_D);
    printf("signed=%d %d %d %d\n", (int)N_NEG, (int)N_ZERO, (int)N_POS, (int)N_NEXT);
    printf("bounded_low_high=%d %d\n", (int)B_LOW, (int)B_HIGH);
    printf("arith=%d %d\n", arith, (int)S_D - (int)S_A);
    printf("classify_folded=%d classify_runtime=%d\n", folded, runtime);
    printf("enum_size_eq_int=%d\n", (int)(sizeof(enum color) == sizeof(int)));
    return 0;
}
