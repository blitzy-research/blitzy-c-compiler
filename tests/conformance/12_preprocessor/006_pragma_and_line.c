/* Area 12 / program 006 - pragma acceptance and the observable EFFECT of a line directive.
   Two of the three subjects are fixed by the standard alone: the operator spelling of a
   pragma must behave as the directive spelling does (C11 6.10.9), and a line directive
   renumbers the line that follows it and may supply a virtual file name (C11 6.10.4).  The
   third - that a packing region pushed and then popped leaves the state it found - is fixed
   by the GCC-COMPATIBLE PACK CONVENTION rather than by the standard, which defines no
   `pack' semantics at all; see WHAT IS ASSERTED INSTEAD below for which authority carries
   which relation.  The file-name macro is never printed raw: before the first
   line directive only a path-independent relation is printed, and afterwards only the
   virtual name the directive itself supplied.  Self-contained: no header is included; printf
   is hand-declared because bcc ships no <stdio.h>.

   A note on this comment's spelling.  The two directive names are written throughout this
   prose WITHOUT their leading hash, and the two macro names are spelled out in words, so
   that a grep for a directive or a macro returns exactly the lines of code that use it and
   nothing from this commentary.  The directives themselves appear in full, at column one, in
   the four places that matter: two for the packing region's push and pop, and two for the
   line numbering - the first renumbering alone, the second supplying a virtual file name as
   well.  The operator spelling appears twice more, in the two places that matter for it.

   NOTHING IN THIS TRANSLATION UNIT SUPPRESSES A DIAGNOSTIC, AND THAT IS A PROPERTY TO
   PRESERVE.  The one pragma property C11 states NORMATIVELY - 6.10.6p1's closing sentence,
   "Any such pragma that is not recognized by the implementation is ignored" - could only be
   asserted by placing a pragma whose name is reserved to the program by construction between
   two identical declarations, and that assertion cannot be made inside the mandatory audit
   gate.  The only way to make it there would be to write a diagnostic-control pragma into the
   source to switch off the very diagnostic the gate raises, and that is worse than not making
   the assertion at all: the gate is what establishes this program's freedom from undefined
   behaviour, and a source that neutralises one of its members around the exact construct under
   test is a source the gate no longer inspects there.  So no directive anywhere in this file
   switches a diagnostic off.

   WHY THE NORMATIVE ASSERTION CANNOT BE MADE HERE, stated explicitly rather than left as a
   silent omission, because constraint C3 requires the reason for anything not tested.  The
   gate is -Wall -Wextra -pedantic -Wconversion -Wsign-conversion -Wshadow -Werror.  -Wall
   enables the unknown-pragma diagnostic and -Werror makes it fatal, so an unrecognisable
   pragma - by construction the only kind whose treatment the standard REQUIRES - stops
   translation.  MEASURED with all four reference drivers and with the alternate reference
   compiler: an invented pragma, a pragma with no tokens at all, and the standard STDC pragma
   forms are each rejected as an unknown pragma.  The suite's gate-deviation contract cannot
   express the removal either: a deviation may only drop a member of the gate, and the
   diagnostic in question is implied by -Wall, so silencing it would mean dropping -Wall
   entire - dozens of unrelated checks - and the contract sanctions only two reductions in any
   case, neither of them this one.  What is excluded is therefore ONE ASSERTION SHAPE, not the
   feature: pragmas are still exercised on every cell, in both of their spellings, and the
   properties asserted instead are the ones a conforming implementation cannot fail whichever
   choice it makes.

   WHAT IS ASSERTED INSTEAD, AND WHICH AUTHORITY CARRIES EACH RELATION.  The subject is the
   packing region's push/pop LIFECYCLE, asserted only as relations so that no layout value is
   needed to compare them.  Three structures are declared identically: one before the region,
   one inside it, one after the pop.

     - If the implementation IGNORES the request, all three are laid out alike, so
       pack_size_restored and pack_align_restored hold trivially.  This branch rests on the
       STANDARD: 6.10.6p1 requires an unrecognised pragma to be ignored.
     - If the implementation RECOGNISES the request, the inner structure may be laid out
       differently, and the pop restores the state the push saved, so the first and third agree
       again.  This branch rests on the GCC-COMPATIBLE PACK CONVENTION, not on C11: the
       standard fixes no meaning for `pack', so an implementation that recognises the pragma
       and gives push/pop some other meaning is not thereby non-conforming.

   pack_size_not_larger and pack_align_not_larger have the same split authority from the other
   side: trivial for an implementation that ignores the request, and grounded in the same
   convention - under which a packing request may shrink an aggregate or leave it alone but
   never enlarge one - for an implementation that honours it.

   WHAT THAT MEANS FOR A DIVERGENCE, WHICH IS THE POINT OF SEPARATING THE TWO AUTHORITIES.
   Both compilers this program is compared against target the GCC-compatible convention, so
   the relations are the right thing to compare and a 0 is worth investigating on either side.
   But a 0 must be read as a departure from that convention rather than as a conformance
   defect, because the repository documents only that a pragma is DISPATCHED
   (docs/technical-specifications.md line 91 and line 490) and names no `pack' behaviour that
   an implementation is required to provide.  The absolute layout effect of packing is
   therefore tested where a documented authority does exist - see the paragraph on
   08_gcc_extensions/005_attribute_packed_aligned.c below.

   No size and no alignment is ever printed as an absolute value, so no implementation-defined
   layout choice reaches the output.

   THE OPERATOR SPELLING IS THE SECOND SUBJECT AND IS STANDARD C RATHER THAN AN EXTENSION.
   C11 6.10.9 defines the _Pragma operator: it destringizes its argument and the result "is
   then processed as if it were a #pragma directive".  The same push/pop region is therefore
   written a second time with the operator, giving pragma_op_size_restored and
   pragma_op_align_restored, and pragma_op_matches_directive asserts that the structure inside
   the operator's region is laid out exactly as the one inside the directive's.  That last
   relation is the one the operator uniquely buys: it holds under both readings above - both
   regions honoured alike, or both ignored alike - and it fails only for an implementation
   whose two spellings disagree, which no conforming implementation may do.  Destringization
   is exercised as a side effect, since the operator's argument reaches the pragma handler
   only after the string literal is taken apart.

   The layout effect of PACKING ITSELF is still not asserted here, and is tested where its
   authority is stronger: 08_gcc_extensions/005_attribute_packed_aligned.c exercises the packed
   and aligned attributes, which the repository's own extension inventory does name as
   supported.  This program asserts only relations, never a layout.

   Why the file-name macro needs the virtual-name technique.  The harness copies each
   program into its own per-cell workspace, so the real source path differs between cells
   and printing it would break byte-exact comparison in every one of them.  Before the
   first line directive the macro is therefore reduced to the relation "is not empty".
   After a line directive supplies a virtual name, the macro expands to a compile-time
   constant written in THIS source, so it is perfectly deterministic and is printed IN
   FULL, byte for byte.  virtual_unit.c names no file, is never opened, and must never be
   created.

   Why the whole virtual name is printed and not merely described.  Printing only the name's
   length and its first and last characters would be exactly the shape of assertion that
   cannot fail usefully: any wrong spelling of the same length beginning with v and ending
   with c - "virtual_uNit.c", "virtuaX_unit.c" - satisfies all three of those relations, so a
   defect in how the line directive's string literal reaches the file-name macro would be
   invisible in every cell.  The full name is safe to print precisely
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
   offset is printed as an absolute value - only a relation between two of them - so an
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

/* The five subjects of the pragma assertions, all declared identically so that no absolute
   layout value is needed to compare them.  LayoutBefore is declared outside every pragma
   region and is the reference every relation below is stated against.  LayoutPacked is
   declared inside a packing region opened with the DIRECTIVE spelling, LayoutAfterPop after
   that region is closed, and LayoutPackedOp and LayoutAfterPopOp are the same pair for the
   OPERATOR spelling.  Each structure holds one unsigned int between two unsigned char
   members, which is the shape whose layout a packing request would visibly change. */
struct LayoutBefore {
    unsigned char a;
    unsigned int b;
    unsigned char c;
};

#pragma pack(push, 1)
struct LayoutPacked {
    unsigned char a;
    unsigned int b;
    unsigned char c;
};
#pragma pack(pop)

struct LayoutAfterPop {
    unsigned char a;
    unsigned int b;
    unsigned char c;
};

/* The same region, opened and closed with the C11 6.10.9 _Pragma operator instead of the
   directive.  The operator destringizes its argument and the result "is then processed as if
   it were a #pragma directive", so whatever an implementation does with the directive form it
   must do with this one -- which is what pragma_op_matches_directive below asserts. */
_Pragma("pack(push, 1)")
struct LayoutPackedOp {
    unsigned char a;
    unsigned int b;
    unsigned char c;
};
_Pragma("pack(pop)")

struct LayoutAfterPopOp {
    unsigned char a;
    unsigned int b;
    unsigned char c;
};

int main(void)
{
    int first = __LINE__;
    int second = __LINE__;

    printf("line_delta=%d\n", second - first);

    /* Pragma acceptance and the push/pop lifecycle, stated as seven relations so that no
       implementation-defined layout value reaches the output.  Every one is 1 whether the
       implementation honours the packing request under the GCC-compatible pack convention or
       discards it, and each falls to 0 for its own distinct failure -- see the header comment
       for the case analysis and for which authority carries which relation. */
    printf("pack_size_restored=%d\n",
           (int)(sizeof(struct LayoutBefore) == sizeof(struct LayoutAfterPop)));
    printf("pack_align_restored=%d\n",
           (int)(_Alignof(struct LayoutBefore) == _Alignof(struct LayoutAfterPop)));
    printf("pack_size_not_larger=%d\n",
           (int)(sizeof(struct LayoutPacked) <= sizeof(struct LayoutBefore)));
    printf("pack_align_not_larger=%d\n",
           (int)(_Alignof(struct LayoutPacked) <= _Alignof(struct LayoutBefore)));
    printf("pragma_op_size_restored=%d\n",
           (int)(sizeof(struct LayoutBefore) == sizeof(struct LayoutAfterPopOp)));
    printf("pragma_op_align_restored=%d\n",
           (int)(_Alignof(struct LayoutBefore) == _Alignof(struct LayoutAfterPopOp)));
    printf("pragma_op_matches_directive=%d\n",
           (int)(sizeof(struct LayoutPacked) == sizeof(struct LayoutPackedOp)
                 && _Alignof(struct LayoutPacked) == _Alignof(struct LayoutPackedOp)));
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
