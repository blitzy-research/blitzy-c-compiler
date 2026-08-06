int printf(const char *, ...);

enum Color { RED = 1, GREEN = 5, BLUE = -3 };
enum Seq { SEQ_A, SEQ_B, SEQ_C };
enum Gap { GAP_LOW = 10, GAP_NEXT, GAP_HIGH = 20, GAP_LAST };

static const char *enumerator_type(void)
{
    return _Generic(RED, int: "int", unsigned int: "unsigned int", default: "other");
}

/* An enumerated type is compatible with some implementation-defined integer
 * type, so listing both `enum Color` and `int` in one _Generic would be a
 * constraint violation wherever the two turn out to be compatible.  Only the
 * enumerated type itself is listed, which is portable.  The compatible type is
 * never printed as a width for the same reason.
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

    printf("fold_values=%d %d %d\n", RED, GREEN, BLUE);
    printf("fold_seq=%d %d %d\n", SEQ_A, SEQ_B, SEQ_C);
    printf("fold_gap=%d %d %d %d\n", GAP_LOW, GAP_NEXT, GAP_HIGH, GAP_LAST);
    printf("fold_difference=%d\n", GREEN - RED);
    printf("fold_product=%d\n", BLUE * 2);
    printf("fold_total=%d\n", RED + GREEN + BLUE);
    printf("fold_index=%d\n", table[SEQ_C]);
    printf("fold_relation=%d\n", GREEN > RED);
    printf("fold_size_relation=%d\n", (int)(sizeof(enum Color) == sizeof(int)));

    /* Runtime variant: the enumerated objects are volatile, so their values are
     * read at run time rather than substituted at compile time.
     *
     * STAGING, AND WHY EVERY VOLATILE READ GETS A STATEMENT OF ITS OWN.  An
     * access to a volatile object is an observable side effect, and C11
     * 6.5.2.2p10 leaves the order of evaluation of a call's arguments
     * unspecified, so a call reading several volatile objects would sequence
     * several side effects in an order the standard does not fix.  Each read
     * therefore happens once, in its own full expression, into a plain int; the
     * calls below pass only plain objects and contain no side effect at all.
     * The suite's authoring rule is at most one side-effecting argument per
     * call, and staging satisfies it structurally rather than by argument.  It
     * costs no discriminating power: a value that arrived through a volatile
     * load stays opaque to the optimizer, so every printed value is still
     * computed at run time rather than folded. */
    int r_red = (int)c_red;
    int r_green = (int)c_green;
    int r_blue = (int)c_blue;
    int r_seq_b = (int)s_b;
    int r_gap_next = (int)g_next;
    int r_gap_last = (int)g_last;

    /* classify reads nothing volatile of its own, but each argument does, so
     * each call is made in a statement of its own for the same reason. */
    int r_class_red = classify(c_red);
    int r_class_green = classify(c_green);
    int r_class_blue = classify(c_blue);

    printf("run_values=%d %d %d\n", r_red, r_green, r_blue);
    printf("run_seq=%d\n", r_seq_b);
    printf("run_gap=%d %d\n", r_gap_next, r_gap_last);
    printf("run_difference=%d\n", r_green - r_red);
    printf("run_product=%d\n", r_blue * 2);
    printf("run_total=%d\n", r_red + r_green + r_blue);
    printf("run_index=%d\n", table[r_seq_b]);
    printf("run_relation=%d\n", r_green > r_red);
    printf("run_switch=%d %d %d\n", r_class_red, r_class_green, r_class_blue);

    printf("enumerator_type=%s\n", enumerator_type());
    printf("enum_object_type=%s\n", enum_object_type());
    printf("enum_sum_type=%s\n", enum_sum_type());
    return 0;
}
