//! JIT compiler for ARM64 basic blocks.
//!
//! On aarch64 hosts, hot basic blocks are compiled to native machine code.
//! On other architectures, the JIT is a no-op and the interpreter is used.
//!
//! # Architecture
//!
//! - `CodeBuffer` — mmap'd page(s) that hold emitted native code
//! - `JitCache` — maps block-start PC → compiled native function pointer
//! - `compile_block()` — translates `&[Instruction]` to native aarch64

mod emit;

use std::collections::HashMap;

use crate::cpu::CpuState;
use crate::decode::Instruction;
use crate::mem::Memory;

/// Signature of a JIT-compiled block.
///
/// Takes mutable references to the CPU state and memory, returns the
/// number of instructions executed in the block. If the block performs
/// a branch, it updates `cpu.pc` accordingly. Returns 0 on error to
/// signal fallback to the interpreter.
///
/// The `Memory` pointer is passed for blocks that contain load/store,
/// but most integer-only blocks won't touch it.
pub type JitBlockFn = unsafe fn(cpu: *mut CpuState, mem: *mut Memory) -> u64;

// ── Code buffer (mmap'd executable memory) ────────────────────────────────────

/// A page-aligned buffer of executable memory.
pub struct CodeBuffer {
    ptr: *mut u8,
    len: usize,
    offset: usize,
    /// Whether allocation was attempted.
    attempted: bool,
}

// SAFETY: The CodeBuffer is only used on one thread (same as the interpreter).
unsafe impl Send for CodeBuffer {}

impl CodeBuffer {
    /// Create an empty code buffer. Allocation is deferred until first use.
    pub fn new_lazy() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            len: 0,
            offset: 0,
            attempted: false,
        }
    }

    /// Ensure the code buffer has been allocated. Returns true if usable.
    pub fn ensure_allocated(&mut self, min_size: usize) -> bool {
        if self.attempted {
            return !self.ptr.is_null();
        }
        self.attempted = true;

        #[cfg(not(all(target_arch = "aarch64", target_family = "unix")))]
        {
            let _ = min_size;
            log::debug!("JIT code buffer allocation unsupported on this host — JIT disabled");
            false
        }

        #[cfg(all(target_arch = "aarch64", target_family = "unix"))]
        {
            let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
            let len = (min_size + page_size - 1) & !(page_size - 1);

            // macOS requires MAP_JIT for pages that will be both writable and executable.
            #[cfg(target_os = "macos")]
            let flags = libc::MAP_PRIVATE | libc::MAP_ANON | libc::MAP_JIT;
            #[cfg(not(target_os = "macos"))]
            let flags = libc::MAP_PRIVATE | libc::MAP_ANON;

            let ptr = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    len,
                    libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
                    flags,
                    -1,
                    0,
                )
            };
            if ptr == libc::MAP_FAILED || ptr.is_null() {
                log::debug!("JIT code buffer mmap failed — JIT disabled");
                return false;
            }
            self.ptr = ptr as *mut u8;
            self.len = len;
            true
        }
    }

    /// Remaining capacity in bytes.
    pub fn remaining(&self) -> usize {
        if self.ptr.is_null() {
            return 0;
        }
        self.len - self.offset
    }

    /// Emit a single 32-bit ARM64 instruction.
    #[inline]
    pub fn emit32(&mut self, insn: u32) {
        assert!(self.offset + 4 <= self.len, "JIT code buffer overflow");
        unsafe {
            let dst = self.ptr.add(self.offset) as *mut u32;
            dst.write_unaligned(insn);
        }
        self.offset += 4;
    }

    /// Emit raw bytes.
    pub fn emit_bytes(&mut self, bytes: &[u8]) {
        assert!(
            self.offset + bytes.len() <= self.len,
            "JIT code buffer overflow"
        );
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.ptr.add(self.offset), bytes.len());
        }
        self.offset += bytes.len();
    }

    /// Get a function pointer to the start of a region at the given offset.
    pub fn fn_ptr_at(&self, offset: usize) -> JitBlockFn {
        assert!(offset < self.len);
        unsafe { std::mem::transmute(self.ptr.add(offset)) }
    }

    /// Current write offset (start of next emitted code).
    pub fn current_offset(&self) -> usize {
        self.offset
    }

    /// Flush instruction cache for recently emitted code.
    /// Required on aarch64 because icache and dcache are not coherent.
    #[cfg(target_arch = "aarch64")]
    pub fn flush_icache(&self, start: usize, end: usize) {
        // Use sys_icache_invalidate on macOS
        #[cfg(target_os = "macos")]
        unsafe {
            extern "C" {
                fn sys_icache_invalidate(start: *mut libc::c_void, size: usize);
            }
            sys_icache_invalidate(self.ptr.add(start) as *mut _, end - start);
        }
        #[cfg(not(target_os = "macos"))]
        {
            // On Linux, use inline assembly for cache maintenance
            unsafe {
                let start_ptr = self.ptr.add(start) as usize;
                let end_ptr = self.ptr.add(end) as usize;
                // Data cache clean + instruction cache invalidate
                let mut addr = start_ptr;
                while addr < end_ptr {
                    std::arch::asm!(
                        "dc cvau, {addr}",
                        addr = in(reg) addr,
                    );
                    addr += 64; // cache line size
                }
                std::arch::asm!("dsb ish");
                addr = start_ptr;
                while addr < end_ptr {
                    std::arch::asm!(
                        "ic ivau, {addr}",
                        addr = in(reg) addr,
                    );
                    addr += 64;
                }
                std::arch::asm!("dsb ish", "isb");
            }
        }
    }

    #[cfg(not(target_arch = "aarch64"))]
    pub fn flush_icache(&self, _start: usize, _end: usize) {
        // No-op on non-aarch64
    }
}

impl Drop for CodeBuffer {
    fn drop(&mut self) {
        #[cfg(all(target_arch = "aarch64", target_family = "unix"))]
        if !self.ptr.is_null() && self.len > 0 {
            unsafe {
                libc::munmap(self.ptr as *mut _, self.len);
            }
        }
    }
}

// ── JIT Cache ─────────────────────────────────────────────────────────────────

/// Cache of JIT-compiled basic blocks.
pub struct JitCache {
    /// Compiled block function pointers, keyed by block-start PC.
    blocks: HashMap<u64, JitBlockFn>,
    /// The executable code buffer.
    code: CodeBuffer,
    /// Requested buffer size for lazy allocation.
    buffer_size: usize,
    /// Number of blocks compiled.
    pub compiled_count: u64,
    /// Number of JIT block executions (hits).
    pub exec_count: u64,
    /// Minimum execution count before a block is compiled.
    pub hot_threshold: u64,
    /// Per-block execution counters for hotness tracking.
    counters: HashMap<u64, u64>,
}

impl JitCache {
    /// Create a new JIT cache with the given code buffer size and hotness threshold.
    pub fn new(buffer_size: usize, hot_threshold: u64) -> Self {
        Self {
            blocks: HashMap::new(),
            code: CodeBuffer::new_lazy(),
            compiled_count: 0,
            exec_count: 0,
            hot_threshold,
            counters: HashMap::new(),
            buffer_size,
        }
    }

    /// Check if a block is hot enough to compile, incrementing its counter.
    pub fn is_hot(&mut self, pc: u64) -> bool {
        let count = self.counters.entry(pc).or_insert(0);
        *count += 1;
        *count >= self.hot_threshold
    }

    /// Look up a compiled block.
    pub fn get(&self, pc: u64) -> Option<JitBlockFn> {
        self.blocks.get(&pc).copied()
    }

    /// Compile a block and store the result.
    ///
    /// Returns `Some(fn)` on success, `None` if the block couldn't be compiled
    /// (e.g., contains unsupported instructions or code buffer is full).
    #[cfg(target_arch = "aarch64")]
    pub fn compile(&mut self, pc: u64, block: &[Instruction]) -> Option<JitBlockFn> {
        // Lazy-allocate the code buffer on first compile attempt.
        if !self.code.ensure_allocated(self.buffer_size) {
            return None;
        }

        if self.code.remaining() < block.len() * 64 {
            return None;
        }

        // macOS ARM64: switch JIT pages to writable before emitting code.
        #[cfg(target_os = "macos")]
        unsafe {
            pthread_jit_write_protect_np(false);
        }

        let start = self.code.current_offset();
        if let Some(()) = emit::emit_block(&mut self.code, block) {
            self.code.flush_icache(start, self.code.current_offset());
            // Switch back to executable before running JIT code.
            #[cfg(target_os = "macos")]
            unsafe {
                pthread_jit_write_protect_np(true);
            }
            let func = self.code.fn_ptr_at(start);
            self.blocks.insert(pc, func);
            self.compiled_count += 1;
            Some(func)
        } else {
            // Rewind on failure
            self.code.offset = start;
            // Switch back to executable even on failure.
            #[cfg(target_os = "macos")]
            unsafe {
                pthread_jit_write_protect_np(true);
            }
            None
        }
    }

    /// On non-aarch64, compilation always fails (fall back to interpreter).
    #[cfg(not(target_arch = "aarch64"))]
    pub fn compile(&mut self, _pc: u64, _block: &[Instruction]) -> Option<JitBlockFn> {
        None
    }
}

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
extern "C" {
    fn pthread_jit_write_protect_np(enabled: bool);
}
