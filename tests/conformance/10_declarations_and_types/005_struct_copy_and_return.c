/* Two aggregates of different sizes are constructed, copied, passed and returned
 * by value.  How either of them travels is the ABI's decision, not this
 * program's, and only the member values that come back out are compared.  Both
 * sizes are printed rather than assumed, so they are compared as well. */

int printf(const char *, ...);

struct small2 { int a; int b; };
struct large8 { int a; int b; int c; int d; int e; int f; int g; int h; };

static struct small2 make_small(int base)
{
    struct small2 s;
    s.a = base;
    s.b = base + 1;
    return s;
}

static struct large8 make_large(int base)
{
    struct large8 l;
    l.a = base;
    l.b = base + 1;
    l.c = base + 2;
    l.d = base + 3;
    l.e = base + 4;
    l.f = base + 5;
    l.g = base + 6;
    l.h = base + 7;
    return l;
}

static struct small2 doubled(struct small2 v)
{
    v.a = v.a * 2;
    v.b = v.b * 2;
    return v;
}

static int consume_first(struct large8 v) { return v.a + v.b + v.c + v.d; }
static int consume_last(struct large8 v) { return v.e + v.f + v.g + v.h; }

int main(void)
{
    volatile int vbase = 4;
    struct small2 sf = make_small(1);
    struct small2 sr = make_small(vbase);
    struct large8 lf = make_large(1);
    struct large8 lr = make_large(vbase);
    struct small2 copy = sf;
    struct small2 through = doubled(sf);
    int head = consume_first(lf);
    int tail = consume_last(lf);

    printf("small_folded=%d %d\n", sf.a, sf.b);
    printf("small_runtime=%d %d\n", sr.a, sr.b);
    printf("large_folded=%d %d %d %d %d %d %d %d\n",
           lf.a, lf.b, lf.c, lf.d, lf.e, lf.f, lf.g, lf.h);
    printf("large_runtime=%d %d %d %d %d %d %d %d\n",
           lr.a, lr.b, lr.c, lr.d, lr.e, lr.f, lr.g, lr.h);
    printf("copy=%d %d\n", copy.a, copy.b);
    printf("passthrough=%d %d unchanged=%d %d\n",
           through.a, through.b, sf.a, sf.b);
    printf("consume=%d %d\n", head, tail);
    printf("sizes=%d %d\n", (int)sizeof(struct small2), (int)sizeof(struct large8));
    return 0;
}
