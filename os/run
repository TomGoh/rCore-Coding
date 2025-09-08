#!/bin/bash

set -e

echo "=== Cleaning previous build ==="
make clean

echo "=== Building OS binary ==="
make release

echo "=== Running with QEMU ==="
echo "Press Ctrl+A then X to exit QEMU"
qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios ../bootloader/rustsbi-qemu.bin \
    -device loader,file=target/riscv64gc-unknown-none-elf/release/os.bin,addr=0x80200000