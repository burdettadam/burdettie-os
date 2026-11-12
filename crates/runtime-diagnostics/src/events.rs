//! Diagnostic event types for runtime failure analysis.

/// A structured diagnostic event emitted during runtime operation.
///
/// Each variant captures the context needed to diagnose a specific
/// compatibility failure or runtime path decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagEvent {
    // ── Selector dispatch failures ─────────────────────────────────
    /// A selector was not found on any class in the hierarchy.
    SelectorMissing {
        selector: String,
        class_name: String,
        receiver: u64,
    },

    /// A super-send selector was not found.
    SuperSelectorMissing {
        selector: String,
        receiver: u64,
        super_class: u64,
    },

    /// The forwarding chain was entered (no direct IMP found).
    ForwardingEntered {
        selector: String,
        receiver: u64,
        isa: u64,
    },

    // ── Class lookup failures ──────────────────────────────────────
    /// `objc_getClass` could not find a class by name.
    ClassMissing { name: String },

    /// `class_createInstance` given an unknown class address.
    ClassAddrUnknown { addr: u64 },

    /// `objc_msgSendSuper2` was given an unknown class address.
    Super2UnknownClass { current_class: u64 },

    /// A category targets a class that is not registered.
    CategoryOrphan { class_addr: u64 },

    // ── Symbol resolution failures ─────────────────────────────────
    /// A symbol was blocked by the safe-list policy.
    SymbolBlocked { name: String },

    /// A symbol was not found by the host resolver (dlsym returned null).
    SymbolMissing { name: String },

    /// A chained-fixup bind could not be resolved (non-weak).
    BindFailed { ordinal: usize, symbol: String },

    // ── Host fallback paths ────────────────────────────────────────
    /// Dispatch hit a tagged/unaligned pointer — forwarded to host semantics.
    HostFallback {
        reason: String,
        selector: String,
        receiver: u64,
    },

    /// An ISA address is not known to our runtime (unknown object).
    UnknownIsa {
        isa: u64,
        selector: String,
        receiver: u64,
    },

    // ── Method dispatch tracing (verbose / opt-in) ─────────────────
    /// A method was successfully dispatched.
    MethodDispatch {
        selector: String,
        isa: u64,
        imp: u64,
    },

    // ── Resource-request graph ─────────────────────────────────────
    /// A resource (dylib, image, symbol) was requested.
    ResourceRequest { kind: String, name: String },

    /// A resource request was successfully resolved.
    ResourceResolved {
        kind: String,
        name: String,
        addr: u64,
    },

    /// A resource request failed.
    ResourceFailed {
        kind: String,
        name: String,
        reason: String,
    },
}

impl std::fmt::Display for DiagEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagEvent::SelectorMissing {
                selector,
                class_name,
                receiver,
            } => {
                write!(
                    f,
                    "selector missing: -[{class_name} {selector}] (receiver {receiver:#x})"
                )
            }
            DiagEvent::SuperSelectorMissing {
                selector,
                receiver,
                super_class,
            } => {
                write!(f, "super selector missing: [{selector}] on {receiver:#x} (super={super_class:#x})")
            }
            DiagEvent::ForwardingEntered {
                selector,
                receiver,
                isa,
            } => {
                write!(
                    f,
                    "forwarding: [{selector}] on {receiver:#x} (isa={isa:#x})"
                )
            }
            DiagEvent::ClassMissing { name } => {
                write!(f, "class missing: '{name}'")
            }
            DiagEvent::ClassAddrUnknown { addr } => {
                write!(f, "unknown class address: {addr:#x}")
            }
            DiagEvent::Super2UnknownClass { current_class } => {
                write!(f, "msgSendSuper2: unknown class at {current_class:#x}")
            }
            DiagEvent::CategoryOrphan { class_addr } => {
                write!(f, "category targets unknown class at {class_addr:#x}")
            }
            DiagEvent::SymbolBlocked { name } => {
                write!(f, "symbol blocked by policy: {name}")
            }
            DiagEvent::SymbolMissing { name } => {
                write!(f, "symbol not found: {name}")
            }
            DiagEvent::BindFailed { ordinal, symbol } => {
                write!(f, "bind failed: ordinal {ordinal} '{symbol}'")
            }
            DiagEvent::HostFallback {
                reason,
                selector,
                receiver,
            } => {
                write!(f, "host fallback ({reason}): [{selector}] on {receiver:#x}")
            }
            DiagEvent::UnknownIsa {
                isa,
                selector,
                receiver,
            } => {
                write!(f, "unknown ISA {isa:#x}: [{selector}] on {receiver:#x}")
            }
            DiagEvent::MethodDispatch { selector, isa, imp } => {
                write!(f, "dispatch: [{selector}] isa={isa:#x} → imp={imp:#x}")
            }
            DiagEvent::ResourceRequest { kind, name } => {
                write!(f, "resource request: {kind} '{name}'")
            }
            DiagEvent::ResourceResolved { kind, name, addr } => {
                write!(f, "resource resolved: {kind} '{name}' at {addr:#x}")
            }
            DiagEvent::ResourceFailed { kind, name, reason } => {
                write!(f, "resource failed: {kind} '{name}': {reason}")
            }
        }
    }
}
