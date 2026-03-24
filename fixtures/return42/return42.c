// A trivial fixture designed for in-process controlled execution.
// Returns 42 from main() without calling any external functions.
// No printf, no libc calls — just compute and return.
//
// Build: clang -arch arm64 -o return42 return42.c -nostdlib -e _main
// Note: -nostdlib avoids any dylib dependencies.

// We define _main directly as the entry point.
// The loader will call this with argc/argv, we just return 42.
int main(int argc, char **argv) {
    // Simple computation to verify code actually ran
    int result = 40;
    result += 2;  // should be 42
    return result;
}
