use core::alloc::Layout;
use core::fmt;
use core::ptr;
use core::mem;
use core::convert::TryInto;

use crate::allocator::linked_list::LinkedList;
use crate::allocator::util::*;
use crate::allocator::LocalAlloc;
use crate::console::kprintln;

/// A simple allocator that allocates based on size classes.
///   bin 0 (2^3 bytes)    : handles allocations in (0, 2^3]
///   bin 1 (2^4 bytes)    : handles allocations in (2^3, 2^4]
///   ...
///   bin 29 (2^22 bytes): handles allocations in (2^31, 2^32]
///   
///   map_to_bin(size) -> k
///   

pub struct Allocator {
	// FIXME: Add the necessary fields.
	free_lists: [LinkedList; 30],
	start: usize,
	end: usize
}

fn get_bin_size(layout: Layout) -> usize {
    let mut smallest_bin = 0;
    for i in 0..30 {
        if layout.size() + mem::size_of::<usize>() <= 2 << (i + 3) {
            smallest_bin = i;
            break;
        }
    }
    smallest_bin
}

impl Allocator {
	/// Creates a new bin allocator that will allocate memory from the region
	/// starting at address `start` and ending at address `end`.
	pub fn new(start: usize, end: usize) -> Allocator {
		let mut free_lists = [LinkedList::new(); 30];
		let mut curr_start = start;
		while end.saturating_sub(curr_start) >= (2 << 3) {
			let mut largest_bin = 0;
			for i in 0..30 {
				if end.saturating_sub(curr_start) >= (2 << (i + 3)) {
					largest_bin = i;
				} else {
					break;
				}
			}
			unsafe {free_lists[largest_bin].push(curr_start as *mut usize);}
			curr_start += 2 << largest_bin;
		}

		Allocator{free_lists, start, end}
	}
}

impl LocalAlloc for Allocator {
	/// Allocates memory. Returns a pointer meeting the size and alignment
	/// properties of `layout.size()` and `layout.align()`.
	///
	/// If this method returns an `Ok(addr)`, `addr` will be non-null address
	/// pointing to a block of storage suitable for holding an instance of
	/// `layout`. In particular, the block will be at least `layout.size()`
	/// bytes large and will be aligned to `layout.align()`. The returned block
	/// of storage may or may not have its contents initialized or zeroed.
	///
	/// # Safety
	///
	/// The _caller_ must ensure that `layout.size() > 0` and that
	/// `layout.align()` is a power of two. Parameters not meeting these
	/// conditions may result in undefined behavior.
	///
	/// # Errors
	///
	/// Returning null pointer (`core::ptr::null_mut`)
	/// indicates that either memory is exhausted
	/// or `layout` does not meet this allocator's
	/// size or alignment constraints.
	unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let smallest_bin = get_bin_size(layout);

        let chunk_start = match self.free_lists[smallest_bin].peek() {
            None => {
                panic!("failed layout size: {}", layout.size());
                return core::ptr::null_mut();
            },
            Some(ptr) => {
                panic!("success layout size: {}", layout.size());
                ptr.offset(mem::size_of::<usize>().try_into().unwrap())
            }
        } as usize;
        let chunk_end = chunk_start + (2 << smallest_bin);

        if chunk_end.saturating_sub(align_up(chunk_start, layout.align())) >= layout.size() {
            self.free_lists[smallest_bin].pop().expect("Free list pop failed while peek succeeded");
            //return chunk_start as *mut u8;
            align_up(chunk_start, layout.align()) as *mut u8
        } else {
            panic!("fail2 layout size: {}", layout.size());
            core::ptr::null_mut()
        }
	}

	/// Deallocates the memory referenced by `ptr`.
	///
	/// # Safety
	///
	/// The _caller_ must ensure the following:
	///
	///   * `ptr` must denote a block of memory currently allocated via this
	///		allocator
	///   * `layout` must properly represent the original layout used in the
	///		allocation call that returned `ptr`
	///
	/// Parameters not meeting these conditions may result in undefined
	/// behavior.
	unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let smallest_bin = get_bin_size(layout);
        self.free_lists[smallest_bin].push(ptr as *mut usize);
	}
}

// FIXME: Implement `Debug` for `Allocator`.
