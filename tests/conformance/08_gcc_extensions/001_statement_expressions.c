/* Area 08 - GCC extensions: statement expressions in value position. */
int printf(const char *, ...);

#define SE_MAX(a, b)   ({ int mx_a = (a); int mx_b = (b); mx_a > mx_b ? mx_a : mx_b; })
#define SE_SQUARE(v)   ({ int sq_v = (v); sq_v * sq_v; })
#define SE_TRIANGLE(n) ({ int tr_s = 0; int tr_i; for (tr_i = 1; tr_i <= (n); tr_i++) { tr_s += tr_i; } tr_s; })

int main(void)
{
    int f_max      = SE_MAX(7, 12);
    int f_square   = SE_SQUARE(9);
    int f_triangle = SE_TRIANGLE(10);
    int f_nested   = SE_MAX(SE_SQUARE(3), SE_TRIANGLE(4));
    int f_direct   = ({ int d_t = 5; d_t + 1; });
    int f_void_val = ({ int w_t = 0; w_t += 3; w_t += 4; w_t; });

    volatile int v7 = 7, v12 = 12, v9 = 9, v10 = 10, v3 = 3, v4 = 4, v5 = 5;
    int r_max      = SE_MAX(v7, v12);
    int r_square   = SE_SQUARE(v9);
    int r_triangle = SE_TRIANGLE(v10);
    int r_nested   = SE_MAX(SE_SQUARE(v3), SE_TRIANGLE(v4));
    int r_direct   = ({ int e_t = v5; e_t + 1; });
    int r_void_val = ({ int y_t = 0; y_t += v3; y_t += v4; y_t; });

    printf("se_folded_max=%d\n", f_max);
    printf("se_folded_square=%d\n", f_square);
    printf("se_folded_triangle=%d\n", f_triangle);
    printf("se_folded_nested=%d\n", f_nested);
    printf("se_folded_direct=%d\n", f_direct);
    printf("se_folded_seq=%d\n", f_void_val);
    printf("se_folded_arg=%d\n", SE_MAX(2, ({ int a_t = 8; a_t - 3; })));
    printf("se_runtime_max=%d\n", r_max);
    printf("se_runtime_square=%d\n", r_square);
    printf("se_runtime_triangle=%d\n", r_triangle);
    printf("se_runtime_nested=%d\n", r_nested);
    printf("se_runtime_direct=%d\n", r_direct);
    printf("se_runtime_seq=%d\n", r_void_val);
    printf("se_runtime_arg=%d\n", SE_MAX(2, ({ int b_t = v5; b_t + 3; })));
    return 0;
}
