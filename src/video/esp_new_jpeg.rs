#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
pub use bindings::*;

// From https://github.com/discapes/rumble-rs/blob/cbe6804f4ad3e08e9fb77955fa3a7108485d4db4/src/jpeg.rs

extern crate alloc;
use alloc::alloc::{Layout, alloc, dealloc};
use core::{ffi::c_void, ptr};

// ---------------------------------------------------------------------------
// ESP-IDF heap_caps stubs — the .a library calls these internally
//
// Strategy: allocate with Rust's global allocator using Layout(size, align=1),
// store (raw_ptr, alloc_size) in a header just before the returned pointer.
// ---------------------------------------------------------------------------

const META_WORDS: usize = 2; // raw_ptr + alloc_size
const META_BYTES: usize = META_WORDS * core::mem::size_of::<usize>();

unsafe fn caps_alloc_inner(count: usize, size: usize, align: usize) -> *mut u8 {
    let payload = match count.checked_mul(size) {
        Some(0) | None => return ptr::null_mut(),
        Some(p) => p,
    };
    let align = align.max(core::mem::size_of::<usize>());
    let alloc_size = payload + META_BYTES + align;
    unsafe {
        let layout = Layout::from_size_align_unchecked(alloc_size, 1);
        let raw = alloc(layout);
        if raw.is_null() {
            return ptr::null_mut();
        }
        raw.write_bytes(0, alloc_size);
        // Aligned user pointer with room for header
        let base = raw as usize + META_BYTES;
        let user_addr = (base + align - 1) & !(align - 1);
        let meta = user_addr as *mut usize;
        meta.sub(1).write(raw as usize);
        meta.sub(2).write(alloc_size);
        user_addr as *mut u8
    }
}

unsafe fn caps_free_inner(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let meta = ptr as *mut usize;
        let raw = meta.sub(1).read() as *mut u8;
        let alloc_size = meta.sub(2).read();
        let layout = Layout::from_size_align_unchecked(alloc_size, 1);
        dealloc(raw, layout);
    }
}

/// `void *heap_caps_calloc_prefer(size_t n, size_t size, size_t num, ...)`
/// The variadic caps arguments are ignored — we just allocate from Rust's heap.
#[unsafe(no_mangle)]
pub extern "C" fn heap_caps_calloc_prefer(n: usize, size: usize, _num: usize) -> *mut c_void {
    unsafe { caps_alloc_inner(n, size, 4) as *mut c_void }
}

/// `void heap_caps_free(void *ptr)`
#[unsafe(no_mangle)]
pub extern "C" fn heap_caps_free(ptr: *mut c_void) {
    unsafe { caps_free_inner(ptr as *mut u8) }
}

/// `void *heap_caps_aligned_calloc(size_t alignment, size_t n, size_t size, uint32_t caps)`
#[unsafe(no_mangle)]
pub extern "C" fn heap_caps_aligned_calloc(
    alignment: usize,
    n: usize,
    size: usize,
    _caps: u32,
) -> *mut c_void {
    unsafe { caps_alloc_inner(n, size, alignment) as *mut c_void }
}

// ---------------------------------------------------------------------------
// ESP-IDF logging stubs — the .a library calls these for diagnostics
// ---------------------------------------------------------------------------

/// `void esp_log_write(esp_log_level_t level, const char *tag, const char *format, ...)`
#[unsafe(no_mangle)]
pub extern "C" fn esp_log_write(_level: u32, _tag: *const u8, _fmt: *const u8) {
    // no-op: variadic printf formatting is not practical in no_std
}

/// `void esp_log_level_set(const char *tag, esp_log_level_t level)`
#[unsafe(no_mangle)]
pub extern "C" fn esp_log_level_set(_tag: *const u8, _level: u32) {}

/// `uint32_t esp_log_timestamp(void)`
#[unsafe(no_mangle)]
pub extern "C" fn esp_log_timestamp() -> u32 {
    0
}
