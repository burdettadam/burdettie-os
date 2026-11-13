//! ARM64 CPU state: registers, SP, PC, NZCV condition flags, and SIMD/FP.

/// ARM64 CPU state for the interpreter.
///
/// Models the AArch64 general-purpose register file (x0–x30), stack pointer,
/// program counter, NZCV condition flags, and SIMD/FP registers (v0–v31).
///
/// `repr(C)` guarantees field ordering so JIT-emitted code can access fields
/// at compile-time-known offsets.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct CpuState {
    /// General-purpose registers x0–x30.
    /// x29 = frame pointer (FP), x30 = link register (LR).
    pub x: [u64; 31],
    /// Stack pointer (SP). Separate from x[] per A64 encoding rules.
    pub sp: u64,
    /// Program counter.
    pub pc: u64,
    /// Condition flags.
    pub nzcv: Nzcv,
    /// SIMD/FP registers v0–v31 (128-bit each).
    /// Access patterns: Q0=v0[127:0], D0=v0[63:0], S0=v0[31:0], H0=v0[15:0], B0=v0[7:0].
    pub v: [u128; 32],
    /// Thread-local storage pointer (TPIDR_EL0). Used by pthreads TLS.
    pub tpidr_el0: u64,
    /// Floating-point control register (FPCR).
    pub fpcr: u64,
    /// Floating-point status register (FPSR).
    pub fpsr: u64,
    /// Exclusive monitor for LDXR/STXR emulation.
    /// Set by Load-Exclusive; checked and cleared by Store-Exclusive.
    /// `Some((addr, size))` if an exclusive monitor is active.
    pub exclusive_monitor: Option<(u64, u8)>,
    /// Pointer Authentication keys (ARMv8.3 arm64e).
    /// APIAKey (instruction A key, used by PACIA/AUTIA).
    pub pac_ia_key: u128,
    /// APIBKey (instruction B key).
    pub pac_ib_key: u128,
    /// APDAKey (data A key).
    pub pac_da_key: u128,
    /// APDBKey (data B key).
    pub pac_db_key: u128,
    /// Generic authentication key (APGAKey).
    pub pac_ga_key: u128,
}

/// NZCV condition flags.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Nzcv {
    /// Negative flag — set when result is negative (bit 31/63 set).
    pub n: bool,
    /// Zero flag — set when result is zero.
    pub z: bool,
    /// Carry flag — set on unsigned overflow.
    pub c: bool,
    /// Overflow flag — set on signed overflow.
    pub v: bool,
}

/// Condition codes for conditional branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Eq, // Z==1
    Ne, // Z==0
    Cs, // C==1 (HS)
    Cc, // C==0 (LO)
    Mi, // N==1
    Pl, // N==0
    Vs, // V==1
    Vc, // V==0
    Hi, // C==1 && Z==0
    Ls, // C==0 || Z==1
    Ge, // N==V
    Lt, // N!=V
    Gt, // Z==0 && N==V
    Le, // Z==1 || N!=V
    Al, // always
    Nv, // always (architecturally)
}

impl Condition {
    /// Decode a 4-bit condition code.
    pub fn from_bits(bits: u32) -> Self {
        match bits & 0xF {
            0x0 => Condition::Eq,
            0x1 => Condition::Ne,
            0x2 => Condition::Cs,
            0x3 => Condition::Cc,
            0x4 => Condition::Mi,
            0x5 => Condition::Pl,
            0x6 => Condition::Vs,
            0x7 => Condition::Vc,
            0x8 => Condition::Hi,
            0x9 => Condition::Ls,
            0xA => Condition::Ge,
            0xB => Condition::Lt,
            0xC => Condition::Gt,
            0xD => Condition::Le,
            0xE => Condition::Al,
            _ => Condition::Nv,
        }
    }

    /// Evaluate this condition against the current flags.
    pub fn eval(self, flags: Nzcv) -> bool {
        match self {
            Condition::Eq => flags.z,
            Condition::Ne => !flags.z,
            Condition::Cs => flags.c,
            Condition::Cc => !flags.c,
            Condition::Mi => flags.n,
            Condition::Pl => !flags.n,
            Condition::Vs => flags.v,
            Condition::Vc => !flags.v,
            Condition::Hi => flags.c && !flags.z,
            Condition::Ls => !flags.c || flags.z,
            Condition::Ge => flags.n == flags.v,
            Condition::Lt => flags.n != flags.v,
            Condition::Gt => !flags.z && (flags.n == flags.v),
            Condition::Le => flags.z || (flags.n != flags.v),
            Condition::Al | Condition::Nv => true,
        }
    }

    /// Encode this condition as a 4-bit value for native ARM64 instructions.
    pub fn to_bits(self) -> u32 {
        match self {
            Condition::Eq => 0x0,
            Condition::Ne => 0x1,
            Condition::Cs => 0x2,
            Condition::Cc => 0x3,
            Condition::Mi => 0x4,
            Condition::Pl => 0x5,
            Condition::Vs => 0x6,
            Condition::Vc => 0x7,
            Condition::Hi => 0x8,
            Condition::Ls => 0x9,
            Condition::Ge => 0xA,
            Condition::Lt => 0xB,
            Condition::Gt => 0xC,
            Condition::Le => 0xD,
            Condition::Al => 0xE,
            Condition::Nv => 0xF,
        }
    }
}

impl CpuState {
    /// Create a new CPU state with all registers zeroed.
    pub fn new() -> Self {
        // Initialize PAC keys with pseudo-random values seeded from a constant.
        // In a real implementation these would come from the kernel.
        let seed: u128 = 0xB0AD_E771_E050_BAC0_DEAD_BEEF_CAFE_1234;
        Self {
            x: [0u64; 31],
            sp: 0,
            pc: 0,
            nzcv: Nzcv::default(),
            v: [0u128; 32],
            tpidr_el0: 0,
            fpcr: 0,
            fpsr: 0,
            exclusive_monitor: None,
            pac_ia_key: seed,
            pac_ib_key: seed.wrapping_add(1),
            pac_da_key: seed.wrapping_add(2),
            pac_db_key: seed.wrapping_add(3),
            pac_ga_key: seed.wrapping_add(4),
        }
    }

    /// Read a general-purpose register. Reading x31 returns SP.
    ///
    /// Use [`read_reg_zr`] instead when the encoding uses r31 as the
    /// zero register (XZR/WZR): CSEL/CSINC/CSINV/CSNEG, CCMP/CCMN, etc.
    pub fn read_reg(&self, reg: u8) -> u64 {
        if reg == 31 {
            self.sp
        } else {
            self.x[reg as usize]
        }
    }

    /// Read a general-purpose register. Reading x31 returns **zero** (XZR).
    ///
    /// The ARM64 ISA uses r31 as the zero register in conditional-select,
    /// logical, and some other instruction classes.
    pub fn read_reg_zr(&self, reg: u8) -> u64 {
        if reg == 31 {
            0
        } else {
            self.x[reg as usize]
        }
    }

    /// Write a general-purpose register. Writing to x31 writes SP.
    pub fn write_reg(&mut self, reg: u8, val: u64) {
        if reg == 31 {
            self.sp = val;
        } else {
            self.x[reg as usize] = val;
        }
    }

    /// Write a general-purpose register. Writing to x31 is a no-op (discard).
    pub fn write_reg_zr(&mut self, reg: u8, val: u64) {
        if reg < 31 {
            self.x[reg as usize] = val;
        }
        // reg==31 → WZR/XZR, writes are discarded
    }

    /// Read a W-register (zero-extended lower 32 bits).
    pub fn read_w(&self, reg: u8) -> u32 {
        self.read_reg(reg) as u32
    }

    /// Write a W-register (zero-extends to 64 bits).
    pub fn write_w(&mut self, reg: u8, val: u32) {
        self.write_reg(reg, val as u64);
    }

    /// Update NZCV flags for a 64-bit result.
    pub fn update_nzcv64(&mut self, result: u64, carry: bool, overflow: bool) {
        self.nzcv.n = (result as i64) < 0;
        self.nzcv.z = result == 0;
        self.nzcv.c = carry;
        self.nzcv.v = overflow;
    }

    /// Update NZCV flags for a 32-bit result.
    pub fn update_nzcv32(&mut self, result: u32, carry: bool, overflow: bool) {
        self.nzcv.n = (result as i32) < 0;
        self.nzcv.z = result == 0;
        self.nzcv.c = carry;
        self.nzcv.v = overflow;
    }

    // ── SIMD/FP register access ────────────────────────────────────

    /// Read Qn (full 128-bit vector register).
    pub fn read_q(&self, reg: u8) -> u128 {
        self.v[reg as usize]
    }

    /// Write Qn (full 128-bit vector register).
    pub fn write_q(&mut self, reg: u8, val: u128) {
        self.v[reg as usize] = val;
    }

    /// Read Dn (lower 64 bits of Vn, as f64 bit pattern).
    pub fn read_d(&self, reg: u8) -> u64 {
        self.v[reg as usize] as u64
    }

    /// Write Dn (lower 64 bits; upper bits zeroed per AArch64 convention).
    pub fn write_d(&mut self, reg: u8, val: u64) {
        self.v[reg as usize] = val as u128;
    }

    /// Read Sn (lower 32 bits of Vn, as f32 bit pattern).
    pub fn read_s(&self, reg: u8) -> u32 {
        self.v[reg as usize] as u32
    }

    /// Write Sn (lower 32 bits; upper bits zeroed per AArch64 convention).
    pub fn write_s(&mut self, reg: u8, val: u32) {
        self.v[reg as usize] = val as u128;
    }

    /// Read Hn (lower 16 bits of Vn).
    pub fn read_h(&self, reg: u8) -> u16 {
        self.v[reg as usize] as u16
    }

    /// Write Hn (lower 16 bits; upper bits zeroed).
    pub fn write_h(&mut self, reg: u8, val: u16) {
        self.v[reg as usize] = val as u128;
    }

    /// Write a SIMD vector register. If `q` is true, writes all 128 bits.
    /// If `q` is false, writes the lower 64 bits and zeroes the upper 64.
    pub fn write_v(&mut self, reg: u8, val: u128, q: bool) {
        if q {
            self.v[reg as usize] = val;
        } else {
            self.v[reg as usize] = val & 0xFFFF_FFFF_FFFF_FFFF;
        }
    }
}

impl Default for CpuState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CpuState basics ────────────────────────────────────────────

    #[test]
    fn test_new_all_zeroed() {
        let cpu = CpuState::new();
        for i in 0..31 {
            assert_eq!(cpu.x[i], 0, "x[{i}] not zero");
        }
        assert_eq!(cpu.sp, 0);
        assert_eq!(cpu.pc, 0);
        assert!(!cpu.nzcv.n);
        assert!(!cpu.nzcv.z);
        assert!(!cpu.nzcv.c);
        assert!(!cpu.nzcv.v);
        for i in 0..32 {
            assert_eq!(cpu.v[i], 0, "v[{i}] not zero");
        }
    }

    #[test]
    fn test_read_reg_31_returns_sp() {
        let mut cpu = CpuState::new();
        cpu.sp = 0x1000;
        assert_eq!(cpu.read_reg(31), 0x1000);
    }

    #[test]
    fn test_read_reg_0_returns_x0() {
        let mut cpu = CpuState::new();
        cpu.x[0] = 42;
        assert_eq!(cpu.read_reg(0), 42);
    }

    #[test]
    fn test_read_reg_zr_31_returns_zero() {
        let mut cpu = CpuState::new();
        cpu.sp = 0xFFFF;
        assert_eq!(cpu.read_reg_zr(31), 0);
    }

    #[test]
    fn test_write_reg_31_sets_sp() {
        let mut cpu = CpuState::new();
        cpu.write_reg(31, 0xABCD);
        assert_eq!(cpu.sp, 0xABCD);
    }

    #[test]
    fn test_write_reg_zr_31_is_noop() {
        let mut cpu = CpuState::new();
        cpu.sp = 0x1000;
        cpu.write_reg_zr(31, 0xDEAD);
        assert_eq!(cpu.sp, 0x1000); // unchanged
    }

    #[test]
    fn test_write_read_reg_roundtrip() {
        let mut cpu = CpuState::new();
        cpu.write_reg(0, 0x123456789ABCDEF0);
        assert_eq!(cpu.read_reg(0), 0x123456789ABCDEF0);
    }

    #[test]
    fn test_read_w_lower_32() {
        let mut cpu = CpuState::new();
        cpu.x[5] = 0xFFFFFFFF_12345678;
        assert_eq!(cpu.read_w(5), 0x12345678);
    }

    #[test]
    fn test_write_w_zero_extends() {
        let mut cpu = CpuState::new();
        cpu.x[3] = 0xFFFFFFFF_FFFFFFFF;
        cpu.write_w(3, 0x80000001);
        assert_eq!(cpu.x[3], 0x00000000_80000001);
    }

    // ── NZCV ───────────────────────────────────────────────────────

    #[test]
    fn test_update_nzcv64_zero() {
        let mut cpu = CpuState::new();
        cpu.update_nzcv64(0, false, false);
        assert!(cpu.nzcv.z);
        assert!(!cpu.nzcv.n);
    }

    #[test]
    fn test_update_nzcv64_negative() {
        let mut cpu = CpuState::new();
        cpu.update_nzcv64(u64::MAX, false, false); // -1 in two's complement
        assert!(cpu.nzcv.n);
        assert!(!cpu.nzcv.z);
    }

    #[test]
    fn test_update_nzcv64_carry() {
        let mut cpu = CpuState::new();
        cpu.update_nzcv64(1, true, false);
        assert!(cpu.nzcv.c);
        assert!(!cpu.nzcv.v);
    }

    #[test]
    fn test_update_nzcv64_overflow() {
        let mut cpu = CpuState::new();
        cpu.update_nzcv64(1, false, true);
        assert!(cpu.nzcv.v);
        assert!(!cpu.nzcv.c);
    }

    #[test]
    fn test_update_nzcv32_zero() {
        let mut cpu = CpuState::new();
        cpu.update_nzcv32(0, false, false);
        assert!(cpu.nzcv.z);
        assert!(!cpu.nzcv.n);
    }

    #[test]
    fn test_update_nzcv32_negative() {
        let mut cpu = CpuState::new();
        cpu.update_nzcv32(0x80000000, false, false);
        assert!(cpu.nzcv.n);
        assert!(!cpu.nzcv.z);
    }

    #[test]
    fn test_update_nzcv32_carry_and_overflow() {
        let mut cpu = CpuState::new();
        cpu.update_nzcv32(0, true, true);
        assert!(cpu.nzcv.z);
        assert!(cpu.nzcv.c);
        assert!(cpu.nzcv.v);
    }

    // ── SIMD/FP registers ──────────────────────────────────────────

    #[test]
    fn test_write_read_d_roundtrip() {
        let mut cpu = CpuState::new();
        cpu.write_d(0, 0xDEADBEEFCAFEBABE);
        assert_eq!(cpu.read_d(0), 0xDEADBEEFCAFEBABE);
    }

    #[test]
    fn test_write_read_s_roundtrip() {
        let mut cpu = CpuState::new();
        cpu.write_s(0, 0x3F800000); // 1.0f32
        assert_eq!(cpu.read_s(0), 0x3F800000);
    }

    #[test]
    fn test_write_read_q_roundtrip() {
        let mut cpu = CpuState::new();
        let val: u128 = 0xDEADBEEFCAFEBABE_0123456789ABCDEF;
        cpu.write_q(0, val);
        assert_eq!(cpu.read_q(0), val);
    }

    #[test]
    fn test_write_read_h_roundtrip() {
        let mut cpu = CpuState::new();
        cpu.write_h(0, 0xBEEF);
        assert_eq!(cpu.read_h(0), 0xBEEF);
    }

    #[test]
    fn test_write_d_zeroes_upper_bits() {
        let mut cpu = CpuState::new();
        cpu.write_q(0, u128::MAX);
        cpu.write_d(0, 0x1234);
        assert_eq!(cpu.read_q(0), 0x1234);
    }

    #[test]
    fn test_write_s_zeroes_upper_bits() {
        let mut cpu = CpuState::new();
        cpu.write_q(0, u128::MAX);
        cpu.write_s(0, 0x42);
        assert_eq!(cpu.read_q(0), 0x42);
    }

    // ── Condition ──────────────────────────────────────────────────

    #[test]
    fn test_condition_from_bits_roundtrip() {
        let expected = [
            Condition::Eq,
            Condition::Ne,
            Condition::Cs,
            Condition::Cc,
            Condition::Mi,
            Condition::Pl,
            Condition::Vs,
            Condition::Vc,
            Condition::Hi,
            Condition::Ls,
            Condition::Ge,
            Condition::Lt,
            Condition::Gt,
            Condition::Le,
            Condition::Al,
            Condition::Nv,
        ];
        for (i, &cond) in expected.iter().enumerate() {
            assert_eq!(Condition::from_bits(i as u32), cond, "mismatch at bits={i}");
        }
    }

    #[test]
    fn test_condition_eval_eq_z_set() {
        let flags = Nzcv {
            n: false,
            z: true,
            c: false,
            v: false,
        };
        assert!(Condition::Eq.eval(flags));
    }

    #[test]
    fn test_condition_eval_eq_z_clear() {
        let flags = Nzcv {
            n: false,
            z: false,
            c: false,
            v: false,
        };
        assert!(!Condition::Eq.eval(flags));
    }

    #[test]
    fn test_condition_eval_ge_n_eq_v() {
        // N==V (both false) → GE true
        let flags = Nzcv {
            n: false,
            z: false,
            c: false,
            v: false,
        };
        assert!(Condition::Ge.eval(flags));
        // N==V (both true) → GE true
        let flags = Nzcv {
            n: true,
            z: false,
            c: false,
            v: true,
        };
        assert!(Condition::Ge.eval(flags));
        // N!=V → GE false
        let flags = Nzcv {
            n: true,
            z: false,
            c: false,
            v: false,
        };
        assert!(!Condition::Ge.eval(flags));
    }

    #[test]
    fn test_condition_eval_gt() {
        // !Z && N==V → true
        let flags = Nzcv {
            n: false,
            z: false,
            c: false,
            v: false,
        };
        assert!(Condition::Gt.eval(flags));
        // Z==1 → false even if N==V
        let flags = Nzcv {
            n: false,
            z: true,
            c: false,
            v: false,
        };
        assert!(!Condition::Gt.eval(flags));
        // N!=V → false
        let flags = Nzcv {
            n: true,
            z: false,
            c: false,
            v: false,
        };
        assert!(!Condition::Gt.eval(flags));
    }

    #[test]
    fn test_condition_eval_al_always_true() {
        let flags = Nzcv {
            n: false,
            z: false,
            c: false,
            v: false,
        };
        assert!(Condition::Al.eval(flags));
        let flags = Nzcv {
            n: true,
            z: true,
            c: true,
            v: true,
        };
        assert!(Condition::Al.eval(flags));
    }
}
