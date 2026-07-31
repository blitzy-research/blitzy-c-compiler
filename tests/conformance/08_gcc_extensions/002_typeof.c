/* Area 08 - GCC extensions: typeof / __typeof__ on expressions and declarations. */
int printf(const char *, ...);

#define TO_SWAP(a, b) do { __typeof__(a) sw_t = (a); (a) = (b); (b) = sw_t; } while (0)
#define TO_ADD(a, b)  ({ __typeof__((a) + (b)) ad_t = (a) + (b); ad_t; })

int main(void)
{
    int fa = 40;
    typeof(fa) fb = 2;
    __typeof__(fa) fc = fa + fb;
    typeof(fa + fb) fd = 100;
    unsigned int fu = 3u;
    __typeof__(fu) fv = fu * 5u;
    int *fp = &fa;
    typeof(fp) fq = fp;
    typeof(*fp) fr = *fq + 1;
    int f_swap_a = 11, f_swap_b = 22;
    int f_add = TO_ADD(fa, fb);

    volatile int va = 40;
    volatile int vb = 2;
    __typeof__(va) rc = va + vb;
    typeof(va + vb) rd = va * vb;
    volatile unsigned int vu = 3u;
    __typeof__(vu) rv = vu * 5u;
    int r_swap_a = 11, r_swap_b = 22;
    int r_add = TO_ADD(va, vb);

    TO_SWAP(f_swap_a, f_swap_b);
    TO_SWAP(r_swap_a, r_swap_b);

    printf("to_folded_b=%d\n", fb);
    printf("to_folded_c=%d\n", fc);
    printf("to_folded_d=%d\n", fd);
    printf("to_folded_unsigned=%u\n", fv);
    printf("to_folded_deref=%d\n", *fq);
    printf("to_folded_elem=%d\n", fr);
    printf("to_folded_add=%d\n", f_add);
    printf("to_folded_swap=%d %d\n", f_swap_a, f_swap_b);
    printf("to_same_as_int=%d\n", (unsigned)sizeof(typeof(fa)) == (unsigned)sizeof(int));
    printf("to_same_as_ptr=%d\n", (unsigned)sizeof(typeof(fp)) == (unsigned)sizeof(int *));
    printf("to_long_rank=%d\n", (unsigned)sizeof(__typeof__(fa + 1L)) == (unsigned)sizeof(long));
    printf("to_unsigned_rank=%d\n", (unsigned)sizeof(__typeof__(fu)) == (unsigned)sizeof(unsigned int));
    printf("to_runtime_c=%d\n", rc);
    printf("to_runtime_d=%d\n", rd);
    printf("to_runtime_unsigned=%u\n", rv);
    printf("to_runtime_add=%d\n", r_add);
    printf("to_runtime_swap=%d %d\n", r_swap_a, r_swap_b);
    return 0;
}
