/* The discriminating cases are values whose low bits are zero but which are
 * nonzero overall: a conforming conversion to _Bool yields 1, while a
 * truncating implementation would yield 0.
 */
int printf(const char *, ...);

static const char *bool_promotes(void)
{
    volatile _Bool b = 1;
    return _Generic(+b, int: "int", unsigned int: "unsigned int", default: "other");
}

static const char *bool_sum_type(void)
{
    volatile _Bool a = 1;
    volatile _Bool b = 1;
    return _Generic(a + b, int: "int", _Bool: "_Bool", default: "other");
}

int main(void)
{
    volatile int i_zero = 0;
    volatile int i_one = 1;
    volatile int i_neg = -1;
    volatile int i_256 = 256;
    volatile unsigned int u_high = 0x10000u;
    volatile long long l_high = 0x100000000LL;
    volatile double d_small = 0.25;
    volatile double d_zero = 0.0;
    volatile _Bool b_true = 1;
    volatile _Bool b_false = 0;

    printf("fold_from_zero=%d\n", (int)(_Bool)0);
    printf("fold_from_one=%d\n", (int)(_Bool)1);
    printf("fold_from_neg=%d\n", (int)(_Bool)(-1));
    printf("fold_from_256=%d\n", (int)(_Bool)256);
    printf("fold_from_u_high=%d\n", (int)(_Bool)0x10000u);
    printf("fold_from_l_high=%d\n", (int)(_Bool)0x100000000LL);
    printf("fold_from_double=%d\n", (int)(_Bool)0.25);
    printf("fold_from_double_zero=%d\n", (int)(_Bool)0.0);
    printf("fold_back_to_int=%d\n", (int)(_Bool)5 + 10);
    printf("fold_sum=%d\n", (int)((_Bool)1 + (_Bool)1));
    printf("fold_not=%d\n", !(_Bool)0);
    printf("fold_negate=%d\n", -(int)(_Bool)1);

    /* Runtime variant: the sources are volatile, so each conversion is applied
     * to a value read at run time rather than to a constant. */
    printf("run_from_zero=%d\n", (int)(_Bool)i_zero);
    printf("run_from_one=%d\n", (int)(_Bool)i_one);
    printf("run_from_neg=%d\n", (int)(_Bool)i_neg);
    printf("run_from_256=%d\n", (int)(_Bool)i_256);
    printf("run_from_u_high=%d\n", (int)(_Bool)u_high);
    printf("run_from_l_high=%d\n", (int)(_Bool)l_high);
    printf("run_from_double=%d\n", (int)(_Bool)d_small);
    printf("run_from_double_zero=%d\n", (int)(_Bool)d_zero);
    printf("run_back_to_int=%d\n", (int)b_true + 10);
    printf("run_sum=%d\n", (int)b_true + (int)b_true);
    printf("run_not=%d\n", !b_false);
    printf("run_negate=%d\n", -(int)b_true);

    printf("bool_promotes=%s\n", bool_promotes());
    printf("bool_sum_type=%s\n", bool_sum_type());
    return 0;
}
