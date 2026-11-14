use super::*;

struct Images {
    code: [u32; 4],
    data: [u64; 2],
}

impl Images {
    fn new() -> Self {
        Self {
            code: [0xd65f_03c0; 4], // Source-owned AArch64 RET instructions.
            data: [0; 2],
        }
    }

    fn code(&self) -> u64 {
        self.code.as_ptr() as u64
    }

    fn dso(&self) -> u64 {
        self.data.as_ptr() as u64
    }

    fn enter(&self) -> LaunchExitScope<'_> {
        // SAFETY: the borrowed Images owns the interpreter code/data bytes for
        // this entire scope. These tests inspect registrations without native
        // calls; their ARM64 instruction addresses remain aligned and valid.
        unsafe {
            LaunchExitScope::enter(
                self,
                ExitRoute::Arm64Interpreter,
                vec![
                    ExitImageRange {
                        image: 0,
                        start: self.code(),
                        end: self.code() + 16,
                        executable: true,
                    },
                    ExitImageRange {
                        image: 1,
                        start: self.dso(),
                        end: self.dso() + 16,
                        executable: false,
                    },
                ],
            )
        }
    }
}

#[test]
fn registration_rejects_foreign_code_dso_and_worker_thread() {
    let images = Images::new();
    let scope = images.enter();
    let registry = scope.registry();
    for address in [0, images.code() + 1, images.code() + 16, images.dso()] {
        assert_eq!(registry.register(address, ExitCallbackAbi::Plain, 0, 0), -1);
    }
    assert_eq!(
        registry.register(images.code(), ExitCallbackAbi::Cxa, 17, 1),
        -1
    );
    let worker_registry = Arc::clone(&registry);
    let address = images.code();
    assert_eq!(
        std::thread::spawn(move || {
            assert!(LaunchExitRegistry::current().is_none());
            worker_registry.register(address, ExitCallbackAbi::Plain, 0, 0)
        })
        .join()
        .unwrap(),
        -1
    );
    assert!(registry
        .take_next(ExitRoute::Arm64Interpreter, None)
        .is_none());
}

#[test]
fn dso_filter_preserves_argument_image_and_reverse_order() {
    let images = Images::new();
    let scope = images.enter();
    let registry = scope.registry();
    assert_eq!(
        registry.register(images.code(), ExitCallbackAbi::Plain, 0, 0),
        0
    );
    assert_eq!(
        registry.register(images.code() + 4, ExitCallbackAbi::Cxa, 41, images.dso()),
        0
    );
    assert_eq!(
        registry.register(
            images.code() + 8,
            ExitCallbackAbi::Cxa,
            42,
            images.dso() + 8
        ),
        0
    );
    assert_eq!(
        registry.register(images.code() + 12, ExitCallbackAbi::Cxa, 43, images.dso()),
        0
    );

    // An interpreted callback can never be selected for native invocation.
    registry.finalize_native(None);
    let last = registry
        .take_next(ExitRoute::Arm64Interpreter, Some(images.dso()))
        .unwrap();
    assert_eq!(
        (last.address, last.argument, last.dso),
        (images.code() + 12, 43, images.dso())
    );
    assert_eq!(
        (last.image, last.dso_image, last.abi),
        (0, Some(1), ExitCallbackAbi::Cxa)
    );
    assert_eq!(
        registry
            .take_next(ExitRoute::Arm64Interpreter, Some(images.dso()))
            .unwrap()
            .argument,
        41
    );
    assert!(registry
        .take_next(ExitRoute::Arm64Interpreter, Some(images.dso()))
        .is_none());
    assert_eq!(
        registry
            .take_next(ExitRoute::Arm64Interpreter, None)
            .unwrap()
            .argument,
        42
    );
    assert_eq!(
        registry
            .take_next(ExitRoute::Arm64Interpreter, None)
            .unwrap()
            .abi,
        ExitCallbackAbi::Plain
    );
    assert!(registry
        .take_next(ExitRoute::Arm64Interpreter, None)
        .is_none());
}

#[test]
fn nested_and_abandoned_launches_cannot_drain_one_another() {
    let outer_images = Images::new();
    let inner_images = Images::new();
    let outer_scope = outer_images.enter();
    let outer = outer_scope.registry();
    assert_eq!(
        outer.register(outer_images.code(), ExitCallbackAbi::Plain, 0, 0),
        0
    );
    let abandoned = {
        let inner_scope = inner_images.enter();
        let inner = inner_scope.registry();
        assert_eq!(
            inner.register(inner_images.code(), ExitCallbackAbi::Plain, 0, 0),
            0
        );
        assert_eq!(
            outer.register(outer_images.code(), ExitCallbackAbi::Plain, 0, 0),
            -1
        );
        assert!(outer.take_next(ExitRoute::Arm64Interpreter, None).is_none());
        inner
    };
    assert!(abandoned
        .take_next(ExitRoute::Arm64Interpreter, None)
        .is_none());
    assert_eq!(
        abandoned.register(inner_images.code(), ExitCallbackAbi::Plain, 0, 0),
        -1
    );
    assert_eq!(
        outer
            .take_next(ExitRoute::Arm64Interpreter, None)
            .unwrap()
            .address,
        outer_images.code()
    );
    drop(outer_scope);
    assert!(LaunchExitRegistry::current().is_none());
}

#[test]
fn immediate_exit_discards_without_registration_or_route_reuse() {
    let images = Images::new();
    let scope = images.enter();
    let registry = scope.registry();
    assert_eq!(
        registry.register(images.code(), ExitCallbackAbi::Plain, 0, 0),
        0
    );
    registry.request_exit(ExitKind::Immediate, 37);
    assert!(registry
        .take_next(ExitRoute::Arm64Interpreter, None)
        .is_none());
    assert_eq!(
        registry.register(images.code(), ExitCallbackAbi::Plain, 0, 0),
        -1
    );
    assert_eq!(registry.requested_exit(), Some((ExitKind::Immediate, 37)));
}
