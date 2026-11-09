//! The tests' allocator: the system one, counting what a thread asks for while it measures, so a test measures the
//! memory of its own code while the others run beside it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Meter;

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
    static NOW: Cell<isize> = const { Cell::new(0) };
    static PEAK: Cell<isize> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ON.try_with(|on| {
            if on.get() {
                let now = NOW.with(|n| {
                    n.set(n.get() + layout.size() as isize);
                    n.get()
                });
                PEAK.with(|p| p.set(p.get().max(now)));
            }
        });
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _ = ON.try_with(|on| {
            if on.get() {
                NOW.with(|n| n.set(n.get() - layout.size() as isize));
            }
        });
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static METER: Meter = Meter;

/// The most bytes `run` held at once on this thread above what it started with.
pub fn peak_of(run: impl FnOnce()) -> usize {
    NOW.with(|n| n.set(0));
    PEAK.with(|p| p.set(0));
    ON.with(|on| on.set(true));
    run();
    ON.with(|on| on.set(false));
    PEAK.with(|p| p.get()).max(0) as usize
}
