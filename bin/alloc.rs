use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::*};

/// 分配内存计数, from the std ref.
struct Counter;

static MAX_MEM_USED: AtomicUsize = AtomicUsize::new(0);
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = unsafe { System.alloc(layout) };
        if !ret.is_null() {
            let a = ALLOCATED.fetch_add(layout.size(), AcqRel);
            MAX_MEM_USED.fetch_max(a + layout.size(), Relaxed);
        }
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            System.dealloc(ptr, layout);
        }
        ALLOCATED.fetch_sub(layout.size(), AcqRel);
    }
}

#[global_allocator]
static A: Counter = Counter;

pub(crate) fn max_mem_used() -> usize {
    return MAX_MEM_USED.load(Acquire);
}
