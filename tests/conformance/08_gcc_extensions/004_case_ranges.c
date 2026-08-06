int printf(const char *, ...);

static int bucket_of(int v)
{
    switch (v) {
    case -20 ... -11: return 5;
    case 0 ... 9:     return 1;
    case 10 ... 19:   return 2;
    case 20:          return 3;
    case 21 ... 30:   return 4;
    default:          return 0;
    }
}

static int hex_of(int ch)
{
    switch (ch) {
    case '0' ... '9': return ch - '0';
    case 'a' ... 'f': return ch - 'a' + 10;
    case 'A' ... 'F': return ch - 'A' + 10;
    default:          return -1;
    }
}

static int single_range(int v)
{
    switch (v) {
    case 5 ... 5: return 42;
    default:      return 0;
    }
}

static int fallthrough_range(int v)
{
    int n = 0;
    switch (v) {
    case 1 ... 3:
        n += 1;
        /* fall through */
    case 4 ... 6:
        n += 10;
        break;
    case 7 ... 9:
        n += 100;
        break;
    default:
        break;
    }
    return n;
}

int main(void)
{
    int i;
    int b0 = 0, b1 = 0, b2 = 0, b3 = 0, b4 = 0, b5 = 0;
    volatile int vb = 13;
    volatile int vh = 'c';

    for (i = -25; i <= 35; i++) {
        switch (bucket_of(i)) {
        case 0: b0++; break;
        case 1: b1++; break;
        case 2: b2++; break;
        case 3: b3++; break;
        case 4: b4++; break;
        case 5: b5++; break;
        default: break;
        }
    }

    printf("cr_bucket_neg=%d\n", b5);
    printf("cr_bucket_b1=%d\n", b1);
    printf("cr_bucket_b2=%d\n", b2);
    printf("cr_bucket_b3=%d\n", b3);
    printf("cr_bucket_b4=%d\n", b4);
    printf("cr_bucket_default=%d\n", b0);
    printf("cr_hex_digit=%d\n", hex_of('7'));
    printf("cr_hex_lower=%d\n", hex_of('e'));
    printf("cr_hex_upper=%d\n", hex_of('B'));
    printf("cr_hex_none=%d\n", hex_of('z'));
    printf("cr_single_hit=%d\n", single_range(5));
    printf("cr_single_miss=%d\n", single_range(6));
    printf("cr_fall_low=%d\n", fallthrough_range(2));
    printf("cr_fall_mid=%d\n", fallthrough_range(5));
    printf("cr_fall_high=%d\n", fallthrough_range(8));
    printf("cr_fall_none=%d\n", fallthrough_range(99));
    printf("cr_runtime_bucket=%d\n", bucket_of(vb));
    printf("cr_runtime_hex=%d\n", hex_of(vh));
    return 0;
}
