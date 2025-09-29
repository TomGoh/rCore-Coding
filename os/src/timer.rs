use riscv::register::time;
use sbi_rt::set_timer;

use crate::config::CLOCK_FREQ;
const TICKS_PER_SEC: usize = 100;
const MSEC_PER_SEC: usize = 1_000_000;

pub fn get_time() -> usize {
    time::read()
}

pub fn get_time_ms() -> usize {
    get_time() / (CLOCK_FREQ / MSEC_PER_SEC)
}

pub fn set_next_trigger() {
    set_timer((get_time() + CLOCK_FREQ / TICKS_PER_SEC) as u64);
}