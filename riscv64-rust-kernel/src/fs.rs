use crate::println;
use crate::virtio_blk;

const BLOCK: usize = 512;
const MAX_FILES: usize = 32;
const NAMELEN: usize = 28;
const TOTAL_SECTORS: u64 = 131072;
const BITMAP_WORDS: usize = (TOTAL_SECTORS as usize + 63) / 64;

struct Inode {
    name: [u8; NAMELEN],
    size: usize,
    start: u64,
    used: bool,
}

pub struct SysFs {
    inodes: [Inode; MAX_FILES],
    sb: [u64; BITMAP_WORDS],
}

impl SysFs {
    pub fn new() -> Self {
        let mut fs = Self {
            inodes: core::array::from_fn(|_| Inode { name: [0; NAMELEN], size: 0, start: 0, used: false }),
            sb: [0u64; BITMAP_WORDS],
        };
        // Mark sector 0 as allocated (boot block / MBR)
        fs.sb[0] = 1;
        // Find and load existing bitmap from disk if valid
        fs.load_bitmap();
        fs.inodes[0] = Inode {
            name: { let mut n = [0; NAMELEN]; n[0] = b'/'; n },
            size: 0, start: 0, used: true,
        };
        fs
    }

    // Stub: could read a superblock from disk to restore bitmap
    fn load_bitmap(&mut self) {
        // For now, assume clean filesystem
        // Mark first 64 sectors as used (reserved area for bitmap/inodes)
        self.sb[0] = !0u64 >> (64 - 64);
    }

    fn alloc_sectors(&mut self, count: usize) -> Option<u64> {
        let mut run = 0u64;
        let mut start = 0u64;
        for i in 64..TOTAL_SECTORS {
            let word = i as usize / 64;
            let bit = i as u64 % 64;
            let taken = (self.sb[word] >> bit) & 1;
            if taken == 0 {
                if run == 0 { start = i; }
                run += 1;
                if run >= count as u64 {
                    for j in 0..count as u64 {
                        let w = (start + j) as usize / 64;
                        let b = (start + j) as u64 % 64;
                        self.sb[w] |= 1 << b;
                    }
                    return Some(start);
                }
            } else {
                run = 0;
            }
        }
        None
    }

    fn free_sectors(&mut self, start: u64, count: usize) {
        for i in 0..count as u64 {
            let w = (start + i) as usize / 64;
            let b = (start + i) as u64 % 64;
            self.sb[w] &= !(1 << b);
        }
    }

    fn find_inode(&self, name: &str) -> Option<usize> {
        let b = name.as_bytes();
        self.inodes.iter().position(|inode| {
            inode.used && &inode.name[..b.len().min(NAMELEN)] == b
        })
    }

    pub fn create(&mut self, name: &str, size: usize) -> bool {
        if self.find_inode(name).is_some() { return false; }
        let slot = match self.inodes.iter_mut().find(|i| !i.used) { Some(s) => s, None => return false };
        let nlen = name.len().min(NAMELEN - 1);
        let mut buf = [0u8; NAMELEN];
        buf[..nlen].copy_from_slice(&name.as_bytes()[..nlen]);
        *slot = Inode { name: buf, size, start: 0, used: true };
        true
    }

    pub fn write(&mut self, name: &str, data: &[u8]) -> bool {
        let idx = match self.find_inode(name) { Some(i) => i, None => return false };
        let sectors = (data.len() + BLOCK - 1) / BLOCK;
        let start = match self.alloc_sectors(sectors) { Some(s) => s, None => return false };
        let inode = &mut self.inodes[idx];
        inode.start = start;
        inode.size = data.len();
        let mut buf = [0u8; BLOCK];
        for i in 0..sectors {
            let off = i * BLOCK;
            let len = (data.len() - off).min(BLOCK);
            buf[..len].copy_from_slice(&data[off..off + len]);
            if len < BLOCK { buf[len..].fill(0); }
            if !virtio_blk::write_sector(start + i as u64, &buf) { return false; }
        }
        true
    }

    pub fn read(&self, name: &str, buf: &mut [u8]) -> Option<usize> {
        let inode = self.inodes.iter().find(|i| {
            i.used && name.as_bytes() == &i.name[..name.len().min(NAMELEN)]
        })?;
        let sectors = (inode.size + BLOCK - 1) / BLOCK;
        let mut total = 0usize;
        let mut sb = [0u8; BLOCK];
        for i in 0..sectors {
            if !virtio_blk::read_sector(inode.start + i as u64, &mut sb) { return None; }
            let cl = (inode.size - total).min(BLOCK).min(buf.len() - total);
            buf[total..total + cl].copy_from_slice(&sb[..cl]);
            total += cl;
            if total >= buf.len() { break; }
        }
        Some(total)
    }

    pub fn ls(&self) {
        for inode in self.inodes.iter() {
            if !inode.used { continue; }
            let end = inode.name.iter().position(|&c| c == 0).unwrap_or(NAMELEN);
            let s = core::str::from_utf8(&inode.name[..end]).unwrap_or("?");
            println!("  {} ({}b)", s, inode.size);
        }
    }
}

static mut FS: Option<SysFs> = None;

pub fn init() {
    unsafe { FS = Some(SysFs::new()); }
    println!("  filesystem ready");
}

pub fn write_file(name: &str, data: &[u8]) -> bool {
    unsafe { FS.as_mut().map(|fs| fs.create(name, data.len()) && fs.write(name, data)).unwrap_or(false) }
}

pub fn read_file(name: &str, buf: &mut [u8]) -> Option<usize> {
    unsafe { FS.as_ref().and_then(|fs| fs.read(name, buf)) }
}

pub fn ls() {
    unsafe { FS.as_ref().map(|fs| fs.ls()); }
}
