int printf(const char *, ...);

#define EXT_MAX(a, b) __extension__ ({ \
    __extension__ __typeof__(a) ext_max_x_ = (a); \
    __extension__ __typeof__(b) ext_max_y_ = (b); \
    ext_max_x_ > ext_max_y_ ? ext_max_x_ : ext_max_y_; \
})

#define EXT_MIN(a, b) __extension__ ({ \
    __extension__ __typeof__(a) ext_min_p_ = (a); \
    __extension__ __typeof__(b) ext_min_q_ = (b); \
    ext_min_p_ < ext_min_q_ ? ext_min_p_ : ext_min_q_; \
})

static int ext_scale(int v)
{
    return __extension__ ({ int ext_scale_t_ = v; ext_scale_t_ * 3 + 1; });
}

int main(void)
{
    int folded_stmt = __extension__ ({ int ext_a_ = 4; int ext_b_ = 9; ext_a_ + ext_b_; });
    __extension__ long long folded_ll = 1234567890123LL;
    __extension__ __typeof__(folded_stmt) folded_typeof = folded_stmt * 2;
    unsigned folded_ull;
    volatile int vruntime = 6;
    volatile long long vll = 1000000000000LL;
    int runtime_stmt;
    int runtime_max;
    long long runtime_ll;

    folded_ull = (unsigned)(__extension__ (folded_stmt + 87));

    runtime_stmt = __extension__ ({ int ext_r_ = vruntime; ext_r_ * ext_r_ + 1; });
    runtime_max = EXT_MAX(vruntime, 11);
    runtime_ll = __extension__ (vll + 23LL);

    printf("ext_folded_stmt=%d\n", folded_stmt);
    printf("ext_folded_ll=%lld\n", folded_ll);
    printf("ext_folded_typeof=%d\n", folded_typeof);
    printf("ext_folded_ull=%u\n", folded_ull);
    printf("ext_folded_max=%d\n", EXT_MAX(4, 9));
    printf("ext_folded_min=%d\n", EXT_MIN(4, 9));
    printf("ext_folded_nested=%d\n", EXT_MAX(EXT_MIN(7, 1), 5));
    printf("ext_folded_scale=%d\n", ext_scale(4));
    printf("ext_folded_arg=%d\n", ext_scale(__extension__ ({ int ext_q_ = 2; ext_q_ + 1; })));
    printf("ext_runtime_stmt=%d\n", runtime_stmt);
    printf("ext_runtime_max=%d\n", runtime_max);
    printf("ext_runtime_min=%d\n", EXT_MIN((int)vruntime, 11));
    printf("ext_runtime_ll=%lld\n", runtime_ll);
    printf("ext_runtime_scale=%d\n", ext_scale((int)vruntime));
    return 0;
}
