use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::ThreadId;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use platform_core::alloc::Allocator;
use platform_core::clock::{CalendarComponents, Clock};
use platform_core::fs::FilesystemProvider;
use platform_core::hash::Hasher;
use platform_core::memory::MemoryMapper;
use platform_core::process::ProcessInfo;
use platform_core::rng::Rng;
use platform_core::symbols::HostSymbolResolver;
use platform_core::threading::{CondPtr, MutexPtr, Threading};
use platform_core::unwind::{NoopUnwindRegistry, UnwindRegistry};
use platform_core::{PalError, PlatformProvider, Protection};
use sha2::Digest;

pub(crate) struct FallbackPlatform {
    alloc: FallbackAllocator,
    memory: Arc<FallbackMemoryMapper>,
    symbols: FallbackSymbolResolver,
    unwind: NoopUnwindRegistry,
    clock: FallbackClock,
    rng: FallbackRng,
    process: FallbackProcessInfo,
    fs: FallbackFilesystem,
    hash: FallbackHasher,
    threading: FallbackThreading,
}

impl FallbackPlatform {
    pub(crate) fn new() -> Self {
        Self {
            alloc: FallbackAllocator,
            memory: Arc::new(FallbackMemoryMapper),
            symbols: FallbackSymbolResolver,
            unwind: NoopUnwindRegistry,
            clock: FallbackClock,
            rng: FallbackRng,
            process: FallbackProcessInfo::new(),
            fs: FallbackFilesystem,
            hash: FallbackHasher,
            threading: FallbackThreading,
        }
    }
}

impl Default for FallbackPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformProvider for FallbackPlatform {
    fn alloc(&self) -> &dyn Allocator {
        &self.alloc
    }

    fn memory(&self) -> Arc<dyn MemoryMapper> {
        self.memory.clone()
    }

    fn symbols(&self) -> &dyn HostSymbolResolver {
        &self.symbols
    }

    fn unwind(&self) -> &dyn UnwindRegistry {
        &self.unwind
    }

    fn clock(&self) -> &dyn Clock {
        &self.clock
    }

    fn rng(&self) -> &dyn Rng {
        &self.rng
    }

    fn process(&self) -> &dyn ProcessInfo {
        &self.process
    }

    fn fs(&self) -> &dyn FilesystemProvider {
        &self.fs
    }

    fn hash(&self) -> &dyn Hasher {
        &self.hash
    }

    fn threading(&self) -> &dyn Threading {
        &self.threading
    }
}

struct FallbackAllocator;

impl Allocator for FallbackAllocator {
    fn calloc(&self, count: usize, size: usize) -> *mut u8 {
        unsafe { libc::calloc(count, size) as *mut u8 }
    }

    fn malloc(&self, size: usize) -> *mut u8 {
        unsafe { libc::malloc(size) as *mut u8 }
    }

    fn realloc(&self, ptr: *mut u8, new_size: usize) -> *mut u8 {
        unsafe { libc::realloc(ptr.cast(), new_size) as *mut u8 }
    }

    fn free(&self, ptr: *mut u8) {
        unsafe { libc::free(ptr.cast()) }
    }
}

struct FallbackMemoryMapper;

impl MemoryMapper for FallbackMemoryMapper {
    fn page_size(&self) -> usize {
        4096
    }

    fn map_anonymous(&self, size: usize) -> Result<*mut u8, PalError> {
        let layout = page_layout(size)?;
        let ptr = unsafe { alloc_zeroed(layout) };
        if ptr.is_null() {
            Err(PalError::Memory(format!(
                "allocation failed for {size} bytes"
            )))
        } else {
            Ok(ptr)
        }
    }

    fn protect(&self, _addr: *mut u8, _size: usize, _prot: Protection) -> Result<(), PalError> {
        Ok(())
    }

    fn unmap(&self, addr: *mut u8, size: usize) -> Result<(), PalError> {
        if addr.is_null() {
            return Ok(());
        }
        let layout = page_layout(size)?;
        unsafe {
            dealloc(addr, layout);
        }
        Ok(())
    }
}

fn page_layout(size: usize) -> Result<Layout, PalError> {
    Layout::from_size_align(size.max(1), 4096)
        .map_err(|e| PalError::Memory(format!("invalid allocation layout: {e}")))
}

struct FallbackSymbolResolver;

impl HostSymbolResolver for FallbackSymbolResolver {
    fn resolve(&self, _name: &str) -> Option<*const ()> {
        None
    }

    fn resolve_in(&self, _handle: *mut (), _name: &str) -> Option<*const ()> {
        None
    }

    fn open_library(&self, _path: &str) -> Option<*mut ()> {
        None
    }

    fn default_handle(&self) -> *mut () {
        std::ptr::null_mut()
    }
}

struct FallbackClock;

impl Clock for FallbackClock {
    fn wall_time_usec(&self) -> (i64, i32) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        (now.as_secs() as i64, now.subsec_micros() as i32)
    }

    fn monotonic_ns(&self) -> u64 {
        static START: OnceLock<Instant> = OnceLock::new();
        START.get_or_init(Instant::now).elapsed().as_nanos() as u64
    }

    fn utc_offset_sec(&self) -> i64 {
        0
    }

    fn timezone_name(&self) -> String {
        "UTC".to_string()
    }

    fn localtime(&self, unix_sec: i64) -> CalendarComponents {
        let days = unix_sec.div_euclid(86_400);
        let secs_of_day = unix_sec.rem_euclid(86_400);
        let (year, mon, mday) = civil_from_days(days);
        let jan1 = days_from_civil(year, 1, 1);
        CalendarComponents {
            sec: (secs_of_day % 60) as i32,
            min: ((secs_of_day / 60) % 60) as i32,
            hour: (secs_of_day / 3600) as i32,
            mday: mday as i32,
            mon: mon as i32 - 1,
            year: year - 1900,
            wday: (days + 4).rem_euclid(7) as i32,
            yday: (days - jan1) as i32,
            isdst: 0,
            gmtoff: 0,
        }
    }

    fn mktime(&self, components: &mut CalendarComponents) -> i64 {
        let year = components.year + 1900;
        let mon = components.mon + 1;
        let days = days_from_civil(year, mon as u32, components.mday as u32);
        days * 86_400
            + components.hour as i64 * 3600
            + components.min as i64 * 60
            + components.sec as i64
    }

    fn sleep_secs(&self, seconds: f64) {
        if seconds > 0.0 {
            std::thread::sleep(Duration::from_secs_f64(seconds));
        }
    }
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    ((y + i64::from(m <= 2)) as i32, m as u32, d as u32)
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let mut y = year as i64;
    let m = month as i64;
    y -= i64::from(m <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

struct FallbackRng;

impl Rng for FallbackRng {
    fn fill_bytes(&self, buf: &mut [u8]) {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher as _};

        let mut offset = 0;
        while offset < buf.len() {
            let value = RandomState::new().build_hasher().finish().to_ne_bytes();
            let n = (buf.len() - offset).min(value.len());
            buf[offset..offset + n].copy_from_slice(&value[..n]);
            offset += n;
        }
    }
}

struct FallbackFilesystem;

impl FilesystemProvider for FallbackFilesystem {
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn read_link(&self, path: &Path) -> std::io::Result<PathBuf> {
        std::fs::read_link(path)
    }

    fn read_dir(&self, path: &Path) -> std::io::Result<Vec<String>> {
        std::fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
            .collect()
    }

    fn current_dir(&self) -> std::io::Result<PathBuf> {
        std::env::current_dir()
    }
}

struct FallbackHasher;

impl Hasher for FallbackHasher {
    fn sha256(&self, data: &[u8]) -> [u8; 32] {
        let digest = sha2::Sha256::digest(data);
        digest.into()
    }
}

struct FallbackProcessInfo {
    main_thread: ThreadId,
}

impl FallbackProcessInfo {
    fn new() -> Self {
        Self {
            main_thread: std::thread::current().id(),
        }
    }
}

impl ProcessInfo for FallbackProcessInfo {
    fn pid(&self) -> i32 {
        std::process::id() as i32
    }

    fn is_main_thread(&self) -> bool {
        std::thread::current().id() == self.main_thread
    }
}

struct HostMutex {
    recursive: bool,
    state: Mutex<HostMutexState>,
    changed: Condvar,
}

#[derive(Default)]
struct HostMutexState {
    locked: bool,
    owner: Option<ThreadId>,
    recursion: usize,
}

impl HostMutex {
    fn new(recursive: bool) -> Self {
        Self {
            recursive,
            state: Mutex::new(HostMutexState::default()),
            changed: Condvar::new(),
        }
    }

    fn lock(&self) {
        let current = std::thread::current().id();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if self.recursive && state.owner.as_ref() == Some(&current) {
            state.recursion += 1;
            return;
        }
        while state.locked {
            state = self.changed.wait(state).unwrap_or_else(|e| e.into_inner());
        }
        state.locked = true;
        state.owner = Some(current);
        state.recursion = 1;
    }

    fn unlock(&self) {
        let current = std::thread::current().id();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.owner.as_ref() != Some(&current) {
            return;
        }
        if self.recursive && state.recursion > 1 {
            state.recursion -= 1;
            return;
        }
        state.locked = false;
        state.owner = None;
        state.recursion = 0;
        self.changed.notify_one();
    }

    fn trylock(&self) -> bool {
        let current = std::thread::current().id();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if self.recursive && state.owner.as_ref() == Some(&current) {
            state.recursion += 1;
            return true;
        }
        if state.locked {
            return false;
        }
        state.locked = true;
        state.owner = Some(current);
        state.recursion = 1;
        true
    }
}

struct HostCond {
    generation: Mutex<u64>,
    changed: Condvar,
}

impl HostCond {
    fn new() -> Self {
        Self {
            generation: Mutex::new(0),
            changed: Condvar::new(),
        }
    }

    fn signal(&self, all: bool) {
        let mut generation = self.generation.lock().unwrap_or_else(|e| e.into_inner());
        *generation = generation.wrapping_add(1);
        if all {
            self.changed.notify_all();
        } else {
            self.changed.notify_one();
        }
    }
}

fn mutex_registry() -> &'static Mutex<HashMap<usize, Arc<HostMutex>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, Arc<HostMutex>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cond_registry() -> &'static Mutex<HashMap<usize, Arc<HostCond>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, Arc<HostCond>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn mutex_for(ptr: MutexPtr, recursive: bool) -> Option<Arc<HostMutex>> {
    let key = ptr as usize;
    if key == 0 {
        return None;
    }
    let mut registry = mutex_registry().lock().unwrap_or_else(|e| e.into_inner());
    Some(
        registry
            .entry(key)
            .or_insert_with(|| Arc::new(HostMutex::new(recursive)))
            .clone(),
    )
}

fn cond_for(ptr: CondPtr) -> Option<Arc<HostCond>> {
    let key = ptr as usize;
    if key == 0 {
        return None;
    }
    let mut registry = cond_registry().lock().unwrap_or_else(|e| e.into_inner());
    Some(
        registry
            .entry(key)
            .or_insert_with(|| Arc::new(HostCond::new()))
            .clone(),
    )
}

struct FallbackThreading;

impl Threading for FallbackThreading {
    fn mutex_size(&self) -> usize {
        std::mem::size_of::<usize>()
    }

    fn cond_size(&self) -> usize {
        std::mem::size_of::<usize>()
    }

    fn mutex_init(&self, ptr: MutexPtr) {
        let _ = mutex_for(ptr, false);
    }

    fn mutex_init_recursive(&self, ptr: MutexPtr) {
        let _ = mutex_for(ptr, true);
    }

    fn mutex_lock(&self, ptr: MutexPtr) {
        if let Some(mutex) = mutex_for(ptr, false) {
            mutex.lock();
        }
    }

    fn mutex_unlock(&self, ptr: MutexPtr) {
        if let Some(mutex) = mutex_for(ptr, false) {
            mutex.unlock();
        }
    }

    fn mutex_trylock(&self, ptr: MutexPtr) -> bool {
        mutex_for(ptr, false).is_some_and(|mutex| mutex.trylock())
    }

    fn mutex_destroy(&self, ptr: MutexPtr) {
        let key = ptr as usize;
        let mut registry = mutex_registry().lock().unwrap_or_else(|e| e.into_inner());
        registry.remove(&key);
    }

    fn cond_init(&self, ptr: CondPtr) {
        let _ = cond_for(ptr);
    }

    fn cond_wait(&self, cond: CondPtr, mutex: MutexPtr) {
        let Some(cond) = cond_for(cond) else {
            return;
        };
        let mut generation = cond.generation.lock().unwrap_or_else(|e| e.into_inner());
        let seen = *generation;
        self.mutex_unlock(mutex);
        while *generation == seen {
            generation = cond
                .changed
                .wait(generation)
                .unwrap_or_else(|e| e.into_inner());
        }
        drop(generation);
        self.mutex_lock(mutex);
    }

    fn cond_signal(&self, ptr: CondPtr) {
        if let Some(cond) = cond_for(ptr) {
            cond.signal(false);
        }
    }

    fn cond_broadcast(&self, ptr: CondPtr) {
        if let Some(cond) = cond_for(ptr) {
            cond.signal(true);
        }
    }

    fn cond_destroy(&self, ptr: CondPtr) {
        let key = ptr as usize;
        let mut registry = cond_registry().lock().unwrap_or_else(|e| e.into_inner());
        registry.remove(&key);
    }
}
