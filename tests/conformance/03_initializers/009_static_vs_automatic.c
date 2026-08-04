/* Storage duration decides how often an initializer runs, and that is the whole subject of
 * this program.  A static-duration initializer takes effect ONCE, before the program begins;
 * an automatic-duration initializer takes effect on EVERY entry to its block.  A compiler can
 * implement one correctly and the other incorrectly, and only a program that observes both
 * across repeated calls can tell them apart.
 *
 * per_call() is invoked three times over three pairs of objects matched by shape and by
 * initializer -- s_counter/a_counter, s_arr/a_arr, s_st/a_st -- printing EVERY member of all six
 * before mutating them, applying the SAME mutation to every member of both halves of every pair,
 * then printing every member again.  Nothing is summarized and nothing is sampled: both struct
 * members appear on the pre-mutation line and on the post-mutation line of every round, so all four
 * struct observations per round -- s_st.p, s_st.q, a_st.p, a_st.q -- are made twice, and a member
 * that is mutated but never read back afterwards does not exist here.  Because nothing but storage
 * duration differs between the halves, any difference in the printed sequence is attributable to
 * storage duration alone:
 *
 *   - the static halves are initialized once, so their pre-mutation values ADVANCE across
 *     rounds: s_counter reads 100, 110, 120; s_arr reads 1,2,3 then 101,102,103 then
 *     201,202,203; s_st reads {7,8} then {8,9} then {9,10};
 *   - the automatic halves are re-initialized on every entry, so their pre-mutation values
 *     are IDENTICAL every round: a_counter is always 100, a_arr always 1,2,3, a_st always
 *     {7, 8};
 *   - the post-mutation lines carry the same contrast one increment further: s_st advances
 *     {8, 9}, {9, 10}, {10, 11} while a_st reads {8, 9} in every round.  A compiler that re-ran a
 *     function-scope static initializer, or failed to re-run an automatic one, is therefore caught
 *     on the post-mutation line as well as on the pre-mutation one.  The result of a mutation is
 *     observable twice over, which is what makes the contrast hard to fake: on the post-mutation
 *     line of the round that performed it, and again on the PRE-mutation line of the next round,
 *     where the static half carries the previous round's increment forward while the automatic half
 *     has been re-initialized back to its literal.
 *
 * Round 1 is where the halves legitimately coincide, which is itself the evidence that both
 * were initialized correctly; rounds 2 and 3 carry the diagnostic weight.  A compiler that
 * re-ran a function-scope static initializer on every call would print s_counter=100 three
 * times; one that failed to re-initialize an automatic object would print a_counter=110 in
 * round 2.  Either is visible on a single line.
 *
 * File scope covers the same question at the other storage class: an initialized scalar,
 * array and struct, a const array, and an array with no initializer at all, which has static
 * storage duration and is therefore guaranteed zero (C11 6.7.9p10) rather than reading as
 * whatever any statement stored.  g_static_scalar is mutated after the call loop and re-read,
 * showing a mutable static object is writable, while g_static_const is only ever read.
 *
 * Every element and member is read back individually and every in-function line carries its
 * round number, so a single wrong slot localizes to one initializer form in one invocation.
 * No operand is qualified volatile anywhere, by design: the static/automatic pairing is
 * itself this program's two-variant structure, performing each computation twice over
 * identically initialized operands that differ only in storage duration, and that is what
 * the run-time act already forces.  A function-scope static is a prime inlining and
 * constant-propagation target, so identical output at -O0, -O1 and -O2 is a genuine
 * semantic-preservation check that forcing every access back to memory would suppress rather
 * than prove.  Qualifying the operands volatile would change the accesses under test instead,
 * and the printed sequence must be the same at -O0, -O1 and -O2 however the initializations
 * are realized.
 *
 * Undefined-behaviour freedom: every accumulated value is tiny.  Stated individually rather
 * than summarized, because a summary that rounded them together would be wrong: after the
 * three calls s_counter is 130, s_arr holds 301, 302 and 303, s_st.p is 10 and s_st.q is 11
 * (each starts one apart and each is incremented once per call), and g_static_scalar ends at
 * 1005.  Every automatic twin is re-initialized on entry, so a_counter ends each call at 110,
 * a_arr at 101, 102 and 103, and a_st at 8 and 9.  No signed overflow is possible on any
 * target (INT_MAX is at least 2147483647 everywhere).  Every
 * subscript is a literal or a loop counter strictly in bounds, so the pointer arithmetic the
 * language performs for it never leaves its object; no one-past-end pointer is dereferenced, no
 * address of an object is taken, and no pointer value is printed, compared or converted -- the only
 * pointers are the format-string literals decaying at each printf call.  There are no shifts, casts
 * or aliasing, no object is modified twice between sequence points (each += is its own full
 * expression, never embedded in a call), and every printf argument is a plain read.
 * Every value computed and printed is an int printed with %d, so the narrower pointer and
 * wide-integer widths of i686 (see docs/technical-specifications.md lines 457-462) cannot be
 * observed; no plain-char value is read as a number and no struct representation is printed, so
 * neither char signedness nor padding matters. */

int printf(const char *, ...);

struct pair { int p; int q; };

static int          g_static_scalar = 1000;
static int          g_static_arr[4] = { 11, 12, 13, 14 };
static struct pair  g_static_st     = { 21, 22 };
static const int    g_static_const[3] = { 31, 32, 33 };
static int          g_static_bss[3];

static void per_call(int round)
{
    static int  s_counter      = 100;
    static int  s_arr[3]       = { 1, 2, 3 };
    static struct pair s_st    = { 7, 8 };
    int         a_counter      = 100;
    int         a_arr[3]       = { 1, 2, 3 };
    struct pair a_st           = { 7, 8 };
    int i;

    printf("round=%d s_counter=%d a_counter=%d\n", round, s_counter, a_counter);
    for (i = 0; i < 3; i++) {
        printf("round=%d s_arr[%d]=%d a_arr[%d]=%d\n", round, i, s_arr[i], i, a_arr[i]);
    }
    printf("round=%d s_st.p=%d s_st.q=%d a_st.p=%d a_st.q=%d\n",
           round, s_st.p, s_st.q, a_st.p, a_st.q);

    s_counter += 10;
    a_counter += 10;
    for (i = 0; i < 3; i++) {
        s_arr[i] += 100;
        a_arr[i] += 100;
    }
    s_st.p += 1;
    s_st.q += 1;
    a_st.p += 1;
    a_st.q += 1;

    printf("round=%d after s_counter=%d a_counter=%d\n", round, s_counter, a_counter);
    for (i = 0; i < 3; i++) {
        printf("round=%d after s_arr[%d]=%d a_arr[%d]=%d\n", round, i, s_arr[i], i, a_arr[i]);
    }
    /* All four members, matching the pre-mutation line above member for member.  Both
     * halves of the pair are mutated on both members, so printing only .p would leave the
     * two .q mutations unobserved: a backend that dropped the increment of the second
     * member of a two-member struct, or that mutated the wrong member, would produce
     * byte-identical output and the divergence would escape every oracle.  Observing .q
     * is also what makes the static/automatic contrast complete, because s_st.q has to
     * ADVANCE across rounds (9, 10, 11) while a_st.q has to be re-initialized and so reads
     * 9 in every round. */
    printf("round=%d after s_st.p=%d s_st.q=%d a_st.p=%d a_st.q=%d\n",
           round, s_st.p, s_st.q, a_st.p, a_st.q);
}

int main(void)
{
    int r;
    int i;

    printf("g_static_scalar=%d\n", g_static_scalar);
    for (i = 0; i < 4; i++) {
        printf("g_static_arr[%d]=%d\n", i, g_static_arr[i]);
    }
    printf("g_static_st.p=%d\n", g_static_st.p);
    printf("g_static_st.q=%d\n", g_static_st.q);
    for (i = 0; i < 3; i++) {
        printf("g_static_const[%d]=%d\n", i, g_static_const[i]);
    }
    for (i = 0; i < 3; i++) {
        printf("g_static_bss[%d]=%d\n", i, g_static_bss[i]);
    }
    for (r = 1; r <= 3; r++) {
        per_call(r);
    }
    g_static_scalar += 5;
    printf("g_static_scalar_after=%d\n", g_static_scalar);
    return 0;
}
