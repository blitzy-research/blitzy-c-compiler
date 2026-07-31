/* The _Noreturn function really never returns: it terminates the program by
 * calling exit, so control never falls off its end, which would be undefined
 * behaviour. */

int printf(const char *, ...);
_Noreturn void exit(int);

static int reachable_probe(int v) { return v + 1; }

_Noreturn static void terminate_now(int code)
{
    printf("noreturn_reached=1 code=%d\n", code);
    exit(code);
}

int main(void)
{
    int before = reachable_probe(0);

    printf("before=%d\n", before);
    printf("reachable=%d\n", reachable_probe(before));
    terminate_now(0);
    printf("after_noreturn=1\n");
}
