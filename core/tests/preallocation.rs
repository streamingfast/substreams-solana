//! A huge length prefix never reserves more than the input can hold.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use substreams_solana_core::borsh::{decode, DecodeError, Pubkey};

struct Tracking;

static LARGEST: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static TRACKING: Cell<bool> = const { Cell::new(false) };
}

unsafe impl GlobalAlloc for Tracking {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACKING.with(Cell::get) {
            LARGEST.fetch_max(layout.size(), Ordering::Relaxed);
        }
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if TRACKING.with(Cell::get) {
            LARGEST.fetch_max(new_size, Ordering::Relaxed);
        }
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static ALLOCATOR: Tracking = Tracking;

#[test]
fn huge_length_prefix_on_a_large_buffer() {
    // 1 MiB of addresses behind a length prefix claiming a million of them.
    let mut data = 1_000_000u32.to_le_bytes().to_vec();
    data.resize(4 + (1 << 20), 0);

    TRACKING.with(|t| t.set(true));
    let result = decode::<Vec<Pubkey>>(&data);
    TRACKING.with(|t| t.set(false));

    assert!(matches!(result, Err(DecodeError::UnexpectedEof { .. })));
    // Growing by doubling up to the 32,768 addresses the input holds: 1 MiB. Trusting the
    // prefix would reserve 32 MB at once.
    assert!(LARGEST.load(Ordering::Relaxed) <= 1 << 20);
}
