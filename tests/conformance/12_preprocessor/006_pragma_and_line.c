/* Area 12 / program 006 - pragma acceptance and the observable EFFECT of a line directive.
   Both subjects are chosen so that the standard, rather than any implementation's
   documentation, fixes what must happen: an unrecognised pragma must be ignored, a
   diagnostic-control pragma cannot change what a program prints, and a line directive
   renumbers the line that follows it and may supply a virtual file name.  The file-name
   macro is never printed raw: before the first line directive only a path-independent
   relation is printed, and afterwards only the virtual name the directive itself supplied.
   Self-contained: no header is included; printf is hand-declared because bcc ships no
   <stdio.h>.

   A note on this comment's spelling.  The two directive names are written throughout this
   prose WITHOUT their leading hash, and the two macro names are spelled out in words, so
   that a grep for a directive or a macro returns exactly the lines of code that use it and
   nothing from this commentary.  The directives themselves appear in full, at column one,
   in the five places that matter: three for the deliberately unrecognisable pragma and the
   diagnostic-control pair that brackets it, and two for the line numbering - the first
   renumbering alone, the second supplying a virtual file name as well.

   WHY THIS PROGRAM DOES NOT ASSERT A PACKING PRAGMA'S EFFECT, recorded here because an
   earlier form of it did.  That form declared four structures under pack(push, 1), a nested
   pack(push, 2) and two pops, and printed each one's exact size, alignment and member
   offsets.  It is unsound as a differential oracle on two independent counts, either
   sufficient on its own:

     - The behaviour of a pragma the implementation DOES recognise is implementation-defined
       (C11 6.10.6p1), and the standard goes further than merely permitting a different
       layout: a recognised pragma is allowed to make translation fail outright.  A compiler
       that recognises the packing pragma and declines this particular request is therefore
       within its rights, and printing its layout as an exact number would report that
       conformance as a defect.
     - No document in this repository describes a packing pragma for the compiler under test.
       The documented inventory names the directive but not one pragma it honours, so a
       divergence arising from the push/pop lifecycle could not be attributed to a documented
       limitation either - leaving it neither a sound comparison nor a recordable expected
       divergence.

   The layout effect of PACKING ITSELF is not lost, and is not tested here because it is
   tested where its authority is stronger: 08_gcc_extensions/005_attribute_packed_aligned.c
   exercises the packed and aligned attributes, which the repository's own extension
   inventory does name as supported.  What this program keeps is the one pragma property
   C11 states NORMATIVELY, in the closing sentence of 6.10.6p1 - "Any such pragma that is
   not recognized by the implementation is ignored" - which is a requirement rather than a
   permission and is directly observable.

   How that requirement is made observable.  A pragma whose name is reserved to this program
   by construction sits between two structure declarations that are identical in every
   respect.  An implementation obeying the requirement leaves their layout indistinguishable;
   one that acted on a request it was required to discard changes one of them, and the size
   and alignment equalities below fall to 0.  The equalities are printed rather than the
   sizes themselves, so no implementation-defined layout value can reach the output: what is
   asserted is that the two declarations agree, which every conforming implementation
   guarantees whatever layout it chooses for them.

   The second pragma property, and why it needs no separate assertion of its own.  The
   diagnostic-control pragma that brackets the unrecognisable one changes only which messages
   the translator emits.  Standard error is captured into finding artifacts but never
   compared, so recognising or ignoring it leaves every printed byte identical - which the
   whole of this program's output attests, rather than one line of it.  It is present because
   the audit gate is -Wall -Wextra -pedantic -Wconversion -Wsign-conversion -Wshadow -Werror,
   under which an unknown pragma is an error rather than a note: suppressing that one
   diagnostic FROM INSIDE, on the one line it applies to, and restoring the previous state
   immediately afterwards is strictly better than asking this program's record for a relaxed
   gate, because the gate then stays at full strength for this program as for every other.

   Why the file-name macro needs the virtual-name technique.  The harness copies each
   program into its own per-cell workspace, so the real source path differs between cells
   and printing it would break byte-exact comparison in every one of them.  Before the
   first line directive the macro is therefore reduced to the relation "is not empty".
   After a line directive supplies a virtual name, the macro expands to a compile-time
   constant written in THIS source, so it is perfectly deterministic and is printed IN
   FULL, byte for byte.  virtual_unit.c names no file, is never opened, and must never be
   created.

   Why the whole virtual name is printed and not merely described.  An earlier form of this
   program printed only the name's length and its first and last characters, and that is
   exactly the shape of assertion that cannot fail usefully: any wrong spelling of the same
   length beginning with v and ending with c - "virtual_uNit.c", "virtuaX_unit.c" - passed
   every oracle in all twelve cells, so a defect in how the line directive's string literal
   reaches the file-name macro was invisible.  The full name is safe to print precisely
   because it is a literal written above rather than a path the harness chose, so printing
   it whole costs nothing in determinism and closes the gap.  The length and endpoint
   relations are kept beside it: they are cheap, and each one localises a different kind of
   corruption - a truncation, a lost first byte, a lost terminator - to its own line.

   Layout independence.  A line directive numbers the line that FOLLOWS it, so every printf
   below that reads the line-number macro sits on the line immediately after its own
   directive: the printed number is the one the directive set, and not a function of where
   the rest of this file happens to sit.  The first two reads of that macro are on adjacent
   lines, so their difference is one wherever they appear.

   Determinism and hermeticity.  Every printed value is an int printed with %d or a string
   constant written in this source, so nothing target-varying, address-derived, time-derived,
   random or locale-dependent can reach stdout.  No structure size, alignment or member
   offset is printed as an absolute value - only the equality of two of them - so an
   implementation's layout choices cannot be observed.  No plain-char value and no
   width-dependent value is printed.  No translation-date or translation-time macro is used,
   no library routine other than printf is called, no storage is allocated, and nothing
   outside this translation unit is read.  Every input is a literal written here, and the
   file is pure US-ASCII.

   Undefined-behaviour freedom.  The only indexing is into a string literal at index 0 and
   at index len - 1, where len is the length this program just computed for that same
   literal, so both are in bounds.  There is no signed overflow, no shift, no division, no
   aliasing violation, no read of uninitialised storage, no object modified twice between
   sequence points, and no argument of any call has a side effect.  No pointer value is
   converted to an integer and no address is printed. */
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

/* The two subjects of the unrecognised-pragma requirement: identical declarations that
   straddle a pragma no implementation can recognise, because its name is reserved to this
   program by construction.  C11 6.10.6p1 requires such a pragma to be ignored, so the two
   must agree on size and on alignment whatever layout the implementation chooses for them. */
struct LayoutBefore {
    unsigned char a;
    unsigned int b;
    unsigned char c;
};

#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wunknown-pragmas"
#pragma BCC_CONFORMANCE_UNRECOGNISED_PRAGMA_006 layout probe
#pragma GCC diagnostic pop

struct LayoutAfter {
    unsigned char a;
    unsigned int b;
    unsigned char c;
};

int main(void)
{
    int first = __LINE__;
    int second = __LINE__;

    printf("line_delta=%d\n", second - first);

    /* The unrecognised-pragma requirement, stated as two equalities so that no
       implementation-defined layout value reaches the output.  Both are 1 under every
       conforming implementation and each falls to 0 for its own failure: a discarded
       pragma that nevertheless changed the following declaration's size, and one that
       changed its alignment. */
    printf("ignored_pragma_size_unchanged=%d\n",
           (int)(sizeof(struct LayoutBefore) == sizeof(struct LayoutAfter)));
    printf("ignored_pragma_align_unchanged=%d\n",
           (int)(_Alignof(struct LayoutBefore) == _Alignof(struct LayoutAfter)));
    printf("real_file_nonempty=%d\n", (int)(slen(__FILE__) > 0));
#line 700
    printf("line_after_directive=%d\n", __LINE__);
    printf("line_after_next=%d\n", __LINE__);
#line 900 "virtual_unit.c"
    printf("virtual_line=%d\n", __LINE__);
    printf("virtual_file=%s\n", __FILE__);
    printf("virtual_file_len=%d\n", slen(__FILE__));
    printf("virtual_file_first_is_v=%d\n", (int)(__FILE__[0] == 'v'));
    printf("virtual_file_last_is_c=%d\n", (int)(__FILE__[slen(__FILE__) - 1] == 'c'));
    return 0;
}
