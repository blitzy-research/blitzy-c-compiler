/* No conversion whose argument width varies by target is used: no long length
   modifier, no size_t length modifier, no pointer conversion and no long double
   length modifier.  Every integer argument is an int, an unsigned int, a long
   long or an unsigned long long. */
int printf(const char *, ...);

static volatile int         rt_i  = -12345;
static volatile unsigned    rt_u  = 4294967295u;
static volatile long long   rt_ll = -1234567890123LL;
static volatile double      rt_d  = 2.5;

int main(void)
{
    int          i  = -42;
    unsigned int u  = 4000000000u;
    long long    ll = -9007199254740993LL;
    unsigned long long ull = 18446744073709551615ULL;
    short        s  = -300;
    signed char  sc = -100;
    double       d  = 1.5;
    float        f  = 0.25f;

    /* Snapshots of the four volatile objects. Declared here with the other objects
     * and assigned below, so each volatile access is a statement of its own. */
    int                sn_i;
    unsigned int       sn_u;
    long long          sn_ll;
    double             sn_d;

    printf("d=%d i=%i\n", i, i);
    printf("u=%u o=%o x=%x X=%X\n", u, u, u, u);
    printf("lld=%lld llu=%llu llx=%llx\n", ll, ull, ull);
    printf("hd=%hd hhd=%hhd\n", s, sc);
    printf("c=%c s=%s pct=%%\n", 'Q', "text");
    printf("width=[%5d] left=[%-5d] zero=[%05d]\n", 42, 42, 42);
    printf("plus=[%+d] space=[% d] neg=[%+d]\n", 42, 42, -42);
    printf("alt_x=[%#x] alt_o=[%#o]\n", 255u, 255u);
    printf("prec_s=[%.3s] width_s=[%6s] left_s=[%-6s]\n", "abcdef", "ab", "ab");
    printf("star=[%*d] star_prec=[%.*s]\n", 6, 42, 2, "abcdef");
    printf("f=%.6f e=%.6e g=%.6g\n", d, d, d);
    printf("f_from_float=%.6f\n", (double)f);
    printf("zero_f=%.6f neg_f=%.6f\n", 0.0, -0.5);
    printf("big_lld=%lld small_lld=%lld\n", 9007199254740993LL,
           -9007199254740993LL);
    printf("cast_int_to_ll=%lld\n", (long long)i);
    printf("cast_uint_to_ull=%llu\n", (unsigned long long)u);
    printf("sizeof_int=%d sizeof_ll=%d\n", (int)sizeof(int),
           (int)sizeof(long long));

    /* Each volatile object is read exactly once, in its own statement.  An access to
     * a volatile object is an observable side effect, and the relative order of side
     * effects within one argument list is unspecified, so the earlier form of these
     * calls depended on unspecified evaluation order in two distinct ways: the first
     * call read two different volatile objects in one argument list, and the last read
     * the SAME volatile object twice.  Snapshotting first separates every access from
     * the next by a sequence point, and costs the test nothing -- the snapshots are
     * loads from volatile storage, so each conversion is still fed a runtime value the
     * optimizer may not fold. */
    sn_i  = rt_i;
    sn_u  = rt_u;
    sn_ll = rt_ll;
    sn_d  = rt_d;

    printf("rt_i=%d rt_u=%u\n", sn_i, sn_u);
    printf("rt_lld=%lld\n", sn_ll);
    printf("rt_f=%.6f\n", sn_d);
    printf("rt_hex=%x rt_oct=%o\n", sn_u, sn_u);
    return 0;
}
