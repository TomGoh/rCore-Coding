# rCore-Coding Build Guide

This document explains the build system configuration and quality checks.

## Quick Start

```bash
# From the os directory
make build    # Build with clippy checks
make run      # Build and run in QEMU
make clean    # Clean build artifacts
```

## Quality Checks

The project has integrated quality checks that run automatically:

### 1. Code Formatting (rustfmt)

**Configuration:** [`rustfmt.toml`](rustfmt.toml) at project root

```toml
imports_granularity = "Preserve"
reorder_imports = true
edition = "2021"
newline_style = "Unix"
wrap_comments = false
comment_width = 100
```

**Check manually:**
```bash
# Check formatting
cd os && cargo fmt --all --check
cd user && cargo fmt --all --check

# Auto-fix formatting
cd os && cargo fmt --all
cd user && cargo fmt --all
```

### 2. Clippy Linting

**Integration:** Runs automatically during `make build`

**Check manually:**
```bash
cd os && cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings
cd user && cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings
```

**Note:** We use `--target` (not `--all-targets`) to exclude test targets which don't compile in no_std environments.

## VSCode Configuration

### OS Directory: [`os/.vscode/settings.json`](os/.vscode/settings.json)

- ✅ Auto-format on save
- ✅ Auto-organize imports on save
- ✅ Clippy integration (shows errors/warnings in real-time)
- ✅ Clippy runs with `-D warnings` (treats warnings as errors)

### User Directory: [`user/.vscode/settings.json`](user/.vscode/settings.json)

Same configuration as OS directory for consistency.

## Build Process

### Makefile Flow (OS)

```
make build
  └─> clippy check (cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings)
      └─> user build
          └─> clippy check (cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings)
          └─> cargo build --release
      └─> kernel build (cargo build --release)
```

### Makefile Flow (User)

```
make build
  └─> clippy check
      └─> cargo build --release
```

## CI/CD (GitHub Actions)

**Workflow:** [`.github/workflows/kernel-build.yml`](.github/workflows/kernel-build.yml)

### Jobs:

1. **lint** - Runs formatting and clippy checks
   - Check formatting: `cargo fmt --all --check`
   - Clippy lint: `cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings`

2. **build** - Builds the kernel
   - Runs: `make build`

3. **qemu** - Tests in QEMU
   - Runs: `make run` with timeout

### Triggering:
- Runs on every push to any branch
- Runs on every pull request

## Troubleshooting

### "error[E0463]: can't find crate for `test`"

**Cause:** Using `--all-targets` which includes test targets that don't compile in no_std.

**Solution:** Use `--target riscv64gc-unknown-none-elf` without `--all-targets`.

### "found crate compiled by an incompatible version of rustc"

**Cause:** Cached build artifacts from different rustc version.

**Solution:**
```bash
cd os && cargo clean
cd user && cargo clean
```

### Clippy warnings in CI but not locally

**Cause:** Different clippy versions or missing `rustfmt.toml`.

**Solution:** Ensure you have:
1. [`rustfmt.toml`](rustfmt.toml) at project root
2. Latest nightly toolchain: `rustup update nightly`

## Best Practices

1. **Before committing:**
   ```bash
   cd os && cargo fmt --all && cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings
   cd user && cargo fmt --all && cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings
   ```

2. **Let VSCode handle formatting:**
   - Save files to auto-format
   - Imports will be auto-organized

3. **Check CI status:**
   - GitHub Actions runs all checks
   - Fix any failures before merging

## Summary of Files

| File | Purpose |
|------|---------|
| `rustfmt.toml` | Project-wide formatting rules |
| `os/.vscode/settings.json` | VSCode settings for OS crate |
| `user/.vscode/settings.json` | VSCode settings for user crate |
| `os/Makefile` | Build rules with clippy integration |
| `user/Makefile` | Build rules with clippy integration |
| `.github/workflows/kernel-build.yml` | CI/CD pipeline |

## Testing the Complete Pipeline

```bash
# Clean everything
cd os && make clean
cd ../user && make clean

# Test formatting
cd ../os && cargo fmt --all --check
cd ../user && cargo fmt --all --check

# Test clippy
cd ../os && cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings
cd ../user && cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings

# Test build
cd ../os && make build

# All commands should succeed with no errors!
```
