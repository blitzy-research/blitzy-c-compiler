int printf(const char *, ...);

struct pair { int p; int q; };

static int g_sparse[8]        = { [7] = 70, [3] = 30, [0] = 1 };
static int g_continue[6]      = { [1] = 11, 12, 13, [0] = 10 };
static int g_implied[]        = { [4] = 50 };
static int g_constexpr[6]     = { [2 + 1] = 4, [1 * 2] = 3, [0] = 1 };
static int g_row_then_elem[2][3] = { [0] = { 1, 2, 3 }, [1][0] = 4 };
static int g_elem_then_row[2][3] = { [1][0] = 4, [0] = { 1, 2, 3 } };
static int g_elem_only[2][3]  = { [0][0] = 1, [0][1] = 2, [1][2] = 6 };
static struct pair g_st_memb[3] = { [0].p = 1, [0].q = 2, [2].p = 5, [2].q = 6 };
static struct pair g_st_brace[3] = { [2] = { 5, 6 }, [0] = { 1, 2 } };

int main(void)
{
    int l_sparse[8]   = { [7] = 170, [3] = 130, [0] = 101 };
    int l_continue[6] = { [1] = 111, 112, 113, [0] = 110 };
    int i;
    int j;

    for (i = 0; i < 8; i++) {
        printf("g_sparse[%d]=%d\n", i, g_sparse[i]);
    }
    for (i = 0; i < 6; i++) {
        printf("g_continue[%d]=%d\n", i, g_continue[i]);
    }
    printf("g_implied_len=%d\n", (int)(sizeof g_implied / sizeof g_implied[0]));
    for (i = 0; i < (int)(sizeof g_implied / sizeof g_implied[0]); i++) {
        printf("g_implied[%d]=%d\n", i, g_implied[i]);
    }
    for (i = 0; i < 6; i++) {
        printf("g_constexpr[%d]=%d\n", i, g_constexpr[i]);
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("g_row_then_elem[%d][%d]=%d\n", i, j, g_row_then_elem[i][j]);
        }
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("g_elem_then_row[%d][%d]=%d\n", i, j, g_elem_then_row[i][j]);
        }
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 3; j++) {
            printf("g_elem_only[%d][%d]=%d\n", i, j, g_elem_only[i][j]);
        }
    }
    for (i = 0; i < 3; i++) {
        printf("g_st_memb[%d].p=%d\n", i, g_st_memb[i].p);
        printf("g_st_memb[%d].q=%d\n", i, g_st_memb[i].q);
    }
    for (i = 0; i < 3; i++) {
        printf("g_st_brace[%d].p=%d\n", i, g_st_brace[i].p);
        printf("g_st_brace[%d].q=%d\n", i, g_st_brace[i].q);
    }
    for (i = 0; i < 8; i++) {
        printf("l_sparse[%d]=%d\n", i, l_sparse[i]);
    }
    for (i = 0; i < 6; i++) {
        printf("l_continue[%d]=%d\n", i, l_continue[i]);
    }
    return 0;
}
