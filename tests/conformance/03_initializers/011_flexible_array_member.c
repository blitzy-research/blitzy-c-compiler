/* Flexible array member declaration and sizing (C11 6.7.2.1p18).
 *
 * A flexible array member is an incomplete array as the last member of a struct
 * that has more than one named member, and it contributes NOTHING to sizeof the
 * struct.  The usual idiom for populating one -- a heap allocation sized as the
 * fixed header plus n elements -- is unavailable here: no header may be included,
 * so no allocator is even declared, and the suite is hermetic, so no dynamic
 * allocation is performed at all.  The storage is therefore an
 * allocated-equivalent STATIC object.
 *
 * WHAT THIS PROGRAM DELIBERATELY DOES NOT DO, AND WHY -- ELEMENT ACCESS THROUGH
 * THE FLEXIBLE MEMBER ITSELF.
 *
 * An earlier form of this program subscripted the flexible member through a
 * `struct fam *` aimed at the union arm, `p->data[i]`.  That access is not
 * defensible under C11 6.7.2.1p18, which says the member "behaves as if that
 * member were replaced with the longest array (with the same element type) that
 * would not make the structure larger than THE OBJECT BEING ACCESSED", and adds
 * that if the replacement array would have no elements "the behavior is undefined
 * if any attempt is made to access that element".  The object being accessed
 * there is the `struct fam` union member, whose size is `sizeof(struct fam)` --
 * four bytes, the header alone -- so the replacement array has no elements and
 * every one of those subscripts was an out-of-bounds access, on the read side and
 * on the write side alike.  The union makes the two arms ALIAS legitimately; it
 * does not make either arm larger than it is declared to be.
 *
 * The only construction the standard unambiguously supports is an allocated object
 * genuinely larger than the header, and that is out of reach here: a portable
 * declaration of an allocation function needs `size_t`, which is `unsigned int` on
 * the 32-bit target and `unsigned long` on the other three, so one spelling cannot
 * serve all four; the corpus forbids including the header that would define it; and
 * a declaration without a prototype was measured to be rejected outright by the
 * reference compiler AND by the alternate reference compiler under the mandatory
 * warning gate.  Reaching for it would trade a soundness defect for a portability
 * one.
 *
 * So the exclusion is narrow, deliberate and recorded.  Everything about the
 * feature that CAN be observed without that access still is: the declaration is
 * accepted, the sizeof contribution is measured, the incomplete type crosses a
 * function boundary, and the member the two union arms share is written through one
 * arm and read through the other -- which is exactly the access C11 6.5.2.3p6
 * sanctions for a union of structures with a common initial sequence.  Element data
 * is carried by the SIZED arm, where the elements are actually declared and where
 * subscripting them is beyond argument.  Per constraint C3 this is stated here and
 * in this program's expectation record rather than left silent, and no language
 * feature is dropped: what is excluded is one unsound way of reaching a feature that
 * remains under test.
 *
 * Direct initialization -- static struct fam d = { 3, {1,2,3} }; -- is not
 * available either: initializing a flexible array member is a compiler extension
 * rather than standard C, and the mandatory -pedantic -Werror gate exists precisely
 * to reject one, with no deviation sanctioned for this area.  The exclusion is
 * scoped to that one SPELLING of the initializer and rests on the plan rather than
 * on difficulty: this program's entry prescribes access through allocated-equivalent
 * static storage and assigns GNU extensions to the 08_gcc_extensions area, and the
 * reason is recorded in this program's expectation record.  It costs no coverage of
 * the FEATURE -- declaration, the sizeof contribution, access through the incomplete
 * type across a function boundary, and mutation through both union arms are all
 * exercised below, at both storage durations.
 *
 * The storage is therefore spelled as a union pairing the flexible-array struct with
 * a same-prefix SIZED struct, which is gate-clean at -O0, -O1 and -O2 and
 * sanitizer-clean, and which appears once at file scope and once at block scope so
 * that the data-section image and the emitted-store path are both covered.
 *
 * The union is also the better answer on its own merits, not merely the one that
 * compiles.  A bare "static int backing[N]" cast to "struct fam *" would be a
 * strict-aliasing violation: the storage would be written as an int array and read
 * through an unrelated struct type.  Placing both types in the SAME union makes
 * the aliasing legitimate by construction -- C11 6.5.2.3p6 permits inspecting the
 * common initial sequence of structures that share a union, which here is the
 * leading "int count", and the two data arrays begin at the same offset with the
 * same element type, so every access reads or writes storage last written through
 * an lvalue of that same type.
 *
 * Every value is int and every conversion is printed with %d, sizeof is never
 * applied to a pointer, and both sizeof results are cast to int before reaching the
 * variadic call, so no target-varying width reaches the output.  Because int is 4
 * bytes and 4-byte aligned on all four targets, no padding sits between count and
 * data in either union arm and both printed sizes follow by derivation:
 * sizeof(struct fam) is 4, the flexible array member adding nothing, and
 * sizeof(union fam_storage) is 4 + 8*4 = 36.  No object representation is inspected
 * and nothing is read byte by byte, so neither padding nor endianness can influence
 * the output, and no address is ever printed -- the two views are reached only
 * through int-typed lvalues at identical offsets.
 */

int printf(const char *, ...);

struct fam { int count; int data[]; };

/* The same structure with the flexible member omitted, so that C11 6.7.2.1p18's sizing
 * rule -- the size is as if the member were omitted, allowing only extra trailing
 * padding -- can be asserted as the inequality the standard actually states rather than
 * as an equality it does not require. */
struct fam_header { int count; };

#define FAM_CAP 8

union fam_storage {
    struct fam flex;
    struct { int count; int data[FAM_CAP]; } sized;
};

static union fam_storage g_store = {
    .sized = { 5, { 100, 200, 300, 400, 500, 0, 0, 0 } }
};

/* Crosses a function boundary carrying the INCOMPLETE type, which is the part of the
 * feature a function signature can express: the callee knows only `struct fam`, whose
 * flexible member has no elements, so the one member it may read is the one both union
 * arms share.  Reading it here is what C11 6.5.2.3p6 sanctions for a union of structures
 * with a common initial sequence, and it is the value every loop below is bounded by. */
static int prefix_count(const struct fam *p)
{
    return p->count;
}

/* Sums the live prefix.  The count is taken through the flexible-array arm and the
 * elements through the sized arm, so one call exercises both views of the same storage
 * without subscripting a member that has no elements.  The bound never exceeds FAM_CAP,
 * so every subscript is inside the array it is declared against. */
static int sum_prefix(const union fam_storage *u)
{
    int i;
    int total = 0;
    for (i = 0; i < u->flex.count; i++) {
        total += u->sized.data[i];
    }
    return total;
}

int main(void)
{
    struct fam *p = &g_store.flex;
    /* Automatic-duration twin of the same technique: identical union, identical
     * designated initializer, but materialized by emitted stores on entry to this block
     * rather than as a data-section image.  A compiler can get the static image right
     * and the run-time stores wrong, so both paths are held side by side. */
    union fam_storage l_store = {
        .sized = { 4, { 11, 22, 33, 44, 0, 0, 0, 0 } }
    };
    struct fam *lp = &l_store.flex;
    int i;

    /* Sizing.  The absolute size of the flexible-array struct is printed because it is
     * four on every one of the four targets by derivation -- one int member, and int is
     * four bytes and four-byte aligned everywhere -- and the standard's own sizing rule
     * is printed as the inequality it states, so an implementation that chose extra
     * trailing padding would satisfy it rather than be accused by it. */
    printf("sizeof_struct_fam=%d\n", (int)sizeof(struct fam));
    printf("sizeof_storage=%d\n", (int)sizeof(union fam_storage));
    printf("fam_size_as_if_omitted=%d\n",
           (int)(sizeof(struct fam) >= sizeof(struct fam_header)));

    /* Read-back phase.  The prefix was written through the sized arm by the designated
     * initializer; the COUNT is read back through the flexible-array arm, both directly
     * and across a function boundary that knows only the incomplete type, and the
     * ELEMENTS are read through the sized arm where they are declared.  Every live
     * element is printed individually so a single wrong slot localizes to one initializer
     * form rather than to the aggregate. */
    printf("count=%d\n", p->count);
    printf("count_arms_agree=%d\n", (int)(p->count == g_store.sized.count));
    printf("count_via_boundary=%d\n", prefix_count(p));
    for (i = 0; i < p->count; i++) {
        printf("data[%d]=%d\n", i, g_store.sized.data[i]);
    }
    printf("sum=%d\n", sum_prefix(&g_store));

    /* Mutation phase: separate statements, so no object is modified twice between
     * sequence points.  The count is grown through the FLEXIBLE-ARRAY arm and read back
     * through the sized one, which is the shared-member access the standard sanctions;
     * the elements are rewritten through the sized arm.  The highest index touched is 5
     * and FAM_CAP is 8, so every access remains in bounds, and the largest printed value
     * is 2133, far inside INT_MAX. */
    g_store.sized.data[2] = 333;
    g_store.flex.count = 6;
    g_store.sized.data[5] = 600;
    printf("after_count=%d\n", g_store.sized.count);
    printf("after_count_via_boundary=%d\n", prefix_count(&g_store.flex));
    for (i = 0; i < g_store.sized.count; i++) {
        printf("after[%d]=%d\n", i, g_store.sized.data[i]);
    }
    printf("after_sum=%d\n", sum_prefix(&g_store));

    /* The automatic twin, read back and mutated through the same two arms.  Its highest
     * touched index is 4 against a capacity of 8, and its largest printed value is 242 --
     * l_after_sum, the sum of 11, 99, 33, 44 and 55 -- so every access is in bounds and no
     * overflow is reachable. */
    printf("l_count=%d\n", lp->count);
    printf("l_count_via_boundary=%d\n", prefix_count(lp));
    for (i = 0; i < lp->count; i++) {
        printf("l_data[%d]=%d\n", i, l_store.sized.data[i]);
    }
    printf("l_sum=%d\n", sum_prefix(&l_store));
    l_store.sized.data[1] = 99;
    l_store.flex.count = 5;
    l_store.sized.data[4] = 55;
    printf("l_after_count=%d\n", l_store.sized.count);
    for (i = 0; i < l_store.sized.count; i++) {
        printf("l_after[%d]=%d\n", i, l_store.sized.data[i]);
    }
    printf("l_after_sum=%d\n", sum_prefix(&l_store));
    return 0;
}
