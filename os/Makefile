# RISC-V Kernel Build Configuration
TARGET := riscv64gc-unknown-none-elf
MODE := debug
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := target/$(TARGET)/$(MODE)/os.bin

# Build the kernel ELF and create binary
build: $(KERNEL_BIN)

# Build the ELF file
$(KERNEL_ELF):
	@echo "Building kernel ELF..."
	@cargo build

# Create binary from ELF
$(KERNEL_BIN): $(KERNEL_ELF)
	@echo "Creating binary file..."
	@rust-objcopy --strip-all $(KERNEL_ELF) -O binary $(KERNEL_BIN)
	@echo "Binary created: $(KERNEL_BIN)"

# Release build
release:
	@echo "Building release kernel..."
	@cargo build --release
	@rust-objcopy --strip-all target/$(TARGET)/release/os -O binary target/$(TARGET)/release/os.bin
	@echo "Release binary created: target/$(TARGET)/release/os.bin"

# Clean build artifacts
clean:
	@cargo clean

# Run (for future use with emulator)
run: build
	@echo "Binary ready for deployment: $(KERNEL_BIN)"

.PHONY: build release clean run