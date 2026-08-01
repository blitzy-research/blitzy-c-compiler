/* Flexible array member declaration, sizing and access (C11 6.7.2.1p18).
 *
 * A flexible array member is an incomplete array as the last member of a struct
 * that has more than one named member, and it contributes NOTHING to sizeof the
 * struct.  The usual idiom for populating one -- a heap allocation sized as the
 * fixed header plus n elements -- is unavailable here: no header may be included,
 * so no allocator is even declared, and the suite is hermetic, so no dynamic
 * allocation is performed at all.  The storage is therefore an
 * allocated-equivalent STATIC object.
 *
 * Two techniques were probed against the mandatory strict gate and the result
 * eliminated one of them outright, so the choice below is measured rather than
 * stylistic:
 *
 *   - Direct initialization -- static struct fam d = { 3, {1,2,3} }; -- is
 *     REJECTED: "error: initialization of a flexible array member" under
 *     -Werror=pedantic.  Initializing a flexible array member is a compiler
 *     extension, not standard C, and -pedantic exists precisely to reject one.
 *     No gate deviation is sanctioned for this area, so this technique is out.
 *   - A file-scope union pairing the flexible-array struct with a same-prefix
 *     SIZED struct is accepted, and is gate-clean at -O0, -O1 and -O2 as well as
 *     sanitizer-clean.  That is what appears below.
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
 * Portability: every value is int and every conversion is printed with %d, so
 * nothing here depends on a type whose width was measured to differ between i686
 * and the three 64-bit targets, and sizeof is never applied to a pointer.  Both
 * sizeof results are cast to int before reaching the variadic call, so no wider
 * type is passed.  int is 4 bytes and 4-byte aligned on all four targets, hence
 * no padding sits between count and data in either union arm and both printed
 * sizes are target-invariant: sizeof(struct fam) is 4, because the flexible array
 * member adds nothing, and sizeof(union fam_storage) is 4 + 8*4 = 36.  No object
 * representation is inspected and nothing is read byte by byte, so neither
 * padding nor endianness can influence the output, and no address is ever
 * printed -- the two views are reached only through int-typed lvalues at
 * identical offsets.
 */

int printf(const char *, ...);

struct fam { int count; int data[]; };

#define FAM_CAP 8

union fam_storage {
    struct fam flex;
    struct { int count; int data[FAM_CAP]; } sized;
};

static union fam_storage g_store = {
    .sized = { 5, { 100, 200, 300, 400, 500, 0, 0, 0 } }
};

/* Reads the flexible array member across a function boundary, where the callee
 * knows only the incomplete type and must take the element count from the object
 * itself.  The loop bound is p->count, which never exceeds FAM_CAP, so every
 * subscript stays inside the backing array. */
static int sum_prefix(const struct fam *p)
{
    int i;
    int total = 0;
    for (i = 0; i < p->count; i++) {
        total += p->data[i];
    }
    return total;
}

int main(void)
{
    struct fam *p = &g_store.flex;
    int i;

    /* Read-back phase: the prefix was written through the sized arm by the
     * designated initializer and is read here through the flexible-array arm.
     * Every live element is printed individually so a single wrong slot localizes
     * to one initializer form rather than to the aggregate. */
    printf("sizeof_struct_fam=%d\n", (int)sizeof(struct fam));
    printf("sizeof_storage=%d\n", (int)sizeof(union fam_storage));
    printf("count=%d\n", p->count);
    for (i = 0; i < p->count; i++) {
        printf("data[%d]=%d\n", i, p->data[i]);
    }
    printf("sum=%d\n", sum_prefix(p));

    /* Mutation phase: three separate statements, so no object is modified twice
     * between sequence points.  One element is rewritten in place, the count is
     * grown by one, and a previously zero slot is filled -- all through the
     * flexible-array arm, proving both arms address the same storage.  The
     * highest index touched is 5 and FAM_CAP is 8, so every access remains in
     * bounds, and the largest printed value is 2133, far inside INT_MAX. */
    g_store.flex.data[2] = 333;
    g_store.flex.count = 6;
    g_store.flex.data[5] = 600;
    printf("after_count=%d\n", g_store.flex.count);
    for (i = 0; i < g_store.flex.count; i++) {
        printf("after[%d]=%d\n", i, g_store.flex.data[i]);
    }
    printf("after_sum=%d\n", sum_prefix(&g_store.flex));
    return 0;
}
