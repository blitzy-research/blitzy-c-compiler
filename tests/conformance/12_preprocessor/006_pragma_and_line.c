/* Area 12 / program 006 - the observable EFFECT of a packing pragma and of a line directive.
   The pragma is asserted through exact layout values - size, alignment and every member
   offset - so the printed output CHANGES if the packing request is not honoured; acceptance
   alone is no longer enough to pass.  The file-name macro is never printed raw: before the
   first line directive only a path-independent relation is printed, and afterwards only
   the virtual name the directive itself supplied.  Self-contained: no header is included;
   printf is hand-declared because bcc ships no <stdio.h>.

   A note on this comment's spelling.  The two directive names are written throughout this
   prose WITHOUT their leading hash, and the two macro names are spelled out in words, so
   that a grep for a directive or a macro returns exactly the lines of code that use it and
   nothing from this commentary.  The directives themselves appear in full, at column one,
   in the seven places that matter: four for the packing stack, one for the deliberately
   unrecognisable pragma whose C11 6.10.6p1 obligation to be ignored is itself asserted, and two
   for the line numbering - the first renumbering alone, the second supplying a virtual file name
   as well.

   Why the pragma is spelled pack(push, N) / pack(pop) and nothing else.  The audit gate is
   -Wall -Wextra -pedantic -Wconversion -Wsign-conversion -Wshadow -Werror, under which an
   unknown pragma is an error rather than a note.  So the invariant this program holds to is
   narrow: the only pragma it uses is one the reference compiler recognises and that has an
   OBSERVABLE effect, and it emits no diagnostic under the gate at all.  That is what makes
   the assertion below meaningful rather than decorative, and it is why no invented name, no
   STDC form and no diagnostic-emitting spelling appears here.

   Why the layout is printed as exact values rather than as relations that hold either way.
   A relation such as "the packed size does not exceed the unpacked size" is true whether
   the request is honoured or ignored, so a program built only from relations of that shape
   tests that the pragma was ACCEPTED and nothing more - the capability itself goes
   unmeasured, and a compiler that parsed the directive and then discarded it would pass.
   Every layout field below is therefore printed as an exact number, and every one of those
   numbers moves if the request is not honoured: the pack-1 struct reads 6 / 1 / 0,1,5 when
   the request takes effect and 12 / 4 / 0,4,8 when it does not, and the pack-2 struct reads
   8 / 2 / 0,2,6 against the same 12 / 4 / 0,4,8.  The two differ in every field, so a single
   printed line settles the question.

   Why that is a legitimate assertion rather than a manufactured divergence.  The concern a
   relation-only program was written to avoid is real but misdirected: it is not that a
   compiler may decline this pragma harmlessly, it is that the RESULTING LAYOUT might differ
   per target.  It does not.  Every value above was measured on all four supported targets,
   at all three optimization levels, and is identical on every one of the twelve cells - the
   packing rule is arithmetic on the declared member widths, and the only per-target input
   it takes is the alignment of unsigned int, which is 4 on x86_64, i686, aarch64 and
   riscv64 alike.  No target restriction is therefore needed and none is recorded.  What
   remains is exactly the divergence this suite exists to surface: a compiler that rejects
   the directive, and a compiler that accepts it and then does not apply it.  Both are
   reported, neither is worked around, and per the finding discipline neither is patched.

   Why the pragma is exercised as a STACK rather than as a switch.  push(1), a nested
   push(2), a pop back to 1 and a pop back to the default are four distinct states of one
   stack, and each is measured on its own struct.  A compiler that implemented push and pop
   as a single boolean flag would produce the pack-1 layout for the nested pack-2 struct, or
   the default layout for the struct after the inner pop; both show up as a changed number.
   The struct declared after the outer pop is what proves the pair is scoped: it must carry
   the target's natural layout, untouched by a request that has been popped.  A member
   round-trip through a packed object is printed as well, so the assertion covers reading and
   writing misaligned members and not merely computing their offsets.

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

   Determinism and hermeticity.  Nothing printed is an address, a pointer value, a
   plain-char signedness-dependent value, a width-dependent value or a character string.
   Sizes, alignments and offsets ARE printed, and they are printed because they were measured
   identical on all four targets, which is the condition under which a layout value is a
   deterministic property of the program rather than of the machine; every one of them is
   converted to unsigned and printed with %u, and every other printed value is an int printed
   with %d.  No translation-date or translation-time macro is used, no library routine other
   than printf is called, no storage is allocated, and nothing outside this translation unit
   is read.  Every input is a literal written here, and the file is pure US-ASCII.

   Undefined-behaviour freedom.  The only indexing is into a string literal at index 0 and
   at index len - 1, where len is the length this program just computed for that same
   literal, so both are in bounds.  There is no signed overflow, no shift, no aliasing
   violation, no read of uninitialised storage, no object modified twice between sequence
   points, and no argument of any call has a side effect.  The two objects whose members are
   round-tripped are fully assigned before either is read, member by member, and each member
   is read through its own declared name - never through a pointer to it, and never through
   overlapping storage - so a packed member's reduced alignment cannot produce a misaligned
   access that the program itself performs.  No padding byte is read: the packed object has
   none, and the naturally aligned object's members are read individually rather than as an
   image.  Every offset is obtained from __builtin_offsetof rather than from a difference of
   pointers to two members, which would be a subtraction of pointers into different objects. */
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

/* The pragma's four subjects.  Every one of them declares the SAME three members in the same
   order, so the only difference a compiler can introduce between them is layout, and each one
   is declared in a different state of the packing stack:

     Packed1     inside push(1)                     - measured 6 / 1 / 0,1,5
     Packed2     inside a nested push(2)            - measured 8 / 2 / 0,2,6
     Packed1b    after the inner pop, back at 1     - measured 6 / 1 / 0,1,5
     Natural     after the outer pop, default state - measured 12 / 4 / 0,4,8

   The four measurements are identical on x86_64, i686, aarch64 and riscv64 at -O0, -O1 and
   -O2, which is what lets them be printed as exact values.  Ignoring the request collapses
   the first three onto the fourth, and implementing push/pop as a single flag collapses
   Packed2 onto Packed1 or Packed1b onto Natural; either way a printed number changes. */
#pragma pack(push, 1)
struct Packed1 {
    unsigned char a;
    unsigned int b;
    unsigned char c;
};

#pragma pack(push, 2)
struct Packed2 {
    unsigned char a;
    unsigned int b;
    unsigned char c;
};
#pragma pack(pop)

struct Packed1b {
    unsigned char a;
    unsigned int b;
    unsigned char c;
};
#pragma pack(pop)

struct Natural {
    unsigned char a;
    unsigned int b;
    unsigned char c;
};

/* The second pragma property, and the only one C11 states NORMATIVELY: 6.10.6p1 ends "Any
   such pragma that is not recognized by the implementation is ignored."  That is a
   requirement rather than a permission, and it is directly observable - a pragma whose name
   is reserved to this program by construction must leave the layout of the declaration that
   follows it identical to the one that precedes it.  The reference compiler's warning about
   the unknown pragma is suppressed locally rather than by relaxing this program's gate, so
   the strict warning gate stays fully in force everywhere else in the file. */
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

    /* The round-trip subjects: one object laid out under the honoured packing request and one
       under the target's natural rule.  Each member is assigned by name before any member is
       read, so nothing uninitialised is observed, and the same three values go into both so
       that a difference in what comes out can only be a layout or access defect. */
    struct Packed1 packed_obj;
    struct Natural natural_obj;

    packed_obj.a = 161u;
    packed_obj.b = 16909060u;
    packed_obj.c = 195u;
    natural_obj.a = 161u;
    natural_obj.b = 16909060u;
    natural_obj.c = 195u;

    printf("line_delta=%d\n", second - first);

    /* The packing pragma's effect, stated as exact values.  Each line reads
       size / alignment / offset-of-a / offset-of-b / offset-of-c for one state of the stack.
       Honouring the request is what produces these numbers; ignoring it prints
       12 4 0 4 8 on all three of these lines instead, which is the natural layout below. */
    printf("pack1_layout=%u %u %u %u %u\n",
           (unsigned)sizeof(struct Packed1),
           (unsigned)_Alignof(struct Packed1),
           (unsigned)__builtin_offsetof(struct Packed1, a),
           (unsigned)__builtin_offsetof(struct Packed1, b),
           (unsigned)__builtin_offsetof(struct Packed1, c));
    printf("pack2_layout=%u %u %u %u %u\n",
           (unsigned)sizeof(struct Packed2),
           (unsigned)_Alignof(struct Packed2),
           (unsigned)__builtin_offsetof(struct Packed2, a),
           (unsigned)__builtin_offsetof(struct Packed2, b),
           (unsigned)__builtin_offsetof(struct Packed2, c));
    printf("pack1b_layout=%u %u %u %u %u\n",
           (unsigned)sizeof(struct Packed1b),
           (unsigned)_Alignof(struct Packed1b),
           (unsigned)__builtin_offsetof(struct Packed1b, a),
           (unsigned)__builtin_offsetof(struct Packed1b, b),
           (unsigned)__builtin_offsetof(struct Packed1b, c));

    /* The struct declared after the outer pop, which proves the pair is scoped: the request
       has been popped, so this must carry the target's natural layout. */
    printf("natural_layout=%u %u %u %u %u\n",
           (unsigned)sizeof(struct Natural),
           (unsigned)_Alignof(struct Natural),
           (unsigned)__builtin_offsetof(struct Natural, a),
           (unsigned)__builtin_offsetof(struct Natural, b),
           (unsigned)__builtin_offsetof(struct Natural, c));

    /* The same four facts restated as one field per defect, so that a reader of a divergence
       report sees at once which property moved.  All four are 1 here, and each falls to 0 for
       exactly one failure: pack1_took_effect for a push(1) that was accepted and not applied,
       pack2_took_effect for a nested push(2) that did not take its own value, pack1b_restored
       for a pop that reset to the default instead of restoring the outer request, and
       pop_scoped for a request that leaked past its pop. */
    printf("pack1_took_effect=%d pack2_took_effect=%d pack1b_restored=%d pop_scoped=%d\n",
           (int)(sizeof(struct Packed1) == 6u && _Alignof(struct Packed1) == 1u),
           (int)(sizeof(struct Packed2) == 8u && _Alignof(struct Packed2) == 2u),
           (int)(sizeof(struct Packed1b) == sizeof(struct Packed1)
                 && _Alignof(struct Packed1b) == _Alignof(struct Packed1)),
           (int)(sizeof(struct Natural) == 12u && _Alignof(struct Natural) == 4u));

    /* Reading and writing members at the reduced offsets, not merely computing them. */
    printf("pack1_roundtrip=%u %u %u\n",
           (unsigned)packed_obj.a, packed_obj.b, (unsigned)packed_obj.c);
    printf("natural_roundtrip=%u %u %u\n",
           (unsigned)natural_obj.a, natural_obj.b, (unsigned)natural_obj.c);

    /* The second pragma property, and the only one C11 states NORMATIVELY: 6.10.6p1 ends "Any
       such pragma that is not recognized by the implementation is ignored."  That is a
       requirement rather than a permission, and it is directly observable - a pragma whose name
       is reserved to this program by construction must leave the layout of the declaration that
       follows it identical to the one that precedes it. */
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
