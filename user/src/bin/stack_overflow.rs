#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

#[allow(unconditional_recursion)]
fn f(depth: usize) {
    if depth.is_multiple_of(10) {
        println!("depth = {}", depth);
    }
    f(depth + 1);
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("It should trigger segmentation fault!");
    f(0);
    0
}
