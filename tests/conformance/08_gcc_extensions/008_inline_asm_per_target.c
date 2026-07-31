/* Area 08 - GCC extensions: inline assembly with operand constraints,
   guarded per architecture. Every branch performs the SAME logical
   32-bit integer operation, so printed output is identical on all four
   backends and oracle (b) can compare it directly. */
int printf(const char *, ...);

/* Sum of two ints. Exercises a read-write ("+r") output operand. */
static int asm_add(int a, int b)
{
#if defined(__x86_64__) || defined(__amd64__)
    int r = a;
    __asm__ __volatile__ ("addl %1, %0" : "+r" (r) : "r" (b) : "cc");
    return r;
#elif defined(__i386__) || defined(__i386)
    int r = a;
    __asm__ __volatile__ ("addl %1, %0" : "+r" (r) : "r" (b) : "cc");
    return r;
#elif defined(__aarch64__) || defined(__arm64__)
    int r;
    __asm__ __volatile__ ("add %w0, %w1, %w2" : "=r" (r) : "r" (a), "r" (b));
    return r;
#elif defined(__riscv) || defined(__riscv__)
    int r;
    __asm__ __volatile__ ("addw %0, %1, %2" : "=r" (r) : "r" (a), "r" (b));
    return r;
#else
#error "008_inline_asm_per_target: no inline-assembly branch selected; the target architecture predefined macro (__x86_64__ / __i386__ / __aarch64__ / __riscv) was not recognized"
#endif
}

/* Difference of two ints. Exercises a matching ("0") input constraint. */
static int asm_sub(int a, int b)
{
#if defined(__x86_64__) || defined(__amd64__)
    int r;
    __asm__ __volatile__ ("subl %2, %0" : "=r" (r) : "0" (a), "r" (b) : "cc");
    return r;
#elif defined(__i386__) || defined(__i386)
    int r;
    __asm__ __volatile__ ("subl %2, %0" : "=r" (r) : "0" (a), "r" (b) : "cc");
    return r;
#elif defined(__aarch64__) || defined(__arm64__)
    int r;
    __asm__ __volatile__ ("sub %w0, %w1, %w2" : "=r" (r) : "r" (a), "r" (b));
    return r;
#elif defined(__riscv) || defined(__riscv__)
    int r;
    __asm__ __volatile__ ("subw %0, %1, %2" : "=r" (r) : "r" (a), "r" (b));
    return r;
#else
#error "008_inline_asm_per_target: no inline-assembly branch selected; the target architecture predefined macro (__x86_64__ / __i386__ / __aarch64__ / __riscv) was not recognized"
#endif
}

/* Bitwise exclusive or of two ints. Exercises a second read-write output. */
static int asm_xor(int a, int b)
{
#if defined(__x86_64__) || defined(__amd64__)
    int r = a;
    __asm__ __volatile__ ("xorl %1, %0" : "+r" (r) : "r" (b) : "cc");
    return r;
#elif defined(__i386__) || defined(__i386)
    int r = a;
    __asm__ __volatile__ ("xorl %1, %0" : "+r" (r) : "r" (b) : "cc");
    return r;
#elif defined(__aarch64__) || defined(__arm64__)
    int r;
    __asm__ __volatile__ ("eor %w0, %w1, %w2" : "=r" (r) : "r" (a), "r" (b));
    return r;
#elif defined(__riscv) || defined(__riscv__)
    int r;
    __asm__ __volatile__ ("xor %0, %1, %2" : "=r" (r) : "r" (a), "r" (b));
    return r;
#else
#error "008_inline_asm_per_target: no inline-assembly branch selected; the target architecture predefined macro (__x86_64__ / __i386__ / __aarch64__ / __riscv) was not recognized"
#endif
}

int main(void)
{
    volatile int va = 1000;
    volatile int vb = 37;
    volatile int vc = -1000;
    int fa, fs, fx, fsn, fxn;
    int ra, rs, rx, rsn, rxn;
    int matches;

    fa  = asm_add(1000, 37);
    fs  = asm_sub(1000, 37);
    fx  = asm_xor(1000, 37);
    fsn = asm_sub(37, 1000);
    fxn = asm_xor(-1000, 37);

    ra  = asm_add((int)va, (int)vb);
    rs  = asm_sub((int)va, (int)vb);
    rx  = asm_xor((int)va, (int)vb);
    rsn = asm_sub((int)vb, (int)va);
    rxn = asm_xor((int)vc, (int)vb);

    matches = (fa == 1000 + 37) && (fs == 1000 - 37) && (fx == (1000 ^ 37))
           && (fsn == 37 - 1000) && (fxn == (-1000 ^ 37))
           && (ra == fa) && (rs == fs) && (rx == fx)
           && (rsn == fsn) && (rxn == fxn);

    printf("asm_branch_selected=1\n");
    printf("asm_folded_add=%d\n", fa);
    printf("asm_folded_sub=%d\n", fs);
    printf("asm_folded_xor=%d\n", fx);
    printf("asm_folded_sub_neg=%d\n", fsn);
    printf("asm_folded_xor_neg=%d\n", fxn);
    printf("asm_runtime_add=%d\n", ra);
    printf("asm_runtime_sub=%d\n", rs);
    printf("asm_runtime_xor=%d\n", rx);
    printf("asm_runtime_sub_neg=%d\n", rsn);
    printf("asm_runtime_xor_neg=%d\n", rxn);
    printf("asm_chain=%d\n", asm_add(asm_sub((int)va, (int)vb), asm_xor((int)va, (int)vb)));
    printf("asm_matches_c=%d\n", matches);
    return 0;
}
