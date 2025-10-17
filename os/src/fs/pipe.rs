use crate::{fs::File, sync::UPSafeCell, syscall, task::suspend_current_and_run_next};
use alloc::sync::{Arc, Weak};
use riscv::register::hgeie::read;

const RING_BUFFER_SIZE: usize = 32;

pub struct Pipe {
    readable: bool,
    writable: bool,
    buffer: Arc<UPSafeCell<PipeRingBuffer>>,
}

#[derive(Copy, Clone, PartialEq)]
enum RingBufferStatus {
    Full,
    Empty,
    Normal,
}

pub struct PipeRingBuffer {
    arr: [u8; RING_BUFFER_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
    write_end: Option<Weak<Pipe>>,
}

impl Pipe {
    pub fn read_end_with_buffer(buffer: Arc<UPSafeCell<PipeRingBuffer>>) -> Self {
        Self {
            readable: true,
            writable: false,
            buffer,
        }
    }

    pub fn write_end_with_buffer(buffer: Arc<UPSafeCell<PipeRingBuffer>>) -> Self {
        Self {
            readable: false,
            writable: true,
            buffer,
        }
    }
}

impl PipeRingBuffer {
    pub fn new() -> Self {
        Self {
            arr: [0; RING_BUFFER_SIZE],
            head: 0,
            tail: 0,
            status: RingBufferStatus::Empty,
            write_end: None,
        }
    }

    pub fn set_write_end(&mut self, write_end: &Arc<Pipe>) {
        self.write_end = Some(Arc::downgrade(write_end));
    }

    pub fn write_byte(&mut self, byte: u8) {
        self.status = RingBufferStatus::Normal;
        self.arr[self.tail] = byte;
        self.tail = (self.tail + 1) % RING_BUFFER_SIZE;
        if self.tail == self.head {
            self.status = RingBufferStatus::Full;
        }
    }

    pub fn read_byte(&mut self) -> u8 {
        self.status = RingBufferStatus::Normal;
        let byte = self.arr[self.head];
        self.head = (self.head + 1) % RING_BUFFER_SIZE;
        if self.head == self.tail {
            self.status = RingBufferStatus::Empty;
        }
        byte
    }

    pub fn available_read(&self) -> usize {
        if self.status == RingBufferStatus::Empty {
            0
        } else if self.tail > self.head {
            self.tail - self.head
        } else {
            RING_BUFFER_SIZE - (self.head - self.tail)
        }
    }

    pub fn available_write(&self) -> usize {
        if self.status == RingBufferStatus::Full {
            0
        } else if self.tail >= self.head {
            RING_BUFFER_SIZE - (self.tail - self.head) - 1
        } else {
            self.head - self.tail - 1
        }
    }

    pub fn all_write_ends_closed(&self) -> bool {
        self.write_end.as_ref().unwrap().upgrade().is_none()
    }
}
impl File for Pipe {
    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }

    fn read(&self, buf: crate::mm::UserBuffer) -> usize {
        assert!(self.readable);

        let read_len_requested = buf.len();
        let mut buf_iter = buf.into_iter();
        let mut read_len = 0usize;

        loop {
            let mut ring_buffer = self.buffer.exclusive_access();
            let available_read = ring_buffer.available_read();
            if available_read == 0 {
                if ring_buffer.all_write_ends_closed() {
                    return read_len;
                }
                drop(ring_buffer);
                suspend_current_and_run_next();
                continue;
            }
            for _ in 0..available_read {
                if let Some(byte_ref) = buf_iter.next() {
                    unsafe {
                        *byte_ref = ring_buffer.read_byte();
                    }
                    read_len += 1;
                    if read_len == read_len_requested {
                        return read_len_requested;
                    }
                } else {
                    return read_len;
                }
            }
        }
    }

    fn write(&self, buf: crate::mm::UserBuffer) -> usize {
        assert!(self.writable);

        let written_len_requested = buf.len();
        let mut buf_iter = buf.into_iter();
        let mut written_len = 0usize;

        loop {
            let mut ring_buffer = self.buffer.exclusive_access();
            let written_space_remaining = ring_buffer.available_write();
            if written_space_remaining == 0 {
                drop(ring_buffer);
                suspend_current_and_run_next();
                continue;
            }

            for _ in 0..written_space_remaining {
                if let Some(byte_ref) = buf_iter.next() {
                    ring_buffer.write_byte(unsafe { *byte_ref });
                    written_len += 1;
                    if written_len == written_len_requested {
                        return written_len_requested;
                    }
                } else {
                    return written_len;
                }
            }
        }
    }
}

pub fn make_pipe() -> (Arc<Pipe>, Arc<Pipe>) {
    let buffer = Arc::new(unsafe { UPSafeCell::new(PipeRingBuffer::new()) });
    let read_end = Arc::new(Pipe::read_end_with_buffer(buffer.clone()));
    let write_end = Arc::new(Pipe::write_end_with_buffer(buffer.clone()));
    buffer.exclusive_access().set_write_end(&write_end);

    (read_end, write_end)
}
