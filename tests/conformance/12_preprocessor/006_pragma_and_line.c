/* Area 12 / program 006 - pragma acceptance and the observable effect of a line directive.
   The pragma is asserted through a RELATION on struct sizes, so the output is identical
   whether or not the compiler honours the packing request; only a compiler that ERRORS on
   the pragma produces a divergence.  The file-name macro is never printed raw: before the
   first line directive only a path-independent relation is printed, and afterwards only
   the virtual name the directive itself supplied.  Self-contained: no header is included;
   printf is hand-declared because bcc ships no <stdio.h>.

   A note on this comment's spelling.  The two directive names are written throughout this
   prose WITHOUT their leading hash, and the two macro names are spelled out in words, so
   that a grep for a directive or a macro returns exactly the lines of code that use it and
   nothing from this commentary.  The directives themselves appear in full, at column one,
   in the three places that matter.

   Why the pragma is spelled pack(push, 1) / pack(pop) and nothing else.  The audit gate is
   -Wall -Wextra -pedantic -Wconversion -Wsign-conversion -Wshadow -Werror, under which an
   UNKNOWN pragma is an error rather than a note.  pack is the one spelling measured clean
   under that gate that also has an OBSERVABLE effect, which is what makes the assertion
   below meaningful rather than decorative.  A once directive in a main file, every STDC
   form and any invented name were each measured to fail the gate, and a message pragma
   emits a diagnostic, so none of them appears here.

   Why the struct sizes are printed as relations and never as raw values.  Honouring a
   packing request is a legitimate implementation choice, so a program that printed the raw
   size of the packed struct would report a divergence against any compiler that declined -
   a defect manufactured by the test rather than found by it.  Every relation below holds
   equally when the request is honoured and when it is ignored, so the only divergence this
   program can raise is a real one: a compiler that rejects the pragma outright.  That is
   "report, never work around" applied at authoring time.

   Why the file-name macro needs the virtual-name technique.  The harness copies each
   program into its own per-cell workspace, so the real source path differs between cells
   and printing it would break byte-exact comparison in every one of them.  Before the
   first line directive the macro is therefore reduced to the relation "is not empty".
   After a line directive supplies a virtual name, the macro expands to a compile-time
   constant written in THIS source, so its length and its first and last characters are
   perfectly deterministic and may be printed as values.  virtual_unit.c names no file, is
   never opened, and must never be created.

   Layout independence.  A line directive numbers the line that FOLLOWS it, so every printf
   below that reads the line-number macro sits on the line immediately after its own
   directive: the printed number is the one the directive set, and not a function of where
   the rest of this file happens to sit.  The first two reads of that macro are on adjacent
   lines, so their difference is one wherever they appear.

   Determinism and hermeticity.  Nothing printed is an address, a pointer value, a raw
   size, a plain-char signedness-dependent value, a width-dependent value or a character
   string; every conversion is %d and every printed value is an int.  No translation-date
   or translation-time macro is used, no library routine other than printf is called, no
   storage is allocated, and nothing outside this translation unit is read.  Every input is
   a literal written here, and the file is pure US-ASCII.

   Undefined-behaviour freedom.  The only indexing is into a string literal at index 0 and
   at index len - 1, where len is the length this program just computed for that same
   literal, so both are in bounds.  There is no signed overflow, no shift, no aliasing
   violation, no read of uninitialised storage, no object modified twice between sequence
   points, and no argument of any call has a side effect. */
int printf(const char *, ...);

/* Length of a NUL-terminated string, computed locally so this program stays a single-file
   reproducer that includes nothing.  The counter is int and the strings walked here are a
   handful of characters long, so neither a conversion nor an overflow can arise; measured
   clean under -Wconversion and -Wsign-conversion. */
static int slen(const char *s)
{
    int n = 0;
    while (s[n] != '\0') {
        n = n + 1;
    }
    return n;
}

/* The pragma's subject.  Only this struct sits inside the push/pop pair, so the pair's
   scoping is itself part of what is exercised: struct Plain is declared after the pop and
   must be left alone by the request.  Both structs declare the same three members, so the
   only difference a compiler can introduce between them is layout. */
#pragma pack(push, 1)
struct MaybePacked {
    unsigned char a;
    unsigned int b;
    unsigned char c;
};
#pragma pack(pop)

struct Plain {
    unsigned char a;
    unsigned int b;
    unsigned char c;
};

int main(void)
{
    int first = __LINE__;
    int second = __LINE__;

    printf("line_delta=%d\n", second - first);
    printf("pack_relation_holds=%d\n",
           (int)(sizeof(struct MaybePacked) <= sizeof(struct Plain)));
    printf("pack_min_size_holds=%d\n", (int)(sizeof(struct MaybePacked) >= 6u));
    printf("plain_min_size_holds=%d\n", (int)(sizeof(struct Plain) >= 6u));
    printf("real_file_nonempty=%d\n", (int)(slen(__FILE__) > 0));
#line 700
    printf("line_after_directive=%d\n", __LINE__);
    printf("line_after_next=%d\n", __LINE__);
#line 900 "virtual_unit.c"
    printf("virtual_line=%d\n", __LINE__);
    printf("virtual_file_len=%d\n", slen(__FILE__));
    printf("virtual_file_first_is_v=%d\n", (int)(__FILE__[0] == 'v'));
    printf("virtual_file_last_is_c=%d\n", (int)(__FILE__[slen(__FILE__) - 1] == 'c'));
    return 0;
}
