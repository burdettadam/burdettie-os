#include <stdio.h>

// Test ARM64e basic functionality:
// 1. Simple printf call (goes through __auth_got)
// 2. Function pointer call (PAC-signed on arm64e)
// 3. Return code check

static int helper(int x) {
    return x * 2;
}

int main(void) {
    printf("Test 1: printf through auth_got\n");

    int (*fp)(int) = helper;
    int result = fp(21);
    printf("Test 2: function pointer result=%d (expected 42)\n", result);
    if (result != 42) {
        printf("FAIL: expected 42, got %d\n", result);
        return 1;
    }

    printf("All arm64e tests passed!\n");
    return 0;
}
