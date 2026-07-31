/* Area 11 / 002 - string literal concatenation, indexing and termination.
   No string literal is ever modified.  Every element is read through
   unsigned char.  Source file is pure US-ASCII. */
int printf(const char *, ...);

static const char concatenated[] = "abc" "def" "ghi";
static const char with_empty[]   = "" "xy" "" "z" "";
static const char across_lines[] = "one-"
                                   "two-"
                                   "three";
static const char with_escapes[] = "A\tB" "\x43" "D";
static const char embedded_nul[] = "pq\0rs";

static volatile int zero = 0;

static int literal_length(const char *s)
{
    int n = 0;
    while (s[n] != '\0') {
        n++;
    }
    return n;
}

int main(void)
{
    int i;

    printf("concat_size=%d\n", (int)sizeof concatenated);
    printf("concat_len=%d\n", literal_length(concatenated));
    for (i = 0; i < (int)sizeof concatenated - 1; i++) {
        printf("concat[%d]=%d\n", i, (int)(unsigned char)concatenated[i]);
    }
    printf("concat_terminator=%d\n",
           (int)(unsigned char)concatenated[(int)sizeof concatenated - 1]);

    printf("empty_size=%d\n", (int)sizeof with_empty);
    printf("empty_len=%d\n", literal_length(with_empty));
    printf("empty_bytes=%d,%d,%d\n",
           (int)(unsigned char)with_empty[0],
           (int)(unsigned char)with_empty[1],
           (int)(unsigned char)with_empty[2]);

    printf("lines_size=%d\n", (int)sizeof across_lines);
    printf("lines_len=%d\n", literal_length(across_lines));
    printf("lines_first=%d lines_last=%d\n",
           (int)(unsigned char)across_lines[0],
           (int)(unsigned char)across_lines[(int)sizeof across_lines - 2]);

    printf("escapes_size=%d\n", (int)sizeof with_escapes);
    for (i = 0; i < (int)sizeof with_escapes - 1; i++) {
        printf("escapes[%d]=%d\n", i, (int)(unsigned char)with_escapes[i]);
    }

    printf("nul_size=%d\n", (int)sizeof embedded_nul);
    printf("nul_len=%d\n", literal_length(embedded_nul));
    for (i = 0; i < (int)sizeof embedded_nul; i++) {
        printf("nul[%d]=%d\n", i, (int)(unsigned char)embedded_nul[i]);
    }

    printf("direct_index=%d\n", (int)(unsigned char)"hello"[1]);
    printf("direct_size=%d\n", (int)sizeof "hello");
    printf("direct_terminator=%d\n", (int)(unsigned char)"hello"[5]);

    printf("runtime_concat0=%d\n", (int)(unsigned char)concatenated[zero]);
    printf("runtime_concat_len=%d\n", literal_length(concatenated + zero));
    return 0;
}
