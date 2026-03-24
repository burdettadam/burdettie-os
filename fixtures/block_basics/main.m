// block_basics — T11 fixture: blocks runtime (global, stack, captured vars)
//
// Compiled with: clang -fblocks -fno-objc-arc -o app main.m
// (Uses plain C blocks syntax, no ObjC runtime dependency for blocks)
#include <stdio.h>
#include <Block.h>

int main() {
    // 1. Global block (no captures) — should invoke without Block_copy
    int (^global_blk)(void) = ^{ return 42; };
    int g = global_blk();
    if (g != 42) { printf("FAIL: global=%d\n", g); return 1; }

    // 2. Stack block with capture — needs Block_copy for safe heap use
    int x = 10;
    int (^stack_blk)(int) = ^(int y) { return x + y; };

    // Invoke on stack first (safe before scope ends)
    int s1 = stack_blk(5);
    if (s1 != 15) { printf("FAIL: stack=%d\n", s1); return 2; }

    // 3. Block_copy to heap
    int (^heap_blk)(int) = Block_copy(stack_blk);
    if (!heap_blk) { printf("FAIL: copy null\n"); return 3; }

    int h1 = heap_blk(7);
    if (h1 != 17) { printf("FAIL: heap=%d\n", h1); return 4; }

    // 4. Second Block_copy of heap block (refcount bump, same pointer)
    int (^heap2)(int) = Block_copy(heap_blk);
    if (heap2 != heap_blk) { printf("FAIL: copy2 ptr\n"); return 5; }

    // 5. Block_release twice (matches two copies)
    Block_release(heap2);
    Block_release(heap_blk);

    // 6. Global block through Block_copy — should return same pointer
    int (^gcopy)(void) = Block_copy(global_blk);
    if (gcopy != global_blk) { printf("FAIL: global copy ptr\n"); return 6; }
    Block_release(gcopy);  // no-op for global

    printf("T11: all checks passed\n");
    return 0;
}
