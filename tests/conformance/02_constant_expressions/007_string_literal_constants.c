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
 * Every computed value is printed twice -- once from a constant expression the
 * folder can evaluate at translation time (fold_*), once through an operand derived
 * from a volatile object the compiler must re-read (runtime_*) -- with a third line
 * asserting the two agree.
 *
 * The one construct with no run-time counterpart is sizeof itself, which every
 * conforming implementation evaluates at translation time for a non-variable type.
 * Its twin is therefore the walked length: every fold_size_ line is matched by a
 * fold_len_ / runtime_len_ / len_agree_ triple over the same object, the folded
 * length being sizeof minus one and the runtime length a bounded walk to the
 * terminator.  Exactly one object is exempt -- the array that deliberately carries
 * no terminator -- and its exemption is stated at its declaration, with its reason.
 *
 * Two normalizations keep the four backends comparable.  Every character handled
 * here lies inside 0x00-0x7F, so its numeric value is identical whether plain char
 * is signed or unsigned, and every numeric print additionally goes through an
 * explicit (int)(unsigned char) conversion.  Every size is cast to int before it is
 * printed, because sizeof yields size_t, and sizeof is only ever applied to a
 * literal or an array, never to one of the pointers declared below, which would
 * print a target-dependent width and manufacture a divergence out of nothing.
 *
 * EXECUTION-CHARACTER-SET ACCOUNTING, which is a SEPARATE property from signedness
 * and is not settled by the conversion above.  This program observes fifteen
 * letters, three digits and six escape sequences -- some as numeric codes on a %d
 * line, some as bytes rendered through %c or %s across six whole-literal renderings
 * -- so the execution character set is genuinely part of its observable behaviour
 * and C11 5.2.1 leaves that set implementation-defined.  Every code the program
 * prints or renders is therefore pinned by a _Static_assert in the compile-time
 * block below, so an implementation whose character set differed would fail to
 * TRANSLATE rather than print different bytes that an oracle would attribute to the
 * compiler under test.  The two numeric escapes are exempt because C11 6.4.4.4p7
 * fixes them by value rather than by character set; the six simple escapes are not,
 * because 6.4.4.4p4 leaves those implementation-defined, and they are pinned.
 *
 * A NOTE ON POINTER-ARITHMETIC SPELLING.  C11 6.5.2.1p2 defines E1[E2] as identical
 * to (*((E1)+(E2))), so subscripting a literal already exercises pointer arithmetic
 * on it.  The explicit *(p + n) form is exercised as well, but through a named
 * pointer and a decayed array rather than written directly on a literal, because
 * both reference compilers this suite supports must accept the program unchanged and
 * one of them diagnoses `"literal" + n` by default as a suspected concatenation
 * mistake, which -Werror would turn into a failure.  Nothing is given up -- the same
 * access is exercised three ways -- and nothing is silenced: no pragma, no dropped
 * flag, no gate deviation.
 *
 * Freedom from undefined behaviour.  No string literal is ever written through;
 * where a mutable array is needed it is a separate object filled element by element.
 * Every subscript is provably within bounds and no one-past-end pointer is
 * dereferenced.  The array that deliberately lacks a terminator is never printed
 * with %s and never walked -- only its four known elements are read.  The addresses
 * of two literals are never compared, because whether identical literals share
 * storage is unspecified.  Each volatile object is read exactly once, into a local,
 * in a statement of its own, so no call receives more than one side-effecting
 * argument.
 *
 * No header is named: bcc ships no stdio.h, its bundled set being the nine required
 * freestanding headers plus a bonus stdatomic.h, ten files in all
 * (docs/project-guide.md line 212).  string.h is absent from that set, so strlen is
 * unavailable and lengths are computed here by a bounded walk.  Only ordinary narrow
 * literals appear; the wide and Unicode forms belong to area 11.
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

/* THE REST OF THE EXECUTION CHARACTER SET THIS PROGRAM PRINTS, PINNED.
 *
 * The five assertions above pin only the codes they name.  This program prints a
 * great deal more of the character set than that: bound_table dumps the numeric
 * codes of a through h, exact_fit dumps all four of its elements t, e, x and t,
 * fold_index_values prints a, d and f, fold_char_digit_nine prints the code of
 * the digit 9, six lines render
 * whole literals with %s (text, onetwo, abcd, onetwothree, gamma and the escape
 * rendering), four further observations render characters with %c and eight
 * escape codes are printed as numbers -- each of those twelve appearing twice,
 * once folded and once recomputed at run time.  Every
 * one of those is an implementation-defined value: C11 5.2.1 leaves the members
 * and the codes of the execution character set implementation-defined, and the
 * (int)(unsigned char) conversion applied throughout normalizes plain char's
 * SIGNEDNESS, which is a different property entirely and says nothing about which
 * code a letter carries.
 *
 * Two of the nine escape spellings this program handles need no assertion.  C11
 * 6.4.4.4p7 fixes the value of an octal-escape or hexadecimal-escape character
 * constant NUMERICALLY -- the escape denotes the value the digits spell, whatever
 * the character set -- so \101 and \x41 are 65 by the standard rather than by the
 * environment, and the two assertions above exist to relate them to the letter A,
 * not to establish their codes.  The remaining seven escapes are a different
 * matter: 6.4.4.4p4 makes every SIMPLE escape sequence denote a member of the
 * execution character set
 * whose value is implementation-defined, so \t, \\, \", \', \n, \r and \? all
 * need pinning, and the pin for \t is above.  \? is pinned because it is one of
 * the six bytes the escape rendering writes out; it stands in the horizontal
 * tab's place there for the reason recorded at that array's declaration.
 *
 * The pins below are grouped into three assertions -- letters, digits, escapes --
 * rather than spelled one per code, so the accounting reads as three facts rather
 * than twenty-three.  An implementation whose execution
 * character set differed FAILS TO TRANSLATE with a named diagnostic, rather than
 * printing different numbers and different rendered bytes for oracle (a) to
 * charge to the compiler under test.  All four supported targets were measured to
 * use the same ASCII-family basic execution character set, so the condition holds
 * on every enabled cell; a future target that disagreed would announce itself at
 * the build, which is where an environment difference belongs.  The record's
 * impl_defined_notes carries the same accounting in prose. */
_Static_assert('a' == 97 && 'b' == 98 && 'c' == 99 && 'd' == 100 && 'e' == 101
                   && 'f' == 102 && 'g' == 103 && 'h' == 104 && 'm' == 109
                   && 'n' == 110 && 'o' == 111 && 'r' == 114 && 't' == 116
                   && 'w' == 119 && 'x' == 120,
               "every letter this program prints or renders sits at its ASCII code");
_Static_assert('0' == 48 && '7' == 55 && '9' == 57,
               "the decimal digits this program prints sit at their ASCII codes");
_Static_assert('\\' == 92 && '\"' == 34 && '\'' == 39 && '\n' == 10 && '\r' == 13
                   && '\?' == 63,
               "the six remaining simple escapes sit at their ASCII codes");

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

/* The escapes that can be RENDERED, as opposed to only counted.  Every byte here
   is a graphic character, and that is a hard requirement of the expectation record
   rather than a matter of taste.  The golden output is carried in the record's
   expected_stdout field, and a field value is written verbatim into a report row,
   into a tab-separated summary column and into a findings manifest, so a raw
   control byte there would forge a column boundary or erase the line that carries
   it, making the record say something other than what was measured -- a forged
   column in the summary can go as far as relabelling a verdict.  The record
   format therefore admits no control character but the line feed that joins its
   own lines, and the harness refuses a record containing one.  So a program whose
   stdout carried a tab could not have a conforming record at all -- and this was
   the one program in the corpus that did, which is precisely why its record was
   the one that could not be written.
   The three escapes that would produce a control byte -- horizontal tab, newline
   and carriage return -- are consequently covered by escape_bytes above, whose
   elements are printed as NUMBERS, and the question-mark escape stands in the
   tab's place here so that this array still holds six escape-produced bytes and
   its size and length lines are unaffected.  Excluding the newline and the
   carriage return is obvious: either would move the output cursor and destroy the
   line structure a byte-exact comparison depends on.
   Nothing is given up by the exclusion.  The tab is still fully under test,
   four times over: its numeric code is printed from escape_bytes[0] folded and
   again through a volatile-derived index, printed again as fold_char_tab and
   runtime_char_tab, and pinned at translation time by _Static_assert('\t' == 9)
   above.  Only the RENDERING of a control byte is dropped, and that is the part
   of the job that belongs to something else: converting the escape to byte 9 is
   the COMPILER's work and stays asserted, whereas writing byte 9 out to a stream
   exercises the library rather than the compiler.  The same convention is
   followed by 11_literals_and_strings/001_character_escapes.c, which owns escape
   coverage in breadth and prints every escape it handles as a number and never
   as a byte.
   Contents in order: the question-mark escape, backslash, double quote, single
   quote, the octal escape 101 and the hexadecimal escape 41 -- the last two both
   denoting A. */
static const char escape_render[] = "\?\\\"\'\101\x41";

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
