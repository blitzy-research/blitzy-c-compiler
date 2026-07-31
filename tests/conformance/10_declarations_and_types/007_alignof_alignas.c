/* Area 10 / 007 -- _Alignof and _Alignas observed through alignment RESIDUES
 * and 0/1 relations. No address is ever printed. Only char, short, int and
 * float alignments are printed raw, because those four are identical on all
 * targets; double, long long, long and pointer alignments differ (4 on i686
 * versus 8 elsewhere) and appear only as bounded relations. */

int printf(const char *, ...);

struct padded { char c; _Alignas(8) int aligned_member; };

static _Alignas(16) unsigned char buf16[32];
static _Alignas(32) int wide_aligned = 7;

int main(void)
{
    _Alignas(8) unsigned char local8[8];
    struct padded p;
    unsigned long long a_buf = (unsigned long long)(const void *)buf16;
    unsigned long long a_wide = (unsigned long long)(const void *)&wide_aligned;
    unsigned long long a_local = (unsigned long long)(const void *)local8;
    unsigned long long a_member = (unsigned long long)(const void *)&p.aligned_member;

    local8[0] = 1;
    p.c = 2;
    p.aligned_member = 3;

    printf("alignof_stable=%d %d %d %d\n",
           (int)_Alignof(char), (int)_Alignof(short),
           (int)_Alignof(int), (int)_Alignof(float));
    printf("alignof_bounded=%d %d %d %d\n",
           (int)(_Alignof(double) >= 4u && _Alignof(double) <= 8u),
           (int)(_Alignof(long long) >= 4u && _Alignof(long long) <= 8u),
           (int)(_Alignof(long) >= 4u && _Alignof(long) <= 8u),
           (int)(_Alignof(void *) >= 4u && _Alignof(void *) <= 8u));
    printf("alignas_struct=%d %d\n",
           (int)_Alignof(struct padded), (int)sizeof(struct padded));
    printf("residues=%d %d %d %d\n",
           (int)(a_buf % 16ULL), (int)(a_wide % 32ULL),
           (int)(a_local % 8ULL), (int)(a_member % 8ULL));
    printf("readback=%d %d %d\n", (int)local8[0], (int)p.c, p.aligned_member);
    return 0;
}
