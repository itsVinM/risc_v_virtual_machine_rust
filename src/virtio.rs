use crate::mmu::DRAM_BASE;

const MAGIC:       u64 = 0x000;
const VERSION:     u64 = 0x004;
const DEVICE_ID:   u64 = 0x008;
const VENDOR_ID:   u64 = 0x00C;
const DEVICE_FEATURES: u64 = 0x010;
const DEVICE_FEATURES_SEL: u64 = 0x014;
const DRIVER_FEATURES: u64 = 0x020;
const DRIVER_FEATURES_SEL: u64 = 0x024;
const QUEUE_SEL:   u64 = 0x030;
const QUEUE_NUM_MAX: u64 = 0x034;
const QUEUE_NUM:   u64 = 0x038;
const QUEUE_READY: u64 = 0x044;
const INTERRUPT_STATUS: u64 = 0x060;
const INTERRUPT_ACK: u64 = 0x064;
const STATUS:      u64 = 0x070;
const QUEUE_DESC_LOW: u64 = 0x080;
const QUEUE_DESC_HIGH: u64 = 0x084;
const QUEUE_DRIVER_LOW: u64 = 0x090;
const QUEUE_DRIVER_HIGH: u64 = 0x094;
const QUEUE_DEVICE_LOW: u64 = 0x0A0;
const QUEUE_DEVICE_HIGH: u64 = 0x0A4;

const VIRTIO_F_VERSION_1: u64 = 1 << 32;
const VRING_DESC_F_NEXT: u16 = 1;

pub struct Virtio {
    device_features_sel: u32,
    driver_features_sel: u32,
    driver_features: u64,
    queue_sel: u32,
    queue_num: u32,
    queue_ready: u32,
    status: u32,
    interrupt_status: u32,
    desc_addr: u64,
    avail_addr: u64,
    used_addr: u64,
    last_avail_idx: u16,
    pub disk: Vec<u8>,
    pub irq_pending: bool,
}

impl Virtio {
    pub fn new() -> Self {
        Self {
            device_features_sel: 0,
            driver_features_sel: 0,
            driver_features: 0,
            queue_sel: 0,
            queue_num: 0,
            queue_ready: 0,
            status: 0,
            interrupt_status: 0,
            desc_addr: 0,
            avail_addr: 0,
            used_addr: 0,
            last_avail_idx: 0,
            disk: vec![0u8; 64 * 1024 * 1024],
            irq_pending: false,
        }
    }

    pub fn read32(&self, offset: u64) -> u32 {
        match offset {
            MAGIC => 0x74726976,
            VERSION => 2,
            DEVICE_ID => 2,
            VENDOR_ID => 0x554d4551,
            DEVICE_FEATURES => {
                if self.device_features_sel == 0 {
                    VIRTIO_F_VERSION_1 as u32
                } else if self.device_features_sel == 1 {
                    (VIRTIO_F_VERSION_1 >> 32) as u32
                } else { 0 }
            }
            QUEUE_NUM_MAX => 8,
            QUEUE_READY => self.queue_ready,
            INTERRUPT_STATUS => self.interrupt_status,
            STATUS => self.status,
            _ => 0,
        }
    }

    pub fn write32(&mut self, offset: u64, val: u32) {
        match offset {
            DEVICE_FEATURES_SEL => self.device_features_sel = val,
            DRIVER_FEATURES => {
                if self.driver_features_sel == 0 {
                    self.driver_features = (self.driver_features & 0xFFFFFFFF_00000000) | val as u64;
                } else {
                    self.driver_features = (self.driver_features & 0x00000000_FFFFFFFF) | (val as u64) << 32;
                }
            }
            DRIVER_FEATURES_SEL => self.driver_features_sel = val,
            QUEUE_SEL => self.queue_sel = val,
            QUEUE_NUM => self.queue_num = val,
            QUEUE_READY => self.queue_ready = val,
            INTERRUPT_ACK => self.interrupt_status &= !val,
            STATUS => self.status = val,
            QUEUE_DESC_LOW  => self.desc_addr  = (self.desc_addr  & 0xFFFFFFFF_00000000) | val as u64,
            QUEUE_DESC_HIGH => self.desc_addr  = (self.desc_addr  & 0x00000000_FFFFFFFF) | (val as u64) << 32,
            QUEUE_DRIVER_LOW  => self.avail_addr = (self.avail_addr & 0xFFFFFFFF_00000000) | val as u64,
            QUEUE_DRIVER_HIGH => self.avail_addr = (self.avail_addr & 0x00000000_FFFFFFFF) | (val as u64) << 32,
            QUEUE_DEVICE_LOW  => self.used_addr  = (self.used_addr  & 0xFFFFFFFF_00000000) | val as u64,
            QUEUE_DEVICE_HIGH => self.used_addr  = (self.used_addr  & 0x00000000_FFFFFFFF) | (val as u64) << 32,
            _ => {}
        }
    }

    pub fn handle_notify(&mut self, dram: &mut [u8]) {
        let num = self.queue_num.min(8) as u16;
        if num == 0 || self.desc_addr == 0 || self.avail_addr == 0 || self.used_addr == 0 { return; }

        let avail_off = pa_to_off(self.avail_addr);
        let avail_idx = read_u16_off(dram, avail_off.wrapping_add(2));
        while self.last_avail_idx != avail_idx {
            let head = read_u16_off(dram, avail_off.wrapping_add(4 + self.last_avail_idx as usize * 2));
            self.process_chain(head, dram);
            self.last_avail_idx = self.last_avail_idx.wrapping_add(1);
        }
        self.irq_pending = true;
        self.interrupt_status |= 1;
    }

    fn process_chain(&mut self, head: u16, dram: &mut [u8]) {
        let mut idx = head;
        let mut desc_addr = [0u64; 3];
        let mut desc_len = [0u32; 3];
        let mut desc_flags = [0u16; 3];
        let mut count = 0u32;

        loop {
            let base = pa_to_off(self.desc_addr.wrapping_add(idx as u64 * 16));
            desc_addr[count as usize] = u64::from_le_bytes(dram[base..base + 8].try_into().unwrap_or([0; 8]));
            desc_len[count as usize] = u32::from_le_bytes(dram[base + 8..base + 12].try_into().unwrap_or([0; 4]));
            desc_flags[count as usize] = u16::from_le_bytes(dram[base + 12..base + 14].try_into().unwrap_or([0; 2]));
            idx = u16::from_le_bytes(dram[base + 14..base + 16].try_into().unwrap_or([0; 2]));
            count += 1;
            if desc_flags[count as usize - 1] & VRING_DESC_F_NEXT == 0 { break; }
            if count >= 3 { break; }
        }
        if count < 2 { return; }

        let hdr_off = pa_to_off(desc_addr[0]);
        let blk_type = u32::from_le_bytes(dram[hdr_off..hdr_off + 4].try_into().unwrap_or([0; 4]));
        let sector = u64::from_le_bytes(dram[hdr_off + 8..hdr_off + 16].try_into().unwrap_or([0; 8]));

        let data_off = pa_to_off(desc_addr[1]);
        let data_len = desc_len[1] as usize;

        let disk_off = (sector * 512) as usize;
        if blk_type == 0 {
            let end = (disk_off + data_len).min(self.disk.len());
            let src = &self.disk[disk_off..end];
            let dst = &mut dram[data_off..data_off + src.len()];
            dst.copy_from_slice(src);
        } else {
            let end = (disk_off + data_len).min(self.disk.len());
            let src = &dram[data_off..data_off + data_len.min(end - disk_off)];
            self.disk[disk_off..disk_off + src.len()].copy_from_slice(src);
        }

        if count >= 3 {
            let st_off = pa_to_off(desc_addr[2]);
            if st_off < dram.len() { dram[st_off] = 0; }
        }

        let used_off = pa_to_off(self.used_addr);
        let used_idx = read_u16_off(dram, used_off.wrapping_add(2));
        dram[used_off.wrapping_add(4)..used_off.wrapping_add(8)].copy_from_slice(&(head as u32).to_le_bytes());
        dram[used_off.wrapping_add(8)..used_off.wrapping_add(12)].copy_from_slice(&(data_len as u32).to_le_bytes());
        dram[used_off.wrapping_add(2)..used_off.wrapping_add(4)]
            .copy_from_slice(&used_idx.wrapping_add(1).to_le_bytes());
    }
}

fn pa_to_off(pa: u64) -> usize {
    (pa - DRAM_BASE) as usize
}

fn read_u16_off(dram: &[u8], off: usize) -> u16 {
    u16::from_le_bytes(dram[off..off + 2].try_into().unwrap_or([0; 2]))
}
