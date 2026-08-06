/* Union initialization and designated union members.  Nine initializer forms are
 * exercised here: a positional initializer, which must select the FIRST member; a
 * designator naming a non-first scalar member; a designator naming an aggregate
 * member; a designator naming an array member, both completely and partially
 * initialized; a union read back through a second member; a union whose member is
 * an array, including a nested element designator; a union nested inside a struct
 * with scalar members on either side of it; and an array of unions whose elements
 * mix designated and positional initializers.  Forms 1, 3 and 7 are then repeated
 * at automatic storage duration, because a static initializer is settled before
 * program startup while an automatic one is performed on entry to the block, and
 * the two paths can diverge independently.
 *
 * That pairing, rather than a volatile-operand variant, is what makes this program
 * discriminate across optimization levels: the same declarations are compiled at
 * every level and the values read back may not change, whatever the implementation
 * transforms either path into.  A volatile operand would add nothing here, because
 * the subject is what an initializer produces rather than what an operator
 * computes.
 *
 * Every member and element is read back on its own line, including the elements an
 * initializer leaves implicitly zero.
 *
 * Every union except g_pun is read through exactly the member its initializer
 * stored, so no padding byte is ever observed and no object representation is
 * relied upon.  g_pun is the single deliberate exception, and it is legal rather
 * than undefined: C11 6.5.2.3 and its footnote 95 sanction reading a union member
 * other than the one last stored in order to reinterpret an object
 * representation.  Three further facts make that reinterpretation complete and
 * target-invariant here.  sizeof(int) is 4 and b is exactly unsigned char[4], so
 * the four bytes read are precisely the four bytes of the int and no padding byte
 * is among them - which matters, because the contents of padding are unspecified.
 * unsigned char has no trap representations, so each of the four bytes has a
 * well-defined value.  And all four supported targets are little-endian
 * (docs/technical-specifications.md lines 457-462, and independently measured), so
 * 0x01020304 reads back as 4, 3, 2, 1 on every backend.  This is the only place
 * in this feature area that observes an object representation, which makes it a
 * direct cross-backend check on union member layout.
 *
 * Every value this program computes and prints is an int, an unsigned or an unsigned
 * char, printed with %d and %u alone and every unsigned char cast to int first, so no
 * target-varying width reaches the output; the remaining objects are the format-string
 * literals and the const char * that designates each of them, whose bytes printf copies
 * through unchanged and none of which is ever read as a number.  b is declared unsigned char
 * explicitly rather than plain char, so plain-char's implementation-defined signedness cannot
 * reach any printed byte.
 * The three printed sizes follow only from sizeof(int) == 4 with 4-byte alignment
 * and sizeof(unsigned char) == 1: sizeof(union mixed) is max(4, 8, 4) == 8 because
 * struct pair is two ints, and sizeof(union witharr) is max(4, 12) == 12.
 *
 * Undefined-behaviour freedom.  No arithmetic is performed beyond the loop
 * counters, so there is no overflow and no shift of any kind, and every subscript
 * is strictly in bounds: i < 4 indexes b[4] and i < 3 indexes a[3], so no
 * one-past-end pointer is even formed.  No pointer is cast - the reinterpretation
 * in g_pun goes through a union member access, which is the sanctioned mechanism,
 * and not through a cast, which would be an aliasing violation.  No object is
 * modified after its initialization.  Every member or element an initializer does
 * not name is implicitly zero by C11 6.7.9p19 and p21, so no uninitialized storage
 * is read anywhere - which is what makes the zero lines assertions rather than
 * accidents.
 *
 * No header is included and no preprocessor directive appears at all: bcc ships no
 * stdio.h, its bundled set being the nine required freestanding headers plus a bonus
 * stdatomic.h, ten files in all (docs/project-guide.md, "9 bundled freestanding
 * headers"). */

int printf(const char *, ...);

struct pair { int p; int q; };

union simple  { int i; unsigned u; };
union mixed   { int i; struct pair s; unsigned char b[4]; };
union witharr { int i; int a[3]; };

struct holder { int tag; union simple u; int trailer; };

/* Forms 1 through 9 at static storage duration, in order: g_first is positional
 * and must therefore initialize i, the first member; g_desig_i and g_desig_u name
 * a scalar member, the second of them a non-first one; g_desig_s names an
 * aggregate member; g_desig_b and g_desig_b_pt name an array member completely
 * and partially; g_pun is the justified reinterpretation; g_arr_whole and
 * g_arr_desig cover an array member written positionally and through a nested
 * element designator; g_holder descends through a struct into a union member with
 * a scalar on either side; and g_uarr is an array of unions mixing designated and
 * positional element initializers. */
static union simple  g_first      = { 5 };
static union simple  g_desig_i    = { .i = -7 };
static union simple  g_desig_u    = { .u = 4000000000u };
static union mixed   g_desig_s    = { .s = { 11, 12 } };
static union mixed   g_desig_b    = { .b = { 1, 2, 3, 4 } };
static union mixed   g_desig_b_pt = { .b = { 9 } };
static union mixed   g_pun        = { .i = 0x01020304 };
static union witharr g_arr_whole  = { .a = { 21, 22, 23 } };
static union witharr g_arr_desig  = { .a = { [2] = 33 } };
static struct holder g_holder     = { .tag = 41, .u.i = 42, .trailer = 43 };
static union simple  g_uarr[3]    = { { .i = 51 }, { .u = 52u }, { 53 } };

int main(void)
{
    /* Automatic-duration twins of forms 1, 3 and 7. */
    union simple  l_first     = { 61 };
    union mixed   l_desig_s   = { .s = { 71, 72 } };
    union witharr l_arr_desig = { .a = { [1] = 82 } };
    int i;

    printf("g_first.i=%d\n", g_first.i);
    printf("g_desig_i.i=%d\n", g_desig_i.i);
    printf("g_desig_u.u=%u\n", g_desig_u.u);
    printf("g_desig_s.s.p=%d\n", g_desig_s.s.p);
    printf("g_desig_s.s.q=%d\n", g_desig_s.s.q);
    for (i = 0; i < 4; i++) {
        printf("g_desig_b.b[%d]=%d\n", i, (int)g_desig_b.b[i]);
    }
    /* Elements 1 through 3 were never named by the initializer, so each must read
     * back as zero rather than as whatever the storage happened to hold. */
    for (i = 0; i < 4; i++) {
        printf("g_desig_b_pt.b[%d]=%d\n", i, (int)g_desig_b_pt.b[i]);
    }
    /* The one representation-level observation, justified in the header comment:
     * little-endian on every target, so this prints 4, 3, 2, 1. */
    for (i = 0; i < 4; i++) {
        printf("g_pun.b[%d]=%d\n", i, (int)g_pun.b[i]);
    }
    for (i = 0; i < 3; i++) {
        printf("g_arr_whole.a[%d]=%d\n", i, g_arr_whole.a[i]);
    }
    /* Only element 2 was designated, so elements 0 and 1 must be zero. */
    for (i = 0; i < 3; i++) {
        printf("g_arr_desig.a[%d]=%d\n", i, g_arr_desig.a[i]);
    }
    /* tag and trailer bracket the union, so a designator- or layout-resolution
     * error shifts one of them rather than only the union member. */
    printf("g_holder.tag=%d\n", g_holder.tag);
    printf("g_holder.u.i=%d\n", g_holder.u.i);
    printf("g_holder.trailer=%d\n", g_holder.trailer);
    /* Each element is read through the member its own initializer stored: i, then
     * u, then i again for the positional element. */
    printf("g_uarr[0].i=%d\n", g_uarr[0].i);
    printf("g_uarr[1].u=%u\n", g_uarr[1].u);
    printf("g_uarr[2].i=%d\n", g_uarr[2].i);
    printf("sizeof_simple=%d\n", (int)sizeof(union simple));
    printf("sizeof_mixed=%d\n", (int)sizeof(union mixed));
    printf("sizeof_witharr=%d\n", (int)sizeof(union witharr));
    printf("l_first.i=%d\n", l_first.i);
    printf("l_desig_s.s.p=%d\n", l_desig_s.s.p);
    printf("l_desig_s.s.q=%d\n", l_desig_s.s.q);
    for (i = 0; i < 3; i++) {
        printf("l_arr_desig.a[%d]=%d\n", i, l_arr_desig.a[i]);
    }
    return 0;
}
