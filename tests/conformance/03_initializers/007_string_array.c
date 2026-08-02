/* Character arrays initialized from string literals, in both of the forms C11
   6.7.9p14 defines.  An array whose declared width is exactly the length of its
   literal receives the literal's bytes and the terminating null is simply NOT
   copied -- that is well defined, not an overflow.  An array with room to spare
   receives the terminator too, and every element beyond the literal is
   implicitly zero by 6.7.9p21.

   The exact-width declarations are the point of this program and must never be
   "fixed" by widening them.  A compiler that helpfully reserved a byte for a
   terminator would either store one past the end of the object -- an
   out-of-bounds write the sanitizer gate would catch -- or size the array
   wrongly, and sizeof_g_exact_u=3 together with sizeof_g_grid_exact=10 are the
   lines that detect it.  g_grid_exact and g_grid_room differ only in row width,
   which makes the terminator's presence or absence a single-line difference
   between two otherwise identical groups.

   Plain char is never printed as itself, because its signedness is
   implementation defined.  Two mechanisms are applied together: every array the
   requirement names is spelled unsigned char or signed char and read back
   through (int), whose value is unambiguous by declaration; and every
   plain-char object, plus the const char * dereference, is read back through
   (int)(unsigned char), which the dump_plain helper exists solely to
   centralise.  Each cast is a value conversion of an already-loaded character,
   never pointer punning.  Every literal here is US-ASCII, so in practice the
   two spellings agree -- the cast is applied unconditionally anyway, so the
   rule holds by construction and cannot be broken by a later edit that
   introduces a high-bit character.

   Every printed value is an int, printed with %d, and each sizeof is taken of a
   character array, so its value is a byte count identical on all four targets;
   sizeof is deliberately never taken of g_ptr.  No header is named: bcc ships
   no stdio.h, its bundled set being the nine required freestanding headers plus
   a bonus stdatomic.h, ten files in all (docs/project-guide.md line 212), so
   printf is declared by hand. */
int printf(const char *, ...);

/* Exact width, no room for a terminator: g_exact_u, g_exact_s, g_grid_exact.
   With room, so the terminator is stored: g_room_u, g_room_s, g_grid_room.
   Wider still, so trailing elements are implicitly zero: g_extra_u, g_room_s.
   g_braced is the "optionally enclosed in braces" spelling, g_concat exercises
   adjacent literal concatenation, g_escapes carries hex, octal and simple
   escapes, g_desig is a designated character array, and g_ptr contrasts a
   literal initializing a pointer with the array cases above. */
static unsigned char g_exact_u[3]  = "abc";
static unsigned char g_room_u[4]   = "abc";
static unsigned char g_extra_u[6]  = "abc";
static signed char   g_exact_s[3]  = "xyz";
static signed char   g_room_s[6]   = "xyz";
static char          g_plain[5]    = "hi";
static char          g_braced[4]   = { "abc" };
static char          g_concat[7]   = "ab" "cde";
static unsigned char g_escapes[4]  = "\x41\102\n";
static char          g_grid_exact[2][5] = { "hello", "world" };
static char          g_grid_room[2][6]  = { "hello", "world" };
static unsigned char g_desig[6]    = { [0] = 'a', [2] = 'c' };
static const char   *g_ptr         = "lit";

/* EXECUTION CHARACTER SET, pinned rather than assumed.  This program prints the numeric
 * codes of the characters in its literals, and C11 5.2.1 leaves those values to the
 * implementation: nothing in the standard says 'a' is 97.  All four supported targets were
 * measured to use ASCII, and the two assertions below pin exactly the characters this
 * program depends on, so an implementation with a different execution character set fails
 * to TRANSLATE rather than printing different numbers that an oracle would then have to
 * attribute to a compiler defect.  The two numeric escapes in g_escapes need no assertion:
 * C11 6.4.4.4p7 fixes \\x41 and \\102 at 65 and 66 by their numeric value regardless of
 * charset, which is precisely why they are spelled numerically.  The simple escape \\n does
 * need one, because 6.4.4.4p4 gives it a charset-dependent value. */
_Static_assert('a' == 97 && 'b' == 98 && 'c' == 99 && 'd' == 100 && 'e' == 101
                   && 'h' == 104 && 'i' == 105 && 'l' == 108 && 'o' == 111
                   && 'r' == 114 && 't' == 116 && 'w' == 119 && 'x' == 120
                   && 'y' == 121 && 'z' == 122,
               "every letter this program prints sits at its ASCII code");
_Static_assert('\n' == 10, "the simple escape \\n is the ASCII line feed");

/* Each helper reads exactly n elements, and every call below passes the
   declared element count, so an exact-width array is never read past its last
   element and no non-existent terminator is ever touched. */
static void dump_u(const char *name, const unsigned char *a, int n)
{
    int i;
    for (i = 0; i < n; i++) {
        printf("%s[%d]=%d\n", name, i, (int)a[i]);
    }
}

static void dump_s(const char *name, const signed char *a, int n)
{
    int i;
    for (i = 0; i < n; i++) {
        printf("%s[%d]=%d\n", name, i, (int)a[i]);
    }
}

/* The plain-char reader, and the only place the signedness normalisation is
   spelled, so it cannot be forgotten at a call site. */
static void dump_plain(const char *name, const char *a, int n)
{
    int i;
    for (i = 0; i < n; i++) {
        printf("%s[%d]=%d\n", name, i, (int)(unsigned char)a[i]);
    }
}

int main(void)
{
    /* Automatic-duration twins of the static cases: the same two forms, but
       performed on entry to the block at run time rather than settled at
       translation time. */
    unsigned char l_exact_u[3] = "def";
    char          l_room[4]    = "gh";
    int i;
    int j;

    dump_u("g_exact_u", g_exact_u, 3);
    dump_u("g_room_u", g_room_u, 4);
    dump_u("g_extra_u", g_extra_u, 6);
    dump_s("g_exact_s", g_exact_s, 3);
    dump_s("g_room_s", g_room_s, 6);
    dump_plain("g_plain", g_plain, 5);
    dump_plain("g_braced", g_braced, 4);
    dump_plain("g_concat", g_concat, 7);
    dump_u("g_escapes", g_escapes, 4);
    dump_u("g_desig", g_desig, 6);
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 5; j++) {
            printf("g_grid_exact[%d][%d]=%d\n", i, j, (int)(unsigned char)g_grid_exact[i][j]);
        }
    }
    for (i = 0; i < 2; i++) {
        for (j = 0; j < 6; j++) {
            printf("g_grid_room[%d][%d]=%d\n", i, j, (int)(unsigned char)g_grid_room[i][j]);
        }
    }
    /* Four bytes of a three-character literal: its characters and its
       terminator, all within the object the literal denotes. */
    for (i = 0; i < 4; i++) {
        printf("g_ptr[%d]=%d\n", i, (int)(unsigned char)g_ptr[i]);
    }
    printf("sizeof_g_exact_u=%d\n", (int)sizeof g_exact_u);
    printf("sizeof_g_room_u=%d\n", (int)sizeof g_room_u);
    printf("sizeof_g_extra_u=%d\n", (int)sizeof g_extra_u);
    printf("sizeof_g_concat=%d\n", (int)sizeof g_concat);
    printf("sizeof_g_grid_exact=%d\n", (int)sizeof g_grid_exact);
    printf("sizeof_g_grid_room=%d\n", (int)sizeof g_grid_room);
    dump_u("l_exact_u", l_exact_u, 3);
    dump_plain("l_room", l_room, 4);
    return 0;
}
