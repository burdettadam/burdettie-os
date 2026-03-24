// Thread-local variable fixture for TLV descriptor testing.
#include <stdio.h>

_Thread_local int tls_counter = 42;
_Thread_local int tls_flag = 0;

int main(void) {
    tls_counter += 1;
    tls_flag = 1;
    printf("counter=%d flag=%d\n", tls_counter, tls_flag);
    return 0;
}
