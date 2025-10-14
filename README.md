# rCore-Coding

A RISC-V operating system kernel implementation based on rCore-Tutorial.

## Quick Start

```bash
# Clone the repository
git clone https://github.com/yourusername/rCore-Coding.git
cd rCore-Coding

# Setup git hooks (optional but recommended)
./setup-hooks.sh

# Build and run
cd os
make build    # Build the kernel
make run      # Run in QEMU
```

## Project Structure

```
rCore-Coding/
├── os/                     # Kernel source code
│   ├── src/               # Kernel implementation
│   ├── .vscode/           # VSCode settings
│   └── Makefile           # Build configuration
├── user/                   # User-space applications
│   ├── src/               # User programs
│   ├── .vscode/           # VSCode settings
│   └── Makefile           # Build configuration
├── .github/workflows/      # CI/CD configuration
├── rustfmt.toml           # Code formatting rules
└── BUILD_GUIDE.md         # Detailed build documentation
```

## Development Setup

### Prerequisites

- Rust nightly toolchain
- QEMU RISC-V emulator
- cargo-binutils

### Installation

```bash
# Install Rust nightly
rustup install nightly
rustup default nightly

# Add RISC-V target
rustup target add riscv64gc-unknown-none-elf

# Install components
rustup component add rust-src llvm-tools-preview rustfmt clippy

# Install cargo-binutils
cargo install cargo-binutils

# Install QEMU (Ubuntu/Debian)
sudo apt-get install qemu-system-misc
```

### VSCode Setup

The project includes VSCode settings that provide:

- ✅ **Auto-format on save** - Code is automatically formatted when you save
- ✅ **Auto-organize imports** - Imports are sorted alphabetically
- ✅ **Clippy integration** - Real-time linting with clippy
- ✅ **Target specification** - Automatically uses `riscv64gc-unknown-none-elf`

Open the `os/` or `user/` directory in VSCode to activate these settings.

## Quality Checks

The project enforces code quality through:

### 1. Formatting (rustfmt)

```bash
# Check formatting
cd os && cargo fmt --all --check
cd user && cargo fmt --all --check

# Auto-fix formatting
cd os && cargo fmt --all
cd user && cargo fmt --all
```

### 2. Linting (clippy)

```bash
# Run clippy
cd os && cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings
cd user && cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings
```

**Note:** Clippy checks are integrated into `make build` and run automatically.

## Building

```bash
cd os

# Build kernel and user apps
make build

# Build and run in QEMU
make run

# Clean build artifacts
make clean

# Disassemble kernel
make disasm
```

## Continuous Integration

GitHub Actions automatically runs on every push:

1. **Lint checks** - Formatting and clippy
2. **Build** - Compiles kernel and user apps
3. **QEMU test** - Runs in emulator

See [`.github/workflows/kernel-build.yml`](.github/workflows/kernel-build.yml) for details.

## Git Hooks

The project includes a pre-commit hook that runs quality checks before allowing commits.

### Setup

```bash
./setup-hooks.sh
```

This will install a hook that checks:
- Code formatting (rustfmt)
- Clippy lints

### Skip Hook (Not Recommended)

```bash
git commit --no-verify
```

## Documentation

- [`BUILD_GUIDE.md`](BUILD_GUIDE.md) - Detailed build system documentation
- [`os/src/`](os/src/) - Inline code documentation
- [rCore-Tutorial](https://rcore-os.cn/rCore-Tutorial-Book-v3/) - Official tutorial

## Common Issues

### "error[E0463]: can't find crate for `test`"

**Solution:** This happens with `--all-targets`. Use `--target riscv64gc-unknown-none-elf` instead.

### "found crate compiled by an incompatible version of rustc"

**Solution:** Clean and rebuild:
```bash
cd os && cargo clean
cd user && cargo clean
make build
```

### Build fails after pull

**Solution:** Ensure dependencies are up to date:
```bash
rustup update nightly
cd os && cargo update
cd user && cargo update
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Ensure all quality checks pass:
   ```bash
   cd os && cargo fmt --all && cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings
   cd user && cargo fmt --all && cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings
   ```
5. Submit a pull request

## License

[Your License Here]

## Acknowledgments

Based on [rCore-Tutorial](https://github.com/rcore-os/rCore-Tutorial-v3).
