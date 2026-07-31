/* Area 05 - Pointer arithmetic and function pointers
 * 003_struct_member_access: member access through pointers, including
 * nested members and array members, via both -> and (*p). forms.
 *
 * No header is included; printf is hand-declared.  No address or pointer
 * value is printed; pointer facts appear only as comparisons and as
 * differences cast to long long.
 */

int printf(const char *, ...);

struct inner {
    int x;
    int y;
};

struct outer {
    int id;
    struct inner in;
    int vec[4];
    struct inner arr[3];
};

struct node {
    int value;
    struct node *next;
};

static struct outer o = {
    7,
    { 11, 12 },
    { 21, 22, 23, 24 },
    { { 31, 32 }, { 33, 34 }, { 35, 36 } }
};

static struct outer table[3] = {
    { 100, { 1, 2 }, { 3, 4, 5, 6 }, { { 7, 8 }, { 9, 10 }, { 11, 12 } } },
    { 200, { 13, 14 }, { 15, 16, 17, 18 }, { { 19, 20 }, { 21, 22 }, { 23, 24 } } },
    { 300, { 25, 26 }, { 27, 28, 29, 30 }, { { 31, 32 }, { 33, 34 }, { 35, 36 } } }
};

static struct node n3 = { 3, 0 };
static struct node n2 = { 2, &n3 };
static struct node n1 = { 1, &n2 };

int main(void)
{
    struct outer *p = &o;
    struct inner *q;
    int *v;
    volatile int vidx;
    int k;

    /* ---- arrow and dereference-dot forms on the same object ---- */
    printf("arrow_id=%d\n", p->id);
    printf("dot_id=%d\n", (*p).id);
    {
        int via_arrow = p->id;
        int via_dot = (*p).id;
        printf("arrow_eq_dot=%d\n", via_arrow == via_dot);
    }

    /* ---- nested members ---- */
    printf("nested_x=%d\n", p->in.x);
    printf("nested_y=%d\n", p->in.y);
    printf("nested_via_dot=%d %d\n", (*p).in.x, (*p).in.y);

    /* ---- pointer to a nested member ---- */
    q = &p->in;
    printf("inner_ptr_x=%d\n", q->x);
    printf("inner_ptr_y=%d\n", q->y);
    printf("inner_ptr_same_object=%d\n", q == &o.in);

    /* ---- array member ---- */
    printf("vec_all=%d %d %d %d\n",
           p->vec[0], p->vec[1], p->vec[2], p->vec[3]);
    v = p->vec;                     /* array member decays to a pointer */
    printf("vec_decay=%d %d\n", v[0], v[3]);
    printf("vec_decay_same=%d\n", v == &p->vec[0]);
    printf("vec_span=%lld\n", (long long)(&p->vec[4] - &p->vec[0]));
    printf("vec_interior=%lld\n", (long long)(&p->vec[3] - &p->vec[1]));

    /* ---- array of nested structs ---- */
    printf("arr0=%d %d\n", p->arr[0].x, p->arr[0].y);
    printf("arr2=%d %d\n", p->arr[2].x, p->arr[2].y);
    q = p->arr;                     /* struct array member decays too */
    printf("arr_decay=%d %d\n", q->x, q->y);
    ++q;
    printf("arr_decay_step=%d %d\n", q->x, q->y);
    printf("arr_decay_index=%lld\n", (long long)(q - p->arr));

    /* ---- write through a pointer, then read back ---- */
    p->id = 8;
    p->in.x = 13;
    p->vec[2] = 99;
    p->arr[1].y = 44;
    printf("written_id=%d\n", o.id);
    printf("written_nested=%d\n", o.in.x);
    printf("written_vec=%d\n", o.vec[2]);
    printf("written_arr=%d\n", o.arr[1].y);

    /* ---- stepping across an array of structs ---- */
    p = table;
    printf("table0_id=%d\n", p->id);
    ++p;
    printf("table1_id=%d\n", p->id);
    printf("table1_nested=%d %d\n", p->in.x, p->in.y);
    printf("table1_vec=%d %d\n", p->vec[1], p->vec[3]);
    printf("table1_arr=%d %d\n", p->arr[2].x, p->arr[2].y);
    p += 1;
    printf("table2_id=%d\n", p->id);
    printf("table_index=%lld\n", (long long)(p - table));
    printf("table_span=%lld\n", (long long)((table + 3) - table));

    /* ---- self-referential structure walked through pointers ---- */
    {
        struct node *n = &n1;
        int sum = 0;
        int steps = 0;
        while (n != 0) {
            sum += n->value;
            ++steps;
            n = n->next;
        }
        printf("list_sum=%d\n", sum);
        printf("list_steps=%d\n", steps);
        printf("list_second=%d\n", n1.next->value);
        printf("list_third=%d\n", n1.next->next->value);
        printf("list_tail_null=%d\n", n1.next->next->next == 0);
    }

    /* ---- runtime variant: volatile index selects the element ---- */
    vidx = 2;
    k = vidx;
    p = table + k;
    printf("runtime_id=%d\n", p->id);
    printf("runtime_nested=%d %d\n", p->in.x, p->in.y);
    printf("runtime_vec=%d\n", p->vec[k]);
    printf("runtime_arr=%d %d\n", p->arr[k].x, p->arr[k].y);
    printf("runtime_index=%lld\n", (long long)(p - table));

    vidx = 1;
    k = vidx;
    printf("runtime_member_of_index=%d\n", table[k].vec[k]);
    printf("runtime_nested_of_index=%d\n", table[k].arr[k].x);
    q = &table[k].arr[k];
    printf("runtime_inner_ptr=%d %d\n", q->x, q->y);
    printf("runtime_inner_same=%d\n", q == &table[1].arr[1]);

    return 0;
}
