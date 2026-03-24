//! Host symbol resolution abstraction.

/// Resolve symbol names to host in-process addresses.
pub trait HostSymbolResolver: Send + Sync {
    /// Resolve a symbol name (without leading underscore) in the default handle.
    fn resolve(&self, name: &str) -> Option<*const ()>;

    /// Resolve a symbol name within a specific library handle.
    fn resolve_in(&self, handle: *mut (), name: &str) -> Option<*const ()>;

    /// Open a dynamic library by path, returning an opaque handle.
    fn open_library(&self, path: &str) -> Option<*mut ()>;

    /// Return the default lookup handle (e.g., RTLD_DEFAULT).
    fn default_handle(&self) -> *mut ();
}
