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
 *    7. subscripted directly, with no intervening variable, at every index
 *    8. member-selected directly, at every member
 *    9. designated array literal, read at every slot -- the designated one and
 *       the three left implicitly zero
 *   10. sizeof applied to a compound literal
 *
 * Forms 4 to 6 are observed from INSIDE the function they are passed to, so every
 * member and element that crossed the parameter-passing boundary gets its own
 * labelled line, and the encoded return values remain only as additional
 * cross-checks.  Across forms 4 to 9 that is sixteen directly labelled values --
 * two for each pair-valued literal, three for each three-element array literal
 * and four for the four-element designated array -- so no initialized slot is
 * represented only by a sum, an encoding or a sampled neighbour.  That matters
 * because a checksum or an encoding can be reproduced by two compensating
 * errors, such as a swapped pair of members, whereas a per-slot line cannot.
 *
 * Two further cases sit beside them: a compound literal in a loop body, which by
 * C11 6.5.2.5p5 denotes ONE unnamed object per source occurrence with the lifetime
 * of its enclosing block, so each trip round the loop REINITIALIZES that same object
 * from the current counter rather than creating a further one -- the program reads
 * the reinitialized VALUE and never the object's identity, since its address is never
 * taken -- and two file-scope literals, which have static rather than automatic
 * storage duration.
 *
 * Only int is computed and only %d is printed, so nothing here depends on a
 * target-varying width, on plain-char signedness, or on padding bytes, whose
 * contents are unspecified.  No address is ever printed: every pointer below is
 * dereferenced and its pointee reported instead.
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
 * boundary while the values consumed directly in main are the other variant, so
 * a compound literal is exercised both as an argument crossing a function
 * boundary and as an operand read in place.  Whatever an implementation
 * transforms either route into, the printed values may not change.  A
 * `volatile` twin is deliberately NOT added here: the accumulation across the
 * loop below already forces real run-time work, and qualifying an operand would
 * change the accesses under test rather than the initializer producing them.
 *
 * Each helper prints EVERY member or element it received, from inside itself,
 * before returning an encoded value.  Printing from inside the callee is what
 * makes the observation meaningful: the value read there arrived through the
 * parameter-passing path under test -- by value for take_val, by address for
 * take_ptr, and as a decayed pointer for sum3 -- so a member that a calling
 * convention delivered to the wrong place shows up on its own labelled line.  The
 * encoded returns are kept as ADDITIONAL cross-checks, never as substitutes:
 * p * 100 + q and a[0] + a[1] + a[2] agree with the per-member lines only if both
 * the passing path and the callee's own reads are correct, whereas an encoding or
 * a sum on its own could be produced by two compensating errors.  The largest
 * value any of them yields is 506, far inside the guaranteed range of int, so no
 * signed overflow is possible.
 *
 * Each call is a statement of its own whose result is stored in a local before it
 * is printed, so no printf argument expression has a side effect even though the
 * helpers now write to stdout, and the order of the printed lines is fixed by
 * statement order rather than by an unspecified argument evaluation order.
 */
static int take_val(struct pair v)
{
    printf("take_val_p=%d\n", v.p);
    printf("take_val_q=%d\n", v.q);
    return v.p * 100 + v.q;
}

static int take_ptr(const struct pair *v)
{
    printf("take_ptr_p=%d\n", v->p);
    printf("take_ptr_q=%d\n", v->q);
    return v->p * 100 + v->q;
}

static int sum3(const int *a)
{
    printf("sum3_a0=%d\n", a[0]);
    printf("sum3_a1=%d\n", a[1]);
    printf("sum3_a2=%d\n", a[2]);
    return a[0] + a[1] + a[2];
}

int main(void)
{
    struct pair v_expr = (struct pair){ 1, 2 };
    struct pair v_desig = (struct pair){ .q = 9 };
    struct nest v_nested = (struct nest){ 11, (struct pair){ 12, 13 } };
    int i;
    int loop_total = 0;
    int r_take_val;
    int r_take_ptr;
    int r_sum3;

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

    /* Forms 4 to 6.  The two statements that pass an address -- the & literal and
     * the array literal that decays to a pointer -- use it only for the duration
     * of the call, which completes inside the same full expression, so neither
     * pointer outlives the object it designates and neither is stored anywhere.
     * Comparing the by-value lines against the by-address lines separates a
     * calling-convention defect from a lifetime defect: one changes and the other
     * does not.  Each helper emits its members first, from inside itself, so the
     * three per-form groups read as members then encoded total.
     */
    r_take_val = take_val((struct pair){ 3, 4 });
    printf("take_val_arg=%d\n", r_take_val);
    r_take_ptr = take_ptr(&(struct pair){ 5, 6 });
    printf("take_ptr_arg=%d\n", r_take_ptr);
    r_sum3 = sum3((int[]){ 10, 20, 30 });
    printf("sum3_arg=%d\n", r_sum3);

    /* Forms 7 to 10, applied to a literal with no intervening variable at all,
     * which is where a compiler that mishandles the unnamed object's address
     * computation or lifetime shows up first.  EVERY element and member of these
     * literals is read, not a sample: all three indices of the three-element
     * array, both members of the pair, and all four slots of the designated array
     * including the three left implicitly zero, so a value written to the wrong
     * slot cannot hide in an index nobody looked at.  Every subscript is strictly
     * in bounds, and sizeof is applied to an int[3] rather than to a pointer, so
     * the answer is 12 on all four targets.
     */
    printf("cl_index0=%d\n", (int[]){ 41, 42, 43 }[0]);
    printf("cl_index1=%d\n", (int[]){ 41, 42, 43 }[1]);
    printf("cl_index2=%d\n", (int[]){ 41, 42, 43 }[2]);
    printf("cl_member_p=%d\n", (struct pair){ 51, 52 }.p);
    printf("cl_member_q=%d\n", (struct pair){ 51, 52 }.q);
    printf("cl_desig_arr0=%d\n", (int[4]){ [3] = 64 }[0]);
    printf("cl_desig_arr1=%d\n", (int[4]){ [3] = 64 }[1]);
    printf("cl_desig_arr2=%d\n", (int[4]){ [3] = 64 }[2]);
    printf("cl_desig_arr3=%d\n", (int[4]){ [3] = 64 }[3]);
    printf("cl_sizeof=%d\n", (int)sizeof(int[3]){ 1, 2, 3 });

    /* Form 1 again inside a loop body.  The literal is one source occurrence, so it
     * denotes ONE unnamed object with the lifetime of the enclosing block (C11
     * 6.5.2.5p5); each evaluation REINITIALIZES that object from the current value of
     * i rather than producing a further object.  This program reads that value and
     * never the object's identity, and deliberately takes no address, so nothing here
     * rests on where the object lives or on how the implementation materializes it
     * each trip round.  The value is
     * copied into a named object.  The accumulation
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
