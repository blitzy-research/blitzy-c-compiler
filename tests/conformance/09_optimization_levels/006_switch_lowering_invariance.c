/* 006_switch_lowering_invariance.c -- Area 09, conformance suite.
 *
 * Claim under test: switch results are IDENTICAL at -O0, -O1 and -O2, and across
 * all four backends, whichever lowering strategy each compiler chooses.  Label
 * density is what selects the strategy, so this program deliberately contains a
 * DENSE contiguous label set (which invites a jump table), a SPARSE widely
 * separated label set (which invites a comparison chain or binary search) and a
 * fallthrough group with a shared body.  Selectors are derived from volatile
 * storage so no switch can be folded to a single constant return.
 */

int printf(const char *, ...);

/* A DENSE label set: contiguous values invite a jump-table lowering. */
static int dense(int selector)
{
    switch (selector) {
    case 0:  return 100;
    case 1:  return 101;
    case 2:  return 102;
    case 3:  return 103;
    case 4:  return 104;
    case 5:  return 105;
    case 6:  return 106;
    case 7:  return 107;
    case 8:  return 108;
    case 9:  return 109;
    case 10: return 110;
    case 11: return 111;
    case 12: return 112;
    case 13: return 113;
    case 14: return 114;
    case 15: return 115;
    default: return -1;
    }
}

/* A SPARSE label set: widely separated values invite a comparison chain or a
   binary search instead of a jump table. */
static int sparse(int selector)
{
    switch (selector) {
    case -1000:  return 1;
    case 0:      return 2;
    case 7:      return 3;
    case 100:    return 4;
    case 1000:   return 5;
    case 65536:  return 6;
    case 1000000: return 7;
    default:     return 0;
    }
}

/* Fallthrough plus a shared body, which lowering must preserve exactly. */
static int grouped(int selector)
{
    int acc = 0;
    switch (selector) {
    case 1:
    case 2:
    case 3:
        acc += 10;
        /* falls through */
    case 4:
        acc += 20;
        break;
    case 5:
        acc += 40;
        break;
    default:
        acc += 80;
        break;
    }
    return acc;
}

int main(void)
{
    volatile int v_zero = 0;
    int base = v_zero;
    int i;

    for (i = 0; i < 17; i++) {
        printf("dense[%d]=%d\n", i, dense(base + i));
    }
    printf("dense[neg]=%d\n", dense(base - 5));

    printf("sparse[-1000]=%d\n", sparse(base - 1000));
    printf("sparse[0]=%d\n", sparse(base));
    printf("sparse[7]=%d\n", sparse(base + 7));
    printf("sparse[100]=%d\n", sparse(base + 100));
    printf("sparse[1000]=%d\n", sparse(base + 1000));
    printf("sparse[65536]=%d\n", sparse(base + 65536));
    printf("sparse[1000000]=%d\n", sparse(base + 1000000));
    printf("sparse[42]=%d\n", sparse(base + 42));

    for (i = 0; i < 7; i++) {
        printf("grouped[%d]=%d\n", i, grouped(base + i));
    }
    return 0;
}
