/* Flexible array member declaration, sizing and ELEMENT ACCESS (C11 6.7.2.1p18).
 *
 * A flexible array member is an incomplete array as the last member of a struct
 * that has more than one named member, and it contributes NOTHING to sizeof the
 * struct.  Three things about it are observable and all three are observed here:
 * the declaration is accepted, the member's zero contribution to sizeof is
 * measured, and its ELEMENTS are read and written through the member itself --
 * `p->data[i]` -- which is the access the compiler must compute an address for and
 * therefore the part of the feature a layout or address-arithmetic defect would
 * corrupt.
 *
 * HOW THE ELEMENT ACCESS IS MADE WELL DEFINED, WHICH IS THE WHOLE DESIGN OF THIS
 * PROGRAM.  C11 6.7.2.1p18 says the member "behaves as if that member were replaced
 * with the longest array (with the same element type) that would not make the
 * structure larger than THE OBJECT BEING ACCESSED", and adds that if the replacement
 * array would have no elements "the behavior is undefined if any attempt is made to
 * access that element".  So the accessed object must genuinely be larger than the
 * header, and 6.7.2.1p20's own EXAMPLE 2 shows the construction that makes it so: an
 * allocation of `sizeof(struct fam)` plus room for n elements, after which the object
 * behaves as if `data` had n elements.  That is exactly what fam_alloc does below,
 * with n == FAM_CAP, so every subscript in FAM_LIVE_MAX range is inside the
 * replacement array by the standard's own account rather than by inference.
 *
 * WHY THE ALLOCATION RATHER THAN A UNION ARM.  Aiming a `struct fam *` at a union arm
 * whose declared size is four bytes -- the header alone -- and subscripting that would be
 * undefined: under the rule above the replacement array there has no elements, so every
 * such subscript is out of bounds, and the only way to keep it would be to exclude
 * element access altogether.  The allocated object needs neither compromise -- it avoids
 * the undefined behaviour AND keeps the coverage.
 *
 * WHY AN ALLOCATOR CAN BE DECLARED HERE WITHOUT A HEADER AND WITHOUT LOSING A TARGET.
 * The corpus rule is that a program includes no header and hand-declares the libc
 * prototypes it needs, and the obstacle was always that `malloc` takes `size_t`, which
 * is `unsigned int` on the 32-bit target and `unsigned long` on the other three, so no
 * single spelling serves all four and an unprototyped declaration is rejected by the
 * mandatory gate.  The size type is therefore SELECTED BY THE ARCHITECTURE MACRO the
 * compiler predefines, in the same style as 12_preprocessor/004_predefined_macros.c
 * and 10_declarations_and_types/007_alignof_alignas.c, with an error directive on the
 * unrecognised branch so a fifth target cannot silently inherit a wrong width.  The
 * declaration then matches the target's own `size_t` exactly, which was MEASURED
 * clean -- no -Wbuiltin-declaration-mismatch -- under the full gate with all four
 * reference drivers at -O0, -O1 and -O2.  Hermeticity is untouched: an allocation
 * reaches no file, no socket and no path, and every block obtained is released before
 * main returns, which the address sanitizer's leak checker verifies.
 *
 * ALLOCATION FAILURE IS HANDLED DETERMINISTICALLY RATHER THAN ASSUMED AWAY.  The
 * request is 36 bytes; if it were nevertheless refused, printing the elements would
 * dereference a null pointer.  Each allocation is therefore checked, and a refusal
 * prints one fixed diagnostic line and returns a fixed non-zero status inside the
 * suite's 0-125 range -- so even that path is byte-deterministic and would be reported
 * as an exit-code divergence rather than as a crash.
 *
 * Direct initialization -- static struct fam d = { 3, {1,2,3} }; -- is still not
 * available, and that exclusion is unchanged: initializing a flexible array member is
 * a compiler extension rather than standard C, and the mandatory -pedantic -Werror
 * gate exists precisely to reject one, with no deviation sanctioned for this area.
 * The exclusion is scoped to that one SPELLING of the initializer and rests on the
 * plan rather than on difficulty -- GNU extensions belong to the 08_gcc_extensions
 * area -- and the reason is recorded in this program's expectation record.  It costs
 * no coverage: the allocated object below is filled by assignment through the
 * flexible member, which is the access an initializer would otherwise have performed.
 *
 * THE UNION IS KEPT, because it tests something the allocated object cannot.  It
 * pairs the flexible-array struct with a same-prefix SIZED struct, so the member the
 * two arms share can be written through one arm and read through the other -- exactly
 * the access C11 6.5.2.3p6 sanctions for a union of structures with a common initial
 * sequence -- and it appears once at file scope and once at block scope so that the
 * data-section image and the emitted-store path are both covered.  A compiler can get
 * the static image right and the run-time stores wrong, so both are held side by side.
 * Only the SHARED member is reached through the flexible arm of a union; the union's
 * elements are always reached through the sized arm, where they are declared.  The
 * union also makes its aliasing legitimate by construction: a bare "static int
 * backing[N]" cast to "struct fam *" would be a strict-aliasing violation, whereas
 * placing both types in the SAME union permits inspecting their common initial
 * sequence, and the two data arrays begin at the same offset with the same element
 * type.
 *
 * Every value is int and every conversion is printed with %d, sizeof is never
 * applied to a pointer, and every sizeof result is cast to int before reaching the
 * variadic call, so no target-varying width reaches the output.  Because int is 4
 * bytes and 4-byte aligned on all four targets, no padding sits between count and
 * data in either union arm and both printed sizes follow by derivation:
 * sizeof(struct fam) is 4, the flexible array member adding nothing, and
 * sizeof(union fam_storage) is 4 + 8*4 = 36.  No object representation is inspected
 * and nothing is read byte by byte, so neither padding nor endianness can influence
 * the output, and no address is ever printed -- neither the allocated block's nor
 * either union view's, which are reached only through int-typed lvalues.
 */

int printf(const char *, ...);

/* The target's own size_t width, selected by the architecture macro the compiler
 * predefines rather than by a header this corpus may not include.  measured: this
 * spelling matches the built-in declaration exactly on all four reference drivers,
 * so the gate raises no built-in-declaration-mismatch diagnostic.  The error
 * directive is deliberate: a fifth target must fail to TRANSLATE rather than inherit
 * a silently wrong argument width. */
#if defined(__i386__) || defined(__i386)
typedef unsigned int fam_alloc_size;
#elif defined(__x86_64__) || defined(__amd64__) || defined(__aarch64__) \
    || defined(__arm64__) || defined(__riscv) || defined(__riscv__)
typedef unsigned long fam_alloc_size;
#else
#error "unsupported target: the allocation-size type must match this target's size_t"
#endif

void *malloc(fam_alloc_size);
void free(void *);

struct fam { int count; int data[]; };

/* The same structure with the flexible member omitted, so that C11 6.7.2.1p18's sizing
 * rule -- the size is as if the member were omitted, allowing only extra trailing
 * padding -- can be asserted as the inequality the standard actually states rather than
 * as an equality it does not require. */
struct fam_header { int count; };

#define FAM_CAP 8

/* The largest count any object in this program ever carries.  Every loop below is
 * bounded by an object's own count, and every count assigned is at most this, which
 * is strictly less than FAM_CAP -- so no subscript can reach the end of the
 * replacement array, and no one-past-the-end pointer is ever formed. */
#define FAM_LIVE_MAX 6

union fam_storage {
    struct fam flex;
    struct { int count; int data[FAM_CAP]; } sized;
};

/* Allocate a struct fam with room for FAM_CAP elements, which is what makes
 * `p->data[i]` well defined for i < FAM_CAP: C11 6.7.2.1p18 sizes the replacement
 * array against the object being accessed, and this object is genuinely
 * sizeof(struct fam) + FAM_CAP * sizeof(int) bytes.  The element count is stored in
 * the header so every consumer takes its bound from the object rather than from a
 * constant repeated at the call site.  Returns a null pointer if the request is
 * refused; every caller checks. */
static struct fam *fam_alloc(int count)
{
    struct fam *p = malloc(sizeof(struct fam) + (fam_alloc_size)FAM_CAP * sizeof(int));
    int i;

    if (p == 0) {
        return 0;
    }
    /* Every element of the replacement array is written before any is read, so no
     * indeterminate value is ever observed: the caller's live prefix is filled by
     * fam_fill afterwards, and the tail is zeroed here. */
    for (i = 0; i < FAM_CAP; i++) {
        p->data[i] = 0;
    }
    p->count = count;
    return p;
}

/* Sums the live prefix THROUGH THE FLEXIBLE MEMBER, taking its bound from the
 * object's own header.  This is the read path a defect in flexible-member address
 * arithmetic would corrupt, and it is deliberately expressed as a separate function
 * so the access also crosses a function boundary carrying the incomplete type. */
static int fam_sum(const struct fam *p)
{
    int i;
    int total = 0;

    for (i = 0; i < p->count; i++) {
        total += p->data[i];
    }
    return total;
}

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

/* Fills the first `count` elements THROUGH THE FLEXIBLE MEMBER with a deterministic
 * arithmetic series, and records the count in the object's header.  Every subscript is
 * below the caller's count, which is at most FAM_LIVE_MAX and therefore inside the
 * FAM_CAP-element replacement array fam_alloc established. */
static void fam_fill(struct fam *p, int count, int first, int step)
{
    int i;

    p->count = count;
    for (i = 0; i < count; i++) {
        p->data[i] = first + i * step;
    }
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
    /* The allocated object, whose whole purpose is that its elements are reached
     * through the flexible member itself. */
    struct fam *ap = fam_alloc(0);
    int i;

    if (ap == 0) {
        printf("alloc_refused=1\n");
        return 3;
    }

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

    /* ELEMENT ACCESS THROUGH THE FLEXIBLE MEMBER, on an object genuinely large enough
     * for it.  Every read and every write below applies a subscript to
     * `struct fam::data`, which is the address computation the union arms could never
     * exercise.  Each element is printed individually so a single wrong slot localizes
     * to one index rather than to the aggregate, and the sum is taken across a function
     * boundary that knows only the incomplete type. */
    printf("alloc_sizeof_header=%d\n", (int)sizeof(struct fam));
    printf("alloc_initial_count=%d\n", ap->count);
    fam_fill(ap, 5, 7, 11);
    printf("alloc_count=%d\n", ap->count);
    for (i = 0; i < ap->count; i++) {
        printf("alloc_data[%d]=%d\n", i, ap->data[i]);
    }
    printf("alloc_sum=%d\n", fam_sum(ap));

    /* Mutation through the flexible member, in separate statements so no object is
     * modified twice between sequence points.  The count grows to FAM_LIVE_MAX, which
     * is 6 against a capacity of 8, so index 5 -- written here before it is read -- is
     * the highest the program ever touches.  The largest printed value is the sum of
     * 7, 999, 29, 40, 51 and 61, which is 1187, far inside INT_MAX. */
    ap->data[1] = 999;
    ap->count = FAM_LIVE_MAX;
    ap->data[5] = 61;
    printf("alloc_after_count=%d\n", ap->count);
    for (i = 0; i < ap->count; i++) {
        printf("alloc_after[%d]=%d\n", i, ap->data[i]);
    }
    printf("alloc_after_sum=%d\n", fam_sum(ap));
    printf("alloc_after_count_via_boundary=%d\n", prefix_count(ap));

    /* Released before main returns, so the address sanitizer's leak checker is
     * satisfied and the program owns no storage at exit. */
    free(ap);
    return 0;
}
