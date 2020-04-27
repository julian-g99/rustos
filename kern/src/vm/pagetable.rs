use core::iter::Chain;
use core::ops::{Deref, DerefMut};
use core::slice::Iter;

use alloc::boxed::Box;
use alloc::fmt;
use core::alloc::{GlobalAlloc, Layout};

use crate::allocator;
use crate::param::*;
use crate::vm::{PhysicalAddr, VirtualAddr};
use crate::ALLOCATOR;

use aarch64::vmsa::*;
use shim::const_assert_size;

use crate::console::kprintln;

#[repr(C)]
pub struct Page([u8; PAGE_SIZE]);
const_assert_size!(Page, PAGE_SIZE);

const OUTER_SHARE: u64 = 0x10;
const INNER_SHARE: u64 = 0x11;

const NORMAL_MEM: u64 = 0x000;
const DEVICE_MEM: u64 = 0x001;

const KERN_RW: u64 = 0x00;
const USER_RW: u64 = 0x01;
const KERN_RO: u64 = 0x10;
const USER_RO: u64 = 0x11;

impl Page {
    pub const SIZE: usize = PAGE_SIZE;
    pub const ALIGN: usize = PAGE_SIZE;

    fn layout() -> Layout {
        unsafe { Layout::from_size_align_unchecked(Self::SIZE, Self::ALIGN) }
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct L2PageTable {
    pub entries: [RawL2Entry; 8192],
}
const_assert_size!(L2PageTable, PAGE_SIZE);

impl L2PageTable {
    /// Returns a new `L2PageTable`
    fn new() -> L2PageTable {
        L2PageTable {entries: [RawL2Entry::new(0); 8192]}
    }

    /// Returns a `PhysicalAddr` of the pagetable.
    pub fn as_ptr(&self) -> PhysicalAddr {
        let val = &self.entries[0] as *const RawL2Entry as u64;
        PhysicalAddr::from(val)
    }
}

#[derive(Copy, Clone)]
pub struct L3Entry(RawL3Entry);

impl L3Entry {
    /// Returns a new `L3Entry`.
    fn new() -> L3Entry {
        L3Entry(RawL3Entry::new(0))
    }

    /// Returns `true` if the L3Entry is valid and `false` otherwise.
    fn is_valid(&self) -> bool {
        self.0.get_value(RawL3Entry::VALID) == 1
    }

    /// Extracts `ADDR` field of the L3Entry and returns as a `PhysicalAddr`
    /// if valid. Otherwise, return `None`.
    fn get_page_addr(&self) -> Option<PhysicalAddr> {
        if self.is_valid() {
            let addr = self.0.get_masked(RawL3Entry::ADDR); // should this be get_value
            Some(PhysicalAddr::from(addr))
        } else {
            None
        }
    }

    //fn set_addr(&mut self, addr: u64, perm: u64, share: u64, attr: u64) {
        //self.0.set_bit(RawL3Entry::AF);
        //self.0.set_value(perm, RawL3Entry::AP);
        //self.0.set_masked(addr, RawL3Entry::ADDR);
        //self.0.set_bit(RawL3Entry::VALID);
        //self.0.set_bit(RawL3Entry::TYPE);
        //self.0.set_bit(RawL3Entry::AF);

        //self.0.set_value(share, RawL3Entry::SH);
        //self.0.set_value(attr, RawL3Entry::ATTR);
    //}

    fn get_addr(&self) -> u64 {
        self.0.get_value(RawL3Entry::ADDR)
    }

    fn set_invalid(&mut self) {
        self.0.clear_bit(RawL3Entry::VALID);
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct L3PageTable {
    pub entries: [L3Entry; 8192],
}
const_assert_size!(L3PageTable, PAGE_SIZE);

impl L3PageTable {
    /// Returns a new `L3PageTable`.
    fn new() -> L3PageTable {
        L3PageTable{ entries: [L3Entry::new(); 8192] }
    }

    /// Returns a `PhysicalAddr` of the pagetable.
    pub fn as_ptr(&self) -> PhysicalAddr {
        //unimplemented!("L3PageTable::as_ptr()")
        let val = &self.entries[0] as *const L3Entry as u64;
        PhysicalAddr::from(val)
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct PageTable {
    pub l2: L2PageTable,
    pub l3: [L3PageTable; 2],
}

impl PageTable {
    /// Returns a new `Box` containing `PageTable`.
    /// Entries in L2PageTable should be initialized properly before return.
    fn new(perm: u64) -> Box<PageTable> {
        let mut page_table = Box::new(PageTable{
            l2: L2PageTable::new(), l3: [L3PageTable::new(), L3PageTable::new()]
        });

        //use RawL2Entry::*;
        for i in 0..2 {
            page_table.l2.entries[i].set_masked(page_table.l3[i].as_ptr().as_u64(), RawL2Entry::ADDR);
            page_table.l2.entries[i].set_value(EntrySh::ISh, RawL2Entry::SH);
            page_table.l2.entries[i].set_bit(RawL2Entry::AF);
            page_table.l2.entries[i].set_value(perm, RawL2Entry::AP);
            page_table.l2.entries[i].set_value(EntryAttr::Mem, RawL2Entry::ATTR);
            page_table.l2.entries[i].set_bit(RawL2Entry::TYPE);
            page_table.l2.entries[i].set_bit(RawL2Entry::VALID);

        }

        page_table
    }

    /// Returns the (L2index, L3index) extracted from the given virtual address.
    /// Since we are only supporting 1GB virtual memory in this system, L2index
    /// should be smaller than 2.
    ///
    /// # Panics
    ///
    /// Panics if the virtual address is not properly aligned to page size.
    /// Panics if extracted L2index exceeds the number of L3PageTable.
    fn locate(va: VirtualAddr) -> (usize, usize) {
        //unimplemented!("PageTable::localte()")
        if !va.is_aligned(PAGE_SIZE) {
            panic!("Virtual address is not aligned to page size. Page size is {:x}, VA is {:x}", PAGE_SIZE, va.as_u64());
        }

        let l2_index = va.l2_index();
        let l3_index = va.l3_index();

        if l2_index >= 2 {
            panic!("l2 index is greater than or equal to 2");
        }

        (l2_index, l3_index)
    }

    /// Returns `true` if the L3entry indicated by the given virtual address is valid.
    /// Otherwise, `false` is returned.
    pub fn is_valid(&self, va: VirtualAddr) -> bool {
        let (l2_index, l3_index) = Self::locate(va);

        self.l3[l2_index].entries[l3_index].is_valid()
    }

    /// Returns `true` if the L3entry indicated by the given virtual address is invalid.
    /// Otherwise, `true` is returned.
    pub fn is_invalid(&self, va: VirtualAddr) -> bool {
        !self.is_valid(va)
    }

    /// Set the given RawL3Entry `entry` to the L3Entry indicated by the given virtual
    /// address.
    pub fn set_entry(&mut self, va: VirtualAddr, entry: RawL3Entry) -> &mut Self {
        //unimplemented!("PageTable::set_entry()")
        let (l2_index, l3_index) = PageTable::locate(va);
        self.l3[l2_index].entries[l3_index].0 = entry;

        self
    }

    /// Returns a base address of the pagetable. The returned `PhysicalAddr` value
    /// will point the start address of the L2PageTable.
    pub fn get_baddr(&self) -> PhysicalAddr {
        self.l2.as_ptr()
    }
}

// FIXME: Implement `IntoIterator` for `&PageTable`.
impl<'a> IntoIterator for &'a PageTable {
    type Item = &'a L3Entry;
    type IntoIter = core::iter::Chain<core::slice::Iter<'a, L3Entry>, core::slice::Iter<'a, L3Entry>>;

    fn into_iter(self) -> Self::IntoIter {
        let first = self.l3[0].entries.iter();
        let second = self.l3[1].entries.iter();

        return first.chain(second);
    }
}

pub struct KernPageTable(Box<PageTable>);

impl KernPageTable {
    /// Returns a new `KernPageTable`. `KernPageTable` should have a `Pagetable`
    /// created with `KERN_RW` permission.
    ///
    /// Set L3entry of ARM physical address starting at 0x00000000 for RAM and
    /// physical address range from `IO_BASE` to `IO_BASE_END` for peripherals.
    /// Each L3 entry should have correct value for lower attributes[10:0] as well
    /// as address[47:16]. Refer to the definition of `RawL3Entry` in `vmsa.rs` for
    /// more details.
    pub fn new() -> KernPageTable {
        //let page_table = PageTable::new(KERN_RW);
        let mut kern_table = KernPageTable(PageTable::new(EntryPerm::KERN_RW));
        let (start, end) = allocator::memory_map().expect("memory map call returns none in KernPageTable::new()");

        let mut curr = 0usize;

        while curr < end {
            let mut entry = RawL3Entry::new(curr as u64);
            entry.set_bit(RawL3Entry::AF);
            entry.set_masked(curr as u64, RawL3Entry::ADDR);
            entry.set_value(EntrySh::ISh, RawL3Entry::SH);
            entry.set_value(EntryPerm::KERN_RW, RawL3Entry::AP);
            entry.set_value(EntryAttr::Mem, RawL3Entry::ATTR);
            entry.set_bit(RawL3Entry::TYPE);
            entry.set_bit(RawL3Entry::VALID);

            kern_table.0.set_entry(VirtualAddr::from(curr), entry);

            curr += PAGE_SIZE;
        }

        curr = IO_BASE;
        while curr < IO_BASE_END {
            let mut entry = RawL3Entry::new(curr as u64);
            entry.set_bit(RawL3Entry::AF);
            entry.set_masked(curr as u64, RawL3Entry::ADDR);
            entry.set_value(EntrySh::OSh, RawL3Entry::SH);
            entry.set_value(EntryPerm::KERN_RW, RawL3Entry::AP);
            entry.set_value(EntryAttr::Dev, RawL3Entry::ATTR);
            entry.set_bit(RawL3Entry::TYPE);
            entry.set_bit(RawL3Entry::VALID);

            kern_table.0.set_entry(VirtualAddr::from(curr), entry);
            curr += PAGE_SIZE;
        }

        kern_table
    }
}

pub enum PagePerm {
    RW,
    RO,
    RWX,
}

pub struct UserPageTable(Box<PageTable>);

impl UserPageTable {
    /// Returns a new `UserPageTable` containing a `PageTable` created with
    /// `USER_RW` permission.
    pub fn new() -> UserPageTable {
        let page_table = PageTable::new(EntryPerm::USER_RW);
        UserPageTable(page_table)
    }

    /// Allocates a page and set an L3 entry translates given virtual address to the
    /// physical address of the allocated page. Returns the allocated page.
    ///
    /// # Panics
    /// Panics if the virtual address is lower than `USER_IMG_BASE`.
    /// Panics if the virtual address has already been allocated.
    /// Panics if allocator fails to allocate a page.
    ///
    /// TODO. use Result<T> and make it failurable
    /// TODO. use perm properly
    pub fn alloc(&mut self, va: VirtualAddr, _perm: PagePerm) -> &mut [u8] {
        if va.as_usize() < USER_IMG_BASE {
            panic!("Virtual address given to UserPageTable::alloc() is less than the image base");
        }

        if self.0.is_valid(va) {
            panic!("Virtual address given to UserPageTable::alloc() alerady has a page allocated");
        }

        let addr = unsafe{ ALLOCATOR.alloc(Page::layout()) };
        if addr == core::ptr::null_mut() {
            panic!("Do not have enough memory to assign new page");
        }

        let mut entry = RawL3Entry::new(addr as u64);
        entry.set_bit(RawL3Entry::AF);
        entry.set_masked(addr as u64, RawL3Entry::ADDR);
        entry.set_value(EntrySh::ISh, RawL3Entry::SH);
        entry.set_value(EntryPerm::USER_RW, RawL3Entry::AP);
        entry.set_value(EntryAttr::Mem, RawL3Entry::ATTR);
        entry.set_bit(RawL3Entry::TYPE);
        entry.set_bit(RawL3Entry::VALID);

        self.0.set_entry(va, entry);

        unsafe {
            core::slice::from_raw_parts_mut(addr, PAGE_SIZE)
        }
    }

    pub fn get_physical_address(&self, va: VirtualAddr) -> u64 {
        let (l2_index, l3_index) = PageTable::locate(va);
        let entry = self.0.l3[l2_index].entries[l3_index].0;
        let addr_base = entry.get_value(RawL3Entry::ADDR);
        let addr_offset = va.get_offset() as u64;
        addr_base << 16 + addr_offset
    }

    pub fn page_value(&self, va: VirtualAddr, size: usize) -> &[u8]{
        let addr = self.get_physical_address(va);

        unsafe {
            core::slice::from_raw_parts(addr as *const u8, size)
        }
    }
}

impl Deref for KernPageTable {
    type Target = PageTable;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for UserPageTable {
    type Target = PageTable;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for KernPageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DerefMut for UserPageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// FIXME: Implement `Drop` for `UserPageTable`.
impl Drop for UserPageTable {
    fn drop(&mut self) {
        //for l3_table in l3.iter_mut() {
            //for entry in l3_table.iter_mut() {
                //if entry.is_valid() {
                    //ALLOCATOR.dealloc(entry.get_addr(), Page::layout());
                    //entry.set_invalid();
                //}
            //}
        //}
        for entry in self.0.into_iter() {
            if entry.is_valid() {
                unsafe { ALLOCATOR.dealloc(entry.get_addr() as *mut u8, Page::layout()); }
            }
        }
    }
}

impl fmt::Debug for UserPageTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let page_table = &self.0;
        f.debug_list().entries((&page_table).into_iter()).finish()
    }
}

impl fmt::Debug for KernPageTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let page_table = &self.0;
        f.debug_list().entries((&page_table).into_iter()).finish()
    }
}
impl fmt::Debug for L3Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
