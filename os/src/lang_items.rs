use core::panic::PanicInfo;
use crate::{println, sbi::shutdown};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "PANIC in file '{}' at line {}: {}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println!("PANIC: {}", info.message());
    }
    shutdown(true)
} 