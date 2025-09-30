use bitflags::*;

use crate::mm::address::PhysPageNum;

bitflags! {
    #[derive(PartialEq)]
    pub struct PTEFlags: usize {
        const V = 1 << 0; // valid
        const R = 1 << 1; // readable
        const W = 1 << 2; // writable
        const X = 1 << 3; // executable
        const U = 1 << 4; // user
        const G = 1 << 5; // global
        const A = 1 << 6; // accessed
        const D = 1 << 7; // dirty
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        Self {
            bits: (ppn.0 << 10) | flags.bits(),
        }
    }

    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }

    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits & 0x3ff).unwrap()
    }

    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }

    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }

    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}