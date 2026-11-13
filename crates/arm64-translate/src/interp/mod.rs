//! Fetch-decode-execute interpreter loop.
//!
//! Steps through ARM64 machine code one instruction at a time, using:
//! - `CpuState` for register/flag state
//! - `Memory` for load/store operations
//! - `FfiBridge` for host function calls
//! - `decode()` for instruction decoding
//!
//! ## Block cache
//!
//! The interpreter maintains a `BlockCache` that maps each basic-block start
//! PC to a pre-decoded `Arc<[Instruction]>`.  On subsequent visits to a basic
//! block the decode step is skipped entirely — only execution is needed.
//! Code pages are immutable after load (Mach-O __TEXT is r-x), so no cache
//! invalidation is required.

mod exec_branch;
mod exec_dp;
mod exec_dp_extended;
mod exec_fp;
mod exec_fp_dp;
mod exec_ls;
mod exec_ls_extended;
mod exec_simd;

use std::collections::HashMap;
use std::sync::Arc;

/// DRY helper: read a u64 from memory or return a MemoryFault exit.
#[macro_export]
macro_rules! mem_read_u64 {
    ($self:expr, $addr:expr, $pc:expr) => {
        match $self.mem.read_u64($addr) {
            Some(v) => v,
            None => {
                return StepResult::Exit(ExitReason::MemoryFault {
                    pc: $pc,
                    addr: $addr,
                    is_write: false,
                })
            }
        }
    };
}

/// DRY helper: read a u32 from memory or return a MemoryFault exit.
#[macro_export]
macro_rules! mem_read_u32 {
    ($self:expr, $addr:expr, $pc:expr) => {
        match $self.mem.read_u32($addr) {
            Some(v) => v,
            None => {
                return StepResult::Exit(ExitReason::MemoryFault {
                    pc: $pc,
                    addr: $addr,
                    is_write: false,
                })
            }
        }
    };
}

/// DRY helper: read a u16 from memory or return a MemoryFault exit.
#[macro_export]
macro_rules! mem_read_u16 {
    ($self:expr, $addr:expr, $pc:expr) => {
        match $self.mem.read_u16($addr) {
            Some(v) => v,
            None => {
                return StepResult::Exit(ExitReason::MemoryFault {
                    pc: $pc,
                    addr: $addr,
                    is_write: false,
                })
            }
        }
    };
}

/// DRY helper: read a u8 from memory or return a MemoryFault exit.
#[macro_export]
macro_rules! mem_read_u8 {
    ($self:expr, $addr:expr, $pc:expr) => {
        match $self.mem.read_u8($addr) {
            Some(v) => v,
            None => {
                return StepResult::Exit(ExitReason::MemoryFault {
                    pc: $pc,
                    addr: $addr,
                    is_write: false,
                })
            }
        }
    };
}

use crate::cpu::CpuState;
use crate::decode::{self, Instruction};
use crate::ffi::FfiBridge;
use crate::jit::JitCache;
use crate::mem::Memory;

/// Why the interpreter stopped.
#[derive(Debug, Clone)]
pub enum ExitReason {
    /// The program returned from main (x0 = exit code).
    Returned { code: i64 },
    /// An explicit exit() call was made.
    Exit { code: i64 },
    /// RET executed with a return address that's outside mapped code.
    ReturnedToHost { lr: u64 },
    /// Hit the maximum instruction count.
    InstructionLimit { count: u64 },
    /// Encountered an instruction the interpreter doesn't implement.
    Unimplemented { pc: u64, raw: u32 },
    /// Memory access fault.
    MemoryFault { pc: u64, addr: u64, is_write: bool },
    /// Branch to unmapped address (not FFI).
    BadBranch { pc: u64, target: u64 },
}

/// Sentinel return address pushed as LR when calling main.
/// When RET pops this, we know main() returned.
pub(super) const SENTINEL_LR: u64 = 0xDEAD_CAFE_FEED_F00D;

/// Maximum instructions before forced exit (infinite loop guard).
const MAX_INSTRUCTIONS: u64 = 100_000_000;

/// Maximum number of instructions per basic block.
///
/// Blocks end at branches, but this cap prevents degenerate straight-line
/// sequences (e.g. large unrolled copies) from occupying excessive cache
/// memory.  64 is well above the average ARM64 basic block length (~6–12).
const MAX_BLOCK_SIZE: usize = 64;

// ── Block cache ───────────────────────────────────────────────────────────────

/// Cache of pre-decoded basic blocks, keyed by block-start PC.
///
/// `Instruction` is `Copy`, so the `Arc<[Instruction]>` can be cloned in
/// O(1) without heap allocation on cache hits.
pub struct BlockCache {
    blocks: HashMap<u64, Arc<[Instruction]>>,
    /// Number of blocks served from the cache (diagnostic).
    pub hits: u64,
    /// Number of blocks built and inserted (diagnostic).
    pub misses: u64,
}

impl BlockCache {
    fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// Return the cache hit rate in [0, 1].
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Decode instructions from `mem` starting at `start_pc` until a block
    /// terminator (branch, SVC, unknown) or `MAX_BLOCK_SIZE` instructions.
    ///
    /// This is a **static** method so it can be called while `BlockCache` is
    /// mutably borrowed, by taking only a `&Memory` argument.
    fn build(start_pc: u64, mem: &Memory) -> Vec<Instruction> {
        let mut insns = Vec::with_capacity(16);
        let mut pc = start_pc;
        loop {
            let Some(raw) = mem.read_u32_mapped(pc) else {
                // Unreadable instruction address — insert a sentinel that
                // will cause a MemoryFault when the execute loop hits it.
                insns.push(Instruction::Unknown { raw: 0 });
                break;
            };
            let insn = decode::decode(raw, pc);
            let is_term = is_block_terminator(insn);
            insns.push(insn);
            if is_term || insns.len() >= MAX_BLOCK_SIZE {
                break;
            }
            pc += 4;
        }
        insns
    }
}

/// Return true if `insn` may change the PC to something other than PC+4.
///
/// These instructions end a basic block.  `Mrs`/`Msr` are *not* terminators —
/// they advance sequentially and can appear mid-block.
#[inline]
fn is_block_terminator(insn: Instruction) -> bool {
    use Instruction::*;
    matches!(
        insn,
        B { .. }
            | Bl { .. }
            | Br { .. }
            | Blr { .. }
            | Ret { .. }
            | Bcond { .. }
            | Cbz { .. }
            | Cbnz { .. }
            | Tbz { .. }
            | Tbnz { .. }
            | Svc { .. }
            | Brk { .. }
            | Unknown { .. }
    )
}

// ── Interpreter ───────────────────────────────────────────────────────────────

/// Internal step result.
pub(super) enum StepResult {
    Continue,
    Exit(ExitReason),
}

/// The ARM64 interpreter.
pub struct Interpreter {
    pub cpu: CpuState,
    pub mem: Memory,
    pub ffi: FfiBridge,
    /// Number of instructions executed in the last `run()` call.
    pub instruction_count: u64,
    /// Pre-decoded basic block cache (eliminates re-decode on hot paths).
    pub block_cache: BlockCache,
    /// JIT compilation cache for native execution of hot blocks.
    pub jit_cache: JitCache,
}

impl Interpreter {
    pub fn new(cpu: CpuState, mem: Memory, ffi: FfiBridge) -> Self {
        // JIT: 4MB code buffer, compile after 50 visits to a block
        let jit = JitCache::new(4 * 1024 * 1024, 50);
        Self {
            cpu,
            mem,
            ffi,
            instruction_count: 0,
            block_cache: BlockCache::new(),
            jit_cache: jit,
        }
    }

    /// Run from the current PC until exit.
    ///
    /// Uses a tiered execution strategy:
    /// 1. Block cache for pre-decoded instruction sequences (cold paths)
    /// 2. JIT compilation for hot basic blocks (aarch64 hosts only)
    pub fn run(&mut self) -> ExitReason {
        let mut count: u64 = 0;
        loop {
            if count >= MAX_INSTRUCTIONS {
                self.instruction_count = count;
                return ExitReason::InstructionLimit { count };
            }

            let pc = self.cpu.pc;

            // ── Tier 2: JIT execution for hot blocks ──────────────────────
            if let Some(jit_fn) = self.jit_cache.get(pc) {
                let insn_count =
                    unsafe { jit_fn(&mut self.cpu as *mut _, &mut self.mem as *mut _) };
                count += insn_count;
                self.jit_cache.exec_count += 1;
                // JIT block updates cpu.pc internally on success.
                // If insn_count == 0, something went wrong; fall through to interpreter.
                if insn_count > 0 {
                    continue;
                }
            }

            // ── Tier 1: Block cache lookup / build ────────────────────────
            if !self.block_cache.blocks.contains_key(&pc) {
                let insns = BlockCache::build(pc, &self.mem);
                self.block_cache.blocks.insert(pc, insns.into());
                self.block_cache.misses += 1;
            } else {
                self.block_cache.hits += 1;
            }

            // Arc::clone is O(1) and releases the HashMap borrow immediately.
            let block = Arc::clone(self.block_cache.blocks.get(&pc).unwrap());

            // ── Tier 2: Attempt JIT compilation for hot blocks ────────────
            if self.jit_cache.is_hot(pc) {
                if let Some(jit_fn) = self.jit_cache.compile(pc, &block) {
                    log::debug!("JIT compiled block at {pc:#x} ({} insns)", block.len());
                    let insn_count =
                        unsafe { jit_fn(&mut self.cpu as *mut _, &mut self.mem as *mut _) };
                    count += insn_count;
                    self.jit_cache.exec_count += 1;
                    if insn_count > 0 {
                        continue;
                    }
                }
            }

            // ── Tier 0: Interpret the block ───────────────────────────────
            for (i, insn) in block.iter().enumerate() {
                let insn_pc = pc + (i as u64) * 4;
                count += 1;
                match self.execute(*insn, insn_pc, 0) {
                    StepResult::Continue => {}
                    StepResult::Exit(reason) => {
                        self.instruction_count = count;
                        log::info!(
                            "Interpreter exited after {count} instructions \
                             (cache: {}/{} hit rate {:.1}%, JIT: {} blocks, {} execs): {reason:?}",
                            self.block_cache.hits,
                            self.block_cache.hits + self.block_cache.misses,
                            self.block_cache.hit_rate() * 100.0,
                            self.jit_cache.compiled_count,
                            self.jit_cache.exec_count,
                        );
                        return reason;
                    }
                }
            }
        }
    }

    /// Execute a single instruction — dispatches to category-specific handlers.
    fn execute(&mut self, insn: Instruction, pc: u64, _raw: u32) -> StepResult {
        use Instruction::*;
        match insn {
            // ── Data processing (integer) ─────────────────────────
            Nop
            | Svc { .. }
            | AddImm { .. }
            | AddsImm { .. }
            | SubImm { .. }
            | SubsImm { .. }
            | Movz { .. }
            | Movk { .. }
            | Movn { .. }
            | Adrp { .. }
            | Adr { .. }
            | LogicalImm { .. }
            | AddReg { .. }
            | AddsReg { .. }
            | SubReg { .. }
            | SubsReg { .. }
            | LogicalReg { .. }
            | Madd { .. }
            | Msub { .. }
            | Div { .. }
            | Csel { .. }
            | Csinc { .. }
            | Csinv { .. }
            | Csneg { .. }
            | Ccmp { .. }
            | Ccmn { .. }
            | ShiftVar { .. }
            | Bitfield { .. }
            | Rev { .. }
            | Rev16 { .. }
            | Rev32 { .. }
            | Clz { .. }
            | Cls { .. }
            | Rbit { .. }
            | Extr { .. }
            | Crc32 { .. } => self.exec_dp(insn, pc),

            // ── Load / Store ──────────────────────────────────────
            LdrImm { .. }
            | StrImm { .. }
            | Ldur { .. }
            | Stur { .. }
            | LdrPre { .. }
            | StrPre { .. }
            | LdrPost { .. }
            | StrPost { .. }
            | LdrbPre { .. }
            | StrbPre { .. }
            | LdrhPre { .. }
            | StrhPre { .. }
            | LdrsbPre { .. }
            | LdrshPre { .. }
            | LdrswPre { .. }
            | LdrbPost { .. }
            | StrbPost { .. }
            | LdrhPost { .. }
            | StrhPost { .. }
            | LdrsbPost { .. }
            | LdrshPost { .. }
            | LdrswPost { .. }
            | Ldp { .. }
            | Stp { .. }
            | LdpPre { .. }
            | StpPre { .. }
            | LdpPost { .. }
            | StpPost { .. }
            | Ldrb { .. }
            | Strb { .. }
            | Ldrh { .. }
            | Strh { .. }
            | Ldrsw { .. }
            | Ldrsb { .. }
            | Ldrsh { .. }
            | LdrReg { .. }
            | StrReg { .. }
            | LdrbReg { .. }
            | StrbReg { .. }
            | LdrhReg { .. }
            | StrhReg { .. }
            | LdrsbReg { .. }
            | LdrshReg { .. }
            | LdrswReg { .. }
            | LdrLiteral { .. }
            | Prfm => self.exec_ls(insn, pc),

            // ── Branch / Call / Return ────────────────────────────
            B { .. }
            | Bl { .. }
            | Br { .. }
            | Blr { .. }
            | Ret { .. }
            | Bcond { .. }
            | Cbz { .. }
            | Cbnz { .. }
            | Tbz { .. }
            | Tbnz { .. }
            | Mrs { .. }
            | Msr { .. }
            | Brk { .. }
            | PacIA { .. }
            | PacIB { .. }
            | PacDA { .. }
            | PacDB { .. }
            | AutIA { .. }
            | AutIB { .. }
            | AutDA { .. }
            | AutDB { .. }
            | Xpaci { .. }
            | Xpacd { .. } => self.exec_branch(insn, pc),

            // ── Load / Store (extended: exclusive, acquire-release, atomics) ─
            Ldar { .. }
            | Ldapr { .. }
            | Stlr { .. }
            | Ldarb { .. }
            | Stlrb { .. }
            | Ldarh { .. }
            | Stlrh { .. }
            | LoadExclusive { .. }
            | LoadExclusiveByte { .. }
            | LoadExclusiveHalf { .. }
            | StoreExclusive { .. }
            | StoreExclusiveByte { .. }
            | StoreExclusiveHalf { .. }
            | Clrex
            | LseLoad { .. }
            | Swp { .. }
            | Cas { .. } => self.exec_ls_extended(insn, pc),

            // ── FP / SIMD data processing ─────────────────────────
            FpLdrImm { .. }
            | FpStrImm { .. }
            | FpLdur { .. }
            | FpStur { .. }
            | FpLdrPre { .. }
            | FpStrPre { .. }
            | FpLdrPost { .. }
            | FpStrPost { .. }
            | FpLdp { .. }
            | FpStp { .. }
            | FpLdpPre { .. }
            | FpStpPre { .. }
            | FpLdpPost { .. }
            | FpStpPost { .. }
            | FpLdrReg { .. }
            | FpStrReg { .. }
            | FpLdrLiteral { .. }
            | FMovReg { .. }
            | FMovToFp { .. }
            | FMovToGpr { .. }
            | FMovImm { .. }
            | Scvtf { .. }
            | Ucvtf { .. }
            | Fcvtzs { .. }
            | Fcvtzu { .. }
            | Fcvt { .. }
            | Fcmp { .. }
            | FcmpZero { .. }
            | Fadd { .. }
            | Fsub { .. }
            | Fmul { .. }
            | Fdiv { .. }
            | Fabd { .. }
            | Fneg { .. }
            | Fabs { .. }
            | Fsqrt { .. }
            | Fcsel { .. }
            | Fmadd { .. }
            | Fmsub { .. }
            | Fnmadd { .. }
            | Fnmsub { .. }
            | Frint { .. } => self.exec_fp(insn, pc),

            // ── SIMD vector ───────────────────────────────────────
            SimdDupGeneral { .. }
            | SimdDupElement { .. }
            | SimdInsGeneral { .. }
            | SimdInsElement { .. }
            | SimdUmov { .. }
            | SimdSmov { .. }
            | SimdMovi { .. }
            | SimdAdd { .. }
            | SimdSub { .. }
            | SimdAnd { .. }
            | SimdOrr { .. }
            | SimdEor { .. }
            | SimdCmp { .. }
            | SimdCnt { .. }
            | SimdAddv { .. } => self.exec_simd(insn, pc),

            // ── Unknown ───────────────────────────────────────────
            Unknown { raw: r } => {
                log::warn!("Unknown instruction {r:#010x} at {pc:#x}");
                StepResult::Exit(ExitReason::Unimplemented { pc, raw: r })
            }
        }
    }
}

/// Shift a 12-bit immediate by 0 or 12.
pub(super) fn shift_imm12(imm: u32, shift: u8) -> u32 {
    if shift == 1 {
        imm << 12
    } else {
        imm
    }
}

/// Set up an interpreter to run a Mach-O binary's main().
///
/// - `entry_pc`: address of the entry point (text_base + entry_offset)
/// - `argc`, `argv_ptr`, `envp_ptr`: C-style arguments (host pointers
///   cast to u64, placed in x0/x1/x2)
pub fn setup_for_main(
    mem: Memory,
    ffi: FfiBridge,
    entry_pc: u64,
    argc: i32,
    argv_ptr: u64,
    envp_ptr: u64,
) -> Interpreter {
    let mut cpu = CpuState::new();
    cpu.pc = entry_pc;
    cpu.sp = mem.initial_sp();
    cpu.x[0] = argc as u64;
    cpu.x[1] = argv_ptr;
    cpu.x[2] = envp_ptr;
    cpu.x[30] = SENTINEL_LR; // LR = sentinel so RET from main triggers exit
    Interpreter::new(cpu, mem, ffi)
}

#[cfg(test)]
mod block_cache_tests {

    use crate::test_helpers::{enc_bcond, enc_movz, enc_subs_imm, test_interp};

    /// Verify the block cache accumulates hits on a tight countdown loop.
    ///
    /// Layout (100-iteration loop):
    ///   +0  MOVZ X0, #100        ; load counter
    ///   +4  SUBS X0, X0, #1     ; decrement, set flags  ← loop-back target
    ///   +8  B.NE -4              ; branch to +4 while X0 != 0
    ///   +12 RET                  ; (appended by test_interp)
    ///
    /// Expected blocks and first-visit (miss) / re-visit (hit) counts:
    ///   Block@+0  = [MOVZ, SUBS, B.NE]  — visited once: 1 miss
    ///   Block@+4  = [SUBS, B.NE]        — first visit: 1 miss; next 98: 98 hits
    ///   Block@+12 = [RET]               — visited once: 1 miss
    ///
    /// Total: misses=3, hits=98, hit_rate ≈ 97 %.
    #[test]
    fn block_cache_hit_rate_for_hot_loop() {
        // NE condition encoding is 1 (cond=0b0001).
        let insns = [
            enc_movz(true, 0, 100, 0),          // MOVZ X0, #100
            enc_subs_imm(true, 0, 0, 1, false), // SUBS X0, X0, #1
            enc_bcond(1, -4),                   // B.NE -4  (back to SUBS)
        ];
        let mut t = test_interp(&insns);
        t.run_ok(); // asserts clean Returned exit

        let cache = &t.interp.block_cache;
        assert!(
            cache.hits > 0,
            "expected block cache hits for a looping program, got hits={}",
            cache.hits
        );
        let hit_rate = cache.hit_rate();
        assert!(
            hit_rate > 0.9,
            "expected >90 % hit rate for a tight loop, got {hit_rate:.2}"
        );
    }

    /// Verify that a straight-line program (no branches) produces exactly one
    /// block and zero cache hits.
    #[test]
    fn block_cache_straight_line_has_no_hits() {
        let insns = [enc_movz(true, 1, 42, 0)]; // MOVZ X1, #42  then RET
        let mut t = test_interp(&insns);
        t.run_ok();

        let cache = &t.interp.block_cache;
        assert_eq!(
            cache.hits, 0,
            "straight-line code should not produce cache hits"
        );
        assert!(cache.misses >= 1, "expected at least one cache miss");
    }
}
