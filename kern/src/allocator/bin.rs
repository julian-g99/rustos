use core::alloc::Layout;
use core::fmt;
use core::ptr;
use core::mem;
use core::convert::TryInto;
use core::cmp::max;


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
	curr_start: usize, //since it changes change
	end: usize
}

fn get_bin_size(layout: Layout) -> usize {
    let mut smallest_bin = 0;
    if layout.size() > 1 << (29 + 3) {
        return 30;
    }
    for i in 0..30 {
        let bin_size = 1 << (i + 3);
        if bin_size >= layout.size() && bin_size >= layout.align() { //does this need to be integer multiple?
            smallest_bin = i;
            break
        }
    }
    smallest_bin
}

impl Allocator {
	/// Creates a new bin allocator that will allocate memory from the region
	/// starting at address `start` and ending at address `end`.
	pub fn new(start: usize, end: usize) -> Allocator {
		let mut free_lists = [LinkedList::new(); 30];
		//let mut curr_start = start;
		//while end.saturating_sub(curr_start) >= (2 << 3) {
			//let mut largest_bin = 0;
			//for i in 0..30 {
				//if end.saturating_sub(curr_start) >= (2 << (i + 3)) {
					//largest_bin = i;
				//} else {
					//break;
				//}
			//}
			//unsafe {free_lists[largest_bin].push(curr_start as *mut usize);}
			//curr_start += 2 << (largest_bin + 3);
		//}

		Allocator{free_lists, curr_start: start, end}
	}
}

fn is_valid(layout: Layout) -> bool {
    layout.align().is_power_of_two() && layout.size() > 0
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
        //return core::ptr::null_mut();
        let smallest_bin = get_bin_size(layout);
        if !(is_valid(layout) && smallest_bin != 30) {
            return core::ptr::null_mut();
        }

        let align = max(layout.align(), 4); //so the pi doesn't hang on read_sector

        for ptr in self.free_lists[smallest_bin].iter_mut() {
            if ptr.value() as usize % align == 0 {
                return ptr.pop() as *mut u8;
            }
        }

        //try to find another bin as a bump allocator
        let aligned_start = align_up(self.curr_start, align); //might not be aligned on first call
        for i in 0..30 {
            let bin_size = 1 << (i + 3);
            if bin_size >= layout.size()
                && bin_size <= self.end.saturating_sub(aligned_start) {
                    self.curr_start = aligned_start + bin_size;
                    return aligned_start as *mut u8;
            }
        }

        return core::ptr::null_mut();
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
