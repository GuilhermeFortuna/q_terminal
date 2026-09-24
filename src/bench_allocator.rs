//! Process-wide Rust allocation counter enabled only by the frame benchmark feature.

#[cfg(feature = "bench-allocations")]
mod enabled {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    static COUNTING: AtomicBool = AtomicBool::new(false);
    static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

    struct CountingAllocator;

    // The allocator delegates every operation to `System`; the atomics only count successful
    // allocation and reallocation requests while the benchmark's warmed-up interval is active.
    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let ptr = unsafe { System.alloc(layout) };
            if !ptr.is_null() && COUNTING.load(Ordering::Relaxed) {
                ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            }
            ptr
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            let ptr = unsafe { System.alloc_zeroed(layout) };
            if !ptr.is_null() && COUNTING.load(Ordering::Relaxed) {
                ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            }
            ptr
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) };
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
            if !new_ptr.is_null() && COUNTING.load(Ordering::Relaxed) {
                ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            }
            new_ptr
        }
    }

    #[global_allocator]
    static ALLOCATOR: CountingAllocator = CountingAllocator;

    #[unsafe(no_mangle)]
    pub extern "C" fn q_terminal_bench_allocations_start() {
        ALLOCATIONS.store(0, Ordering::Relaxed);
        COUNTING.store(true, Ordering::Release);
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn q_terminal_bench_allocations_stop() -> u64 {
        COUNTING.store(false, Ordering::Release);
        ALLOCATIONS.load(Ordering::Relaxed)
    }
}
#[cfg(feature = "bench-allocations")]
pub use enabled::{q_terminal_bench_allocations_start, q_terminal_bench_allocations_stop};

#[cfg(not(feature = "bench-allocations"))]
mod disabled {
    #[unsafe(no_mangle)]
    pub extern "C" fn q_terminal_bench_allocations_start() {}

    #[unsafe(no_mangle)]
    pub extern "C" fn q_terminal_bench_allocations_stop() -> u64 {
        0
    }
}
#[cfg(not(feature = "bench-allocations"))]
pub use disabled::{q_terminal_bench_allocations_start, q_terminal_bench_allocations_stop};
