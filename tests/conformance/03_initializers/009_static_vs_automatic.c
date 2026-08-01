/* Storage duration decides how often an initializer runs, and that is the whole subject of
 * this program.  A static-duration initializer is materialized ONCE, before the program
 * begins, into a data or BSS section; an automatic-duration initializer is materialized on
 * EVERY entry to its block, by code the backend emits.  A compiler can implement one
 * correctly and the other incorrectly, and only a program that observes both across
 * repeated calls can tell them apart.
 *
 * per_call() is invoked three times over three pairs of objects matched by shape and by
 * initializer -- s_counter/a_counter, s_arr/a_arr, s_st/a_st -- printing all six before
 * mutating them, applying the SAME mutation to both halves of every pair, then printing
 * again.  Because nothing but storage duration differs between the halves, any difference in
 * the printed sequence is attributable to storage duration alone:
 *
 *   - the static halves are initialized once, so their pre-mutation values ADVANCE across
 *     rounds: s_counter reads 100, 110, 120; s_arr reads 1,2,3 then 101,102,103 then
 *     201,202,203; s_st.p reads 7, 8, 9;
 *   - the automatic halves are re-initialized on every entry, so their pre-mutation values
 *     are IDENTICAL every round: a_counter is always 100, a_arr always 1,2,3, a_st always
 *     {7, 8}.
 *
 * Round 1 is where the halves legitimately coincide, which is itself the evidence that both
 * were initialized correctly; rounds 2 and 3 carry the diagnostic weight.  A compiler that
 * re-ran a function-scope static initializer on every call would print s_counter=100 three
 * times; one that failed to re-initialize an automatic object would print a_counter=110 in
 * round 2.  Either is visible on a single line.
 *
 * File scope covers the section-placement half of the same question: an initialized scalar,
 * array and struct (.data), a const array (typically .rodata), and an array with no
 * initializer at all, which has static storage duration and is therefore guaranteed zero
 * (C11 6.7.9p10) from BSS rather than from any emitted store.  g_static_scalar is mutated
 * after the call loop and re-read, showing that a .data object is writable, while
 * g_static_const is only ever read.
 *
 * Every element and member is read back and printed individually, and every in-function line
 * carries its round number, so a single wrong slot localizes to one initializer form in one
 * invocation.  No operand is qualified volatile anywhere, by design: a function-scope static
 * is a prime inlining and constant-propagation target, so identical output at -O0, -O1 and
 * -O2 is a genuine semantic-preservation check that forcing every access back to memory
 * would suppress rather than prove.  The static/automatic pairing is itself this program's
 * two-variant structure, performing each computation twice over identically initialized
 * operands that differ only in storage duration.
 *
 * Undefined-behaviour freedom: every accumulated value is tiny -- s_counter peaks at 130,
 * s_arr elements at 303, s_st members at 10, g_static_scalar at 1005 -- so no signed
 * overflow is possible on any target (INT_MAX is at least 2147483647 everywhere).  Every
 * subscript is strictly in bounds, no one-past-end pointer is formed, there are no shifts,
 * casts or aliasing, no object is modified twice between sequence points (each += is its own
 * full expression, never embedded in a call), and every printf argument is a plain read.
 * Only int and %d are used, so the narrower pointer and wide-integer widths of i686 (see
 * docs/technical-specifications.md lines 457-462) cannot be observed; no character data and
 * no struct representation is printed, so neither char signedness nor padding matters. */

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
    printf("round=%d after s_st.p=%d a_st.p=%d\n", round, s_st.p, a_st.p);
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
