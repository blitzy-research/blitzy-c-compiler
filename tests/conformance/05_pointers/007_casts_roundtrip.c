/* Object-pointer casts round-tripped through an integer, VERIFIED before the result
 * is used for anything.
 *
 * THE CONVERSION IS IMPLEMENTATION-DEFINED IN BOTH DIRECTIONS, AND THAT IS THE
 * PREMISE THIS PROGRAM IS BUILT ON.  C11 6.3.2.3p6 makes the result of converting
 * a pointer to an integer type implementation-defined and does not even promise the
 * value is in the range of any integer type.  C11 6.3.2.3p5 makes the reverse
 * conversion implementation-defined too, and spells out how far that goes: the
 * result "might not be correctly aligned, might not point to an entity of the
 * referenced type, and might be a trap representation".  C11 Annex J.3.13 lists
 * both directions among the behaviours a conforming implementation must document
 * and may choose freely.
 *
 * EQUAL WIDTH IS NECESSARY BUT NOWHERE NEAR SUFFICIENT.  A carrier at least as wide
 * as `void *` rules out truncation -- and that is the only thing it rules out.  It
 * says nothing about which mapping the implementation picks, and it does not promise
 * that converting back yields a pointer to the original object.  Reasoning from
 * width to "the round trip is exact, so dereferencing the result is defined" is a
 * non-sequitur, and a program that made that leap and then dereferenced
 * unconditionally would itself be undefined on a conforming implementation whose
 * mapping is not the identity: the TEST would be at fault, not the compiler, and a
 * differential oracle would be comparing two runs of an invalid program.
 *
 * WHAT THIS PROGRAM DOES INSTEAD.  After every round trip it compares the
 * reconstructed pointer against the original and prints that comparison, and every
 * dereference, every subscript and every pointer difference sits inside a branch
 * guarded by that comparison.  Restoration is therefore OBSERVED, never assumed.
 * On an implementation that does not restore the pointer, each guarded line prints
 * `unrestored` in place of a value: the divergence surfaces as a difference in
 * stdout -- which is precisely what the oracles compare -- instead of being
 * performed as undefined behaviour.  Nothing below depends on the mapping being the
 * identity; the program's job is to report whether it is.
 *
 * THE WIDTH FACTS ARE ENFORCED, NOT ASSUMED.  Two _Static_assert declarations make
 * a target whose carrier could truncate fail to build rather than silently lose
 * bits, and the same two facts are also printed so they take part in the
 * comparison.  `uintptr_t` would be the natural carrier and is deliberately not
 * used: it is an optional typedef and would require a header, and no header may be
 * named here, so built-in integer types carry the value and the assertions
 * establish that they are wide enough.
 *
 * DETERMINISM.  Neither a pointer nor a carrier value is ever printed.  Only
 * comparison results, values read out of designated objects, and pointer
 * differences cast to long long ever reach the output, so nothing depends on where
 * the loader happened to place an object.  Two conversions are ordinary pointer
 * casts rather than integer round trips and are marked as such below, because C11
 * 6.3.2.3p7 does guarantee those: converting to a less strictly aligned type and
 * back yields the original pointer.  They are guarded anyway, so every case in the
 * file reads the same way. */

int printf(const char *, ...);

/* The carrier must be able to hold a pointer without losing bits, and the type
 * bridging back to a pointer must be exactly pointer width so that no conversion
 * between an 8-byte integer and a 4-byte pointer ever occurs on the 32-bit target.
 * Asserted at translation time: a target that broke either relation would fail to
 * build here, which is the correct outcome, because the alternative is a silent
 * truncation that the guards below would then report as a non-restoring mapping. */
_Static_assert(sizeof(unsigned long long) >= sizeof(void *),
               "the integer carrier is at least as wide as a pointer");
_Static_assert(sizeof(unsigned long) == sizeof(void *),
               "the bridge type back to a pointer is exactly pointer width");

/* Three of the printed values are execution-character-set codes for members of the
 * basic character set, whose numeric values C11 5.2.1 leaves to the implementation.
 * They are asserted rather than assumed: on an implementation with a different
 * execution character set this file fails to translate, instead of quietly printing
 * different numbers that an oracle would then have to attribute to a compiler
 * defect.  All four supported targets were measured to use ASCII, which is the
 * charset these three assertions pin. */
_Static_assert('a' == 97, "execution character set places 'a' at 97 (ASCII)");
_Static_assert('f' == 102, "execution character set places 'f' at 102 (ASCII)");
_Static_assert('h' == 104, "execution character set places 'h' at 104 (ASCII)");

struct pair {
    int a;
    int b;
};

static int obj = 7;
static double dobj = 2.5;
static char carr[16] = { 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h',
                         'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p' };
static int iarr[8] = { 10, 20, 30, 40, 50, 60, 70, 80 };
static struct pair sobj = { 3, 4 };
static const int cobj = 99;

int main(void)
{
    unsigned long long carrier;
    volatile int vidx;
    int k;

    /* The two width facts, printed as well as asserted, so they are compared
     * across compilers and backends rather than only checked at build time. */
    printf("carrier_wide_enough=%d\n",
           sizeof(unsigned long long) >= sizeof(void *));
    printf("bridge_is_pointer_width=%d\n",
           sizeof(unsigned long) == sizeof(void *));

    {
        int *p = &obj;
        int *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (int *)(void *)(unsigned long)carrier;
        /* The comparison first, then everything else inside its guard.  This is
         * the shape every case below repeats. */
        printf("int_roundtrip_eq=%d\n", q == p);
        if (q == p) {
            printf("int_roundtrip_value=%d\n", *q);
        } else {
            printf("int_roundtrip_value=unrestored\n");
        }
        /* A comparison needs no guard: it reads no storage through q. */
        printf("int_roundtrip_same_object=%d\n", q == &obj);
    }

    {
        double *p = &dobj;
        double *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (double *)(void *)(unsigned long)carrier;
        printf("double_roundtrip_eq=%d\n", q == p);
        if (q == p) {
            printf("double_roundtrip_value=%.3f\n", *q);
        } else {
            printf("double_roundtrip_value=unrestored\n");
        }
    }

    {
        char *p = carr;
        char *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (char *)(void *)(unsigned long)carrier;
        printf("char_roundtrip_eq=%d\n", q == p);
        if (q == p) {
            printf("char_roundtrip_value=%d\n", (int)(unsigned char)*q);
        } else {
            printf("char_roundtrip_value=unrestored\n");
        }

        p = &carr[5];
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (char *)(void *)(unsigned long)carrier;
        printf("char_interior_eq=%d\n", q == p);
        if (q == p) {
            /* Both the read and the pointer difference are guarded: subtracting
             * two pointers is defined only when both address elements of the same
             * array object, which is established by the comparison and not by the
             * conversion. */
            printf("char_interior_value=%d\n", (int)(unsigned char)*q);
            printf("char_interior_offset=%lld\n", (long long)(q - carr));
        } else {
            printf("char_interior_value=unrestored\n");
            printf("char_interior_offset=unrestored\n");
        }
    }

    {
        struct pair *p = &sobj;
        struct pair *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (struct pair *)(void *)(unsigned long)carrier;
        printf("struct_roundtrip_eq=%d\n", q == p);
        if (q == p) {
            printf("struct_roundtrip_values=%d %d\n", q->a, q->b);
        } else {
            printf("struct_roundtrip_values=unrestored\n");
        }
    }

    {
        const int *p = &cobj;
        const int *q;
        carrier = (unsigned long long)(unsigned long)(const void *)p;
        q = (const int *)(const void *)(unsigned long)carrier;
        printf("const_roundtrip_eq=%d\n", q == p);
        if (q == p) {
            printf("const_roundtrip_value=%d\n", *q);
        } else {
            printf("const_roundtrip_value=unrestored\n");
        }
    }

    {
        int *p = &iarr[3];
        int *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (int *)(void *)(unsigned long)carrier;
        printf("interior_roundtrip_eq=%d\n", q == p);
        if (q == p) {
            /* q designates iarr[3], so q[-1] and q[2] are iarr[2] and iarr[5],
             * both strictly inside an array of eight, and both differences are
             * between elements of that one array. */
            printf("interior_value=%d\n", *q);
            printf("interior_offset=%lld\n", (long long)(q - iarr));
            printf("interior_step_back=%d\n", q[-1]);
            printf("interior_step_forward=%d\n", q[2]);
            printf("interior_diff_matches=%lld\n", (long long)(q - &iarr[0]));
        } else {
            printf("interior_value=unrestored\n");
            printf("interior_offset=unrestored\n");
            printf("interior_step_back=unrestored\n");
            printf("interior_step_forward=unrestored\n");
            printf("interior_diff_matches=unrestored\n");
        }
    }

    {
        /* The null pointer constant is the one case C11 6.3.2.3p3 does settle: an
         * integer constant expression with value 0 converted to a pointer type is a
         * null pointer.  Round-tripping a null pointer through the carrier is still
         * an implementation-defined conversion, so the result is compared and never
         * dereferenced -- there is nothing to guard because nothing is read. */
        int *p = 0;
        int *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (int *)(void *)(unsigned long)carrier;
        printf("null_roundtrip_eq=%d\n", q == p);
        printf("null_roundtrip_is_null=%d\n", q == 0);
    }

    {
        /* Not an integer round trip: a pointer cast to void * and back. C11
         * 6.3.2.3p1 makes this exact, so this case is the control against which
         * the implementation-defined integer cases are read.  Guarded anyway, so
         * every case in the file has the same shape. */
        int *p = &iarr[6];
        void *v = p;
        int *q = (int *)v;
        printf("void_roundtrip_eq=%d\n", q == p);
        if (q == p) {
            printf("void_roundtrip_value=%d\n", *q);
            printf("void_roundtrip_offset=%lld\n", (long long)(q - iarr));
        } else {
            printf("void_roundtrip_value=unrestored\n");
            printf("void_roundtrip_offset=unrestored\n");
        }
    }

    {
        /* Also not an integer round trip: a detour through char *, which C11
         * 6.3.2.3p7 guarantees is exact because char has the least strict
         * alignment.  The byte view is formed and converted back but is never
         * written through, so no aliasing rule is engaged. */
        int *p = &iarr[2];
        char *bytes = (char *)(void *)p;
        int *q = (int *)(void *)bytes;
        printf("bytes_roundtrip_eq=%d\n", q == p);
        if (q == p) {
            printf("bytes_roundtrip_value=%d\n", *q);
        } else {
            printf("bytes_roundtrip_value=unrestored\n");
        }
    }

    /* Runtime variant: the index is read from volatile storage, so the pointer
     * that is round-tripped is not known before the program runs and the whole
     * conversion sequence has to be performed by emitted code rather than folded. */
    vidx = 4;
    k = vidx;
    {
        int *p = &iarr[k];
        int *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (int *)(void *)(unsigned long)carrier;
        printf("runtime_roundtrip_eq=%d\n", q == p);
        if (q == p) {
            printf("runtime_roundtrip_value=%d\n", *q);
            printf("runtime_roundtrip_offset=%lld\n", (long long)(q - iarr));
        } else {
            printf("runtime_roundtrip_value=unrestored\n");
            printf("runtime_roundtrip_offset=unrestored\n");
        }
        printf("runtime_roundtrip_target=%d\n", q == &iarr[4]);
    }

    vidx = 7;
    k = vidx;
    {
        char *p = &carr[k];
        char *q;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        q = (char *)(void *)(unsigned long)carrier;
        printf("runtime_char_eq=%d\n", q == p);
        if (q == p) {
            printf("runtime_char_value=%d\n", (int)(unsigned char)*q);
            printf("runtime_char_offset=%lld\n", (long long)(q - carr));
        } else {
            printf("runtime_char_value=unrestored\n");
            printf("runtime_char_offset=unrestored\n");
        }
    }

    vidx = 1;
    k = vidx;
    {
        /* Four conversions each way in sequence.  The intermediate pointers are
         * converted straight back to the carrier and are never read through, so
         * the only value that needs a guard is the last one. */
        int *p = &iarr[k];
        int *q;
        int i;
        carrier = (unsigned long long)(unsigned long)(void *)p;
        for (i = 0; i < 3; ++i) {
            int *tmp = (int *)(void *)(unsigned long)carrier;
            carrier = (unsigned long long)(unsigned long)(void *)tmp;
        }
        q = (int *)(void *)(unsigned long)carrier;
        printf("repeated_roundtrip_eq=%d\n", q == p);
        if (q == p) {
            printf("repeated_roundtrip_value=%d\n", *q);
        } else {
            printf("repeated_roundtrip_value=unrestored\n");
        }
    }

    return 0;
}
