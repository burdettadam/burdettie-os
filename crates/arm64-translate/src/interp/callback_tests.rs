use super::*;
use crate::ffi::FfiResult;
use crate::test_helpers::*;

#[test]
fn callback_does_not_restore_exclusive_reservation() {
    let mut test = test_interp(&[
        0xaa1e_03f3, // MOV X19, LR
        0xc85f_7c41, // LDXR X1, [X2]
        0xd63f_0200, // BLR X16: callback through deferred FFI.
        0xc803_7c44, // STXR W3, X4, [X2]
        0xaa03_03e0, // MOV X0, X3
        0xaa13_03fe, // MOV LR, X19
        0xd65f_03c0, // RET main
        0xd503_3f5f, // callback: CLREX
        0xd65f_03c0, // RET callback
    ]);
    let callback = test.code_base + 7 * 4;
    test.interp.cpu.x[2] = test.data_base;
    test.interp.cpu.x[4] = 9;
    test.interp.cpu.x[16] = 0x1000;
    assert!(test.interp.mem.write_u64(test.data_base, 7));
    test.interp.ffi.register(
        0x1000,
        "finalize callback",
        Box::new(move |_| {
            FfiResult::with_interpreter(move |interpreter| {
                match interpreter.call_guest_callback(callback, 0) {
                    ExitReason::Returned { .. } => FfiResult::Return(0),
                    reason => FfiResult::Stop(reason),
                }
            })
        }),
    );
    assert!(matches!(
        test.interp.run(),
        ExitReason::Returned { code: 1 }
    ));
    assert_eq!(test.interp.mem.read_u64(test.data_base), Some(7));
    assert_eq!(test.interp.cpu.exclusive_monitor, None);
    assert_eq!(test.interp.instruction_count, 9);
}

#[test]
fn shared_instruction_budget_stops_inside_a_hot_block() {
    let mut test = test_interp(&[
        enc_movz(true, 0, 1000, 0),
        enc_subs_imm(true, 0, 0, 1, false),
        enc_bcond(1, -4),
    ]);
    test.interp.set_instruction_limit(128);
    assert!(matches!(
        test.interp.run(),
        ExitReason::InstructionLimit { count: 128 }
    ));
    assert_eq!(test.interp.instruction_count, 128);
    assert_eq!(test.interp.cpu.x[0], 936);
    assert_eq!(test.interp.cpu.pc, test.code_base + 8);
    assert!(test.interp.block_cache.hits > 0);
}
