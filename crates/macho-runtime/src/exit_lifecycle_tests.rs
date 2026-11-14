use super::*;

fn call(words: &mut Vec<u32>, address: u64) {
    for half in 0..4 {
        let op = if half == 0 { 0xd280_0000 } else { 0xf280_0000 };
        words.push(op | (half << 21) | (((address >> (16 * half)) as u32 & 0xffff) << 5) | 16);
    }
    words.push(0xd63f_0200); // BLR X16
}

fn repeating_callback(nested: bool) -> (Vec<u32>, usize) {
    let mut table = libsystem_shims::ShimTable::new();
    LifecycleShims::install(&mut table);
    let mut words = Vec::new();
    if nested {
        words.extend([0xa9bf_7bfd, 0xd280_0000]); // save LR, X0=null DSO.
        call(&mut words, table.resolve("___cxa_finalize").unwrap());
        words.push(0xa8c1_7bfd);
    }
    words.push(0xd65f_03c0); // main RET, or return after explicit finalize.
    let callback_index = words.len();
    words.extend([
        0xa9bf_7bfd, // STP X29, X30, [SP, #-16]!
        0x10ff_ffe0, // ADR X0, #-4: register this callback again.
    ]);
    call(&mut words, table.resolve("_atexit").unwrap());
    if nested {
        words.push(0xd280_0000);
        call(&mut words, table.resolve("___cxa_finalize").unwrap());
    }
    words.extend([0xa8c1_7bfd, 0xd65f_03c0]);
    (words, callback_index)
}

#[test]
fn callback_drains_share_top_level_instruction_budget() {
    const CHILD: &str = "BURDETTIE_CALLBACK_BUDGET_CHILD";
    if let Ok(mode) = std::env::var(CHILD) {
        let (mut words, callback_index) = repeating_callback(mode == "nested");
        let pointer = words.as_mut_ptr().cast::<u8>();
        let start = pointer as u64;
        let length = words.len() * 4;
        let callback = start + (callback_index * 4) as u64;
        // SAFETY: the borrowed word vector holds source-owned ARM64 code and
        // stays alive through this interpreted scope and all callback subcalls.
        let scope = unsafe {
            LaunchExitScope::enter(
                &words,
                ExitRoute::Arm64Interpreter,
                vec![ExitImageRange {
                    image: 0,
                    start,
                    end: start + length as u64,
                    executable: true,
                }],
            )
        };
        let registry = scope.registry();
        assert_eq!(registry.register(callback, ExitCallbackAbi::Plain, 0, 0), 0);
        let mut bridge =
            crate::interp_bridge::build_interpreter_bridge(&[(start, start + length as u64)]);
        install_interpreter_lifecycle(&mut bridge, Some(Arc::clone(&registry)), true);
        // SAFETY: these instruction bytes remain mapped by the live vector;
        // the interpreter owns its stack and never executes them natively.
        let memory =
            unsafe { arm64_translate::Memory::from_segments(&[(start, pointer, length, false)]) };
        let mut cpu = arm64_translate::CpuState::new();
        cpu.pc = start;
        cpu.sp = memory.initial_sp();
        cpu.x[30] = 0;
        let mut interpreter = Interpreter::new(cpu, memory, bridge);
        interpreter.set_instruction_limit(32);
        assert!(matches!(
            interpreter.run(),
            ExitReason::InstructionLimit { count: 32 }
        ));
        assert_eq!(interpreter.instruction_count, 32);
        assert_eq!(
            registry.register(callback, ExitCallbackAbi::Plain, 0, 0),
            -1
        );
        println!("BURDETTIE_SHARED_CALLBACK_BUDGET_PASSED:{mode}");
        return;
    }

    // A per-callback reset would make the sequential case loop forever. Keep
    // that regression bounded by a watchdog for this particular child only.
    for mode in ["sequential", "nested"] {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "exit_lifecycle::tests::callback_drains_share_top_level_instruction_budget",
                "--nocapture",
            ])
            .env(CHILD, mode)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                let output = child.wait_with_output().unwrap();
                panic!("{mode}: callback drain escaped the instruction budget: {output:?}");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{mode}: {output:?}");
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .contains(&format!("BURDETTIE_SHARED_CALLBACK_BUDGET_PASSED:{mode}")),
            "{mode}: {output:?}"
        );
    }
}
