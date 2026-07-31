/* Shadowing here is confined to the tag, member and label namespaces, because
 * ordinary-identifier shadowing is rejected by the mandated -Wshadow -Werror
 * gate. */

int printf(const char *, ...);

typedef int units_t;

struct point { int x; int y; };

struct holder { int width; int height; };

static int width = 100;

static int outer_point_size(void) { return (int)sizeof(struct point); }

int main(void)
{
    units_t base = 7;
    struct holder h;
    int tag_inner;
    int tag_outer = outer_point_size();
    int local_typedef_value;
    int sibling_first;
    int sibling_second;

    {
        struct point { int a; int b; int c; int d; };
        struct point deep;
        deep.a = 1;
        deep.b = 2;
        deep.c = 3;
        deep.d = 4;
        tag_inner = (int)sizeof deep + deep.a + deep.b + deep.c + deep.d;
    }

    {
        typedef short small_t;
        small_t narrow = 9;
        local_typedef_value = narrow + (int)sizeof(small_t);
    }

    { int reused = 1; sibling_first = reused; }
    { int reused = 2; sibling_second = reused; }

    h.width = 11;
    h.height = 12;

    printf("tag_scope=%d %d\n", tag_outer, tag_inner);
    printf("member_ns=%d %d\n", h.width, width);
    printf("typedef_scope=%d %d\n", base, local_typedef_value);
    printf("sibling_blocks=%d %d\n", sibling_first, sibling_second);
    goto width;
width:
    printf("label_ns=1\n");
    return 0;
}
