//! Source-owned machine-code fixtures exercise the public runtime pipelines.
//! Every case runs in a child: accidental native execution of ARM64 callbacks
//! or premature unmapping must fail the test instead of ending the test suite.

#![cfg(unix)]

use macho_runtime::{Runtime, RuntimeConfig, ShimSet};

struct BufferedStream {
    stream: *mut libc::FILE,
    _buffer: Box<[u8; 4096]>,
}

impl BufferedStream {
    fn new() -> Self {
        // SAFETY: tmpfile creates an owned, automatically removed C stream.
        let stream = unsafe { libc::tmpfile() };
        assert!(!stream.is_null());
        let mut buffer = Box::new([0_u8; 4096]);
        // SAFETY: the stream has not been used and this heap buffer outlives
        // fclose in Drop; the test retains exclusive ownership of both.
        assert_eq!(
            unsafe {
                libc::setvbuf(
                    stream,
                    buffer.as_mut_ptr().cast(),
                    libc::_IOFBF,
                    buffer.len(),
                )
            },
            0
        );
        // SAFETY: a valid stream and a terminated fixed string.
        assert!(unsafe { libc::fputs(c"BUFFERED_GUEST_OUTPUT".as_ptr(), stream) } >= 0);
        Self {
            stream,
            _buffer: buffer,
        }
    }

    fn has_flushed(&self) -> bool {
        let mut status = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: fileno returns this live stream's descriptor, and fstat
        // initializes the complete stat object when it succeeds.
        assert_eq!(
            unsafe { libc::fstat(libc::fileno(self.stream), status.as_mut_ptr()) },
            0
        );
        unsafe { status.assume_init() }.st_size != 0
    }
}

impl Drop for BufferedStream {
    fn drop(&mut self) {
        // SAFETY: stream is owned and still uses the live _buffer field.
        unsafe { libc::fclose(self.stream) };
    }
}

struct Fixture(std::path::PathBuf);

impl Fixture {
    fn write(name: &str, bytes: &[u8]) -> Self {
        let path = std::env::temp_dir().join(format!(
            "burdettie-exit-{}-{name}.macho",
            std::process::id()
        ));
        std::fs::write(&path, bytes).unwrap();
        Self(path)
    }

    fn config(&self) -> RuntimeConfig {
        let mut config = RuntimeConfig::permissive(&self.0);
        config.shim_set = ShimSet {
            process: false,
            tls: false,
            dispatch: false,
            pthread: false,
            mach: false,
            lifecycle: true,
            ptrauth: false,
            objc: false,
            foundation: false,
            random: false,
            darwin_stubs: false,
            cg: false,
        };
        config
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn shim(name: &str) -> u64 {
    let mut table = libsystem_shims::ShimTable::new();
    libsystem_shims::LifecycleShims::install(&mut table);
    table.resolve(name).unwrap()
}

/// One __TEXT segment and LC_MAIN; no SDK, linker or external binary is used.
fn macho(code: &[u8], cpu: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [0xfeed_facf, cpu, 0, 2, 2, 96, 0, 0, 0x19, 72] {
        bytes.extend_from_slice(&u32::to_le_bytes(value));
    }
    bytes.extend_from_slice(b"__TEXT\0\0\0\0\0\0\0\0\0\0");
    for value in [0x1_0000_0000_u64, 0x4000, 0, 0x4000] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [7_u32, 5, 0, 0, 0x8000_0028, 24] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&0x100_u64.to_le_bytes());
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    assert_eq!(bytes.len(), 128);
    bytes.resize(0x100, 0);
    bytes.extend_from_slice(code);
    assert!(bytes.len() < 0x4000);
    bytes.resize(0x4000, 0);
    bytes
}

#[derive(Default)]
struct ArmCode {
    words: Vec<u32>,
    addresses: Vec<(usize, u32, &'static str)>,
    labels: std::collections::HashMap<&'static str, usize>,
}

impl ArmCode {
    fn mov64(&mut self, reg: u32, value: u64) {
        for half in 0..4_u32 {
            let op = if half == 0 { 0xd280_0000 } else { 0xf280_0000 };
            self.words
                .push(op | (half << 21) | (((value >> (16 * half)) as u32 & 0xffff) << 5) | reg);
        }
    }

    fn address(&mut self, reg: u32, label: &'static str) {
        self.addresses.push((self.words.len(), reg, label));
        self.words.push(0);
    }

    fn label(&mut self, label: &'static str) {
        self.labels.insert(label, self.words.len());
    }

    fn call(&mut self, name: &str) {
        self.mov64(16, shim(name));
        self.words.push(0xd63f_0200); // BLR X16 through the real FFI/native route.
    }

    fn register(&mut self, callback: &'static str, dso: &'static str, output: u64, stack: bool) {
        self.address(0, callback);
        if stack {
            self.words.push(0x9100_03e1); // ADD X1, SP, #0: destructor argument.
        } else {
            self.mov64(1, output);
        }
        self.address(2, dso);
        self.call("___cxa_atexit");
    }

    fn append_digit(&mut self, digit: u32, stack_output: Option<u64>) {
        self.words.extend([
            0xf940_0001,                 // LDR X1, [X0]
            0xd280_0142,                 // MOV X2, #10
            0x9b02_7c21,                 // MADD X1, X1, X2, XZR
            0x9100_0021 | (digit << 10), // ADD X1, X1, #digit
            0xf900_0001,                 // STR X1, [X0]
        ]);
        if let Some(output) = stack_output {
            self.mov64(3, output);
            self.words.push(0xf900_0061); // STR X1, [X3]: publish stack result.
        }
    }

    fn finish(mut self) -> Vec<u8> {
        for (position, reg, label) in self.addresses {
            let delta = ((self.labels[label] as i64 - position as i64) * 4) as u32;
            self.words[position] =
                0x1000_0000 | ((delta & 3) << 29) | (((delta >> 2) & 0x7ffff) << 5) | reg;
        }
        self.words.into_iter().flat_map(u32::to_le_bytes).collect()
    }
}

fn arm_fixture(mode: &str, output: u64) -> Vec<u8> {
    let mut code = ArmCode::default();
    code.words.push(0xa9bf_7bfd); // STP X29, X30, [SP, #-16]!
    let stack = mode == "same_stack";
    if stack {
        code.words.push(0xd100_83ff); // SUB SP, SP, #32
        code.mov64(0, 7);
        code.words.push(0xf900_03e0); // STR X0, [SP]
    }
    code.register("first", "dso_first", output, stack);
    code.register("second", "dso_second", output, stack);
    if mode == "plain" {
        code.address(0, "plain");
        code.call("_atexit");
    }

    if mode == "dso" {
        code.address(0, "dso_first");
        code.call("___cxa_finalize");
        code.address(0, "dso_first");
        code.call("___cxa_finalize"); // must not invoke the first callback twice.
    }
    code.mov64(0, 37);
    match mode {
        "normal" | "same_stack" | "callback_svc" => code.call("_exit"),
        "immediate" => code.call("__exit"),
        "abort" => code.call("_abort"),
        "abnormal" => code.words.push(0), // UDF #0, decoded as Unknown.
        "raw_svc" => code.words.push(0xd400_0001), // SVC is unsupported, not normal exit.
        _ => code.words.extend([0xa8c1_7bfd, 0xd65f_03c0]), // LDP/RET main.
    }

    code.label("first");
    code.append_digit(1, stack.then_some(output));
    code.words.push(0xd65f_03c0);
    code.label("second");
    if mode != "depth_limit" {
        code.append_digit(2, stack.then_some(output));
    }
    match mode {
        "recursive_finalize" => {
            code.words.extend([0xa9bf_57f4, 0xaa1e_03f4, 0xaa00_03f5]); // save X20/X21; X20=LR, X21=argument.
            code.mov64(0, 0);
            code.call("___cxa_finalize");
            code.words.push(0xaa15_03e0); // restore argument X0 from X21.
            code.append_digit(3, None);
            code.words.extend([0xaa14_03fe, 0xa8c1_57f4]); // restore LR and X20/X21.
        }
        "recursive_exit" | "callback_immediate" => {
            code.mov64(0, 52);
            code.call(if mode == "callback_immediate" {
                "__exit"
            } else {
                "_exit"
            });
            code.words.push(0); // continuation must never execute.
        }
        "reregister" | "depth_limit" => {
            code.words.push(0xa9bf_7bfd);
            code.register(
                if mode == "reregister" {
                    "third"
                } else {
                    "second"
                },
                "dso_second",
                output,
                false,
            );
            if mode == "depth_limit" {
                code.mov64(0, 0);
                code.call("___cxa_finalize");
            }
            code.words.push(0xa8c1_7bfd);
        }
        "callback_fault" => code.words.push(0),
        "callback_svc" => code.words.push(0xd400_0001),
        _ => {}
    }
    code.words.push(0xd65f_03c0);
    code.label("third");
    code.append_digit(3, None);
    code.words.push(0xd65f_03c0);
    code.label("plain");
    code.mov64(0, output);
    code.append_digit(4, None);
    code.words.push(0xd65f_03c0);
    code.label("dso_first");
    code.words.push(0);
    code.label("dso_second");
    code.words.push(0);
    macho(&code.finish(), 0x0100_000c)
}

#[cfg(target_arch = "x86_64")]
fn native_fixture(output: u64) -> Vec<u8> {
    let mut code = vec![0x55, 0x48, 0x89, 0xe5]; // PUSH/MOV RBP, preserving host ABI.
    let mut patches = Vec::new();
    for index in 0..2 {
        code.extend([0x48, 0x8d, 0x3d]); // LEA RDI, [RIP+callback]
        patches.push((code.len(), index));
        code.extend(0_i32.to_le_bytes());
        code.extend([0x48, 0xbe]); // MOVABS RSI, argument
        code.extend(output.to_le_bytes());
        code.extend([0x31, 0xd2]); // XOR EDX, EDX (null DSO)
        code.extend([0x48, 0xb8]); // MOVABS RAX, __cxa_atexit
        code.extend(shim("___cxa_atexit").to_le_bytes());
        code.extend([0xff, 0xd0]); // CALL RAX
    }
    code.extend([0xb8, 37, 0, 0, 0, 0x5d, 0xc3]); // return 37 with original RBP.
    let mut callbacks = Vec::new();
    for digit in [1, 2] {
        callbacks.push(code.len());
        code.extend([0x48, 0x8b, 0x07]); // MOV RAX, [RDI]
        code.extend([0x48, 0x6b, 0xc0, 10]); // IMUL RAX, RAX, 10
        code.extend([0x48, 0x83, 0xc0, digit]); // ADD RAX, digit
        code.extend([0x48, 0x89, 0x07, 0xc3]); // MOV [RDI], RAX; RET
    }
    for (position, index) in patches {
        let delta = (callbacks[index] as i64 - position as i64 - 4) as i32;
        code[position..position + 4].copy_from_slice(&delta.to_le_bytes());
    }
    macho(&code, 0x0100_0007)
}

#[test]
fn launch_owned_callbacks_follow_real_runtime_paths() {
    const CHILD: &str = "BURDETTIE_LAUNCH_CLEANUP_CASE";
    if let Ok(mode) = std::env::var(CHILD) {
        assert!(matches!(
            arm64_translate::decode::decode(0, 0),
            arm64_translate::Instruction::Unknown { raw: 0 }
        ));
        let buffered = BufferedStream::new();
        let mut observed = Box::new(0_u64);
        let address = (&raw mut *observed) as u64;
        let bytes = if mode == "native_return" {
            #[cfg(target_arch = "x86_64")]
            {
                native_fixture(address)
            }
            #[cfg(not(target_arch = "x86_64"))]
            {
                arm_fixture("return", address)
            }
        } else {
            arm_fixture(&mode, address)
        };
        let fixture = Fixture::write(&mode, &bytes);
        let config = fixture.config();
        let result = if mode == "native_return" {
            Runtime::load_and_run(&fixture.0, &config)
        } else {
            Runtime::load_and_run_interpreted(&fixture.0, &config)
        };
        assert_eq!(
            buffered.has_flushed(),
            !matches!(
                mode.as_str(),
                "immediate"
                    | "callback_immediate"
                    | "abort"
                    | "abnormal"
                    | "callback_fault"
                    | "depth_limit"
                    | "raw_svc"
                    | "callback_svc"
            ),
            "wrong stdio flush for {mode}"
        );
        if matches!(
            mode.as_str(),
            "abnormal" | "callback_fault" | "depth_limit" | "raw_svc" | "callback_svc"
        ) {
            assert!(
                result.is_err(),
                "callback/entry fault became success: {result:?}"
            );
            assert_eq!(
                *observed,
                if matches!(mode.as_str(), "callback_fault" | "callback_svc") {
                    2
                } else {
                    0
                }
            );
            // A new launch after failure must not inherit the abandoned queue.
            *observed = 0;
            let next = Fixture::write("after_error", &arm_fixture("return", address));
            assert_eq!(
                Runtime::load_and_run_interpreted(&next.0, &next.config())
                    .unwrap()
                    .exit_code,
                37
            );
            assert_eq!(*observed, 21);
        } else {
            let expected_status = match mode.as_str() {
                "abort" => 1,
                "recursive_exit" | "callback_immediate" => 52,
                _ => 37,
            };
            assert_eq!(result.unwrap().exit_code, expected_status);
            let expected = match mode.as_str() {
                "immediate" | "abort" => 0,
                "callback_immediate" => 2,
                "dso" => 12,
                "recursive_finalize" => 213,
                "reregister" => 231,
                "same_stack" => 721,
                "plain" => 421,
                _ => 21,
            };
            assert_eq!(
                *observed, expected,
                "wrong callback order/argument for {mode}"
            );
        }
        assert!(libsystem_shims::exit_registry::LaunchExitRegistry::current().is_none());
        println!("BURDETTIE_LAUNCH_CLEANUP_PASSED:{mode}");
        return;
    }

    let mut cases = vec![
        "return",
        "normal",
        "immediate",
        "abort",
        "abnormal",
        "dso",
        "recursive_finalize",
        "recursive_exit",
        "callback_fault",
        "same_stack",
        "reregister",
        "depth_limit",
        "callback_immediate",
        "plain",
        "raw_svc",
        "callback_svc",
    ];
    if cfg!(any(target_arch = "x86_64", target_arch = "aarch64")) {
        cases.push("native_return");
    }
    for mode in cases {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "launch_owned_callbacks_follow_real_runtime_paths",
                "--nocapture",
            ])
            .env(CHILD, mode)
            .env("BURDETTIE_DOMAIN_REWRITE", "false")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{mode} failed or terminated its host: {output:?}"
        );
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .contains(&format!("BURDETTIE_LAUNCH_CLEANUP_PASSED:{mode}")),
            "{mode}: {output:?}"
        );
    }
}
