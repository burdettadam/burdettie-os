//! Process identity information.

/// Query the current process identity.
pub trait ProcessInfo: Send + Sync {
    /// Process ID.
    fn pid(&self) -> i32;

    /// Whether the calling thread is the main thread.
    fn is_main_thread(&self) -> bool;
}
