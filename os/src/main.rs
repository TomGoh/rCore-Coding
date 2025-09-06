#![no_main]
#![no_std]
#![cfg(target_arch = "riscv64")]
mod lang_items;

use core::arch::global_asm;
global_asm!(include_str!("entry.asm"));