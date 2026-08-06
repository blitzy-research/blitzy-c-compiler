/* Every escape result is printed as an explicit integer value read through
   unsigned char, so no plain-char signedness difference can be observed and no
   raw control byte is written to stdout.  The source file is pure US-ASCII. */
int printf(const char *, ...);

static const unsigned char simple_escapes[] =
    "\a" "\b" "\f" "\n" "\r" "\t" "\v" "\\" "\'" "\"" "\?";

static const unsigned char octal_escapes[] =
    "\0" "\1" "\7" "\10" "\77" "\101" "\177" "\200" "\376" "\377";

static const unsigned char hex_escapes[] =
    "\x00" "\x01" "\x07" "\x08" "\x3f" "\x41" "\x7f" "\x80" "\xfe" "\xff";

static volatile int idx_zero = 0;

int main(void)
{
    int i;
    int agree = 1;

    printf("simple_count=%d\n", (int)(sizeof simple_escapes) - 1);
    for (i = 0; i < (int)(sizeof simple_escapes) - 1; i++) {
        printf("simple[%d]=%d\n", i, (int)simple_escapes[i]);
    }

    printf("octal_count=%d\n", (int)(sizeof octal_escapes) - 1);
    for (i = 0; i < (int)(sizeof octal_escapes) - 1; i++) {
        printf("octal[%d]=%d\n", i, (int)octal_escapes[i]);
    }

    printf("hex_count=%d\n", (int)(sizeof hex_escapes) - 1);
    for (i = 0; i < (int)(sizeof hex_escapes) - 1; i++) {
        printf("hex[%d]=%d\n", i, (int)hex_escapes[i]);
    }

    for (i = 0; i < (int)(sizeof octal_escapes) - 1; i++) {
        if (octal_escapes[i] != hex_escapes[i]) {
            agree = 0;
        }
    }
    printf("octal_hex_agree=%d\n", agree);

    printf("const_alert=%d\n", (int)'\a');
    printf("const_backspace=%d\n", (int)'\b');
    printf("const_formfeed=%d\n", (int)'\f');
    printf("const_newline=%d\n", (int)'\n');
    printf("const_return=%d\n", (int)'\r');
    printf("const_tab=%d\n", (int)'\t');
    printf("const_vtab=%d\n", (int)'\v');
    printf("const_backslash=%d\n", (int)'\\');
    printf("const_quote=%d\n", (int)'\'');
    printf("const_dquote=%d\n", (int)'\"');
    printf("const_question=%d\n", (int)'\?');
    printf("const_octal_101=%d\n", (int)'\101');
    printf("const_hex_41=%d\n", (int)'\x41');
    printf("const_nul=%d\n", (int)'\0');

    printf("runtime_simple0=%d\n", (int)simple_escapes[idx_zero]);
    printf("runtime_octal_last=%d\n",
           (int)octal_escapes[idx_zero + (int)(sizeof octal_escapes) - 2]);
    printf("runtime_hex_last=%d\n",
           (int)hex_escapes[idx_zero + (int)(sizeof hex_escapes) - 2]);
    return 0;
}
