extern "C" {
    static _heap_start: u8;
    static _heap_end: u8;
}

const PAGE_SIZE: usize = 4096;
const BITS: usize = 64;

pub struct PageAllocator {
    base: usize,
    pages: usize,
    bitmap: *mut u64,
    bm_words: usize,
    free: usize,
    hint: usize,
}

impl PageAllocator {
    pub fn new() -> Self {
        let base = unsafe { (&_heap_start as *const u8) as usize };
        let end = unsafe { (&_heap_end as *const u8) as usize };
        let heap_sz = end - base;
        let heap_pgs = heap_sz / PAGE_SIZE;

        let words = (heap_pgs + BITS - 1) / BITS;
        let bstart = end - words * 8;

        let mut a = Self {
            base, pages: heap_pgs,
            bitmap: bstart as *mut u64, bm_words: words,
            free: heap_pgs, hint: 0,
        };

        a.init_bm();
        a
    }

    fn init_bm(&mut self) {
        for i in 0..self.bm_words {
            unsafe { *self.bitmap.add(i) = 0; }
        }
        let extra = self.bm_words * BITS - self.pages;
        if extra > 0 {
            unsafe { *self.bitmap.add(self.bm_words - 1) = !0u64 << (BITS - extra); }
        }
    }

    #[inline] fn to_addr(&self, i: usize) -> usize { self.base + i * PAGE_SIZE }
    #[inline] fn to_idx(&self, a: usize) -> Option<usize> {
        if a < self.base { return None; }
        let i = (a - self.base) / PAGE_SIZE;
        if i < self.pages { Some(i) } else { None }
    }

    #[inline]
    fn mark(&mut self, i: usize, take: bool) {
        let w = i / BITS; let b = i % BITS; let m = 1u64 << b;
        unsafe {
            if take {
                if *self.bitmap.add(w) & m == 0 { self.free -= 1; }
                *self.bitmap.add(w) |= m;
            } else {
                if *self.bitmap.add(w) & m != 0 { self.free += 1; }
                *self.bitmap.add(w) &= !m;
            }
        }
    }

    #[inline]
    fn is_free(&self, i: usize) -> bool {
        let w = i / BITS; let b = i % BITS;
        unsafe { (*self.bitmap.add(w) >> b) & 1 == 0 }
    }

    pub fn alloc_page(&mut self) -> Option<usize> {
        if self.free == 0 { return None; }
        let sw = self.hint / BITS;
        for d in 0..self.bm_words {
            let wi = (sw + d) % self.bm_words;
            let v = unsafe { !*self.bitmap.add(wi) };
            if v == 0 { continue; }
            let bi = v.trailing_zeros() as usize;
            let idx = wi * BITS + bi;
            if idx >= self.pages { continue; }
            self.mark(idx, true);
            let addr = self.to_addr(idx);
            unsafe { core::ptr::write_bytes(addr as *mut u8, 0, PAGE_SIZE); }
            self.hint = idx + 1;
            return Some(addr);
        }
        None
    }

    pub fn alloc_contig(&mut self, count: usize) -> Option<usize> {
        if count == 0 || self.free < count { return None; }
        let mut run = 0usize;
        let mut start = 0usize;
        for d in 0..self.pages {
            let idx = (self.hint + d) % self.pages;
            if self.is_free(idx) {
                if run == 0 { start = idx; }
                run += 1;
                if run == count {
                    for i in 0..count { self.mark((start + i) % self.pages, true); }
                    let addr = self.to_addr(start);
                    unsafe { core::ptr::write_bytes(addr as *mut u8, 0, count * PAGE_SIZE); }
                    return Some(addr);
                }
            } else { run = 0; }
        }
        None
    }

    pub fn free_page(&mut self, addr: usize) {
        if let Some(idx) = self.to_idx(addr) { self.mark(idx, false); }
    }

    #[inline] pub fn total(&self) -> usize { self.pages }
    #[inline] pub fn free_count(&self) -> usize { self.free }
}

static mut A: Option<PageAllocator> = None;

pub fn init() {
    unsafe { A = Some(PageAllocator::new()); }
}

pub fn alloc_page() -> Option<usize> {
    unsafe { A.as_mut().and_then(|a| a.alloc_page()) }
}

pub fn alloc_pages(count: usize) -> Option<usize> {
    unsafe { A.as_mut().and_then(|a| a.alloc_contig(count)) }
}

pub fn free_page(addr: usize) {
    unsafe { if let Some(ref mut a) = A { a.free_page(addr); } }
}

pub fn info() -> (usize, usize) {
    unsafe { A.as_ref().map(|a| (a.total(), a.free_count())).unwrap_or((0, 0)) }
}
