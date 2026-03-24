//! Process identity information.

/// Query the current process identity.
pub trait ProcessInfo: Send + Sync {
    /// Process ID.
    fn pid(&self) -> i32;
}
