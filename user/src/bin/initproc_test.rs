#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exec, fork, wait, yield_};

/// Test-mode initproc: runs usertests and shuts down based on result
#[unsafe(no_mangle)]
fn main() -> i32 {
    println!("[initproc_test] Starting test mode...");

    if fork() == 0 {
        // Child process runs usertests
        exec("usertests\0");
        panic!("[initproc_test] exec usertests failed!");
    } else {
        // Parent waits for test completion
        let mut exit_code: i32 = 0;
        loop {
            let pid = wait(&mut exit_code);
            if pid == -1 {
                yield_();
                continue;
            }

            println!(
                "[initproc_test] Test process {} exited with code {}",
                pid, exit_code
            );

            // Tests completed, return the exit code
            // This will cause initproc to exit, which signals the kernel
            return exit_code;
        }
    }
}
