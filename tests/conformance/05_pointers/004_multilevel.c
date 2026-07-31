int printf(const char *, ...);

static int a = 1;
static int b = 2;
static int c = 3;
static int d = 4;

static int  slots[4]  = { 10, 20, 30, 40 };
static int *pslots[4] = { &slots[0], &slots[1], &slots[2], &slots[3] };

int main(void)
{
    int *p;
    int **pp;
    int ***ppp;
    volatile int vidx;
    int k;

    p = &a;
    pp = &p;
    ppp = &pp;

    printf("level1=%d\n", *p);
    printf("level2=%d\n", **pp);
    printf("level3=%d\n", ***ppp);
    printf("level2_is_level1=%d\n", *pp == p);
    printf("level3_is_level2=%d\n", **ppp == p);
    printf("ppp_deref_is_pp=%d\n", *ppp == pp);

    *pp = &b;
    printf("after_pp_retarget=%d\n", *p);
    printf("after_pp_retarget_l3=%d\n", ***ppp);
    printf("after_pp_retarget_points_b=%d\n", p == &b);

    **ppp = &c;
    printf("after_ppp_retarget=%d\n", *p);
    printf("after_ppp_retarget_points_c=%d\n", p == &c);

    ***ppp = 33;
    printf("write_through_three=%d\n", c);
    printf("write_visible_at_l1=%d\n", *p);

    {
        int *other = &d;
        int **pp2 = &other;
        ppp = &pp2;
        printf("redirected_l3=%d\n", ***ppp);
        ***ppp = 44;
        printf("redirected_write=%d\n", d);
        printf("redirected_l1_untouched=%d\n", *p);
        printf("redirected_not_pp=%d\n", *ppp != pp);
    }

    printf("table_values=%d %d %d %d\n",
           *pslots[0], *pslots[1], *pslots[2], *pslots[3]);
    pp = pslots;
    printf("table_decay=%d\n", **pp);
    ++pp;
    printf("table_decay_step=%d\n", **pp);
    printf("table_decay_index=%lld\n", (long long)(pp - pslots));
    printf("table_span=%lld\n", (long long)((pslots + 4) - pslots));
    *pp = &slots[3];
    printf("table_retargeted=%d\n", *pslots[1]);
    printf("table_retargeted_alias=%d\n", pslots[1] == pslots[3]);
    **pp = 41;
    printf("table_write_through=%d\n", slots[3]);

    /* restore the table so the runtime section starts from a known state */
    pslots[1] = &slots[1];
    slots[3] = 40;
    printf("table_restored=%d %d\n", *pslots[1], *pslots[3]);

    {
        int (*parr)[4] = &slots;
        printf("parr_first=%d\n", (*parr)[0]);
        printf("parr_last=%d\n", (*parr)[3]);
        printf("parr_row_span=%lld\n", (long long)((parr + 1) - parr));
        printf("parr_elem_span=%lld\n",
               (long long)(&(*parr)[4] - &(*parr)[0]));
        printf("parr_first_is_slots=%d\n", &(*parr)[0] == slots);
    }

    vidx = 2;
    k = vidx;
    pp = pslots + k;
    printf("runtime_l2=%d\n", **pp);
    printf("runtime_index=%lld\n", (long long)(pp - pslots));
    ppp = &pp;
    printf("runtime_l3=%d\n", ***ppp);
    ***ppp = 31;
    printf("runtime_write_through=%d\n", slots[2]);
    printf("runtime_write_seen_via_table=%d\n", *pslots[2]);

    vidx = 0;
    k = vidx;
    *ppp = pslots + k;
    printf("runtime_moved_l2=%d\n", **pp);
    printf("runtime_moved_index=%lld\n", (long long)(pp - pslots));

    vidx = 3;
    k = vidx;
    pslots[k] = &a;
    a = 55;
    printf("runtime_retargeted=%d\n", *pslots[k]);
    printf("runtime_retargeted_is_a=%d\n", pslots[k] == &a);

    {
        int total = 0;
        int i;
        vidx = 4;
        k = vidx;
        for (i = 0; i < k; ++i) {
            total += *pslots[i];
        }
        printf("runtime_table_total=%d\n", total);
    }

    return 0;
}
