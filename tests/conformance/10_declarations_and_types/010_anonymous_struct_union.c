/* Area 10 / 010 -- anonymous struct and union members and their name lookup.
 * Each anonymous-union member is written and read back through the SAME
 * member, so no type punning is relied upon. All members are int or short, so
 * both aggregate sizes are identical on all four targets. */

int printf(const char *, ...);

struct outer_u {
    int tag;
    union {
        int as_int;
        struct {
            short lo;
            short hi;
        };
    };
};

struct outer_s {
    int lead;
    struct {
        int mid_a;
        int mid_b;
    };
    int trail;
};

static int read_mid(const struct outer_s *p) { return p->mid_a + p->mid_b; }

int main(void)
{
    struct outer_u u;
    struct outer_u halves;
    struct outer_s s;
    volatile int seed = 3;
    int runtime;

    u.tag = 1;
    u.as_int = 16;

    halves.tag = 2;
    halves.lo = (short)5;
    halves.hi = (short)6;

    s.lead = 4;
    s.mid_a = 5;
    s.mid_b = 6;
    s.trail = 7;

    runtime = read_mid(&s) + seed;

    printf("anon_union_direct=%d %d\n", u.tag, u.as_int);
    printf("anon_union_halves=%d %d %d\n", halves.tag, (int)halves.lo, (int)halves.hi);
    printf("anon_struct_lookup=%d %d %d %d\n", s.lead, s.mid_a, s.mid_b, s.trail);
    printf("runtime_lookup=%d\n", runtime);
    printf("sizes=%d %d\n", (int)sizeof(struct outer_u), (int)sizeof(struct outer_s));
    return 0;
}
