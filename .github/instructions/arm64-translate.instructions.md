---
description: "arm64-translate testing conventions. Use when: editing crates/arm64-translate/src/**/*.rs, writing ARM64 decode/execute tests."
applyTo: "crates/arm64-translate/src/**/*.rs"
---

# arm64-translate Test Conventions

## Crate Structure

- `decode/` — A64 instruction decoder: `decode(raw: u32, pc: u64) -> Instruction`
- `interp/` — Fetch-decode-execute interpreter loop
- `cpu.rs` — CpuState: 31 GPRs (x0–x30), SP, PC, NZCV, SIMD/FP (v0–v31)
- `mem.rs` — Memory model: segment-backed + mmap'd stack
- `ffi.rs` — FFI bridge for host function calls
- `test_helpers.rs` — Test infrastructure (test interpreter, encoding helpers)

## Test Infrastructure

Import: `use crate::test_helpers::*;`

### Decode Tests (pure, no state)
```rust
use crate::decode::{decode, Instruction};
#[test]
fn test_decode_add_imm_64() {
    let insn = decode(enc_add_imm(true, 1, 2, 0x10, false), 0x1000);
    assert!(matches!(insn, Instruction::AddImm { sf: true, rd: 1, rn: 2, imm: 0x10, shift: 0 }));
}
```

### Execute Tests (full pipeline via test_interp)
```rust
use crate::test_helpers::*;
#[test]
fn test_exec_add_imm_64() {
    let mut t = test_interp(&[enc_add_imm(true, 0, 1, 0x10, false)]);
    t.interp.cpu.x[1] = 100;
    let cpu = t.run_ok();
    assert_eq!(cpu.x[0], 116);
}
```

### Load/Store Execute Tests
```rust
use crate::test_helpers::*;
#[test]
fn test_exec_ldr_str_64() {
    let mut t = test_interp(&[
        enc_str_imm64(0, 2, 0),   // STR X0, [X2, #0]
        enc_ldr_imm64(1, 2, 0),   // LDR X1, [X2, #0]
    ]);
    t.interp.cpu.x[0] = 0xDEAD;
    t.interp.cpu.x[2] = t.data_base;
    let cpu = t.run_ok();
    assert_eq!(cpu.x[1], 0xDEAD);
}
```

## Encoding Helpers

Available in `test_helpers.rs`:
- DP imm: `enc_add_imm`, `enc_adds_imm`, `enc_sub_imm`, `enc_subs_imm`, `enc_movz`, `enc_movk`, `enc_movn`, `enc_ubfm`, `enc_sbfm`
- DP reg: `enc_add_reg`, `enc_adds_reg`, `enc_sub_reg`, `enc_subs_reg`, `enc_madd`, `enc_msub`, `enc_udiv`, `enc_sdiv`, `enc_mov_reg`, `enc_orr_reg`, `enc_and_reg`, `enc_csel`, `enc_csinc`
- Branch: `enc_b`, `enc_bl`, `enc_br`, `enc_blr`, `enc_ret`, `enc_cbz`, `enc_cbnz`, `enc_bcond`, `enc_tbz`, `enc_tbnz`
- Load/Store: `enc_ldr_imm64`, `enc_ldr_imm32`, `enc_str_imm64`, `enc_str_imm32`, `enc_ldrb_imm`, `enc_strb_imm`, `enc_ldrh_imm`, `enc_strh_imm`, `enc_stp64`, `enc_ldp64`, `enc_stp32`, `enc_ldp32`
- FP: `enc_fadd`, `enc_fsub`, `enc_fmul`, `enc_fdiv`, `enc_fcmp`, `enc_fneg`, `enc_fabs`, `enc_scvtf`, `enc_fcvtzs`, `enc_fmov_reg`, `enc_fmov_to_gpr64`, `enc_fmov_to_fp64`, `enc_fp_ldr_imm64`, `enc_fp_str_imm64`, `enc_fp_ldr_imm32`, `enc_fp_str_imm32`
- System: `enc_nop`, `enc_svc`

For instructions not covered by helpers, use raw hex values with the ARM64 encoding formula.

## Key Rules

1. Tests go inline: `#[cfg(test)] mod tests { use super::*; use crate::test_helpers::*; }`
2. If a test module already exists, ADD tests to it — do not create a duplicate.
3. Use `matches!()` for decode assertions to check variant + key fields.
4. For execute tests, always use `test_interp()` — it handles memory + CPU + sentinel LR.
5. Set data register to `t.data_base` (+ offset) for load/store address operands.
6. `run_ok()` asserts clean return, returns `&CpuState` for assertions.
7. Test both 32-bit (sf=false) and 64-bit (sf=true) variants where applicable.
8. Test edge cases: zero register (x31→XZR or SP depending on encoding), overflow, flags.
9. NZCV: check flags after flag-setting instructions (ADDS, SUBS, CMP, ANDS, FCMP).

## Build

Cargo writes to `target/` by default. For a custom build location, use
`CARGO_TARGET_DIR` and resolve the directory through `cargo metadata`.
Run: `cargo test -p arm64-translate`
