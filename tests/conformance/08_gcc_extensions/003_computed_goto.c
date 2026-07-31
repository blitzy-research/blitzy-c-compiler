int printf(const char *, ...);

int main(void)
{
    static void *const fold_tab[3] = { &&f_a, &&f_b, &&f_end };
    static void *const run_tab[3]  = { &&r_a, &&r_b, &&r_end };
    static void *const loop_tab[2] = { &&lp_body, &&lp_exit };
    int f_acc = 0;
    int f_hits = 0;
    volatile int r_idx = 0;
    int r_acc = 0;
    int r_hits = 0;
    volatile int lp_i = 0;
    int lp_sum = 0;

    goto *fold_tab[0];
f_a:
    f_acc += 10;
    f_hits++;
    goto *fold_tab[1];
f_b:
    f_acc *= 3;
    f_hits++;
    goto *fold_tab[2];
f_end:
    printf("cg_folded_acc=%d\n", f_acc);
    printf("cg_folded_hits=%d\n", f_hits);

    r_idx = 0;
    goto *run_tab[r_idx];
r_a:
    r_acc += 10;
    r_hits++;
    r_idx = 1;
    goto *run_tab[r_idx];
r_b:
    r_acc *= 3;
    r_hits++;
    r_idx = 2;
    goto *run_tab[r_idx];
r_end:
    printf("cg_runtime_acc=%d\n", r_acc);
    printf("cg_runtime_hits=%d\n", r_hits);

    goto *loop_tab[(lp_i < 5) ? 0 : 1];
lp_body:
    lp_sum += lp_i;
    lp_i = lp_i + 1;
    goto *loop_tab[(lp_i < 5) ? 0 : 1];
lp_exit:
    printf("cg_loop_sum=%d\n", lp_sum);
    printf("cg_loop_iters=%d\n", lp_i);
    printf("cg_label_addrs_distinct=%d\n",
           (fold_tab[0] != fold_tab[1]) && (fold_tab[1] != fold_tab[2])
           && (run_tab[0] != run_tab[1]) && (loop_tab[0] != loop_tab[1]));
    return 0;
}
