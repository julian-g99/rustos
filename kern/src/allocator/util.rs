/// Align `addr` downwards to the nearest multiple of `align`.
///
/// The returned usize is always <= `addr.`
///
/// # Panics
///
/// Panics if `align` is not a power of 2.
pub fn align_down(addr: usize, align: usize) -> usize {
    if align & (align - 1) != 0 {
		panic!("Align given in align down isn't a power of 2");
    }

    addr - addr % align
}

/// Align `addr` upwards to the nearest multiple of `align`.
///
/// The returned `usize` is always >= `addr.`
///
/// # Panics
///
/// Panics if `align` is not a power of 2
/// or aligning up overflows the address.
pub fn align_up(addr: usize, align: usize) -> usize {
    if align & (align - 1) != 0 {
        panic!("Align given in align up isn't a power of 2")
    }

    if addr % align == 0 {
        addr
    } else {
        addr + (align - addr % align)
    }
}
