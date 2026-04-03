//! Binary patching for known crash sites in Apple binaries.
//!
//! Some Apple binaries contain proprietary functions that crash under our
//! loader's execution model due to subtle ABI or state assumptions.
//! This module scans mapped binaries for known problematic call sites and
//! replaces them with NOPs before the code is made executable.

use crate::MappedImage;
use macho_parse::ParsedMachO;

/// The ARM64 NOP instruction encoding.
#[cfg(target_arch = "aarch64")]
const NOP: u32 = 0xD503201F;

/// The x86-64 5-byte NOP (0F 1F 44 00 00) used to replace a 5-byte CALL.
#[cfg(target_arch = "x86_64")]
const NOP5: [u8; 5] = [0x0F, 0x1F, 0x44, 0x00, 0x00];

/// Known crash-causing symbols that should be patched out (call → NOP).
const PATCH_TARGETS: &[&str] = &[
    // Apple-proprietary function in swift-frontend's Clang driver that
    // crashes with strlen(5) when invoked from ClangImporter under our
    // loader.  The function adds workaround compiler flags but its
    // internal argument construction corrupts a const char* entry.
    "__ZN5clang6driver6Driver22AddWorkaroundArgumentsERN4llvm3opt12InputArgListE",
];

/// Functions whose entry should be replaced with `mov x0, #0; ret`
/// (return-null stub).  The callers must handle a null/empty return.
///
/// This is more aggressive than PATCH_TARGETS (which NOPs individual
/// call sites).  Use it when the ENTIRE function body is unsafe under
/// our loader.
const STUB_TARGETS: &[&str] = &[
    // Swift's ClangImporter constructs Clang CC1 arguments and invokes
    // the Clang driver internally.  The driver's argument arrays contain
    // corrupt const char* entries (small integers, tagged pointers,
    // unmapped addresses) under our loader.  Stubbing this Swift-specific
    // function to return null makes ClangImporter::create fail gracefully
    // without affecting standalone clang, which uses BuildCompilation
    // directly and works correctly.
    "__ZN5swift13ClangImporter20getClangCC1ArgumentsERNS_10ASTContextEN4llvm18IntrusiveRefCntPtrINS3_3vfs10FileSystemEEEb",
];

/// Patch out known crash-causing function calls in a mapped binary.
///
/// Must be called BEFORE `protect_executable()` while __TEXT is still
/// writable.  Returns the number of instructions patched.
pub fn patch_known_crashes(image: &MappedImage, parsed: &ParsedMachO) -> usize {
    let text_seg = match image.segment("__TEXT") {
        Some(seg) => seg,
        None => return 0,
    };

    let symbols = match &parsed.symbols {
        Some(table) => &table.symbols,
        None => return 0,
    };

    let text_vmaddr = text_seg.vmaddr;
    let text_size = text_seg.size;
    let mut total_patched = 0;

    // Phase 1: NOP individual call sites to PATCH_TARGETS.
    for &target_name in PATCH_TARGETS {
        let target_vmaddr = match symbols
            .iter()
            .find(|s| s.name == target_name)
            .map(|s| s.value)
        {
            Some(v) if v != 0 => v,
            _ => continue,
        };

        total_patched += patch_calls_to(image, text_seg, text_vmaddr, text_size, target_vmaddr);
    }

    // Phase 2: Replace function entries in STUB_TARGETS with return-null.
    for &target_name in STUB_TARGETS {
        let func_vmaddr = match symbols
            .iter()
            .find(|s| s.name == target_name)
            .map(|s| s.value)
        {
            Some(v) if v != 0 => v,
            _ => continue,
        };

        if stub_function_entry(text_seg, text_vmaddr, func_vmaddr) {
            log::info!(
                "stubbed {target_name} at vmaddr {func_vmaddr:#x} → return null"
            );
            total_patched += 1;
        }
    }

    if total_patched > 0 {
        log::info!("patched {total_patched} known crash site(s)");
    }

    total_patched
}

/// Scan __TEXT for `bl` instructions targeting `target_vmaddr` and NOP them.
#[cfg(target_arch = "aarch64")]
fn patch_calls_to(
    image: &MappedImage,
    text_seg: &crate::MappedSegment,
    text_vmaddr: u64,
    text_size: usize,
    target_vmaddr: u64,
) -> usize {
    let mut patched = 0;
    let slide = image.slide;

    // Scan every 4-byte aligned position in __TEXT.
    let count = text_size / 4;
    for i in 0..count {
        let offset = i * 4;
        let ptr = unsafe { text_seg.base.add(offset) as *const u32 };
        let inst = unsafe { ptr.read_unaligned() };

        // Check if this is a BL instruction (opcode bits [31:26] = 100101).
        if inst & 0xFC00_0000 != 0x9400_0000 {
            continue;
        }

        // Decode the signed 26-bit immediate (in units of 4 bytes).
        let imm26 = (inst & 0x03FF_FFFF) as i32;
        let imm26 = if imm26 & 0x0200_0000 != 0 {
            imm26 | !0x03FF_FFFFi32 // sign-extend
        } else {
            imm26
        };

        // Compute the branch target: pc + imm26 * 4.
        let pc = text_vmaddr + offset as u64;
        let branch_target = (pc as i64 + (imm26 as i64) * 4) as u64;

        if branch_target == target_vmaddr {
            // Overwrite with NOP.
            let write_ptr = unsafe { text_seg.base.add(offset) as *mut u32 };
            unsafe { write_ptr.write(NOP) };
            let _ = slide; // suppress unused warning
            log::debug!(
                "patched bl at {:#x} (vmaddr {pc:#x}) targeting {target_vmaddr:#x}",
                write_ptr as u64
            );
            patched += 1;
        }
    }

    patched
}

/// Scan __TEXT for `call` instructions targeting `target_vmaddr` and NOP them.
#[cfg(target_arch = "x86_64")]
fn patch_calls_to(
    image: &MappedImage,
    text_seg: &crate::MappedSegment,
    text_vmaddr: u64,
    text_size: usize,
    target_vmaddr: u64,
) -> usize {
    let mut patched = 0;
    let slide = image.slide;

    // x86_64 CALL rel32 is opcode 0xE8 followed by a 32-bit signed offset.
    if text_size < 5 {
        return 0;
    }
    for i in 0..text_size - 4 {
        let ptr = unsafe { text_seg.base.add(i) };
        let opcode = unsafe { *ptr };
        if opcode != 0xE8 {
            continue;
        }

        let rel32 =
            unsafe { (ptr.add(1) as *const i32).read_unaligned() };
        let pc = text_vmaddr + i as u64 + 5; // PC after the call instruction
        let branch_target = (pc as i64 + rel32 as i64) as u64;

        if branch_target == target_vmaddr {
            let write_ptr = unsafe { text_seg.base.add(i) as *mut [u8; 5] };
            unsafe { write_ptr.write(NOP5) };
            let _ = slide;
            log::debug!(
                "patched call at {:#x} (vmaddr {:#x}) targeting {target_vmaddr:#x}",
                write_ptr as u64,
                text_vmaddr + i as u64
            );
            patched += 1;
        }
    }

    patched
}

/// Replace a function's prologue with `mov x0, #0; ret` (return null).
#[cfg(target_arch = "aarch64")]
fn stub_function_entry(
    text_seg: &crate::MappedSegment,
    text_vmaddr: u64,
    func_vmaddr: u64,
) -> bool {
    if func_vmaddr < text_vmaddr {
        return false;
    }
    let offset = (func_vmaddr - text_vmaddr) as usize;
    if offset + 8 > text_seg.size {
        return false;
    }
    let ptr = unsafe { text_seg.base.add(offset) as *mut u32 };
    // mov x0, #0  (MOVZ x0, #0)
    unsafe { ptr.write(0xD280_0000) };
    // ret  (RET x30)
    unsafe { ptr.add(1).write(0xD65F_03C0) };
    true
}

/// Replace a function's prologue with `xor eax, eax; ret` (return null).
#[cfg(target_arch = "x86_64")]
fn stub_function_entry(
    text_seg: &crate::MappedSegment,
    text_vmaddr: u64,
    func_vmaddr: u64,
) -> bool {
    if func_vmaddr < text_vmaddr {
        return false;
    }
    let offset = (func_vmaddr - text_vmaddr) as usize;
    if offset + 3 > text_seg.size {
        return false;
    }
    let ptr = unsafe { text_seg.base.add(offset) };
    // xor eax, eax (2 bytes: 31 C0)
    unsafe { *ptr = 0x31 };
    unsafe { *ptr.add(1) = 0xC0 };
    // ret (1 byte: C3)
    unsafe { *ptr.add(2) = 0xC3 };
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn bl_decode_roundtrip() {
        // Manually encode a BL to offset +0x29624 (target from known crash).
        let imm26: u32 = 0x29624 / 4; // = 0xA589
        let inst = 0x9400_0000 | (imm26 & 0x03FF_FFFF);
        assert_eq!(inst, 0x9400_A589);

        // Decode it back.
        let decoded_imm = (inst & 0x03FF_FFFF) as i32;
        assert_eq!(decoded_imm, 0xA589);
        // target = pc + decoded_imm * 4
        let pc: u64 = 0x103e90708;
        let target = pc + decoded_imm as u64 * 4;
        assert_eq!(target, 0x103eb9d2c); // AddWorkaroundArguments
    }
}
