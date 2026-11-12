//! Real `dispatch_io` channel implementation.
//!
//! Provides fd-based I/O channels that dispatch read/write operations to
//! completion handlers on specified queues, matching Apple's dispatch_io API.
//!
//! ## Architecture
//!
//! Each `dispatch_io_create` allocates a `Channel` with:
//! - The underlying file descriptor
//! - A target dispatch queue for handler delivery
//! - High/low water marks for buffer sizing
//! - A cleanup handler invoked on close
//!
//! Read/write operations use standard blocking I/O on a thread pool
//! worker, then deliver the result data + error to the handler block
//! on the target queue.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use super::{blocks_runtime::OwnedBlock, dispatch_queue};
use crate::table::ShimTable;

// ── Handle allocator ───────────────────────────────────────────

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(0x00D0_1000_0001);

fn alloc_handle() -> u64 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

// ── Channel types ──────────────────────────────────────────────

/// dispatch_io_type_t constants.
const DISPATCH_IO_STREAM: u64 = 0;
const DISPATCH_IO_RANDOM: u64 = 1;

/// dispatch_io_close flags.
const DISPATCH_IO_STOP: u64 = 0x1;

/// A tracked dispatch_io channel.
struct Channel {
    fd: i32,
    io_type: u64,
    queue: u64,
    cleanup_block: Mutex<Option<OwnedBlock>>,
    closed: AtomicBool,
    high_water: Mutex<usize>,
    low_water: Mutex<usize>,
}

// ── Global channel registry ────────────────────────────────────

fn registry() -> &'static Mutex<HashMap<u64, Arc<Channel>>> {
    static R: OnceLock<Mutex<HashMap<u64, Arc<Channel>>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

// ── Block invocation helpers ───────────────────────────────────

/// Invoke a block with signature:
///   void (^)(bool done, dispatch_data_t data, int error)
///
/// Block layout: isa(8) + flags(4) + reserved(4) + invoke(8)
unsafe fn invoke_io_handler(block: u64, done: bool, data: u64, error: i32) {
    if block == 0 {
        return;
    }
    let block_ptr = block as *const u8;
    let invoke_ptr = *(block_ptr.add(16) as *const usize);
    let invoke: extern "C" fn(*const u8, i32, u64, i32) = std::mem::transmute(invoke_ptr);
    invoke(block_ptr, done as i32, data, error);
}

/// Invoke a cleanup handler: void (^)(int error)
unsafe fn invoke_cleanup(block: u64, error: i32) {
    if block == 0 {
        return;
    }
    let block_ptr = block as *const u8;
    let invoke_ptr = *(block_ptr.add(16) as *const usize);
    let invoke: extern "C" fn(*const u8, i32) = std::mem::transmute(invoke_ptr);
    invoke(block_ptr, error);
}

fn queue_io_handler(queue: u64, handler: OwnedBlock, done: bool, data: u64, error: i32) {
    dispatch_queue::queue_async(
        queue,
        Box::new(move || {
            unsafe { invoke_io_handler(handler.addr(), done, data, error) };
        }),
    );
}

fn queue_cleanup(queue: u64, cleanup: OwnedBlock, error: i32) {
    dispatch_queue::queue_async(
        queue,
        Box::new(move || {
            unsafe { invoke_cleanup(cleanup.addr(), error) };
        }),
    );
}

// ── dispatch_data helpers ──────────────────────────────────────

/// Create a dispatch_data_t from a byte slice.
///
/// Uses the dispatch_data_source module's create function.
fn create_dispatch_data(bytes: &[u8]) -> u64 {
    if bytes.is_empty() {
        return 0; // empty data → dispatch_data_empty
    }
    // Allocate via the data_source registry
    super::dispatch_data_source::create_data_from_bytes(bytes)
}

// ── Shim implementations ──────────────────────────────────────

/// `dispatch_io_t dispatch_io_create(dispatch_io_type_t type, dispatch_fd_t fd,
///     dispatch_queue_t queue, void (^cleanup_handler)(int error))`
extern "C" fn shim_dispatch_io_create(io_type: u64, fd: i32, queue: u64, cleanup: u64) -> u64 {
    let chan = Arc::new(Channel {
        fd,
        io_type,
        queue,
        cleanup_block: Mutex::new(OwnedBlock::from_block(cleanup)),
        closed: AtomicBool::new(false),
        high_water: Mutex::new(usize::MAX), // default: no limit
        low_water: Mutex::new(0),
    });

    let handle = alloc_handle();
    registry().lock().unwrap().insert(handle, chan);
    log::debug!("dispatch_io_create(fd={fd}, type={io_type}) -> {handle:#x}");
    handle
}

/// `dispatch_io_t dispatch_io_create_with_path(dispatch_io_type_t type,
///     const char *path, int oflag, mode_t mode,
///     dispatch_queue_t queue, void (^cleanup_handler)(int error))`
extern "C" fn shim_dispatch_io_create_with_path(
    io_type: u64,
    path: u64,
    oflag: i32,
    mode: u32,
    queue: u64,
    cleanup: u64,
) -> u64 {
    if path == 0 {
        return 0;
    }

    let c_path = unsafe { std::ffi::CStr::from_ptr(path as *const std::ffi::c_char) };
    let fd = unsafe { libc::open(c_path.as_ptr(), oflag, mode as libc::c_uint) };
    if fd < 0 {
        let err = std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EIO);
        // Invoke cleanup with the error
        if let Some(cleanup) = OwnedBlock::from_block(cleanup) {
            unsafe { invoke_cleanup(cleanup.addr(), err) };
        }
        return 0;
    }

    shim_dispatch_io_create(io_type, fd, queue, cleanup)
}

/// `void dispatch_io_read(dispatch_io_t channel, off_t offset,
///     size_t length, dispatch_queue_t queue,
///     dispatch_io_handler_t io_handler)`
extern "C" fn shim_dispatch_io_read(
    channel: u64,
    offset: i64,
    length: usize,
    queue: u64,
    handler: u64,
) {
    let Some(handler) = OwnedBlock::from_block(handler) else {
        return;
    };

    let chan = {
        let reg = registry().lock().unwrap();
        match reg.get(&channel) {
            Some(c) => Arc::clone(c),
            None => {
                // Unknown channel — invoke handler with error
                unsafe { invoke_io_handler(handler.addr(), true, 0, libc::EBADF) };
                return;
            }
        }
    };

    if chan.closed.load(Ordering::Acquire) {
        unsafe { invoke_io_handler(handler.addr(), true, 0, libc::ECANCELED) };
        return;
    }

    let fd = chan.fd;
    let io_type = chan.io_type;
    let high_water = *chan.high_water.lock().unwrap();
    let target_queue = if queue != 0 { queue } else { chan.queue };

    // Perform the read on a background thread, deliver result on the queue
    std::thread::Builder::new()
        .name("dispatch-io-read".into())
        .spawn(move || {
            let read_size = if length == usize::MAX || length == 0 {
                high_water.min(64 * 1024) // default 64KB chunks
            } else {
                length
            };

            let mut buf = vec![0u8; read_size];

            let result = if io_type == DISPATCH_IO_RANDOM && offset >= 0 {
                unsafe { libc::pread(fd, buf.as_mut_ptr() as *mut libc::c_void, read_size, offset) }
            } else {
                unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, read_size) }
            };

            if result < 0 {
                let err = std::io::Error::last_os_error()
                    .raw_os_error()
                    .unwrap_or(libc::EIO);
                queue_io_handler(target_queue, handler, true, 0, err);
            } else {
                let n = result as usize;
                buf.truncate(n);
                let data = create_dispatch_data(&buf);
                let done = n == 0 || (length != usize::MAX && length != 0 && n >= length);
                queue_io_handler(target_queue, handler, done, data, 0);
            }
        })
        .ok();
}

/// `void dispatch_io_write(dispatch_io_t channel, off_t offset,
///     dispatch_data_t data, dispatch_queue_t queue,
///     dispatch_io_handler_t io_handler)`
extern "C" fn shim_dispatch_io_write(
    channel: u64,
    offset: i64,
    data: u64,
    queue: u64,
    handler: u64,
) {
    let Some(handler) = OwnedBlock::from_block(handler) else {
        return;
    };

    let chan = {
        let reg = registry().lock().unwrap();
        match reg.get(&channel) {
            Some(c) => Arc::clone(c),
            None => {
                unsafe { invoke_io_handler(handler.addr(), true, 0, libc::EBADF) };
                return;
            }
        }
    };

    if chan.closed.load(Ordering::Acquire) {
        unsafe { invoke_io_handler(handler.addr(), true, 0, libc::ECANCELED) };
        return;
    }

    let fd = chan.fd;
    let io_type = chan.io_type;
    let target_queue = if queue != 0 { queue } else { chan.queue };

    // Get the data bytes
    let bytes = super::dispatch_data_source::get_data_bytes(data);

    std::thread::Builder::new()
        .name("dispatch-io-write".into())
        .spawn(move || {
            let buf = bytes.unwrap_or_default();
            let result = if io_type == DISPATCH_IO_RANDOM && offset >= 0 {
                unsafe { libc::pwrite(fd, buf.as_ptr() as *const libc::c_void, buf.len(), offset) }
            } else {
                unsafe { libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len()) }
            };

            if result < 0 {
                let err = std::io::Error::last_os_error()
                    .raw_os_error()
                    .unwrap_or(libc::EIO);
                queue_io_handler(target_queue, handler, true, 0, err);
            } else {
                // Remaining data (if partial write, return unwritten portion)
                let written = result as usize;
                let remaining = if written < buf.len() {
                    create_dispatch_data(&buf[written..])
                } else {
                    0
                };
                let done = written >= buf.len();
                queue_io_handler(target_queue, handler, done, remaining, 0);
            }
        })
        .ok();
}

/// `void dispatch_io_close(dispatch_io_t channel, dispatch_io_close_flags_t flags)`
extern "C" fn shim_dispatch_io_close(channel: u64, flags: u64) {
    let chan = {
        let mut reg = registry().lock().unwrap();
        reg.remove(&channel)
    };

    if let Some(chan) = chan {
        chan.closed.store(true, Ordering::Release);

        if flags & DISPATCH_IO_STOP != 0 {
            // DISPATCH_IO_STOP: cancel pending operations
            log::debug!("dispatch_io_close({channel:#x}) with STOP flag");
        }

        // Invoke cleanup handler
        let cleanup = chan.cleanup_block.lock().unwrap().take();
        if let Some(cleanup) = cleanup {
            let queue = chan.queue;
            queue_cleanup(queue, cleanup, 0);
        }
    }
}

/// `void dispatch_io_set_high_water(dispatch_io_t channel, size_t high_water)`
extern "C" fn shim_dispatch_io_set_high_water(channel: u64, high_water: usize) {
    let reg = registry().lock().unwrap();
    if let Some(chan) = reg.get(&channel) {
        *chan.high_water.lock().unwrap() = high_water;
    }
}

/// `void dispatch_io_set_low_water(dispatch_io_t channel, size_t low_water)`
extern "C" fn shim_dispatch_io_set_low_water(channel: u64, low_water: usize) {
    let reg = registry().lock().unwrap();
    if let Some(chan) = reg.get(&channel) {
        *chan.low_water.lock().unwrap() = low_water;
    }
}

/// `void dispatch_io_set_interval(dispatch_io_t channel, uint64_t interval,
///     dispatch_io_interval_flags_t flags)`
extern "C" fn shim_dispatch_io_set_interval(_channel: u64, _interval: u64, _flags: u64) {
    // No-op — interval-based delivery not implemented.
}

/// `dispatch_fd_t dispatch_io_get_descriptor(dispatch_io_t channel)`
extern "C" fn shim_dispatch_io_get_descriptor(channel: u64) -> i32 {
    let reg = registry().lock().unwrap();
    reg.get(&channel).map(|c| c.fd).unwrap_or(-1)
}

/// `void dispatch_read(dispatch_fd_t fd, size_t length,
///     dispatch_queue_t queue,
///     void (^handler)(dispatch_data_t data, int error))`
///
/// Convenience wrapper that creates a channel, reads, and closes.
extern "C" fn shim_dispatch_read(fd: i32, length: usize, queue: u64, handler: u64) {
    let Some(handler) = OwnedBlock::from_block(handler) else {
        return;
    };

    std::thread::Builder::new()
        .name("dispatch-read".into())
        .spawn(move || {
            let read_size = if length == 0 { 64 * 1024 } else { length };
            let mut buf = vec![0u8; read_size];
            let result =
                unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, read_size) };

            if result < 0 {
                let err = std::io::Error::last_os_error()
                    .raw_os_error()
                    .unwrap_or(libc::EIO);
                dispatch_queue::queue_async(
                    queue,
                    Box::new(move || unsafe {
                        let block_ptr = handler.as_ptr();
                        let invoke_ptr = *(block_ptr.add(16) as *const usize);
                        let invoke: extern "C" fn(*const u8, u64, i32) =
                            std::mem::transmute(invoke_ptr);
                        invoke(block_ptr, 0, err);
                    }),
                );
            } else {
                let n = result as usize;
                buf.truncate(n);
                let data = create_dispatch_data(&buf);
                dispatch_queue::queue_async(
                    queue,
                    Box::new(move || unsafe {
                        let block_ptr = handler.as_ptr();
                        let invoke_ptr = *(block_ptr.add(16) as *const usize);
                        let invoke: extern "C" fn(*const u8, u64, i32) =
                            std::mem::transmute(invoke_ptr);
                        invoke(block_ptr, data, 0);
                    }),
                );
            }
        })
        .ok();
}

/// `void dispatch_write(dispatch_fd_t fd, dispatch_data_t data,
///     dispatch_queue_t queue,
///     void (^handler)(dispatch_data_t remaining, int error))`
extern "C" fn shim_dispatch_write(fd: i32, data: u64, queue: u64, handler: u64) {
    let Some(handler) = OwnedBlock::from_block(handler) else {
        return;
    };

    let bytes = super::dispatch_data_source::get_data_bytes(data);

    std::thread::Builder::new()
        .name("dispatch-write".into())
        .spawn(move || {
            let buf = bytes.unwrap_or_default();
            let result = unsafe { libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len()) };

            if result < 0 {
                let err = std::io::Error::last_os_error()
                    .raw_os_error()
                    .unwrap_or(libc::EIO);
                dispatch_queue::queue_async(
                    queue,
                    Box::new(move || unsafe {
                        let block_ptr = handler.as_ptr();
                        let invoke_ptr = *(block_ptr.add(16) as *const usize);
                        let invoke: extern "C" fn(*const u8, u64, i32) =
                            std::mem::transmute(invoke_ptr);
                        invoke(block_ptr, 0, err);
                    }),
                );
            } else {
                dispatch_queue::queue_async(
                    queue,
                    Box::new(move || unsafe {
                        let block_ptr = handler.as_ptr();
                        let invoke_ptr = *(block_ptr.add(16) as *const usize);
                        let invoke: extern "C" fn(*const u8, u64, i32) =
                            std::mem::transmute(invoke_ptr);
                        invoke(block_ptr, 0, 0);
                    }),
                );
            }
        })
        .ok();
}

// ── Installation ───────────────────────────────────────────────

pub struct DispatchIoStubs;

impl DispatchIoStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(
            table,
            (
                "_dispatch_io_create",
                shim_dispatch_io_create as *const () as u64,
                "dispatch_io_create (real fd-based channel)"
            ),
            (
                "_dispatch_io_create_with_path",
                shim_dispatch_io_create_with_path as *const () as u64,
                "dispatch_io_create_with_path (real)"
            ),
            (
                "_dispatch_io_read",
                shim_dispatch_io_read as *const () as u64,
                "dispatch_io_read (real async read)"
            ),
            (
                "_dispatch_io_write",
                shim_dispatch_io_write as *const () as u64,
                "dispatch_io_write (real async write)"
            ),
            (
                "_dispatch_io_close",
                shim_dispatch_io_close as *const () as u64,
                "dispatch_io_close (real cleanup)"
            ),
            (
                "_dispatch_io_set_high_water",
                shim_dispatch_io_set_high_water as *const () as u64,
                "dispatch_io_set_high_water (real)"
            ),
            (
                "_dispatch_io_set_low_water",
                shim_dispatch_io_set_low_water as *const () as u64,
                "dispatch_io_set_low_water (real)"
            ),
            (
                "_dispatch_io_set_interval",
                shim_dispatch_io_set_interval as *const () as u64,
                "dispatch_io_set_interval (stub)"
            ),
            (
                "_dispatch_io_get_descriptor",
                shim_dispatch_io_get_descriptor as *const () as u64,
                "dispatch_io_get_descriptor (real)"
            ),
            (
                "_dispatch_read",
                shim_dispatch_read as *const () as u64,
                "dispatch_read (convenience wrapper)"
            ),
            (
                "_dispatch_write",
                shim_dispatch_write as *const () as u64,
                "dispatch_write (convenience wrapper)"
            )
        );
        log::debug!("DispatchIoStubs: registered 11 dispatch_io shims");
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_create_and_close() {
        // Create a pipe for testing
        let mut fds = [0i32; 2];
        unsafe { libc::pipe(fds.as_mut_ptr()) };

        let handle = shim_dispatch_io_create(DISPATCH_IO_STREAM, fds[0], 0, 0);
        assert_ne!(handle, 0);

        // Verify it's in the registry
        assert!(registry().lock().unwrap().contains_key(&handle));

        // Close it
        shim_dispatch_io_close(handle, 0);

        // No longer in registry
        assert!(!registry().lock().unwrap().contains_key(&handle));

        unsafe {
            libc::close(fds[0]);
            libc::close(fds[1]);
        }
    }

    #[test]
    fn test_high_low_water() {
        let mut fds = [0i32; 2];
        unsafe { libc::pipe(fds.as_mut_ptr()) };

        let handle = shim_dispatch_io_create(DISPATCH_IO_STREAM, fds[0], 0, 0);

        shim_dispatch_io_set_high_water(handle, 4096);
        shim_dispatch_io_set_low_water(handle, 256);

        {
            let reg = registry().lock().unwrap();
            let chan = reg.get(&handle).unwrap();
            assert_eq!(*chan.high_water.lock().unwrap(), 4096);
            assert_eq!(*chan.low_water.lock().unwrap(), 256);
        }

        shim_dispatch_io_close(handle, 0);
        unsafe {
            libc::close(fds[0]);
            libc::close(fds[1]);
        }
    }

    #[test]
    fn test_get_descriptor() {
        let mut fds = [0i32; 2];
        unsafe { libc::pipe(fds.as_mut_ptr()) };

        let handle = shim_dispatch_io_create(DISPATCH_IO_STREAM, fds[0], 0, 0);
        assert_eq!(shim_dispatch_io_get_descriptor(handle), fds[0]);
        assert_eq!(shim_dispatch_io_get_descriptor(0x9999), -1);

        shim_dispatch_io_close(handle, 0);
        unsafe {
            libc::close(fds[0]);
            libc::close(fds[1]);
        }
    }
}
