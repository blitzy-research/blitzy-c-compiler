/* Designated initializers for structure members: the struct counterpart of
 * 004_designated_array.c.  Seven designated forms are constructed at static duration and
 * four of them again at automatic duration, and then every member of every object is read
 * back and printed on its own line.
 *
 * STATIC AND AUTOMATIC DURATION ARE THE TWO VARIANTS HERE.  An initializer for an object of
 * static storage duration is a constant expression settled at translation time, whereas the
 * same initializer on an object of automatic storage duration is performed on entry to the
 * block; the g_ objects take the first path and the l_ objects the second.  That is the
 * two-variant discipline expressed in the terms this feature area has, and it is why no
 * operand here is volatile: a volatile aggregate would change the accesses under test rather
 * than force the initializer to be performed at run time, which the storage duration already
 * does.
 *
 * WHY OMITTING MEMBERS IS GATE-CLEAN.  -Wextra enables -Wmissing-field-initializers and
 * -Werror promotes it, but in C that diagnostic does not fire for a designated initializer,
 * only for a positional partial one such as a lone leading value.  Every initializer below
 * is designated, so leaving members unmentioned is both legal and quiet.  None of these may
 * be rewritten in positional form: doing so would trade the construct under test for a
 * diagnostic.  The program needs no deviation from the suite's default warning gate.
 *
 * Every member of every structure is an int and %d is the only conversion specifier in the
 * file, so nothing whose width varies by target is declared, computed or printed, and no
 * character data appears, so plain-char signedness is irrelevant here.  Members are read
 * individually by name and no object is ever printed, copied or reinterpreted as bytes, so
 * padding -- whose contents the standard leaves unspecified -- is never observed.
 *
 * FREEDOM FROM UNDEFINED BEHAVIOUR.  A member left unmentioned by a designated initializer
 * is initialized as an object of static storage duration would be, that is to zero, for
 * automatic and static duration alike (C11 6.7.9p19 and p21), so every member printed below
 * has a determinate value and no uninitialized storage is ever read.  The only arithmetic in
 * the file is the loop counter's increment over the range 0 through 4, so no overflow is
 * possible and every subscript is strictly inside its array of 4.  There is no shift, no
 * pointer cast, no pointer arithmetic and no aliasing: each object is read only through an
 * lvalue of its own declared type, and nothing is modified after its initialization.
 *
 * No header is brought in and no preprocessor directive appears anywhere in this file: bcc
 * ships no stdio.h, its bundled set being the nine required freestanding headers plus a bonus
 * stdatomic.h, ten files in all (docs/project-guide.md line 212, the one place the repository
 * states the full ten-file inventory; the table at docs/technical-specifications.md lines
 * 202-214 enumerates only the nine required headers and does not list stdatomic.h).  printf
 * is declared by hand instead. */
int printf(const char *, ...);

/* struct outer nests an aggregate between two scalars, which is what lets a designator
 * reach through a member into a submember and what makes the position a positional
 * initializer resumes at after a member designator observable in both directions: the
 * member before the designated one and the member after it are distinct and separately
 * printed.  struct witharr replaces the nested aggregate with an array so that a designator
 * may descend into an element of a member. */
struct inner { int p; int q; };
struct outer { int tag; struct inner in; int trailer; };
struct witharr { int tag; int a[4]; int trailer; };

/* The seven forms, at static storage duration.
 *
 * g_in_order    designators in declaration order, the baseline the other six are read
 *               against.
 * g_reordered   the same three members designated in reverse order.  A defect in designator
 *               resolution shows up here as transposed values rather than as a crash, which
 *               is why the values are chosen to ascend with the member order.
 * g_omitted     the nested member is never mentioned, so both of its submembers are
 *               implicitly zero while the two designated scalars keep their values.
 * g_nested_mem  a designator reaching through the outer structure into a submember, leaving
 *               the sibling submember and the trailing scalar implicitly zero.
 * g_then_pos    the sharpest case in the program.  After the member designator .in the
 *               current object advances to the member following in, so the bare 44 lands in
 *               trailer, and tag -- which precedes the designated member and is never
 *               mentioned -- stays at its implicit zero.  A compiler that restarted at the
 *               first member, or that advanced by the wrong number of members, produces a
 *               visibly different four-line group.
 * g_arr_elem    a designator descending into one element of an array member; the other three
 *               elements and the trailing scalar are implicitly zero.
 * g_arr_whole   an array member designated as a whole, between two designated scalars. */
static struct outer   g_in_order   = { .tag = 1, .in = { 2, 3 }, .trailer = 4 };
static struct outer   g_reordered  = { .trailer = 14, .in = { 12, 13 }, .tag = 11 };
static struct outer   g_omitted    = { .trailer = 24, .tag = 21 };
static struct outer   g_nested_mem = { .in.q = 33, .tag = 31 };
static struct outer   g_then_pos   = { .in = { 42, 43 }, 44 };
static struct witharr g_arr_elem   = { .a[2] = 55, .tag = 51 };
static struct witharr g_arr_whole  = { .tag = 61, .a = { 62, 63, 64, 65 }, .trailer = 66 };

int main(void)
{
    /* Automatic-duration twins of forms one, two, four and six.  These carry the same
     * designated forms into the path where the initializer is performed on entry to the
     * block rather than settled before startup, and l_reordered permutes its designators
     * differently from g_reordered so the two are not the same case twice. */
    struct outer   l_in_order  = { .tag = 71, .in = { 72, 73 }, .trailer = 74 };
    struct outer   l_reordered = { .trailer = 84, .tag = 81, .in = { 82, 83 } };
    struct outer   l_omitted   = { .in.p = 92 };
    struct witharr l_arr_elem  = { .a[0] = 101, .a[3] = 104 };
    int i;

    printf("g_in_order.tag=%d\n", g_in_order.tag);
    printf("g_in_order.in.p=%d\n", g_in_order.in.p);
    printf("g_in_order.in.q=%d\n", g_in_order.in.q);
    printf("g_in_order.trailer=%d\n", g_in_order.trailer);
    printf("g_reordered.tag=%d\n", g_reordered.tag);
    printf("g_reordered.in.p=%d\n", g_reordered.in.p);
    printf("g_reordered.in.q=%d\n", g_reordered.in.q);
    printf("g_reordered.trailer=%d\n", g_reordered.trailer);
    printf("g_omitted.tag=%d\n", g_omitted.tag);
    printf("g_omitted.in.p=%d\n", g_omitted.in.p);
    printf("g_omitted.in.q=%d\n", g_omitted.in.q);
    printf("g_omitted.trailer=%d\n", g_omitted.trailer);
    printf("g_nested_mem.tag=%d\n", g_nested_mem.tag);
    printf("g_nested_mem.in.p=%d\n", g_nested_mem.in.p);
    printf("g_nested_mem.in.q=%d\n", g_nested_mem.in.q);
    printf("g_nested_mem.trailer=%d\n", g_nested_mem.trailer);
    printf("g_then_pos.tag=%d\n", g_then_pos.tag);
    printf("g_then_pos.in.p=%d\n", g_then_pos.in.p);
    printf("g_then_pos.in.q=%d\n", g_then_pos.in.q);
    printf("g_then_pos.trailer=%d\n", g_then_pos.trailer);
    printf("g_arr_elem.tag=%d\n", g_arr_elem.tag);
    for (i = 0; i < 4; i++) {
        printf("g_arr_elem.a[%d]=%d\n", i, g_arr_elem.a[i]);
    }
    printf("g_arr_elem.trailer=%d\n", g_arr_elem.trailer);
    printf("g_arr_whole.tag=%d\n", g_arr_whole.tag);
    for (i = 0; i < 4; i++) {
        printf("g_arr_whole.a[%d]=%d\n", i, g_arr_whole.a[i]);
    }
    printf("g_arr_whole.trailer=%d\n", g_arr_whole.trailer);
    printf("l_in_order.tag=%d\n", l_in_order.tag);
    printf("l_in_order.in.p=%d\n", l_in_order.in.p);
    printf("l_in_order.in.q=%d\n", l_in_order.in.q);
    printf("l_in_order.trailer=%d\n", l_in_order.trailer);
    printf("l_reordered.tag=%d\n", l_reordered.tag);
    printf("l_reordered.in.p=%d\n", l_reordered.in.p);
    printf("l_reordered.in.q=%d\n", l_reordered.in.q);
    printf("l_reordered.trailer=%d\n", l_reordered.trailer);
    printf("l_omitted.tag=%d\n", l_omitted.tag);
    printf("l_omitted.in.p=%d\n", l_omitted.in.p);
    printf("l_omitted.in.q=%d\n", l_omitted.in.q);
    printf("l_omitted.trailer=%d\n", l_omitted.trailer);
    printf("l_arr_elem.tag=%d\n", l_arr_elem.tag);
    for (i = 0; i < 4; i++) {
        printf("l_arr_elem.a[%d]=%d\n", i, l_arr_elem.a[i]);
    }
    printf("l_arr_elem.trailer=%d\n", l_arr_elem.trailer);
    return 0;
}
