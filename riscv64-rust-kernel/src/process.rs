pub struct Process {
    pub id: u64,
}

static mut NEXT_PID: u64 = 1;

pub fn next_pid() -> u64 {
    unsafe {
        let pid = NEXT_PID;
        NEXT_PID += 1;
        pid
    }
}
