//! ARM64 CPU state: registers, SP, PC, NZCV condition flags, and SIMD/FP.

/// ARM64 CPU state for the interpreter.
///
/// Models the AArch64 general-purpose register file (x0–x30), stack pointer,
/// program counter, NZCV condition flags, and SIMD/FP registers (v0–v31).
#[derive(Debug, Clone)]
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
}

/// NZCV condition flags.
#[derive(Debug, Clone, Copy, Default)]
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
}

impl CpuState {
    /// Create a new CPU state with all registers zeroed.
    pub fn new() -> Self {
        Self {
            x: [0u64; 31],
            sp: 0,
            pc: 0,
            nzcv: Nzcv::default(),
            v: [0u128; 32],
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
}

impl Default for CpuState {
    fn default() -> Self {
        Self::new()
    }
}
