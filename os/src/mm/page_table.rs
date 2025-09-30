use bitflags::*;
use alloc::vec;
use alloc::vec::Vec;
use crate::mm::{address::{PhysPageNum, VirtPageNum}, frame_allocator::{frame_alloc, FrameTracker}};

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

pub struct PageTable {
    root_pfn: PhysPageNum,
    frames: Vec<FrameTracker>,
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

impl PageTable {
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        Self {
            root_pfn: frame.ppn,
            frames: vec![frame],
        }
    }

    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let indecies = vpn.indecies();
        let mut curr_table_root_ppn = self.root_pfn;
        let mut ans: Option<&mut PageTableEntry> = None;

        for i in 0..3 {
            let curr_pte = &mut curr_table_root_ppn.get_pte_array()[indecies[i]];
            if i == 2 {
                ans = Some(curr_pte);
                break;
            }
            if !curr_pte.is_valid() {
                let new_frame = frame_alloc()?;
                *curr_pte = PageTableEntry::new(new_frame.ppn, PTEFlags::V);
                self.frames.push(new_frame);
            }
            curr_table_root_ppn = curr_pte.ppn();
        }

        ans
    }

    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let indecies = vpn.indecies();
        let mut curr_table_root_ppn = self.root_pfn;
        let mut ans: Option<&mut PageTableEntry> = None;

        for i in 0..3 {
            let curr_pte = &mut curr_table_root_ppn.get_pte_array()[indecies[i]];
            if i == 2 {
                ans = Some(curr_pte);
                break;
            }
            if !curr_pte.is_valid() {
                return None;
            }
            curr_table_root_ppn = curr_pte.ppn();
        }

        ans
    }

    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    pub fn unmap(&mut self, vpn: VirtPageNum){
        if let Some(pte) = self.find_pte(vpn) {
            assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
            *pte = PageTableEntry::empty();
        } else {
            panic!("vpn {:?} is invalid before unmapping", vpn);
        }
    }

    pub fn from_token(satp: usize) -> Self {
        Self {
            root_pfn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map( |pte| {
            *pte
        })
    }
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_pfn.0
    }
}