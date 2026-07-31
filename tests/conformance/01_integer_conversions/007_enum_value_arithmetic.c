/* Area 01 / program 007 - enumeration constants in arithmetic, and the
 * behaviour of an enumerated type's underlying representation.
 *
 * The compatible type chosen for an enumerated type is implementation-defined,
 * so it is never printed as a width.  It is asserted only as a relation
 * against int, which holds on all four supported targets.
 */
int printf(const char *, ...);

enum Color { RED = 1, GREEN = 5, BLUE = -3 };
enum Seq { SEQ_A, SEQ_B, SEQ_C };
enum Gap { GAP_LOW = 10, GAP_NEXT, GAP_HIGH = 20, GAP_LAST };

static const char *enumerator_type(void)
{
    return _Generic(RED, int: "int", unsigned int: "unsigned int", default: "other");
}

/* An enumerated type is compatible with some implementation-defined integer
 * type, so listing both `enum Color` and `int` in one _Generic is a constraint
 * violation on implementations where they happen to be compatible.  Only the
 * enumerated type itself is listed here, which is portable.
 */
static const char *enum_object_type(void)
{
    volatile enum Color c = GREEN;
    return _Generic(c, enum Color: "enum Color", default: "other");
}

static const char *enum_sum_type(void)
{
    volatile enum Color a = RED;
    /* int can represent every value of enum Color, so the integer promotions
     * guarantee this expression has type int whatever the compatible type is. */
    return _Generic(a + 0, int: "int", unsigned int: "unsigned int", default: "other");
}

static int classify(enum Color c)
{
    switch (c) {
    case RED:   return 100;
    case GREEN: return 200;
    case BLUE:  return 300;
    }
    return -1;
}

int main(void)
{
    static const int table[3] = { 11, 22, 33 };

    volatile enum Color c_red = RED;
    volatile enum Color c_green = GREEN;
    volatile enum Color c_blue = BLUE;
    volatile enum Seq s_b = SEQ_B;
    volatile enum Gap g_next = GAP_NEXT;
    volatile enum Gap g_last = GAP_LAST;

    /* Folded variant: enumeration constants are constant expressions of type int. */
    printf("fold_values=%d %d %d\n", RED, GREEN, BLUE);
    printf("fold_seq=%d %d %d\n", SEQ_A, SEQ_B, SEQ_C);
    printf("fold_gap=%d %d %d %d\n", GAP_LOW, GAP_NEXT, GAP_HIGH, GAP_LAST);
    printf("fold_difference=%d\n", GREEN - RED);
    printf("fold_product=%d\n", BLUE * 2);
    printf("fold_total=%d\n", RED + GREEN + BLUE);
    printf("fold_index=%d\n", table[SEQ_C]);
    printf("fold_relation=%d\n", GREEN > RED);
    printf("fold_size_relation=%d\n", (int)(sizeof(enum Color) == sizeof(int)));

    /* Runtime variant: volatile enumerated objects force real loads. */
    printf("run_values=%d %d %d\n", (int)c_red, (int)c_green, (int)c_blue);
    printf("run_seq=%d\n", (int)s_b);
    printf("run_gap=%d %d\n", (int)g_next, (int)g_last);
    printf("run_difference=%d\n", (int)c_green - (int)c_red);
    printf("run_product=%d\n", (int)c_blue * 2);
    printf("run_total=%d\n", (int)c_red + (int)c_green + (int)c_blue);
    printf("run_index=%d\n", table[(int)s_b]);
    printf("run_relation=%d\n", (int)c_green > (int)c_red);
    printf("run_switch=%d %d %d\n", classify(c_red), classify(c_green), classify(c_blue));

    /* Types involved. */
    printf("enumerator_type=%s\n", enumerator_type());
    printf("enum_object_type=%s\n", enum_object_type());
    printf("enum_sum_type=%s\n", enum_sum_type());
    return 0;
}
