int printf(const char *, ...);

/* Compound literals in expression and argument position -- C11 6.5.2.5.
 *
 * A compound literal is a parenthesised type name followed by a braced
 * initializer list.  It is the one initializer form that creates an object in
 * the middle of an expression rather than in a declaration, so it runs the
 * initializer machinery in expression context and puts the resulting unnamed
 * object's storage duration and lifetime under test as well as its contents.
 *
 * Ten forms are covered, and each is read back on its own line so that a single
 * divergent line localizes the defect to one form:
 *    1. expression position, initializing a declaration
 *    2. with a designator, leaving the unmentioned member implicitly zero
 *    3. nested inside another compound literal
 *    4. argument position, passed by value
 *    5. argument position, passed by address
 *    6. array compound literal as an argument, decaying to a pointer
 *    7. subscripted directly, with no intervening variable
 *    8. member-selected directly
 *    9. designated array literal, read at a zero-filled slot and at the
 *       designated slot
 *   10. sizeof applied to a compound literal
 *
 * Two further cases sit beside them: a fresh unnamed object per loop iteration,
 * and two file-scope literals, which have static rather than automatic storage
 * duration.
 *
 * Determinism and portability: only int is computed and only %d is printed, so
 * nothing here depends on a type whose width differs between the four targets
 * (a pointer and the wider integer types were measured at 4 bytes on i686 and 8
 * on the other three, and none of them appears below), on plain-char signedness
 * (signed on x86-64 and i686, unsigned on AArch64 and RISC-V 64), or on padding
 * bytes, whose contents are unspecified.  No address is ever printed: every
 * pointer below is dereferenced and its pointee reported instead.
 */

struct pair  { int p; int q; };
struct nest  { int tag; struct pair n; };

/* A file-scope compound literal has STATIC storage duration (C11 6.5.2.5p5),
 * so taking its address and keeping it is well defined for the whole run.
 * These two are read at the very end of main deliberately, to contrast with the
 * block-scope literals inside main, whose lifetime ends with their block.
 */
static struct pair *g_file_cl = &(struct pair){ 7, 8 };
static int         *g_file_arr_cl = (int[]){ 91, 92, 93 };

/* These three helpers hold one variant of the computation behind a call
 * boundary, which cannot be folded through at -O0 and is typically inlined at
 * -O2; the values consumed directly in main are the other variant.  Both the
 * folded and the genuinely computed path are therefore exercised, and a
 * `volatile` twin is deliberately NOT added here: the accumulation across the
 * loop below already forces real run-time work, and qualifying an operand would
 * suppress exactly the optimizer behaviour this program exists to survive.
 *
 * Each returns one encoded value rather than printing per member because these
 * three lines test argument PASSING, not element layout: p * 100 + q keeps both
 * members independently recoverable from the printed number, and the per-element
 * layout checking is done by the forms that print member by member.  The largest
 * value any of them yields is 506, far inside the guaranteed range of int, so no
 * signed overflow is possible.
 */
static int take_val(struct pair v)
{
    return v.p * 100 + v.q;
}

static int take_ptr(const struct pair *v)
{
    return v->p * 100 + v->q;
}

static int sum3(const int *a)
{
    return a[0] + a[1] + a[2];
}

int main(void)
{
    struct pair v_expr = (struct pair){ 1, 2 };
    struct pair v_desig = (struct pair){ .q = 9 };
    struct nest v_nested = (struct nest){ 11, (struct pair){ 12, 13 } };
    int i;
    int loop_total = 0;

    /* Forms 1 to 3, member by member.  v_desig.p must print 0: the implicit
     * zeroing of C11 6.7.9p19 and p21 applies inside a compound literal exactly
     * as it does in a declaration, which is also why no read here touches
     * uninitialized storage.
     */
    printf("v_expr.p=%d\n", v_expr.p);
    printf("v_expr.q=%d\n", v_expr.q);
    printf("v_desig.p=%d\n", v_desig.p);
    printf("v_desig.q=%d\n", v_desig.q);
    printf("v_nested.tag=%d\n", v_nested.tag);
    printf("v_nested.n.p=%d\n", v_nested.n.p);
    printf("v_nested.n.q=%d\n", v_nested.n.q);

    /* Forms 4 to 6.  The two lines that pass an address -- the & literal and
     * the array literal that decays to a pointer -- use it only for the
     * duration of the call, which completes inside the same full expression, so
     * neither pointer outlives the object it designates and neither is stored
     * anywhere.  Comparing the by-value line against the by-address line
     * separates a calling-convention defect from a lifetime defect: one changes
     * and the other does not.
     */
    printf("take_val_arg=%d\n", take_val((struct pair){ 3, 4 }));
    printf("take_ptr_arg=%d\n", take_ptr(&(struct pair){ 5, 6 }));
    printf("sum3_arg=%d\n", sum3((int[]){ 10, 20, 30 }));

    /* Forms 7 to 10, applied to a literal with no intervening variable at all,
     * which is where a compiler that mishandles the unnamed object's address
     * computation or lifetime shows up first.  Every subscript is strictly in
     * bounds, and sizeof is applied to an int[3] rather than to a pointer, so
     * the answer is 12 on all four targets.
     */
    printf("cl_index0=%d\n", (int[]){ 41, 42, 43 }[0]);
    printf("cl_index2=%d\n", (int[]){ 41, 42, 43 }[2]);
    printf("cl_member=%d\n", (struct pair){ 51, 52 }.q);
    printf("cl_desig_arr1=%d\n", (int[4]){ [3] = 64 }[1]);
    printf("cl_desig_arr3=%d\n", (int[4]){ [3] = 64 }[3]);
    printf("cl_sizeof=%d\n", (int)sizeof(int[3]){ 1, 2, 3 });

    /* Form 1 again inside a loop body: a fresh unnamed object each iteration,
     * copied into a named object, with no address taken.  The accumulation
     * across the three iterations forces real run-time work and reaches 9, and
     * the compound assignment is a statement of its own, so no object is
     * modified twice between sequence points.
     */
    for (i = 0; i < 3; i++) {
        struct pair fresh = (struct pair){ i, i + 1 };
        printf("loop[%d].p=%d\n", i, fresh.p);
        printf("loop[%d].q=%d\n", i, fresh.q);
        loop_total += fresh.p + fresh.q;
    }
    printf("loop_total=%d\n", loop_total);

    /* A block-scope compound literal lives only until its enclosing block ends
     * (C11 6.5.2.5p5), so the pointer to this one is declared and read entirely
     * inside its own block and never escapes it.  Hoisting the declaration out
     * of the braces would be genuine undefined behaviour -- a use after the
     * object's lifetime -- which would invalidate the oracle rather than test
     * it, and is exactly what the sanitizer gate is watching for here.
     */
    {
        const int *scoped = (const int[]){ 71, 72, 73 };
        for (i = 0; i < 3; i++) {
            printf("scoped[%d]=%d\n", i, scoped[i]);
        }
    }

    /* The static-duration contrast: both of these literals were initialized at
     * file scope, so reading them here, after every block-scope literal above
     * has expired, is well defined.  Both pointers are dereferenced; neither is
     * printed.
     */
    printf("g_file_cl.p=%d\n", g_file_cl->p);
    printf("g_file_cl.q=%d\n", g_file_cl->q);
    for (i = 0; i < 3; i++) {
        printf("g_file_arr_cl[%d]=%d\n", i, g_file_arr_cl[i]);
    }
    return 0;
}
