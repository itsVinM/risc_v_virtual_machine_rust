use crate::allocator;

pub const PAGE_SIZE: usize = 4096;
pub const PAGE_ENTRIES: usize = 512;
pub const PTE_FLAGS_MASK: u64 = 0x3FF;
pub const PTE_PPN_MASK: u64 = !PTE_FLAGS_MASK;

const PTE_V: u64 = 1 << 0;
const PTE_R: u64 = 1 << 1;
const PTE_W: u64 = 1 << 2;
const PTE_X: u64 = 1 << 3;

pub const READ_WRITE: u64 = PTE_R | PTE_W;
pub const READ_EXECUTE: u64 = PTE_R | PTE_X;
pub const READ_WRITE_EXECUTE: u64 = PTE_R | PTE_W | PTE_X;

pub struct PageTable {
    root_paddr: usize,
}

impl PageTable {
    pub fn new() -> Option<Self> {
        let root = allocator::alloc_page()?;
        Some(Self { root_paddr: root })
    }

    #[inline]
    pub fn root_paddr(&self) -> usize {
        self.root_paddr
    }

    #[inline]
    fn read_pte(table: usize, index: usize) -> u64 {
        unsafe { core::ptr::read_volatile((table + index * 8) as *const u64) }
    }

    #[inline]
    fn write_pte(table: usize, index: usize, pte: u64) {
        unsafe { core::ptr::write_volatile((table + index * 8) as *mut u64, pte) }
    }

    /// Walk the page table to find the L3 table for a vaddr.
    /// If `create` is true, allocates missing intermediate tables.
    fn walk(&self, vaddr: usize, create: bool) -> Option<usize> {
        let vpn2 = (vaddr >> 30) & 0x1FF;
        let vpn1 = (vaddr >> 21) & 0x1FF;

        let mut table = self.root_paddr;
        // Level 1: VPN[2] -> L2 table
        let pte1 = Self::read_pte(table, vpn2);
        table = if pte1 & PTE_V == 0 {
            if !create {
                return None;
            }
            let new = allocator::alloc_page()?;
            Self::write_pte(table, vpn2, new as u64 | PTE_V);
            new
        } else {
            (pte1 & PTE_PPN_MASK) as usize
        };

        // Level 2: VPN[1] -> L1 table
        let pte2 = Self::read_pte(table, vpn1);
        table = if pte2 & PTE_V == 0 {
            if !create {
                return None;
            }
            let new = allocator::alloc_page()?;
            Self::write_pte(table, vpn1, new as u64 | PTE_V);
            new
        } else {
            (pte2 & PTE_PPN_MASK) as usize
        };

        // Level 3: VPN[0] -> leaf PTE
        Some(table)
    }

    pub fn map(&mut self, vaddr: usize, paddr: usize, flags: u64) -> bool {
        let vpn0 = (vaddr >> 12) & 0x1FF;
        let table = match self.walk(vaddr, true) {
            Some(t) => t,
            None => return false,
        };
        let pte = (paddr as u64 & PTE_PPN_MASK) | flags | PTE_V | (1 << 6) | (1 << 7);
        Self::write_pte(table, vpn0, pte);
        self.fence();
        true
    }

    pub fn unmap(&mut self, vaddr: usize) {
        let vpn0 = (vaddr >> 12) & 0x1FF;
        if let Some(table) = self.walk(vaddr, false) {
            Self::write_pte(table, vpn0, 0);
            self.fence();
        }
    }

    pub fn translate(&self, vaddr: usize) -> Option<usize> {
        let vpn0 = (vaddr >> 12) & 0x1FF;
        let table = self.walk(vaddr, false)?;
        let pte = Self::read_pte(table, vpn0);
        if pte & PTE_V == 0 {
            return None;
        }
        let ppn = (pte & PTE_PPN_MASK) as usize;
        Some(ppn | (vaddr & 0xFFF))
    }

    pub fn fence(&self) {
        unsafe { core::arch::asm!("sfence.vma"); }
    }
}

pub fn enable(root_paddr: usize) {
    let satp = (8u64 << 60) | (root_paddr as u64 >> 12);
    unsafe {
        core::arch::asm!("csrw satp, {}", in(reg) satp);
        core::arch::asm!("sfence.vma");
    }
}
