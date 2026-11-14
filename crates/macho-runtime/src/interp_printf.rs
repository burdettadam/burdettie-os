//! Explicit Darwin ARM64 LP64 printf adapter for x86_64 Unix hosts.
//!
//! Only literal bytes, %%, %d, %lu and %s are supported. No host va_list is
//! fabricated and no guest format is passed to a host variadic function.

use arm64_translate::{ExitReason, FfiBridge, FfiResult, Memory};
use std::ffi::CString;

const MAX_FORMAT_BYTES: usize = 4096;
const MAX_ARGUMENTS: usize = 64;
const MAX_OUTPUT_BYTES: usize = 64 * 1024;

#[derive(Debug, PartialEq, Eq)]
enum PrintfError {
    MisalignedStack,
    FormatUnreadable,
    UnsupportedConversion,
    TooManyArguments,
    StackUnreadable,
    AddressOverflow,
    StringUnreadable,
    OutputLimit,
}

fn append(output: &mut Vec<u8>, value: &[u8]) -> Result<(), PrintfError> {
    if value.len() > MAX_OUTPUT_BYTES - output.len() {
        return Err(PrintfError::OutputLimit);
    }
    output.extend_from_slice(value);
    Ok(())
}

fn format_printf(memory: &Memory, format: u64, stack: u64) -> Result<Vec<u8>, PrintfError> {
    if stack & 15 != 0 {
        return Err(PrintfError::MisalignedStack);
    }
    let format = memory
        .copy_cstring_bounded(format, MAX_FORMAT_BYTES)
        .map_err(|_| PrintfError::FormatUnreadable)?;
    let mut output = Vec::new();
    let mut cursor = 0;
    let mut arguments = 0;
    let mut slot = stack;
    while cursor < format.len() {
        let byte = format[cursor];
        cursor += 1;
        if byte != b'%' {
            append(&mut output, &[byte])?;
            continue;
        }
        let conversion = *format
            .get(cursor)
            .ok_or(PrintfError::UnsupportedConversion)?;
        cursor += 1;
        if conversion == b'%' {
            append(&mut output, b"%")?;
            continue;
        }
        match conversion {
            b'd' | b's' => {}
            b'l' if format.get(cursor) == Some(&b'u') => cursor += 1,
            _ => return Err(PrintfError::UnsupportedConversion),
        }
        if arguments == MAX_ARGUMENTS {
            return Err(PrintfError::TooManyArguments);
        }
        arguments += 1;
        let raw = memory
            .read_u64_mapped(slot)
            .ok_or(PrintfError::StackUnreadable)?;
        slot = slot.checked_add(8).ok_or(PrintfError::AddressOverflow)?;
        match conversion {
            // Darwin variadic int occupies an 8-byte slot, but only its low
            // 32 bits carry the promoted int value. Upper padding is not data.
            b'd' => append(&mut output, (raw as u32 as i32).to_string().as_bytes())?,
            b'l' => append(&mut output, raw.to_string().as_bytes())?,
            b's' => {
                let string = memory
                    .copy_cstring_bounded(raw, arm64_translate::mem::MAX_EXPORTED_CSTRING_BYTES)
                    .map_err(|_| PrintfError::StringUnreadable)?;
                append(&mut output, &string)?;
            }
            _ => unreachable!("conversion was checked before reading a stack slot"),
        }
    }
    Ok(output)
}

pub(super) fn install(bridge: &mut FfiBridge) {
    let address = super::host_symbol_addr(c"printf");
    if address == 0 {
        return;
    }
    bridge.register(
        address,
        "_printf (Darwin ARM64 stack arguments)",
        Box::new(move |args| {
            let format = args.x[0];
            FfiResult::with_interpreter(move |interpreter| {
                // Use the actual guest SP, not the generic bridge's rounded SP.
                match format_printf(&interpreter.mem, format, interpreter.cpu.sp) {
                    Ok(bytes) => {
                        // Neither the format nor a supported conversion can
                        // introduce interior NULs. The sink receives only owned
                        // storage and a fixed, typed host printf invocation.
                        let output = CString::new(bytes).expect("formatted output has no NUL");
                        // SAFETY: the constant host format consumes exactly one
                        // live owned C-string pointer. The host compiler handles
                        // its own variadic ABI; guest formats never reach libc.
                        let result = unsafe { libc::printf(c"%s".as_ptr(), output.as_ptr()) };
                        FfiResult::Return(result as i64 as u64)
                    }
                    Err(error) => FfiResult::Stop(ExitReason::FfiRejected {
                        pc: interpreter.cpu.pc,
                        target: address,
                        reason: format!("unsupported/unreadable Darwin ARM64 printf: {error:?}"),
                    }),
                }
            })
        }),
    );
}

#[cfg(test)]
#[path = "interp_printf_tests.rs"]
mod tests;
