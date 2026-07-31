/* Every _Generic association yields a small int tag, so the printed values are
 * width-normalized even where the selected type's width is not. */

int printf(const char *, ...);

#define TYPE_TAG(x) _Generic((x), \
    _Bool: 1, \
    char: 2, \
    signed char: 3, \
    unsigned char: 4, \
    short: 5, \
    unsigned short: 6, \
    int: 7, \
    unsigned int: 8, \
    long: 9, \
    unsigned long: 10, \
    long long: 11, \
    float: 12, \
    double: 13, \
    char *: 14, \
    const char *: 15, \
    default: 0)

int main(void)
{
    _Bool b = 1;
    char c = 'A';
    signed char sc = -1;
    unsigned char uc = 200;
    short sh = 3;
    unsigned short ush = 4;
    int i = 5;
    unsigned int ui = 6u;
    long l = 7L;
    unsigned long ul = 8ul;
    long long ll = 9LL;
    float f = 1.5f;
    double d = 2.5;
    char mutable_text[4];
    const char *const_text = "xy";
    volatile int vi = 3;

    mutable_text[0] = 'a';
    mutable_text[1] = 'b';
    mutable_text[2] = 'c';
    mutable_text[3] = 0;

    printf("tags=%d %d %d %d %d %d %d %d %d %d %d %d %d\n",
           TYPE_TAG(b), TYPE_TAG(c), TYPE_TAG(sc), TYPE_TAG(uc),
           TYPE_TAG(sh), TYPE_TAG(ush), TYPE_TAG(i), TYPE_TAG(ui),
           TYPE_TAG(l), TYPE_TAG(ul), TYPE_TAG(ll), TYPE_TAG(f), TYPE_TAG(d));
    printf("pointer_tags=%d %d\n", TYPE_TAG(&mutable_text[0]), TYPE_TAG(const_text));
    printf("default_tag=%d\n", TYPE_TAG((void *)0));
    printf("folded_tag=%d runtime_tag=%d\n", TYPE_TAG(1 + 1), TYPE_TAG(vi + 0));
    printf("promoted_tags=%d %d %d\n", TYPE_TAG(sc + 0), TYPE_TAG(sh + 0), TYPE_TAG(f + 0.0));
    printf("selected_values=%d %d\n", _Generic(i, int: 100, default: 0),
           _Generic(d, double: 200, default: 0));
    printf("selected_folded=%d selected_runtime=%d\n",
           _Generic(i, int: 3 * 7, default: 0),
           _Generic(i, int: vi * 7, default: 0));
    return 0;
}
