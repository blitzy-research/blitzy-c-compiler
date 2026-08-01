/* Area 02 -- Constant Expressions and Folding, program 7 of 8:
 * string-literal constant handling and indexing.
 *
 * WHAT IS UNDER TEST.  A string literal is a constant of array type, and almost
 * everything interesting about one is decided before the program runs: its size
 * is an integer constant expression, adjacent pieces are joined in translation
 * phase 6, a backslash-newline splice is resolved earlier still in phase 2,
 * escape sequences are converted in phase 5, and the resulting array may then be
 * indexed, decayed to a pointer, used to initialize another array, or measured
 * with sizeof in a context that demands a constant expression -- an array bound.
 * Character constants sit on the same seam: they too are integer constant
 * expressions, and they too have a type the standard fixes (int, not char).
 * This program puts each of those properties on a line of its own.
 *
 * WHY EVERY PROPERTY GETS ITS OWN LINE.  A single aggregate value would report
 * only that something disagreed.  One line per property means a single divergent
 * line names the construct that produced it, which is what makes a finding
 * minimizable rather than merely alarming.
 *
 * THE TWO-VARIANT RULE.  Every computed value below is printed twice: once from
 * a constant expression the folder can evaluate at translation time (fold_*),
 * and once through an operand derived from a volatile object, which the compiler
 * must re-read and therefore cannot substitute (runtime_*).  A third line
 * asserts the two agree.  Without the runtime twin, optimization would quietly
 * answer with the constant folder's result and a code-generation defect would
 * never be asked to appear: verified at instruction level during design, where
 * `volatile int x = 7; return x * 6;` emits a genuine multiply at -O2 while the
 * non-volatile form collapses to a single immediate move.
 *
 * The one construct with no run-time counterpart is sizeof itself, which every
 * conforming implementation evaluates at translation time for a non-variable
 * type.  Its twin is therefore the walked length: every fold_size_ line below is
 * matched by a fold_len_ / runtime_len_ / len_agree_ triple over the same
 * object, where the folded length is sizeof minus one and the runtime length is
 * a bounded walk to the terminator.  Exactly one object is exempt -- the array
 * that deliberately carries no terminator -- and its exemption is stated at its
 * declaration, with its reason, rather than left as a silent gap.
 *
 * PLAIN-char SIGNEDNESS NORMALIZATION.  Plain char is signed on x86-64 and i686
 * and unsigned on AArch64 and RISC-V 64.  Every character this program handles
 * lies inside the 0x00-0x7F range, so its numeric value is identical under
 * either choice; on top of that, every numeric print goes through an explicit
 * (int)(unsigned char) conversion, so no signedness-dependent value can reach
 * stdout even by accident.  Characters themselves are printed with %c, whose
 * argument is promoted to int and whose output is the byte either way.
 *
 * WIDTH NORMALIZATION.  sizeof yields size_t, which is 4 bytes on i686 and 8 on
 * the other three targets, so every size is cast to int before it is printed.
 * For the same reason sizeof is only ever applied to a literal or to an array,
 * never to one of the pointers declared below -- sizeof a pointer would print a
 * target-dependent width and manufacture a divergence out of nothing.
 *
 * A NOTE ON POINTER-ARITHMETIC SPELLING.  C11 6.5.2.1p2 defines E1[E2] as
 * identical to (*((E1)+(E2))), so subscripting a literal already exercises
 * pointer arithmetic on it.  The explicit *(p + n) form is exercised as well,
 * but through a named pointer and through a decayed array rather than written
 * directly on a literal, because both reference compilers this suite supports
 * must accept the program unchanged: one of them diagnoses `"literal" + n` by
 * default as a suspected concatenation mistake, and the audit gate's -Werror
 * would turn that suggestion into a failure.  Nothing is given up -- the same
 * access is exercised three ways instead of one -- and nothing is silenced: no
 * pragma, no dropped warning flag, no gate deviation.  Area 02 sanctions none.
 *
 * UNDEFINED-BEHAVIOUR FREEDOM, which is what makes a divergence here mean
 * anything at all.  No string literal is ever written through; every literal is
 * reached only as const.  Where a mutable array is needed it is a separate
 * object filled from a literal element by element.  Every subscript is provably
 * within bounds, including the terminator index of a six-character literal,
 * whose array has seven elements; no one-past-end pointer is ever dereferenced.
 * The one array that deliberately lacks a terminator is never printed with %s
 * and never walked -- only its four known elements are read.  There is no
 * signed overflow, no shift, no aliasing violation, no read of uninitialized
 * storage, no object modified twice between sequence points, and no dependence
 * on padding bytes or on the addresses of unrelated objects.  In particular the
 * addresses of two literals are never compared: whether identical literals
 * share storage is unspecified, and this program may not depend on it.  Each
 * volatile object is read exactly once, into a local, in a statement of its
 * own, so no call ever receives more than one side-effecting argument and no
 * printed value depends on the order in which arguments are evaluated.
 *
 * NO HEADER IS INCLUDED.  The compiler under test ships nine freestanding
 * headers and no stdio.h, so an include would fail on one side of the oracle
 * while succeeding on the other -- a divergence caused by the test rather than
 * by the compiler.  string.h is absent from that set entirely, so strlen is
 * unavailable and lengths are computed here by a bounded walk.  Only ordinary
 * narrow literals appear: the wide and Unicode forms belong to area 11, which is
 * where their support is investigated.  The source is pure US-ASCII.
 */

int printf(const char *, ...);

/* ------------------------------------------------------------------------- *
 * Constant-expression assertions, every one of them satisfied.
 *
 * These are checked by the compiler rather than printed, so a wrong answer here
 * fails the build instead of a comparison.  The two-argument form is used
 * throughout: the message is mandatory in C11, and the one-argument spelling is
 * a later addition that -pedantic rejects.
 * ------------------------------------------------------------------------- */

_Static_assert(sizeof "abc" == 4,
               "a string literal is an array sized to include its terminator");
_Static_assert(sizeof "" == 1,
               "an empty string literal is an array of exactly one null byte");
_Static_assert(sizeof("one" "two") == 7,
               "adjacent literals are concatenated before the array is sized");
_Static_assert(sizeof("a" "b" "c" "d") == 5,
               "any number of adjacent literals concatenate into one array");
_Static_assert(sizeof('a') == sizeof(int),
               "a character constant has type int rather than char");
_Static_assert('A' == 65,
               "the basic execution character set encodes A as 65");
_Static_assert('\t' == 9,
               "the horizontal-tab escape denotes the value 9");
_Static_assert('\101' == 'A',
               "the octal escape 101 denotes the same value as the letter A");
_Static_assert('\x41' == 'A',
               "the hexadecimal escape 41 denotes the same value as the letter A");
_Static_assert('0' + 9 == '9',
               "the decimal digits are contiguous and increasing");

/* ------------------------------------------------------------------------- *
 * Literals reached through named pointers.
 *
 * These exist so that explicit pointer arithmetic can be performed on a literal
 * without spelling it directly on one.  sizeof is never applied to any of them:
 * that would measure a pointer, whose width differs between the targets.
 * ------------------------------------------------------------------------- */

static const char *const empty_literal = "";
static const char *const three_letters = "abc";
static const char *const six_letters = "abcdef";

/* The two inline concatenations, reached the same way so that their folded
   sizes carry walked-length twins as well.  Each is a single literal by the
   time the parser sees it, joined in translation phase 6. */
static const char *const two_pieces = "one" "two";
static const char *const four_pieces = "a" "b" "c" "d";

/* ------------------------------------------------------------------------- *
 * Literals in initializers.
 * ------------------------------------------------------------------------- */

/* Sized from its initializer, so the terminator is included and this array is a
   valid C string. */
static const char sized_from_literal[] = "text";

/* The deliberate counter-case.  C11 6.7.9p14 permits an initializing literal
   exactly one element longer than the array, in which case the terminator is
   dropped.  This array is therefore NOT a C string: it is never printed with
   %s and never walked for a terminator, only read across its four elements.
   That is also why it is the one object below whose size carries no walked
   length twin -- a walk would run past the end of the array, which is precisely
   the undefined behaviour this program exists not to commit.  Its run-time
   observation is the element-by-element dump instead. */
static const char exact_fit[4] = "text";

/* Three adjacent pieces, joined in translation phase 6. */
static const char joined_pieces[] = "one" "two" "three";

/* One piece spelled across two physical lines.  The backslash-newline splice is
   a phase 2 event, so this is a single literal rather than two adjacent ones --
   a different mechanism from joined_pieces above, and worth separating. */
static const char spliced_literal[] = "gam\
ma";

/* Escape sequences, held as unsigned char so that reading an element cannot
   yield a negative value on any target.  Contents in order: horizontal tab,
   backslash, double quote, single quote, newline, carriage return, the octal
   escape 101 and the hexadecimal escape 41 -- the last two both denoting A. */
static const unsigned char escape_bytes[] = "\t\\\"\'\n\r\101\x41";

/* The same escapes without the two that would move the output cursor, so the
   rendered form can occupy a line of its own without disturbing the line
   structure a byte-exact comparison depends on. */
static const char escape_render[] = "\t\\\"\'\101\x41";

/* A literal used as a constant lookup table -- the classic case where a
   constant index folds to an immediate and a computed one does not. */
static const char digit_table[] = "0123456789";

/* sizeof a string literal as an array bound.  An array bound must be an integer
   constant expression, which is exactly what sizeof of a literal is; the bound
   here is nine, the eight characters plus the terminator.  The array is mutable
   and separate from the literal, which is what lets it be filled at run time
   without ever writing through a literal. */
static char bound_table[sizeof "abcdefgh"];

/* ------------------------------------------------------------------------- *
 * Volatile operands.  A volatile object must be re-read on every access, so the
 * compiler may not propagate its initializer into the uses below: every index
 * and value derived from one of these is unknown at translation time and forces
 * a genuine load.  Each is read exactly once, into a local, in its own
 * statement.
 * ------------------------------------------------------------------------- */

static volatile int v_zero = 0;
static volatile int v_three = 3;
static volatile int v_five = 5;
static volatile int v_six = 6;
static volatile int v_seven = 7;
static volatile int v_capital_a = 'A';
static volatile int v_tab = '\t';
static volatile int v_octal_101 = '\101';
static volatile int v_hex_41 = '\x41';
static volatile int v_digit_nine = '9';

/* Length of a null-terminated character array, computed by walking to the
   terminator the caller guarantees is present.  This stands in for strlen,
   which is unavailable because string.h is not among the bundled headers. */
static int literal_length(const char *text)
{
    int length = 0;

    while (text[length] != '\0') {
        length++;
    }
    return length;
}

/* The same walk over unsigned char.  A separate function rather than a cast at
   each call site, because passing an unsigned char pointer where a char pointer
   is expected is itself a diagnostic the gate treats as an error, and a
   dedicated overload states the intent instead of suppressing the report. */
static int unsigned_length(const unsigned char *bytes)
{
    int length = 0;

    while (bytes[length] != '\0') {
        length++;
    }
    return length;
}

int main(void)
{
    int index;
    int fold_length;
    int runtime_length;
    int agree;
    int fold_bound_count;
    int runtime_bound_count;

    /* One volatile read per statement, so no call below receives more than one
       side-effecting argument. */
    int idx_first = v_zero;
    int idx_middle = v_three;
    int idx_last = v_five;
    int idx_terminator = v_six;
    int idx_seven = v_seven;
    int runtime_capital_a = v_capital_a;
    int runtime_tab = v_tab;
    int runtime_octal_101 = v_octal_101;
    int runtime_hex_41 = v_hex_41;
    int runtime_digit_nine = v_digit_nine;

    /* --- Sizes: integer constant expressions, one per literal form. ------ */
    printf("fold_size_empty=%d\n", (int)sizeof "");
    printf("fold_size_abc=%d\n", (int)sizeof "abc");
    printf("fold_size_abcdef=%d\n", (int)sizeof "abcdef");
    printf("fold_size_two_pieces=%d\n", (int)sizeof("one" "two"));
    printf("fold_size_four_pieces=%d\n", (int)sizeof("a" "b" "c" "d"));
    printf("fold_size_sized_from_literal=%d\n", (int)sizeof sized_from_literal);
    printf("fold_size_exact_fit=%d\n", (int)sizeof exact_fit);
    printf("fold_size_joined_pieces=%d\n", (int)sizeof joined_pieces);
    printf("fold_size_spliced_literal=%d\n", (int)sizeof spliced_literal);
    printf("fold_size_escape_bytes=%d\n", (int)sizeof escape_bytes);
    printf("fold_size_escape_render=%d\n", (int)sizeof escape_render);
    printf("fold_size_digit_table=%d\n", (int)sizeof digit_table);

    /* --- The text itself, so a wrong byte is visible and not merely a wrong
           count.  exact_fit is absent here on purpose: it has no terminator,
           and %s would read past the end of the array. ------------------- */
    printf("render_sized_from_literal=%s\n", sized_from_literal);
    printf("render_inline_two_pieces=%s\n", "one" "two");
    printf("render_inline_four_pieces=%s\n", "a" "b" "c" "d");
    printf("render_joined_pieces=%s\n", joined_pieces);
    printf("render_spliced_literal=%s\n", spliced_literal);
    printf("render_escape_render=%s\n", escape_render);

    /* --- The terminator-free array, read only across its known elements. -- */
    for (index = 0; index < (int)sizeof exact_fit; index++) {
        printf("exact_fit[%d]=%d\n", index, (int)(unsigned char)exact_fit[index]);
    }

    /* --- Length: sizeof minus one, against a walk the folder cannot do.
           This is the run-time twin of every size printed above, exact_fit
           excepted for the reason recorded at its declaration. ------------ */
    fold_length = (int)(sizeof "" - 1u);
    runtime_length = literal_length(empty_literal + idx_first);
    printf("fold_len_empty=%d\n", fold_length);
    printf("runtime_len_empty=%d\n", runtime_length);
    printf("len_agree_empty=%d\n", fold_length == runtime_length);

    fold_length = (int)(sizeof "abc" - 1u);
    runtime_length = literal_length(three_letters + idx_first);
    printf("fold_len_abc=%d\n", fold_length);
    printf("runtime_len_abc=%d\n", runtime_length);
    printf("len_agree_abc=%d\n", fold_length == runtime_length);

    fold_length = (int)(sizeof "abcdef" - 1u);
    runtime_length = literal_length(six_letters + idx_first);
    printf("fold_len_abcdef=%d\n", fold_length);
    printf("runtime_len_abcdef=%d\n", runtime_length);
    printf("len_agree_abcdef=%d\n", fold_length == runtime_length);

    fold_length = (int)(sizeof("one" "two") - 1u);
    runtime_length = literal_length(two_pieces + idx_first);
    printf("fold_len_two_pieces=%d\n", fold_length);
    printf("runtime_len_two_pieces=%d\n", runtime_length);
    printf("len_agree_two_pieces=%d\n", fold_length == runtime_length);

    fold_length = (int)(sizeof("a" "b" "c" "d") - 1u);
    runtime_length = literal_length(four_pieces + idx_first);
    printf("fold_len_four_pieces=%d\n", fold_length);
    printf("runtime_len_four_pieces=%d\n", runtime_length);
    printf("len_agree_four_pieces=%d\n", fold_length == runtime_length);

    fold_length = (int)(sizeof sized_from_literal - 1u);
    runtime_length = literal_length(sized_from_literal + idx_first);
    printf("fold_len_sized_from_literal=%d\n", fold_length);
    printf("runtime_len_sized_from_literal=%d\n", runtime_length);
    printf("len_agree_sized_from_literal=%d\n", fold_length == runtime_length);

    fold_length = (int)(sizeof joined_pieces - 1u);
    runtime_length = literal_length(joined_pieces + idx_first);
    printf("fold_len_joined_pieces=%d\n", fold_length);
    printf("runtime_len_joined_pieces=%d\n", runtime_length);
    printf("len_agree_joined_pieces=%d\n", fold_length == runtime_length);

    fold_length = (int)(sizeof spliced_literal - 1u);
    runtime_length = literal_length(spliced_literal + idx_first);
    printf("fold_len_spliced_literal=%d\n", fold_length);
    printf("runtime_len_spliced_literal=%d\n", runtime_length);
    printf("len_agree_spliced_literal=%d\n", fold_length == runtime_length);

    fold_length = (int)(sizeof escape_bytes - 1u);
    runtime_length = unsigned_length(escape_bytes + idx_first);
    printf("fold_len_escape_bytes=%d\n", fold_length);
    printf("runtime_len_escape_bytes=%d\n", runtime_length);
    printf("len_agree_escape_bytes=%d\n", fold_length == runtime_length);

    fold_length = (int)(sizeof escape_render - 1u);
    runtime_length = literal_length(escape_render + idx_first);
    printf("fold_len_escape_render=%d\n", fold_length);
    printf("runtime_len_escape_render=%d\n", runtime_length);
    printf("len_agree_escape_render=%d\n", fold_length == runtime_length);

    fold_length = (int)(sizeof digit_table - 1u);
    runtime_length = literal_length(digit_table + idx_first);
    printf("fold_len_digit_table=%d\n", fold_length);
    printf("runtime_len_digit_table=%d\n", runtime_length);
    printf("len_agree_digit_table=%d\n", fold_length == runtime_length);

    /* --- Subscripting a literal directly, constant index then computed. --- */
    printf("fold_index_chars=%c%c%c\n", "abcdef"[0], "abcdef"[3], "abcdef"[5]);
    printf("runtime_index_chars=%c%c%c\n",
           "abcdef"[idx_first], "abcdef"[idx_middle], "abcdef"[idx_last]);
    printf("fold_index_values=%d,%d,%d\n",
           (int)(unsigned char)"abcdef"[0],
           (int)(unsigned char)"abcdef"[3],
           (int)(unsigned char)"abcdef"[5]);
    printf("runtime_index_values=%d,%d,%d\n",
           (int)(unsigned char)"abcdef"[idx_first],
           (int)(unsigned char)"abcdef"[idx_middle],
           (int)(unsigned char)"abcdef"[idx_last]);
    agree = ("abcdef"[0] == "abcdef"[idx_first])
            && ("abcdef"[3] == "abcdef"[idx_middle])
            && ("abcdef"[5] == "abcdef"[idx_last]);
    printf("index_agree=%d\n", agree);

    /* --- The same access as explicit pointer arithmetic: once through a
           pointer to a literal, once through an array that decayed. ------- */
    printf("fold_pointer_deref=%c\n", *(six_letters + 3));
    printf("runtime_pointer_deref=%c\n", *(six_letters + idx_middle));
    printf("fold_decayed_deref=%c\n", *(sized_from_literal + 3));
    printf("runtime_decayed_deref=%c\n", *(sized_from_literal + idx_middle));
    agree = (*(six_letters + 3) == *(six_letters + idx_middle))
            && (*(sized_from_literal + 3) == *(sized_from_literal + idx_middle));
    printf("pointer_deref_agree=%d\n", agree);

    /* --- The terminator index is in bounds and holds zero.  "abcdef" has
           seven elements, so index six is the last of them. -------------- */
    printf("fold_terminator=%d\n", (int)(unsigned char)"abcdef"[6]);
    printf("runtime_terminator=%d\n",
           (int)(unsigned char)"abcdef"[idx_terminator]);
    printf("terminator_agree=%d\n", "abcdef"[6] == "abcdef"[idx_terminator]);

    /* --- A literal as a lookup table. ------------------------------------ */
    printf("fold_digit_pick=%c%c%c\n",
           digit_table[0], digit_table[7], digit_table[9]);
    printf("runtime_digit_pick=%c%c%c\n",
           digit_table[idx_first], digit_table[idx_seven],
           digit_table[idx_seven + 2]);
    printf("digit_agree=%d\n", digit_table[7] == digit_table[idx_seven]);

    /* --- Escape sequences, each byte named so a divergence identifies the
           escape that produced it rather than an offset. ----------------- */
    printf("fold_escape_tab=%d\n", (int)escape_bytes[0]);
    printf("fold_escape_backslash=%d\n", (int)escape_bytes[1]);
    printf("fold_escape_dquote=%d\n", (int)escape_bytes[2]);
    printf("fold_escape_squote=%d\n", (int)escape_bytes[3]);
    printf("fold_escape_newline=%d\n", (int)escape_bytes[4]);
    printf("fold_escape_return=%d\n", (int)escape_bytes[5]);
    printf("fold_escape_octal_101=%d\n", (int)escape_bytes[6]);
    printf("fold_escape_hex_41=%d\n", (int)escape_bytes[7]);

    printf("runtime_escape_tab=%d\n", (int)escape_bytes[idx_first]);
    printf("runtime_escape_backslash=%d\n", (int)escape_bytes[idx_first + 1]);
    printf("runtime_escape_dquote=%d\n", (int)escape_bytes[idx_first + 2]);
    printf("runtime_escape_squote=%d\n", (int)escape_bytes[idx_first + 3]);
    printf("runtime_escape_newline=%d\n", (int)escape_bytes[idx_first + 4]);
    printf("runtime_escape_return=%d\n", (int)escape_bytes[idx_first + 5]);
    printf("runtime_escape_octal_101=%d\n", (int)escape_bytes[idx_first + 6]);
    printf("runtime_escape_hex_41=%d\n", (int)escape_bytes[idx_first + 7]);

    /* Every escape read with a constant index against the same escape read
       through a volatile-derived one.  The compiler cannot prove the two
       indices are equal, so it must emit both loads. */
    agree = 1;
    for (index = 0; index < (int)(sizeof escape_bytes - 1u); index++) {
        if (escape_bytes[index] != escape_bytes[idx_first + index]) {
            agree = 0;
        }
    }
    printf("escape_agree=%d\n", agree);
    printf("octal_hex_agree=%d\n", escape_bytes[6] == escape_bytes[7]);

    /* --- Character constants: integer constant expressions of type int. --- */
    printf("fold_char_capital_a=%d\n", 'A');
    printf("runtime_char_capital_a=%d\n", runtime_capital_a);
    printf("fold_char_tab=%d\n", '\t');
    printf("runtime_char_tab=%d\n", runtime_tab);
    printf("fold_char_octal_101=%d\n", '\101');
    printf("runtime_char_octal_101=%d\n", runtime_octal_101);
    printf("fold_char_hex_41=%d\n", '\x41');
    printf("runtime_char_hex_41=%d\n", runtime_hex_41);
    printf("fold_char_digit_nine=%d\n", '9');
    printf("runtime_char_digit_nine=%d\n", runtime_digit_nine);
    printf("char_constant_is_int=%d\n", (int)(sizeof('a') == sizeof(int)));
    agree = ('A' == runtime_capital_a) && ('\t' == runtime_tab)
            && ('\101' == runtime_octal_101) && ('\x41' == runtime_hex_41)
            && ('9' == runtime_digit_nine);
    printf("char_const_agree=%d\n", agree);

    /* --- sizeof a literal as an array bound, then the array it sized.  The
           table is filled from a nine-element literal across its nine
           elements, terminator included, so every write is in bounds. ----- */
    fold_bound_count = (int)(sizeof bound_table / sizeof bound_table[0]);
    for (index = 0; index < fold_bound_count; index++) {
        bound_table[index] = "abcdefgh"[index];
    }
    runtime_bound_count = literal_length(bound_table + idx_first) + 1;
    printf("fold_bound_count=%d\n", fold_bound_count);
    printf("runtime_bound_count=%d\n", runtime_bound_count);
    printf("bound_count_agree=%d\n", fold_bound_count == runtime_bound_count);
    for (index = 0; index < fold_bound_count; index++) {
        printf("bound_table[%d]=%d\n", index,
               (int)(unsigned char)bound_table[index]);
    }
    return 0;
}
